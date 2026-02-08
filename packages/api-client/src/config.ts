// API Client Configuration
export interface ApiClientConfig {
  baseUrl: string;
  dataApiUrl?: string;
  timeoutMs?: number;
  maxRetries?: number;
}

let globalConfig: ApiClientConfig = {
  baseUrl: 'http://localhost:3000',
  dataApiUrl: 'http://localhost:8080',
  timeoutMs: 30000,
  maxRetries: 3,
};

export function configureApi(config: ApiClientConfig): void {
  globalConfig = { ...globalConfig, ...config };
}

export function getConfig(): ApiClientConfig {
  return globalConfig;
}
