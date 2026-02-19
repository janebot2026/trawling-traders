// API Client Configuration
export interface ApiClientConfig {
  baseUrl: string;
  dataApiUrl?: string;
  timeoutMs?: number;
  maxRetries?: number;
}
