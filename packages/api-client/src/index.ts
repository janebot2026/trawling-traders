import type {
  Bot,
  BotConfig,
  BotEvent,
  CreateBotRequest,
  GetBotResponse,
  GetEventsResponse,
  GetMetricsResponse,
  ListBotsResponse,
  MetricPoint,
  UpdateBotConfigRequest,
  User,
} from '@trawling-traders/types';

// API client configuration
interface ApiClientConfig {
  baseUrl: string;
  getAuthToken: () => string | null;
}

// API error
export class ApiError extends Error {
  constructor(
    message: string,
    public statusCode: number,
    public response?: Response
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

// Generic API client
class ApiClient {
  private config: ApiClientConfig;

  constructor(config: ApiClientConfig) {
    this.config = config;
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown
  ): Promise<T> {
    const token = this.config.getAuthToken();
    
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };
    
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }

    const response = await fetch(`${this.config.baseUrl}${path}`, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
    });

    if (!response.ok) {
      throw new ApiError(
        `API request failed: ${response.statusText}`,
        response.status,
        response
      );
    }

    return response.json() as Promise<T>;
  }

  get<T>(path: string): Promise<T> {
    return this.request<T>('GET', path);
  }

  post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>('POST', path, body);
  }

  patch<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>('PATCH', path, body);
  }

  delete<T>(path: string): Promise<T> {
    return this.request<T>('DELETE', path);
  }
}

// Trawling Traders API
export class TrawlingTradersApi {
  private client: ApiClient;

  constructor(config: ApiClientConfig) {
    this.client = new ApiClient(config);
  }

  // Auth
  async getCurrentUser(): Promise<User> {
    return this.client.get<User>('/me');
  }

  // Bots
  async listBots(): Promise<ListBotsResponse> {
    return this.client.get<ListBotsResponse>('/bots');
  }

  async createBot(request: CreateBotRequest): Promise<Bot> {
    return this.client.post<Bot>('/bots', request);
  }

  async getBot(botId: string): Promise<GetBotResponse> {
    return this.client.get<GetBotResponse>(`/bots/${botId}`);
  }

  async updateBotConfig(
    botId: string,
    request: UpdateBotConfigRequest
  ): Promise<BotConfig> {
    return this.client.patch<BotConfig>(`/bots/${botId}/config`, request);
  }

  async botAction(botId: string, action: 'pause' | 'resume' | 'redeploy' | 'destroy'): Promise<void> {
    return this.client.post<void>(`/bots/${botId}/actions`, { action });
  }

  // Metrics
  async getMetrics(botId: string, range: '7d' | '30d' = '7d'): Promise<GetMetricsResponse> {
    return this.client.get<GetMetricsResponse>(`/bots/${botId}/metrics?range=${range}`);
  }

  // Events
  async getEvents(botId: string, cursor?: string): Promise<GetEventsResponse> {
    const path = cursor
      ? `/bots/${botId}/events?cursor=${cursor}`
      : `/bots/${botId}/events`;
    return this.client.get<GetEventsResponse>(path);
  }
}

// Singleton instance (configure with your API URL)
let apiInstance: TrawlingTradersApi | null = null;

export function initializeApi(config: ApiClientConfig): TrawlingTradersApi {
  apiInstance = new TrawlingTradersApi(config);
  return apiInstance;
}

export function getApi(): TrawlingTradersApi {
  if (!apiInstance) {
    throw new Error('API not initialized. Call initializeApi first.');
  }
  return apiInstance;
}

// Re-export types
export * from '@trawling-traders/types';
