import { API_URL } from '../config/api';

const API_BASE_URL = `${API_URL}/v1`;
const DATA_API_URL = process.env.DATA_API_URL || 'http://localhost:8080';

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

// Get auth token from Cedros Login (via AsyncStorage)
async function getAuthToken(): Promise<string | null> {
  try {
    // Cedros Login stores token in AsyncStorage
    // We'll need to import from @cedros/login-react-native utils
    const { TokenManager } = await import('@cedros/login-react-native');
    return await TokenManager.getAccessToken();
  } catch {
    return null;
  }
}

// Attempt to refresh the access token
async function refreshAuthToken(): Promise<string | null> {
  try {
    const { TokenManager } = await import('@cedros/login-react-native');
    // TokenManager should have a refresh method that uses the refresh token
    // to get a new access token
    if (typeof TokenManager.refreshAccessToken === 'function') {
      return await TokenManager.refreshAccessToken();
    }
    return null;
  } catch {
    return null;
  }
}

// Clear auth state on permanent auth failure
async function clearAuthState(): Promise<void> {
  try {
    const { TokenManager } = await import('@cedros/login-react-native');
    if (typeof TokenManager.clearTokens === 'function') {
      await TokenManager.clearTokens();
    }
  } catch {
    // Best effort
  }
}

// Default timeout for API requests (30 seconds)
const DEFAULT_TIMEOUT_MS = 30000;

// Timeout error for distinguishing from other errors
export class TimeoutError extends ApiError {
  constructor(message: string = 'Request timed out') {
    super(0, message);
    this.name = 'TimeoutError';
  }
}

// HTTP client with auth, automatic token refresh, and timeout
async function fetchApi(
  endpoint: string,
  options: RequestInit = {},
  isRetry: boolean = false
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
    throw error;
  } finally {
    clearTimeout(timeoutId);
  }

  // Handle 401 Unauthorized - attempt token refresh
  if (response.status === 401 && !isRetry) {
    const newToken = await refreshAuthToken();
    if (newToken) {
      // Retry request with new token
      return fetchApi(endpoint, options, true);
    } else {
      // Refresh failed - clear auth state and throw
      await clearAuthState();
      throw new AuthExpiredError();
    }
  }

  if (!response.ok) {
    const error = await response.text();
    throw new ApiError(response.status, error || 'API request failed');
  }

  return response.json();
}

// Bot API
export const botApi = {
  // List all bots for current user
  async listBots(): Promise<ListBotsResponse> {
    return fetchApi('/bots');
  },

  // Create a new bot
  async createBot(request: CreateBotRequest): Promise<Bot> {
    return fetchApi('/bots', {
      method: 'POST',
      body: JSON.stringify(request),
    });
  },

  // Get bot details with config
  async getBot(botId: string): Promise<GetBotResponse> {
    return fetchApi(`/bots/${botId}`);
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
    return fetchApi(`/bots/${botId}/metrics`);
  },

  // Get bot events
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
  data: dataApi,
};

export default api;
