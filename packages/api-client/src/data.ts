import { DATA_API_URL, fetchDataApi } from './http';

export interface PriceResult {
  symbol: string;
  price: string;
  source: string;
  timestamp: string;
  confidence?: number;
}

export interface PriceBatchResult {
  prices: Record<string, PriceResult>;
  errors: string[];
}

export interface SupportedSymbols {
  crypto: string[];
  stocks: string[];
  etfs: string[];
  metals: string[];
}

export interface DataApiHealthSource {
  source: string;
  is_healthy: boolean;
  success_rate_24h: number;
  avg_latency_ms: number;
}

export interface DataApiHealth {
  status: string;
  sources: DataApiHealthSource[];
}

// Price/Data API (separate service)
export const dataApi = {
  async getPrice(symbol: string, quote: string = 'USD'): Promise<PriceResult> {
    return fetchDataApi(`${DATA_API_URL}/prices/${symbol}?quote=${quote}`);
  },

  async getPricesBatch(symbols: string[]): Promise<PriceBatchResult> {
    return fetchDataApi(`${DATA_API_URL}/prices/batch`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ symbols }),
    });
  },

  async getSupportedSymbols(): Promise<SupportedSymbols> {
    return fetchDataApi(`${DATA_API_URL}/prices/supported`);
  },

  async healthCheck(): Promise<DataApiHealth> {
    return fetchDataApi(`${DATA_API_URL}/health`);
  },
};
