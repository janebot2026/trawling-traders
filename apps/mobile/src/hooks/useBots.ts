import { useState, useEffect, useCallback } from 'react';
import { botApi, userApi, dataApi, AuthExpiredError } from '@trawling-traders/api-client';
import type { Bot, User, BotConfig, BotEvent } from '@trawling-traders/types';

interface UseBotsOptions {
  refreshInterval?: number;
}

export function useBots(options: UseBotsOptions = {}) {
  const { refreshInterval } = options;
  const [bots, setBots] = useState<Bot[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  const fetchBots = useCallback(async () => {
    try {
      setLoading(true);
      const response = await botApi.listBots();
      setBots(response.bots);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Failed to fetch bots'));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchBots();
    
    if (refreshInterval && refreshInterval > 0) {
      const interval = setInterval(fetchBots, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [fetchBots, refreshInterval]);

  return { bots, loading, error, refetch: fetchBots };
}

interface UseBotOptions {
  botId: string;
  refreshInterval?: number;
}

export function useBot(options: UseBotOptions) {
  const { botId, refreshInterval } = options;
  const [bot, setBot] = useState<Bot | null>(null);
  const [config, setConfig] = useState<BotConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  const fetchBot = useCallback(async () => {
    if (!botId) return;
    
    try {
      setLoading(true);
      const response = await botApi.getBot(botId);
      setBot(response.bot);
      setConfig(response.config);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Failed to fetch bot'));
    } finally {
      setLoading(false);
    }
  }, [botId]);

  useEffect(() => {
    fetchBot();
    
    if (refreshInterval && refreshInterval > 0) {
      const interval = setInterval(fetchBot, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [fetchBot, refreshInterval]);

  return { bot, config, loading, error, refetch: fetchBot };
}

export function useUser() {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  const fetchUser = useCallback(async () => {
    try {
      setLoading(true);
      const response = await userApi.getCurrentUser();
      setUser(response);
      setError(null);
    } catch (err) {
      if (err instanceof AuthExpiredError) {
        setUser(null);
      }
      setError(err instanceof Error ? err : new Error('Failed to fetch user'));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchUser();
  }, [fetchUser]);

  return { user, loading, error, refetch: fetchUser };
}

interface UseBotMetricsOptions {
  botId: string;
  range?: '7d' | '30d';
}

export function useBotMetrics(options: UseBotMetricsOptions) {
  const { botId, range = '7d' } = options;
  const [metrics, setMetrics] = useState<Array<{ timestamp: string; value: number }>>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const fetchMetrics = useCallback(async () => {
    if (!botId) return;
    
    try {
      setLoading(true);
      const response = await botApi.getMetrics(botId);
      setMetrics(response.metrics);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Failed to fetch metrics'));
    } finally {
      setLoading(false);
    }
  }, [botId]);

  useEffect(() => {
    fetchMetrics();
  }, [fetchMetrics, range]);

  return { metrics, loading, error, refetch: fetchMetrics };
}

interface UseBotEventsOptions {
  botId: string;
  limit?: number;
}

export function useBotEvents(options: UseBotEventsOptions) {
  const { botId, limit = 50 } = options;
  const [events, setEvents] = useState<BotEvent[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [nextCursor, setNextCursor] = useState<string | undefined>();

  const fetchEvents = useCallback(async () => {
    if (!botId) return;
    
    try {
      setLoading(true);
      const response = await botApi.getEvents(botId);
      setEvents(response.events.slice(0, limit));
      setNextCursor(response.nextCursor);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Failed to fetch events'));
    } finally {
      setLoading(false);
    }
  }, [botId, limit]);

  useEffect(() => {
    fetchEvents();
  }, [fetchEvents]);

  return { events, loading, error, nextCursor, refetch: fetchEvents };
}

export function useCreateBot() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const createBot = useCallback(async (request: Parameters<typeof botApi.createBot>[0]) => {
    try {
      setLoading(true);
      const bot = await botApi.createBot(request);
      setError(null);
      return bot;
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Failed to create bot'));
      throw err;
    } finally {
      setLoading(false);
    }
  }, []);

  return { createBot, loading, error };
}

export function useBotAction() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const performAction = useCallback(async (
    botId: string,
    action: 'pause' | 'resume' | 'redeploy' | 'destroy'
  ) => {
    try {
      setLoading(true);
      await botApi.botAction(botId, action);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(`Failed to ${action} bot`));
      throw err;
    } finally {
      setLoading(false);
    }
  }, []);

  return { performAction, loading, error };
}
