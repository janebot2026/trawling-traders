import { useState, useEffect, useCallback } from 'react';
import { dataApi } from '@trawling-traders/api-client';

interface Price {
  symbol: string;
  price: string;
  source: string;
  timestamp: string;
  confidence?: number;
}

interface UsePriceOptions {
  symbol: string;
  quote?: string;
  refreshInterval?: number;
}

export function usePrice(options: UsePriceOptions) {
  const { symbol, quote = 'USD', refreshInterval = 30000 } = options;
  const [price, setPrice] = useState<Price | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const fetchPrice = useCallback(async () => {
    if (!symbol) return;
    
    try {
      setLoading(true);
      const response = await dataApi.getPrice(symbol, quote);
      setPrice(response);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Failed to fetch price'));
    } finally {
      setLoading(false);
    }
  }, [symbol, quote]);

  useEffect(() => {
    fetchPrice();
    
    if (refreshInterval > 0) {
      const interval = setInterval(fetchPrice, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [fetchPrice, refreshInterval]);

  return { price, loading, error, refetch: fetchPrice };
}

interface UsePricesBatchOptions {
  symbols: string[];
  refreshInterval?: number;
}

export function usePricesBatch(options: UsePricesBatchOptions) {
  const { symbols, refreshInterval = 30000 } = options;
  const [prices, setPrices] = useState<Record<string, Price>>({});
  const [loading, setLoading] = useState(false);
  const [errors, setErrors] = useState<string[]>([]);

  const fetchPrices = useCallback(async () => {
    if (!symbols.length) return;
    
    try {
      setLoading(true);
      const response = await dataApi.getPricesBatch(symbols);
      setPrices(response.prices);
      setErrors(response.errors);
    } catch (err) {
      setErrors([err instanceof Error ? err.message : 'Failed to fetch prices']);
    } finally {
      setLoading(false);
    }
  }, [symbols.join(',')]);

  useEffect(() => {
    fetchPrices();
    
    if (refreshInterval > 0) {
      const interval = setInterval(fetchPrices, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [fetchPrices, refreshInterval]);

  return { prices, loading, errors, refetch: fetchPrices };
}

export function useSupportedSymbols() {
  const [symbols, setSymbols] = useState<{
    crypto: string[];
    stocks: string[];
    etfs: string[];
    metals: string[];
  }>({ crypto: [], stocks: [], etfs: [], metals: [] });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const fetchSymbols = useCallback(async () => {
    try {
      setLoading(true);
      const response = await dataApi.getSupportedSymbols();
      setSymbols(response);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Failed to fetch symbols'));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchSymbols();
  }, [fetchSymbols]);

  return { symbols, loading, error, refetch: fetchSymbols };
}

export function useDataHealth() {
  const [health, setHealth] = useState<{
    status: string;
    sources: Array<{
      source: string;
      is_healthy: boolean;
      success_rate_24h: number;
      avg_latency_ms: number;
    }>;
  } | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const checkHealth = useCallback(async () => {
    try {
      setLoading(true);
      const response = await dataApi.healthCheck();
      setHealth(response);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Health check failed'));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    checkHealth();
  }, [checkHealth]);

  return { health, loading, error, refetch: checkHealth };
}
