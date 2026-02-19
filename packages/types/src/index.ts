// Bot status states
export type BotStatus = 'provisioning' | 'online' | 'offline' | 'paused' | 'error' | 'destroying';

// Trading personas (drives UI complexity)
export type Persona = 'beginner' | 'tweaker' | 'quant_lite';

// Algorithm modes
export type AlgorithmMode = 'trend' | 'mean_reversion' | 'breakout';

// Asset focus options - FOCUSED ON QUALITY ASSETS (xStocks, metals)
// NOT memes by default - Solana execution for serious assets
export type AssetFocus = 'majors' | 'tokenized_equities' | 'tokenized_metals' | 'finance_2' | 'memes' | 'custom';

// Paper vs Live trading
export type TradingMode = 'paper' | 'live';

// Strictness levels
export type Strictness = 'low' | 'medium' | 'high';

export interface AlgorithmFactor {
  factor: string;
  weight: number;
}

export interface TradeableAsset {
  id: string;
  assetFocus: AssetFocus;
  symbol: string;
  name: string;
  tokenAddress: string;
  decimals: number;
  custodian: string;
  isActive: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface AIAssistantOption {
  id: string;
  assistantStyle: Persona;
  captainName: string;
  personalityDescription: string;
  imageKey: string;
  imagePath: string;
  sortOrder: number;
  isActive: boolean;
}

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
  assistantStyle: Persona;
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
  llmModel?: LlmModel;
  llmApiKey: string;
}

// Bot entity (from backend)
export interface Bot {
  id: string;
  userId: string;
  name: string;
  status: BotStatus;
  assistantStyle: Persona;
  
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

export type BotChatRole = 'user' | 'assistant' | 'system';

export interface BotChatMessage {
  id: string;
  botId: string;
  role: BotChatRole;
  content: string;
  timestamp: string;
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
  isAdmin?: boolean;
  subscription?: Subscription;
}

export interface UserAuthMethods {
  emailPassword: boolean;
  google: boolean;
  apple: boolean;
}

export interface UserSettings {
  id: string;
  email?: string;
  displayName?: string;
  defaultAssistantStyle?: Persona;
  picture?: string;
  authMethods: UserAuthMethods;
  createdAt: string;
  updatedAt: string;
}

export interface UpdateUserSettingsRequest {
  displayName?: string;
  defaultAssistantStyle?: Persona;
}

export interface BillingSummary {
  status: string;
  planCode: string;
  maxBots: number;
  botCount: number;
  currentPeriodEnd?: string;
}

export interface NameAvailability {
  available: boolean;
  normalizedName: string;
  suggestedName?: string;
}

// API request/response types
export interface CreateBotRequest {
  name: string;
  assistantStyle?: Persona;
  assetFocus: AssetFocus;
  customAssets?: string[];
  algorithmMode: AlgorithmMode;
  algorithmFactors?: AlgorithmFactor[];
  strictness: Strictness;
  riskCaps: BotConfig['riskCaps'];
  tradingMode: TradingMode;
  llmProvider: LlmProvider;
  llmModel?: LlmModel;
  llmApiKey: string;
  // Telegram integration
  telegramEnabled?: boolean;
  telegramBotToken?: string;
  telegramUserId?: string;
  telegramPairingCode?: string;
}

// OpenClaw config request/response types
export interface UpdateOpenClawConfigRequest {
  llmProvider: LlmProvider;
  llmModel?: LlmModel;
  llmApiKey?: string;
  telegramEnabled?: boolean;
  telegramBotToken?: string;
  telegramUserId?: string;
  telegramPairingCode?: string;
}

export interface OpenClawConfigResponse {
  botId: string;
  llmProvider: string;
  llmModel: string;
  hasLlmApiKey: boolean;
  telegramEnabled: boolean;
  telegramUserId?: string;
  hasTelegramBotToken: boolean;
  hasTelegramPairingCode: boolean;
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
  config: BotConfig | null;
}

export interface GetMetricsResponse {
  metrics: MetricPoint[];
  range: '7d' | '30d';
}

export interface GetEventsResponse {
  events: BotEvent[];
  nextCursor?: string;
}

export interface GetBotChatMessagesResponse {
  messages: BotChatMessage[];
}

export interface PostBotChatMessageRequest {
  content: string;
}

export interface PostBotChatMessageResponse {
  userMessage: BotChatMessage;
  assistantMessage: BotChatMessage;
}

export interface EmailCsvReportRequest {
  reportKind: 'tax' | 'trade-history' | 'full';
  timeframe: '30d' | '90d' | '1y' | 'all';
}

export interface EmailCsvReportResponse {
  success: boolean;
  message: string;
  deliveredTo: string;
  rowsIncluded: number;
}

export interface DocsArticle {
  id: string;
  title: string;
  summary: string;
  content: string[];
}

export interface DocsCategory {
  id: string;
  title: string;
  description: string;
  articles: DocsArticle[];
}

export interface GetDocsResponse {
  categories: DocsCategory[];
}

export interface TrackDocsEventRequest {
  eventType: 'category_opened' | 'article_opened' | 'search';
  categoryId?: string;
  articleId?: string;
  query?: string;
  resultsCount?: number;
}

export interface TrackDocsEventResponse {
  success: boolean;
}
