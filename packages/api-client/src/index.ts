const API_BASE_URL = process.env.API_BASE_URL || 'http://localhost:3000/api/v1';
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

// HTTP client with auth
async function fetchApi(
  endpoint: string,
  options: RequestInit = {}
): Promise<any> {
  const url = `${API_BASE_URL}${endpoint}`;
  
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...((options.headers as Record<string, string>) || {}),
  };

  // TODO: Add auth token from Cedros session
  // const token = await getAuthToken();
  // if (token) headers['Authorization'] = `Bearer ${token}`;

  const response = await fetch(url, {
    ...options,
    headers,
  });

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
