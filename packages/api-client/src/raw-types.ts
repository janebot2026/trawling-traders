import type {
  Bot,
  BotChatMessage,
  BotChatRole,
  BotConfig,
  BotEventType,
  BotStatus,
  Persona,
  AssetFocus,
  AlgorithmMode,
  Strictness,
  TradingMode,
  LlmProvider,
  AIAssistantOption,
  TradeableAsset,
} from '@trawling-traders/types';

// Raw API response types — accept both camelCase and snake_case from backend
export interface RawChatMessage {
  id: string;
  botId?: string; bot_id?: string;
  role: string;
  content: string;
  timestamp?: string; created_at?: string;
}

export interface RawBot {
  id: string;
  userId?: string; user_id?: string;
  name: string;
  status: string;
  assistantStyle?: string; assistant_style?: string;
  dropletId?: string; droplet_id?: string;
  region?: string;
  ipAddress?: string; ip_address?: string;
  agentWallet?: string; agent_wallet?: string;
  desiredVersionId?: string; desired_version_id?: string;
  appliedVersionId?: string; applied_version_id?: string;
  configStatus?: string; config_status?: string;
  createdAt?: string; created_at?: string;
  updatedAt?: string; updated_at?: string;
  lastHeartbeatAt?: string; last_heartbeat_at?: string;
  todayPnl?: number; today_pnl?: number;
  totalPnl?: number; total_pnl?: number;
}

export interface RawBotConfig {
  id: string;
  botId?: string; bot_id?: string;
  version: number;
  createdAt?: string; created_at?: string;
  name?: string;
  assistantStyle?: string; assistant_style?: string;
  iconColor?: string; icon_color?: string;
  assetFocus?: string; asset_focus?: string;
  customAssets?: string[]; custom_assets?: string[];
  algorithmMode?: string; algorithm_mode?: string;
  strictness?: string;
  signalKnobs?: Record<string, unknown>; signal_knobs?: Record<string, unknown>;
  riskCaps?: Record<string, unknown>; risk_caps?: Record<string, unknown>;
  tradingMode?: string; trading_mode?: string;
  llmProvider?: string; llm_provider?: string;
  llmModel?: string; llm_model?: string;
  llmApiKey?: string; llm_api_key?: string;
}

export interface RawTradeableAsset {
  id: string;
  assetFocus?: string; asset_focus?: string;
  symbol: string;
  name: string;
  tokenAddress?: string; token_address?: string;
  decimals: number;
  custodian?: string;
  isActive?: boolean; is_active?: boolean;
  createdAt?: string; created_at?: string;
  updatedAt?: string; updated_at?: string;
}

export interface RawAssistantOption {
  id: string;
  assistantStyle?: string; assistant_style?: string;
  captainName?: string; captain_name?: string;
  personalityDescription?: string; personality_description?: string;
  imageKey?: string; image_key?: string;
  imagePath?: string; image_path?: string;
  sortOrder?: number; sort_order?: number;
  isActive?: boolean; is_active?: boolean;
}

export interface RawMetricPoint {
  timestamp: string;
  value?: number;
  pnl?: number;
}

export interface RawBotEvent {
  id: string;
  botId?: string; bot_id?: string;
  type?: string; event_type?: string;
  timestamp?: string; created_at?: string;
  message?: string;
  metadata?: Record<string, unknown>;
}

export interface RawDocsArticle {
  id: string;
  title: string;
  summary?: string;
  content?: unknown[];
}

export interface RawDocsCategory {
  id: string;
  title: string;
  description?: string;
  articles?: RawDocsArticle[];
}

export function mapChatMessage(raw: RawChatMessage): BotChatMessage {
  return {
    id: raw.id,
    botId: raw.botId ?? raw.bot_id ?? '',
    role: raw.role as BotChatRole,
    content: raw.content,
    timestamp: raw.timestamp ?? raw.created_at ?? '',
  };
}

export function mapBot(raw: RawBot): Bot {
  return {
    id: raw.id,
    userId: raw.userId ?? raw.user_id ?? '',
    name: raw.name,
    status: raw.status as BotStatus,
    assistantStyle: (raw.assistantStyle ?? raw.assistant_style ?? 'beginner') as Persona,
    dropletId: raw.dropletId ?? raw.droplet_id,
    region: raw.region ?? '',
    ipAddress: raw.ipAddress ?? raw.ip_address,
    agentWallet: raw.agentWallet ?? raw.agent_wallet,
    desiredVersionId: raw.desiredVersionId ?? raw.desired_version_id ?? '',
    appliedVersionId: raw.appliedVersionId ?? raw.applied_version_id,
    configStatus: (raw.configStatus ?? raw.config_status ?? 'pending') as Bot['configStatus'],
    createdAt: raw.createdAt ?? raw.created_at ?? '',
    updatedAt: raw.updatedAt ?? raw.updated_at ?? '',
    lastHeartbeatAt: raw.lastHeartbeatAt ?? raw.last_heartbeat_at,
    todayPnl: Number(raw.todayPnl ?? raw.today_pnl ?? 0),
    totalPnl: Number(raw.totalPnl ?? raw.total_pnl ?? 0),
  };
}

export function mapBotConfig(raw: RawBotConfig | null | undefined): BotConfig | null {
  if (!raw) return null;
  return {
    id: raw.id,
    botId: raw.botId ?? raw.bot_id ?? '',
    version: raw.version,
    createdAt: raw.createdAt ?? raw.created_at ?? '',
    name: raw.name ?? '',
    assistantStyle: (raw.assistantStyle ?? raw.assistant_style ?? 'beginner') as Persona,
    iconColor: raw.iconColor ?? raw.icon_color,
    assetFocus: (raw.assetFocus ?? raw.asset_focus ?? 'majors') as AssetFocus,
    customAssets: raw.customAssets ?? raw.custom_assets,
    algorithmMode: (raw.algorithmMode ?? raw.algorithm_mode ?? 'trend') as AlgorithmMode,
    strictness: (raw.strictness ?? 'medium') as Strictness,
    signalKnobs: (raw.signalKnobs ?? raw.signal_knobs) as BotConfig['signalKnobs'],
    riskCaps: (raw.riskCaps ?? raw.risk_caps) as BotConfig['riskCaps'],
    tradingMode: (raw.tradingMode ?? raw.trading_mode ?? 'paper') as TradingMode,
    llmProvider: (raw.llmProvider ?? raw.llm_provider ?? 'openai') as LlmProvider,
    llmModel: raw.llmModel ?? raw.llm_model,
    llmApiKey: raw.llmApiKey ?? raw.llm_api_key ?? '',
  } as BotConfig;
}

export function mapTradeableAsset(asset: RawTradeableAsset): TradeableAsset {
  return {
    id: asset.id,
    assetFocus: (asset.assetFocus ?? asset.asset_focus) as AssetFocus,
    symbol: asset.symbol,
    name: asset.name,
    tokenAddress: asset.tokenAddress ?? asset.token_address ?? '',
    decimals: Number(asset.decimals),
    custodian: asset.custodian ?? '',
    isActive: Boolean(asset.isActive ?? asset.is_active),
    createdAt: asset.createdAt ?? asset.created_at ?? '',
    updatedAt: asset.updatedAt ?? asset.updated_at ?? '',
  };
}

export function mapAssistantOption(option: RawAssistantOption): AIAssistantOption {
  return {
    id: option.id,
    assistantStyle: (option.assistantStyle ?? option.assistant_style) as Persona,
    captainName: option.captainName ?? option.captain_name ?? '',
    personalityDescription:
      option.personalityDescription ?? option.personality_description ?? '',
    imageKey: option.imageKey ?? option.image_key ?? '',
    imagePath: option.imagePath ?? option.image_path ?? '',
    sortOrder: Number(option.sortOrder ?? option.sort_order ?? 0),
    isActive: Boolean(option.isActive ?? option.is_active),
  };
}
