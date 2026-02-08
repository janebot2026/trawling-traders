import { getConfig, type ApiClientConfig } from './config';

import type {
  Bot,
  BotConfig,
  BotEvent,
  CreateBotRequest,
  UpdateBotConfigRequest,
  ListBotsResponse,
  GetBotResponse,
  GetMetricsResponse,
  GetEventsResponse,
  BotActionRequest,
  User,
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

// Auth token provider function type
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
  const config = getConfig();
  const url = `${config.baseUrl}/v1${endpoint}`;

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...((options.headers as Record<string, string>) || {}),
  };

  // Add auth token if auth provider is configured
  if (authConfig) {
    const token = await authConfig.getToken();
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }
  }

  // Create AbortController for timeout
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), config.timeoutMs);

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
      throw new TimeoutError(`Request to ${endpoint} timed out after ${config.timeoutMs}ms`);
    }
    // Retry network errors
    if (retryCount < (config.maxRetries || 3)) {
      const delay = 1000 * Math.pow(2, retryCount);
      await sleep(delay);
      return fetchApi(endpoint, options, isAuthRetry, retryCount + 1);
    }
    throw new NetworkError(error.message || 'Network request failed');
  } finally {
    clearTimeout(timeoutId);
  }

  // Handle 401 Unauthorized - attempt token refresh
  if (response.status === 401 && !isAuthRetry && authConfig?.refreshToken) {
    const newToken = await authConfig.refreshToken();
    if (newToken) {
      return fetchApi(endpoint, options, true, 0);
    } else {
      await authConfig.clearAuth?.();
      throw new AuthExpiredError();
    }
  }

  // Retry 5xx errors with exponential backoff
  if (isRetryableError(response.status) && retryCount < (config.maxRetries || 3)) {
    const delay = 1000 * Math.pow(2, retryCount);
    await sleep(delay);
    return fetchApi(endpoint, options, isAuthRetry, retryCount + 1);
  }

  if (!response.ok) {
    const error = await response.text();

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
  async listBots(): Promise<ListBotsResponse> {
    return fetchApi('/bots');
  },

  async createBot(request: CreateBotRequest): Promise<Bot> {
    return fetchApi('/bots', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  },

  async getBot(botId: string): Promise<GetBotResponse> {
    return fetchApi(`/bots/${botId}`);
  },

  async updateBotConfig(
    botId: string,
    request: UpdateBotConfigRequest
  ): Promise<BotConfig> {
    return fetchApi(`/bots/${botId}/config`, {
      method: 'PATCH',
      body: JSON.stringify(request),
    });
  },

  async botAction(
    botId: string,
    action: BotActionRequest['action']
  ): Promise<void> {
    return fetchApi(`/bots/${botId}/actions`, {
      method: 'POST',
      body: JSON.stringify({ action }),
    });
  },

  async getMetrics(botId: string): Promise<GetMetricsResponse> {
    return fetchApi(`/bots/${botId}/metrics`);
  },

  async getEvents(botId: string): Promise<GetEventsResponse> {
    return fetchApi(`/bots/${botId}/events`);
  },
};

// User API
export const userApi = {
  async getCurrentUser(): Promise<User> {
    return fetchApi('/me');
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
    const config = getConfig();
    const url = `${config.dataApiUrl}/prices/${symbol}?quote=${quote}`;
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
    const config = getConfig();
    const url = `${config.dataApiUrl}/prices/batch`;
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
    const config = getConfig();
    const url = `${config.dataApiUrl}/prices/supported`;
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
    const config = getConfig();
    const url = `${config.dataApiUrl}/health`;
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
  data: dataApi,
};

export default api;
