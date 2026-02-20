import type {
  Bot,
  BotConfig,
  BotEventType,
  CreateBotRequest,
  GetBotChatMessagesResponse,
  GetBotResponse,
  GetEventsResponse,
  GetMetricsResponse,
  ListBotsResponse,
  NameAvailability,
  PostBotChatMessageRequest,
  PostBotChatMessageResponse,
  BotActionRequest,
  TradeableAsset,
  AIAssistantOption,
  UpdateBotConfigRequest,
} from '@trawling-traders/types';
import { fetchApi } from './http';
import {
  mapBot,
  mapBotConfig,
  mapChatMessage,
  mapTradeableAsset,
  mapAssistantOption,
  RawBotEvent,
  RawMetricPoint,
} from './raw-types';

export const botApi = {
  // List all bots for current user
  async listBots(): Promise<ListBotsResponse> {
    const response = await fetchApi('/bots');
    return {
      total: response.total || (response.bots || []).length,
      bots: (response.bots || []).map(mapBot),
    };
  },

  async checkNameAvailability(name: string): Promise<NameAvailability> {
    const response = await fetchApi(`/bots/name-availability?name=${encodeURIComponent(name)}`);
    return {
      available: Boolean(response.available),
      normalizedName: response.normalizedName ?? response.normalized_name ?? name.trim(),
      suggestedName: response.suggestedName ?? response.suggested_name ?? undefined,
    };
  },

  async listTradeableAssets(): Promise<TradeableAsset[]> {
    const response = await fetchApi('/bots/tradeable-assets');
    return (response.assets || []).map(mapTradeableAsset);
  },

  async listAssistantOptions(): Promise<AIAssistantOption[]> {
    const response = await fetchApi('/bots/assistant-options');
    return (response.options || []).map(mapAssistantOption);
  },

  // Create a new bot
  async createBot(request: CreateBotRequest): Promise<Bot> {
    const response = await fetchApi('/bots', {
      method: 'POST',
      body: JSON.stringify({
        name: request.name,
        assistant_style: request.assistantStyle,
        asset_focus: request.assetFocus,
        custom_assets: request.customAssets,
        algorithm_mode: request.algorithmMode,
        algorithm_factors: request.algorithmFactors,
        strictness: request.strictness,
        risk_caps: {
          max_position_size_percent: request.riskCaps.maxPositionSizePercent,
          max_daily_loss_usd: request.riskCaps.maxDailyLossUsd,
          max_drawdown_percent: request.riskCaps.maxDrawdownPercent,
          max_trades_per_day: request.riskCaps.maxTradesPerDay,
        },
        trading_mode: request.tradingMode,
        llm_provider: request.llmProvider,
        llm_model: request.llmModel,
        llm_api_key: request.llmApiKey,
        telegram_enabled: request.telegramEnabled,
        telegram_bot_token: request.telegramBotToken,
        telegram_user_id: request.telegramUserId,
        telegram_pairing_code: request.telegramPairingCode,
      }),
    });
    return mapBot(response);
  },

  // Get bot details with config
  async getBot(botId: string): Promise<GetBotResponse> {
    const response = await fetchApi(`/bots/${botId}`);
    return {
      bot: mapBot(response.bot),
      config: mapBotConfig(response.config),
    };
  },

  // Update bot config
  async updateBotConfig(
    botId: string,
    request: UpdateBotConfigRequest
  ): Promise<BotConfig> {
    const response = await fetchApi(`/bots/${botId}/config`, {
      method: 'PATCH',
      body: JSON.stringify(request),
    });
    return mapBotConfig(response)!;
  },

  // Perform action on bot (pause/resume/redeploy/destroy)
  async botAction(
    botId: string,
    action: BotActionRequest['action']
  ): Promise<void> {
    return fetchApi(`/bots/${botId}/actions`, {
      method: 'POST',
      body: JSON.stringify({ action }),
    });
  },

  // Get bot metrics
  async getMetrics(botId: string): Promise<GetMetricsResponse> {
    const response = await fetchApi(`/bots/${botId}/metrics`);
    return {
      range: response.range || '7d',
      metrics: (response.metrics || []).map((metric: RawMetricPoint) => ({
        timestamp: metric.timestamp,
        value: Number(metric.value ?? metric.pnl ?? 0),
      })),
    };
  },

  // Get bot events
  async getEvents(botId: string): Promise<GetEventsResponse> {
    const response = await fetchApi(`/bots/${botId}/events`);
    return {
      nextCursor: response.nextCursor ?? response.next_cursor,
      events: (response.events || []).map((event: RawBotEvent) => ({
        id: event.id,
        botId: event.botId ?? event.bot_id ?? '',
        type: (event.type ?? event.event_type ?? 'status_change') as BotEventType,
        timestamp: event.timestamp ?? event.created_at ?? '',
        message: event.message ?? '',
        metadata: event.metadata,
      })),
    };
  },

  async getChatMessages(botId: string): Promise<GetBotChatMessagesResponse> {
    const response = await fetchApi(`/bots/${botId}/chat/messages`);
    return {
      messages: (response.messages || []).map(mapChatMessage),
    };
  },

  async postChatMessage(
    botId: string,
    request: PostBotChatMessageRequest
  ): Promise<PostBotChatMessageResponse> {
    const response = await fetchApi(`/bots/${botId}/chat/messages`, {
      method: 'POST',
      body: JSON.stringify(request),
    });
    return {
      userMessage: mapChatMessage(response.user_message),
      assistantMessage: mapChatMessage(response.assistant_message),
    };
  },
};
