import { API_URL } from './config/api';

const API_BASE_URL = `${API_URL}/v1`;
const DATA_API_URL = process.env.DATA_API_URL || 'http://localhost:8080';

import type {
  Bot,
  BillingSummary,
  BotChatMessage,
  BotConfig,
  BotEvent,
  CreateBotRequest,
  DocsCategory,
  EmailCsvReportRequest,
  EmailCsvReportResponse,
  GetDocsResponse,
  NameAvailability,
  GetBotChatMessagesResponse,
  TrackDocsEventRequest,
  TrackDocsEventResponse,
  UpdateUserSettingsRequest,
  UpdateBotConfigRequest,
  ListBotsResponse,
  GetBotResponse,
  GetMetricsResponse,
  GetEventsResponse,
  PostBotChatMessageRequest,
  PostBotChatMessageResponse,
  BotActionRequest,
  User,
  UserSettings,
  AIAssistantOption,
  TradeableAsset,
  BotStatus,
  BotChatRole,
  Persona,
  AssetFocus,
  AlgorithmMode,
  Strictness,
  TradingMode,
  LlmProvider,
  BotEventType,
} from '@trawling-traders/types';

// Generic API error
export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
    public data?: any
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

// Auth session expired error - caller should redirect to login
export class AuthExpiredError extends ApiError {
  constructor(message: string = 'Session expired. Please log in again.') {
    super(401, message);
    this.name = 'AuthExpiredError';
  }
}

// Pluggable auth provider (configured by host app)
export type AuthTokenProvider = () => Promise<string | null>;
export type TokenRefreshFn = () => Promise<string | null>;
export type ClearAuthFn = () => Promise<void>;

interface AuthConfig {
  getToken: AuthTokenProvider;
  refreshToken?: TokenRefreshFn;
  clearAuth?: ClearAuthFn;
}

let authConfig: AuthConfig | null = null;

export function setAuthProvider(config: AuthConfig): void {
  authConfig = config;
}

async function getAuthToken(): Promise<string | null> {
  if (!authConfig?.getToken) return null;
  return authConfig.getToken();
}

async function refreshAuthToken(): Promise<string | null> {
  if (!authConfig?.refreshToken) return null;
  return authConfig.refreshToken();
}

async function clearAuthState(): Promise<void> {
  if (!authConfig?.clearAuth) return;
  await authConfig.clearAuth();
}

// Default timeout for API requests (30 seconds)
const DEFAULT_TIMEOUT_MS = 30000;

// Retry configuration
const MAX_RETRIES = 3;
const INITIAL_RETRY_DELAY_MS = 1000;

// Timeout error for distinguishing from other errors
export class TimeoutError extends ApiError {
  constructor(message: string = 'Request timed out') {
    super(0, message);
    this.name = 'TimeoutError';
  }
}

// Network error for connection failures (offline, DNS, etc)
export class NetworkError extends ApiError {
  constructor(message: string = 'Network error. Please check your connection.') {
    super(0, message);
    this.name = 'NetworkError';
  }
}

// Rate limit error for 429 responses
export class RateLimitError extends ApiError {
  constructor(public retryAfter?: number) {
    super(429, 'Too many requests. Please try again later.');
    this.name = 'RateLimitError';
  }
}

// Server error for 5xx responses
export class ServerError extends ApiError {
  constructor(status: number, message: string = 'Server error. Please try again.') {
    super(status, message);
    this.name = 'ServerError';
  }
}

// Forbidden error for 403 responses
export class ForbiddenError extends ApiError {
  constructor(message: string = 'Access denied.') {
    super(403, message);
    this.name = 'ForbiddenError';
  }
}

// Helper to sleep for exponential backoff
function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

// Check if error is retryable (5xx or network error)
function isRetryableError(status: number): boolean {
  return status >= 500 && status < 600;
}

// HTTP client with auth, automatic token refresh, timeout, and retry
async function fetchApi(
  endpoint: string,
  options: RequestInit = {},
  isAuthRetry: boolean = false,
  retryCount: number = 0
): Promise<any> {
  const url = `${API_BASE_URL}${endpoint}`;

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...((options.headers as Record<string, string>) || {}),
  };

  // Add auth token from Cedros session
  const token = await getAuthToken();
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  // Create AbortController for timeout
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT_MS);

  let response: Response;
  try {
    response = await fetch(url, {
      ...options,
      headers,
      signal: controller.signal,
    });
  } catch (error: any) {
    clearTimeout(timeoutId);
    if (error.name === 'AbortError') {
      throw new TimeoutError(`Request to ${endpoint} timed out after ${DEFAULT_TIMEOUT_MS}ms`);
    }
    // Retry network errors
    if (retryCount < MAX_RETRIES) {
      const delay = INITIAL_RETRY_DELAY_MS * Math.pow(2, retryCount);
      await sleep(delay);
      return fetchApi(endpoint, options, isAuthRetry, retryCount + 1);
    }
    // Wrap in NetworkError for better error differentiation
    throw new NetworkError(error.message || 'Network request failed');
  } finally {
    clearTimeout(timeoutId);
  }

  // Handle 401 Unauthorized - attempt token refresh
  if (response.status === 401 && !isAuthRetry) {
    const newToken = await refreshAuthToken();
    if (newToken) {
      // Retry request with new token
      return fetchApi(endpoint, options, true, 0);
    } else {
      // Refresh failed - clear auth state and throw
      await clearAuthState();
      throw new AuthExpiredError();
    }
  }

  // Retry 5xx errors with exponential backoff
  if (isRetryableError(response.status) && retryCount < MAX_RETRIES) {
    const delay = INITIAL_RETRY_DELAY_MS * Math.pow(2, retryCount);
    await sleep(delay);
    return fetchApi(endpoint, options, isAuthRetry, retryCount + 1);
  }

  if (!response.ok) {
    const error = await response.text();

    // Differentiate error types for better client-side handling
    switch (response.status) {
      case 401:
        throw new AuthExpiredError(error || 'Authentication required');
      case 403:
        throw new ForbiddenError(error || 'Access denied');
      case 429:
        const retryAfter = parseInt(response.headers.get('retry-after') || '60', 10);
        throw new RateLimitError(retryAfter);
      default:
        if (response.status >= 500) {
          throw new ServerError(response.status, error || 'Server error');
        }
        throw new ApiError(response.status, error || 'API request failed');
    }
  }

  return response.json();
}

// Bot API
export const botApi = {
  // List all bots for current user
  async listBots(): Promise<ListBotsResponse> {
    const response = await fetchApi('/bots');
    return {
      total: response.total || (response.bots || []).length,
      bots: (response.bots || []).map(mapBot),
    };
  },

  async checkNameAvailability(name: string): Promise<NameAvailability> {
    const response = await fetchApi(`/bots/name-availability?name=${encodeURIComponent(name)}`);
    return {
      available: Boolean(response.available),
      normalizedName: response.normalizedName ?? response.normalized_name ?? name.trim(),
      suggestedName: response.suggestedName ?? response.suggested_name ?? undefined,
    };
  },

  async listTradeableAssets(): Promise<TradeableAsset[]> {
    const response = await fetchApi('/bots/tradeable-assets');
    return (response.assets || []).map((asset: RawTradeableAsset) => ({
      id: asset.id,
      assetFocus: asset.assetFocus ?? asset.asset_focus,
      symbol: asset.symbol,
      name: asset.name,
      tokenAddress: asset.tokenAddress ?? asset.token_address,
      decimals: Number(asset.decimals),
      custodian: asset.custodian,
      isActive: Boolean(asset.isActive ?? asset.is_active),
      createdAt: asset.createdAt ?? asset.created_at,
      updatedAt: asset.updatedAt ?? asset.updated_at,
    }));
  },

  async listAssistantOptions(): Promise<AIAssistantOption[]> {
    const response = await fetchApi('/bots/assistant-options');
    return (response.options || []).map((option: RawAssistantOption) => ({
      id: option.id,
      assistantStyle: option.assistantStyle ?? option.assistant_style,
      captainName: option.captainName ?? option.captain_name,
      personalityDescription:
        option.personalityDescription ?? option.personality_description,
      imageKey: option.imageKey ?? option.image_key,
      imagePath: option.imagePath ?? option.image_path,
      sortOrder: Number(option.sortOrder ?? option.sort_order ?? 0),
      isActive: Boolean(option.isActive ?? option.is_active),
    }));
  },

  // Create a new bot
  async createBot(request: CreateBotRequest): Promise<Bot> {
    return fetchApi('/bots', {
      method: 'POST',
      body: JSON.stringify({
        name: request.name,
        assistant_style: request.assistantStyle,
        asset_focus: request.assetFocus,
        custom_assets: request.customAssets,
        algorithm_mode: request.algorithmMode,
        algorithm_factors: request.algorithmFactors,
        strictness: request.strictness,
        risk_caps: {
          max_position_size_percent: request.riskCaps.maxPositionSizePercent,
          max_daily_loss_usd: request.riskCaps.maxDailyLossUsd,
          max_drawdown_percent: request.riskCaps.maxDrawdownPercent,
          max_trades_per_day: request.riskCaps.maxTradesPerDay,
        },
        trading_mode: request.tradingMode,
        llm_provider: request.llmProvider,
        llm_model: request.llmModel,
        llm_api_key: request.llmApiKey,
        telegram_enabled: request.telegramEnabled,
        telegram_bot_token: request.telegramBotToken,
        telegram_user_id: request.telegramUserId,
        telegram_pairing_code: request.telegramPairingCode,
      }),
    });
  },

  // Get bot details with config
  async getBot(botId: string): Promise<GetBotResponse> {
    const response = await fetchApi(`/bots/${botId}`);
    return {
      bot: mapBot(response.bot),
      config: mapBotConfig(response.config),
    };
  },

  // Update bot config
  async updateBotConfig(
    botId: string,
    request: UpdateBotConfigRequest
  ): Promise<BotConfig> {
    return fetchApi(`/bots/${botId}/config`, {
      method: 'PATCH',
      body: JSON.stringify(request),
    });
  },

  // Perform action on bot (pause/resume/redeploy/destroy)
  async botAction(
    botId: string,
    action: BotActionRequest['action']
  ): Promise<void> {
    return fetchApi(`/bots/${botId}/actions`, {
      method: 'POST',
      body: JSON.stringify({ action }),
    });
  },

  // Get bot metrics
  async getMetrics(botId: string): Promise<GetMetricsResponse> {
    const response = await fetchApi(`/bots/${botId}/metrics`);
    return {
      range: response.range || '7d',
      metrics: (response.metrics || []).map((metric: RawMetricPoint) => ({
        timestamp: metric.timestamp,
        value: Number(metric.value ?? metric.pnl ?? 0),
      })),
    };
  },

  // Get bot events
  async getEvents(botId: string): Promise<GetEventsResponse> {
    const response = await fetchApi(`/bots/${botId}/events`);
    return {
      nextCursor: response.nextCursor ?? response.next_cursor,
      events: (response.events || []).map((event: RawBotEvent) => ({
        id: event.id,
        botId: event.botId ?? event.bot_id ?? '',
        type: (event.type ?? event.event_type ?? 'status_change') as BotEventType,
        timestamp: event.timestamp ?? event.created_at ?? '',
        message: event.message ?? '',
        metadata: event.metadata,
      })),
    };
  },

  async getChatMessages(botId: string): Promise<GetBotChatMessagesResponse> {
    const response = await fetchApi(`/bots/${botId}/chat/messages`);
    return {
      messages: (response.messages || []).map(mapChatMessage),
    };
  },

  async postChatMessage(
    botId: string,
    request: PostBotChatMessageRequest
  ): Promise<PostBotChatMessageResponse> {
    const response = await fetchApi(`/bots/${botId}/chat/messages`, {
      method: 'POST',
      body: JSON.stringify(request),
    });
    return {
      userMessage: mapChatMessage(response.user_message),
      assistantMessage: mapChatMessage(response.assistant_message),
    };
  },
};

// Raw API response types — accept both camelCase and snake_case from backend
interface RawChatMessage {
  id: string;
  botId?: string; bot_id?: string;
  role: string;
  content: string;
  timestamp?: string; created_at?: string;
}

interface RawBot {
  id: string;
  userId?: string; user_id?: string;
  name: string;
  status: string;
  assistantStyle?: string; assistant_style?: string;
  dropletId?: string; droplet_id?: string;
  region?: string;
  ipAddress?: string; ip_address?: string;
  agentWallet?: string; agent_wallet?: string;
  desiredVersionId?: string; desired_version_id?: string;
  appliedVersionId?: string; applied_version_id?: string;
  configStatus?: string; config_status?: string;
  createdAt?: string; created_at?: string;
  updatedAt?: string; updated_at?: string;
  lastHeartbeatAt?: string; last_heartbeat_at?: string;
  todayPnl?: number; today_pnl?: number;
  totalPnl?: number; total_pnl?: number;
}

interface RawBotConfig {
  id: string;
  botId?: string; bot_id?: string;
  version: number;
  createdAt?: string; created_at?: string;
  name?: string;
  assistantStyle?: string; assistant_style?: string;
  iconColor?: string; icon_color?: string;
  assetFocus?: string; asset_focus?: string;
  customAssets?: string[]; custom_assets?: string[];
  algorithmMode?: string; algorithm_mode?: string;
  strictness?: string;
  signalKnobs?: Record<string, unknown>; signal_knobs?: Record<string, unknown>;
  riskCaps?: Record<string, unknown>; risk_caps?: Record<string, unknown>;
  tradingMode?: string; trading_mode?: string;
  llmProvider?: string; llm_provider?: string;
  llmModel?: string; llm_model?: string;
  llmApiKey?: string; llm_api_key?: string;
}

interface RawTradeableAsset {
  id: string;
  assetFocus?: string; asset_focus?: string;
  symbol: string;
  name: string;
  tokenAddress?: string; token_address?: string;
  decimals: number;
  custodian?: string;
  isActive?: boolean; is_active?: boolean;
  createdAt?: string; created_at?: string;
  updatedAt?: string; updated_at?: string;
}

interface RawAssistantOption {
  id: string;
  assistantStyle?: string; assistant_style?: string;
  captainName?: string; captain_name?: string;
  personalityDescription?: string; personality_description?: string;
  imageKey?: string; image_key?: string;
  imagePath?: string; image_path?: string;
  sortOrder?: number; sort_order?: number;
  isActive?: boolean; is_active?: boolean;
}

interface RawMetricPoint {
  timestamp: string;
  value?: number;
  pnl?: number;
}

interface RawBotEvent {
  id: string;
  botId?: string; bot_id?: string;
  type?: string; event_type?: string;
  timestamp?: string; created_at?: string;
  message?: string;
  metadata?: Record<string, unknown>;
}

interface RawDocsArticle {
  id: string;
  title: string;
  summary?: string;
  content?: unknown[];
}

interface RawDocsCategory {
  id: string;
  title: string;
  description?: string;
  articles?: RawDocsArticle[];
}

function mapChatMessage(raw: RawChatMessage): BotChatMessage {
  return {
    id: raw.id,
    botId: raw.botId ?? raw.bot_id ?? '',
    role: raw.role as BotChatRole,
    content: raw.content,
    timestamp: raw.timestamp ?? raw.created_at ?? '',
  };
}

function mapBot(raw: RawBot): Bot {
  return {
    id: raw.id,
    userId: raw.userId ?? raw.user_id ?? '',
    name: raw.name,
    status: raw.status as BotStatus,
    assistantStyle: (raw.assistantStyle ?? raw.assistant_style ?? 'beginner') as Persona,
    dropletId: raw.dropletId ?? raw.droplet_id,
    region: raw.region ?? '',
    ipAddress: raw.ipAddress ?? raw.ip_address,
    agentWallet: raw.agentWallet ?? raw.agent_wallet,
    desiredVersionId: raw.desiredVersionId ?? raw.desired_version_id ?? '',
    appliedVersionId: raw.appliedVersionId ?? raw.applied_version_id,
    configStatus: (raw.configStatus ?? raw.config_status ?? 'pending') as Bot['configStatus'],
    createdAt: raw.createdAt ?? raw.created_at ?? '',
    updatedAt: raw.updatedAt ?? raw.updated_at ?? '',
    lastHeartbeatAt: raw.lastHeartbeatAt ?? raw.last_heartbeat_at,
    todayPnl: Number(raw.todayPnl ?? raw.today_pnl ?? 0),
    totalPnl: Number(raw.totalPnl ?? raw.total_pnl ?? 0),
  };
}

function mapBotConfig(raw: RawBotConfig | null | undefined): BotConfig | null {
  if (!raw) return null;
  return {
    id: raw.id,
    botId: raw.botId ?? raw.bot_id ?? '',
    version: raw.version,
    createdAt: raw.createdAt ?? raw.created_at ?? '',
    name: raw.name ?? '',
    assistantStyle: (raw.assistantStyle ?? raw.assistant_style ?? 'beginner') as Persona,
    iconColor: raw.iconColor ?? raw.icon_color,
    assetFocus: (raw.assetFocus ?? raw.asset_focus ?? 'majors') as AssetFocus,
    customAssets: raw.customAssets ?? raw.custom_assets,
    algorithmMode: (raw.algorithmMode ?? raw.algorithm_mode ?? 'trend') as AlgorithmMode,
    strictness: (raw.strictness ?? 'medium') as Strictness,
    signalKnobs: (raw.signalKnobs ?? raw.signal_knobs) as BotConfig['signalKnobs'],
    riskCaps: (raw.riskCaps ?? raw.risk_caps) as BotConfig['riskCaps'],
    tradingMode: (raw.tradingMode ?? raw.trading_mode ?? 'paper') as TradingMode,
    llmProvider: (raw.llmProvider ?? raw.llm_provider ?? 'openai') as LlmProvider,
    llmModel: raw.llmModel ?? raw.llm_model,
    llmApiKey: raw.llmApiKey ?? raw.llm_api_key ?? '',
  } as BotConfig;
}

// User API
export const userApi = {
  async getCurrentUser(): Promise<User> {
    return fetchApi('/me');
  },

  async checkDisplayNameAvailability(displayName: string): Promise<NameAvailability> {
    const response = await fetchApi(
      `/account/display-name-availability?display_name=${encodeURIComponent(displayName)}`
    );
    return {
      available: Boolean(response.available),
      normalizedName:
        response.normalizedName ?? response.normalized_name ?? displayName.trim(),
      suggestedName: response.suggestedName ?? response.suggested_name ?? undefined,
    };
  },

  async getSettings(): Promise<UserSettings> {
    const response = await fetchApi('/account/settings');
    return {
      id: response.id,
      email: response.email,
      displayName: response.displayName ?? response.display_name,
      defaultAssistantStyle:
        response.defaultAssistantStyle ?? response.default_assistant_style ?? undefined,
      picture: response.picture,
      authMethods: {
        emailPassword: Boolean(response.authMethods?.emailPassword ?? response.auth_methods?.email_password),
        google: Boolean(response.authMethods?.google ?? response.auth_methods?.google),
        apple: Boolean(response.authMethods?.apple ?? response.auth_methods?.apple),
      },
      createdAt: response.createdAt ?? response.created_at,
      updatedAt: response.updatedAt ?? response.updated_at,
    };
  },

  async updateSettings(request: UpdateUserSettingsRequest): Promise<UserSettings> {
    const response = await fetchApi('/account/settings', {
      method: 'PATCH',
      body: JSON.stringify({
        display_name: request.displayName,
        default_assistant_style: request.defaultAssistantStyle,
      }),
    });

    return {
      id: response.id,
      email: response.email,
      displayName: response.displayName ?? response.display_name,
      defaultAssistantStyle:
        response.defaultAssistantStyle ?? response.default_assistant_style ?? undefined,
      picture: response.picture,
      authMethods: {
        emailPassword: Boolean(response.authMethods?.emailPassword ?? response.auth_methods?.email_password),
        google: Boolean(response.authMethods?.google ?? response.auth_methods?.google),
        apple: Boolean(response.authMethods?.apple ?? response.auth_methods?.apple),
      },
      createdAt: response.createdAt ?? response.created_at,
      updatedAt: response.updatedAt ?? response.updated_at,
    };
  },

  async getBillingSummary(): Promise<BillingSummary> {
    const response = await fetchApi('/account/billing');
    return {
      status: response.status,
      planCode: response.planCode ?? response.plan_code,
      maxBots: Number(response.maxBots ?? response.max_bots ?? 1),
      botCount: Number(response.botCount ?? response.bot_count ?? 0),
      currentPeriodEnd: response.currentPeriodEnd ?? response.current_period_end ?? undefined,
    };
  },
};

export const docsApi = {
  async getDocs(): Promise<GetDocsResponse> {
    const response = await fetchApi('/docs');

    const categories = (response.categories || []).map((category: RawDocsCategory): DocsCategory => ({
      id: category.id,
      title: category.title,
      description: category.description ?? '',
      articles: (category.articles || []).map((article: RawDocsArticle) => ({
        id: article.id,
        title: article.title,
        summary: article.summary ?? '',
        content: Array.isArray(article.content) ? article.content.map((line: unknown) => String(line)) : [],
      })),
    }));

    return { categories };
  },

  async trackEvent(request: TrackDocsEventRequest): Promise<TrackDocsEventResponse> {
    const response = await fetchApi('/docs/analytics', {
      method: 'POST',
      body: JSON.stringify({
        event_type: request.eventType,
        category_id: request.categoryId,
        article_id: request.articleId,
        query: request.query,
        results_count: request.resultsCount,
      }),
    });

    return {
      success: Boolean(response.success),
    };
  },
};

export const reportsApi = {
  async requestEmailCsv(request: EmailCsvReportRequest): Promise<EmailCsvReportResponse> {
    const response = await fetchApi('/reports/email-csv', {
      method: 'POST',
      body: JSON.stringify({
        report_kind: request.reportKind,
        timeframe: request.timeframe,
      }),
    });

    return {
      success: Boolean(response.success),
      message: response.message,
      deliveredTo: response.deliveredTo ?? response.delivered_to,
      rowsIncluded: Number(response.rowsIncluded ?? response.rows_included ?? 0),
    };
  },
};

// Price/Data API (separate service)
export const dataApi = {
  async getPrice(symbol: string, quote: string = 'USD'): Promise<{
    symbol: string;
    price: string;
    source: string;
    timestamp: string;
    confidence?: number;
  }> {
    const url = `${DATA_API_URL}/prices/${symbol}?quote=${quote}`;
    const response = await fetch(url);
    if (!response.ok) {
      throw new ApiError(response.status, 'Failed to fetch price');
    }
    return response.json();
  },

  async getPricesBatch(symbols: string[]): Promise<{
    prices: Record<string, {
      symbol: string;
      price: string;
      source: string;
      timestamp: string;
      confidence?: number;
    }>;
    errors: string[];
  }> {
    const url = `${DATA_API_URL}/prices/batch`;
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ symbols }),
    });
    if (!response.ok) {
      throw new ApiError(response.status, 'Failed to fetch prices');
    }
    return response.json();
  },

  async getSupportedSymbols(): Promise<{
    crypto: string[];
    stocks: string[];
    etfs: string[];
    metals: string[];
  }> {
    const url = `${DATA_API_URL}/prices/supported`;
    const response = await fetch(url);
    if (!response.ok) {
      throw new ApiError(response.status, 'Failed to fetch symbols');
    }
    return response.json();
  },

  async healthCheck(): Promise<{
    status: string;
    sources: Array<{
      source: string;
      is_healthy: boolean;
      success_rate_24h: number;
      avg_latency_ms: number;
    }>;
  }> {
    const url = `${DATA_API_URL}/health`;
    const response = await fetch(url);
    if (!response.ok) {
      throw new ApiError(response.status, 'Health check failed');
    }
    return response.json();
  },
};

// Export all
export const api = {
  bot: botApi,
  user: userApi,
  docs: docsApi,
  reports: reportsApi,
  data: dataApi,
};

export { configureApi } from './config';
export default api;
