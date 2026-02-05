// Bot status states
export type BotStatus = 'provisioning' | 'online' | 'offline' | 'paused' | 'error' | 'destroying';

// Trading personas (drives UI complexity)
export type Persona = 'beginner' | 'tweaker' | 'quant-lite';

// Algorithm modes
export type AlgorithmMode = 'trend' | 'mean-reversion' | 'breakout';

// Asset focus options - FOCUSED ON QUALITY ASSETS (xStocks, metals)
// NOT memes by default - Solana execution for serious assets
export type AssetFocus = 'majors' | 'tokenized-equities' | 'tokenized-metals' | 'memes' | 'custom';

// Paper vs Live trading
export type TradingMode = 'paper' | 'live';

// Strictness levels
export type Strictness = 'low' | 'medium' | 'high';

// LLM providers
export type LlmProvider = 'openai' | 'anthropic' | 'venice' | 'openrouter';

// LLM models by provider
export type LlmModel =
  | 'gpt-4o' | 'gpt-4o-mini' | 'gpt-4-turbo'  // OpenAI
  | 'claude-3-5-sonnet' | 'claude-3-opus' | 'claude-3-haiku'  // Anthropic
  | 'llama-3.1-405b'  // Venice
  | 'auto'  // OpenRouter auto-select
  | string; // Custom model string

// Bot configuration (what user sets)
export interface BotConfig {
  id?: string;
  botId?: string;
  version: number;
  createdAt: string;
  
  // Identity
  name: string;
  persona: Persona;
  iconColor?: string;
  
  // Trading focus - QUALITY ASSETS FIRST
  assetFocus: AssetFocus;
  customAssets?: string[]; // for 'custom' focus
  
  // Algorithm
  algorithmMode: AlgorithmMode;
  strictness: Strictness;
  
  // Signal knobs (Quant-lite only)
  signalKnobs?: {
    volumeConfirmation: boolean;
    volatilityBrake: boolean;
    liquidityFilter: 'low' | 'medium' | 'high';
    correlationBrake: boolean;
  };
  
  // Risk caps
  riskCaps: {
    maxPositionSizePercent: number; // 1-100
    maxDailyLossUsd: number;
    maxDrawdownPercent: number;
    maxTradesPerDay: number;
  };
  
  // Trading mode
  tradingMode: TradingMode;
  
  // Secrets (encrypted server-side)
  llmProvider: LlmProvider;
  llmApiKey: string;
}

// Bot entity (from backend)
export interface Bot {
  id: string;
  userId: string;
  name: string;
  status: BotStatus;
  persona: Persona;
  
  // Provisioning
  dropletId?: string;
  region: string;
  ipAddress?: string;
  
  // Agent wallet - created by bot on VPS via Solana CLI
  agentWallet?: string; // Solana address
  
  // Config state
  desiredVersionId: string;
  appliedVersionId?: string;
  configStatus: 'pending' | 'applied' | 'failed';
  
  // Timestamps
  createdAt: string;
  updatedAt: string;
  lastHeartbeatAt?: string;
  
  // Current performance (from metrics)
  todayPnl?: number;
  totalPnl?: number;
}

// Metric data point
export interface MetricPoint {
  timestamp: string;
  value: number; // equity or pnl
}

// Bot event
export type BotEventType = 
  | 'trade_opened'
  | 'trade_closed'
  | 'stop_triggered'
  | 'config_applied'
  | 'config_failed'
  | 'error'
  | 'status_change';

export interface BotEvent {
  id: string;
  botId: string;
  type: BotEventType;
  timestamp: string;
  message: string;
  metadata?: Record<string, unknown>;
}

// User subscription
export interface Subscription {
  id: string;
  userId: string;
  status: 'active' | 'cancelled' | 'past_due';
  maxBots: number;
  currentPeriodStart: string;
  currentPeriodEnd: string;
}

// User
export interface User {
  id: string;
  email: string;
  subscription?: Subscription;
}

// API request/response types
export interface CreateBotRequest {
  name: string;
  persona: Persona;
  assetFocus: AssetFocus;
  algorithmMode: AlgorithmMode;
  strictness: Strictness;
  riskCaps: BotConfig['riskCaps'];
  tradingMode: TradingMode;
  llmProvider: LlmProvider;
  llmModel?: LlmModel;
  llmApiKey: string;
  // Telegram integration
  telegramEnabled?: boolean;
  telegramBotToken?: string;
}

// OpenClaw config request/response types
export interface UpdateOpenClawConfigRequest {
  llmProvider: LlmProvider;
  llmModel?: LlmModel;
  llmApiKey?: string;
  telegramEnabled?: boolean;
  telegramBotToken?: string;
}

export interface OpenClawConfigResponse {
  botId: string;
  llmProvider: string;
  llmModel: string;
  hasLlmApiKey: boolean;
  telegramEnabled: boolean;
  hasTelegramBotToken: boolean;
  discordEnabled: boolean;
  hasDiscordBotToken: boolean;
  updatedAt: string;
}

export interface UpdateBotConfigRequest {
  config: Partial<BotConfig>;
}

export interface BotActionRequest {
  action: 'pause' | 'resume' | 'redeploy' | 'destroy';
}

export interface ListBotsResponse {
  bots: Bot[];
  total: number;
}

export interface GetBotResponse {
  bot: Bot;
  config: BotConfig;
}

export interface GetMetricsResponse {
  metrics: MetricPoint[];
  range: '7d' | '30d';
}

export interface GetEventsResponse {
  events: BotEvent[];
  nextCursor?: string;
}
