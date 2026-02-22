/**
 * Factory functions for building test data.
 * Each factory returns a complete entity with sensible defaults;
 * callers can override individual fields via the `overrides` parameter.
 */
import type {
  Bot,
  BotConfig,
  BotEvent,
  MetricPoint,
  User,
  BotChatMessage,
} from '@trawling-traders/types';

let _id = 0;
const nextId = () => String(++_id);

export function buildUser(overrides: Partial<User> = {}): User {
  return {
    id: nextId(),
    email: 'trader@example.com',
    isAdmin: false,
    subscription: {
      id: nextId(),
      userId: '',
      status: 'active',
      maxBots: 4,
      currentPeriodStart: '2026-01-01T00:00:00Z',
      currentPeriodEnd: '2026-02-01T00:00:00Z',
    },
    ...overrides,
  };
}

export function buildBot(overrides: Partial<Bot> = {}): Bot {
  const id = overrides.id ?? nextId();
  return {
    id,
    userId: 'user-1',
    name: `TestBot-${id}`,
    status: 'online',
    assistantStyle: 'beginner',
    region: 'nyc1',
    desiredVersionId: 'v1',
    appliedVersionId: 'v1',
    configStatus: 'applied',
    createdAt: '2026-01-15T10:00:00Z',
    updatedAt: '2026-01-15T10:00:00Z',
    lastHeartbeatAt: new Date().toISOString(),
    todayPnl: 12.5,
    totalPnl: 150.0,
    ...overrides,
  };
}

export function buildBotConfig(overrides: Partial<BotConfig> = {}): BotConfig {
  return {
    version: 1,
    createdAt: '2026-01-15T10:00:00Z',
    name: 'TestBot',
    assistantStyle: 'beginner',
    assetFocus: 'majors',
    algorithmMode: 'trend',
    strictness: 'medium',
    riskCaps: {
      maxPositionSizePercent: 10,
      maxDailyLossUsd: 500,
      maxDrawdownPercent: 15,
      maxTradesPerDay: 20,
    },
    tradingMode: 'paper',
    llmProvider: 'openai',
    llmModel: 'gpt-4o',
    llmApiKey: 'sk-test-key',
    ...overrides,
  };
}

export function buildMetrics(count = 7): MetricPoint[] {
  return Array.from({ length: count }, (_, i) => ({
    timestamp: new Date(Date.now() - (count - i) * 86_400_000).toISOString(),
    value: 1000 + Math.random() * 100,
  }));
}

export function buildEvent(overrides: Partial<BotEvent> = {}): BotEvent {
  return {
    id: nextId(),
    botId: 'bot-1',
    type: 'trade_opened',
    timestamp: new Date().toISOString(),
    message: 'Opened AAPL long position',
    ...overrides,
  };
}

export function buildChatMessage(overrides: Partial<BotChatMessage> = {}): BotChatMessage {
  return {
    id: nextId(),
    botId: 'bot-1',
    role: 'user',
    content: 'How is my portfolio doing?',
    timestamp: new Date().toISOString(),
    ...overrides,
  };
}

/** Reset the auto-increment counter (call in beforeEach for stable IDs). */
export function resetIdCounter() {
  _id = 0;
}
