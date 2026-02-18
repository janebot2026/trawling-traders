import React, { useCallback, useEffect, useMemo, useState } from 'react';
import {
  ActivityIndicator,
  Alert,
  Image,
  RefreshControl,
  ScrollView,

  Text,
  TextInput,
  TouchableOpacity,
  View,
} from 'react-native';
import * as Clipboard from 'expo-clipboard';
import { RouteProp, useNavigation, useRoute } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { Bot, BotChatMessage, BotConfig, BotEvent, MetricPoint } from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';
import { OceanBackground } from '../components/OceanBackground';
import { PnlHistoryChart } from '../components/PnlHistoryChart';
import type { RootStackParamList } from '../navigation/AppNavigator';
import { lightTheme } from '../theme';
import { styles } from './BotDetailScreen.styles';

const LOB_AVATAR = require('../../assets/lob-avatar.png');

type BotDetailRouteProp = RouteProp<RootStackParamList, 'BotDetail'>;
type BotDetailNavigationProp = NativeStackNavigationProp<RootStackParamList, 'BotDetail'>;

export function BotDetailScreen() {
  const route = useRoute<BotDetailRouteProp>();
  const navigation = useNavigation<BotDetailNavigationProp>();
  const { botId } = route.params;

  const [bot, setBot] = useState<Bot | null>(null);
  const [config, setConfig] = useState<BotConfig | null>(null);
  const [events, setEvents] = useState<BotEvent[]>([]);
  const [metrics, setMetrics] = useState<MetricPoint[]>([]);
  const [messages, setMessages] = useState<BotChatMessage[]>([]);
  const [draftMessage, setDraftMessage] = useState('');
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isSending, setIsSending] = useState(false);
  const [isActionLoading, setIsActionLoading] = useState(false);

  const fetchBotDetails = useCallback(async () => {
    try {
      const [botResponse, eventsResponse, metricsResponse, messagesResponse] = await Promise.all([
        api.bot.getBot(botId),
        api.bot.getEvents(botId),
        api.bot.getMetrics(botId),
        api.bot.getChatMessages(botId).catch((err: Error) => {
          console.warn('Chat messages fetch failed:', err.message);
          return { messages: [] };
        }),
      ]);

      setBot(botResponse.bot);
      setConfig(botResponse.config);
      setEvents(eventsResponse.events);
      setMetrics(metricsResponse.metrics);
      setMessages(messagesResponse.messages);
    } catch {
      Alert.alert('Error', 'Failed to load bot details.');
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, [botId]);

  useEffect(() => {
    fetchBotDetails();
  }, [fetchBotDetails]);

  const refresh = () => {
    setIsRefreshing(true);
    fetchBotDetails();
  };

  const tradeEvents = useMemo(
    () => events.filter((event) => event.type === 'trade_opened' || event.type === 'trade_closed'),
    [events]
  );

  const handleSettings = () => {
    navigation.navigate('BotBehaviorConfig', { botId });
  };

  const handleAction = async (action: 'pause' | 'resume' | 'redeploy' | 'destroy') => {
    if (!bot) return;

    if (action === 'destroy') {
      Alert.alert('Destroy Bot?', 'This action cannot be undone.', [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Destroy',
          style: 'destructive',
          onPress: async () => {
            setIsActionLoading(true);
            try {
              await api.bot.botAction(botId, 'destroy');
              navigation.navigate('Main');
            } catch {
              Alert.alert('Error', 'Failed to destroy bot.');
              setIsActionLoading(false);
            }
          },
        },
      ]);
      return;
    }

    setIsActionLoading(true);
    try {
      await api.bot.botAction(botId, action);
      await fetchBotDetails();
    } catch {
      Alert.alert('Error', `Failed to ${action} bot.`);
    } finally {
      setIsActionLoading(false);
    }
  };

  const sendMessage = async () => {
    const content = draftMessage.trim();
    if (!content || isSending) return;

    setIsSending(true);
    setDraftMessage('');

    const optimisticUser: BotChatMessage = {
      id: `local-user-${Date.now()}`,
      botId,
      role: 'user',
      content,
      timestamp: new Date().toISOString(),
    };
    setMessages((prev) => [...prev, optimisticUser]);

    try {
      const response = await api.bot.postChatMessage(botId, { content });
      setMessages((prev) => {
        const withoutOptimistic = prev.filter((m) => m.id !== optimisticUser.id);
        return [...withoutOptimistic, response.userMessage, response.assistantMessage];
      });
    } catch {
      setMessages((prev) => prev.filter((m) => m.id !== optimisticUser.id));
      Alert.alert('Chat Error', 'Failed to send message. Check bot LLM configuration.');
      setDraftMessage(content);
    } finally {
      setIsSending(false);
    }
  };

  if (isLoading || !bot) {
    return (
      <OceanBackground>
        <View style={styles.centered}>
          <ActivityIndicator size="large" color={lightTheme.colors.primary[700]} />
        </View>
      </OceanBackground>
    );
  }

  const totalPnl = bot.totalPnl || 0;

  return (
    <OceanBackground>
      <ScrollView
        contentContainerStyle={styles.content}
        refreshControl={
          <RefreshControl
            refreshing={isRefreshing}
            onRefresh={refresh}
            tintColor={lightTheme.colors.primary[700]}
          />
        }
      >
        <View style={styles.header}>
          <View style={styles.headerLeft}>
            <Image source={LOB_AVATAR} style={styles.avatar} />
            <View>
              <Text style={styles.botName}>{bot.name}</Text>
              <Text style={styles.metaText}>{bot.status.toUpperCase()}</Text>
            </View>
          </View>
          <TouchableOpacity style={styles.settingsButton} onPress={handleSettings}>
            <Text style={styles.settingsText}>⚙️</Text>
          </TouchableOpacity>
        </View>

        <View style={styles.card}>
          <View style={styles.cardHeaderRow}>
            <Text style={styles.cardTitle}>P&L Overview</Text>
            <Text style={[styles.pnlValue, { color: totalPnl >= 0 ? lightTheme.colors.bullish[600] : lightTheme.colors.lobster[600] }]}>
              {totalPnl >= 0 ? '+' : ''}${totalPnl.toFixed(2)}
            </Text>
          </View>
          <PnlHistoryChart metrics={metrics} />
        </View>

        <TouchableOpacity
          style={styles.card}
          activeOpacity={0.9}
          onPress={() => navigation.navigate('BotStrategyConfig', { botId })}
        >
          <View style={styles.cardHeaderRow}>
            <Text style={styles.cardTitle}>Strategy</Text>
            <Text style={styles.linkLabel}>Configure</Text>
          </View>
          <Text style={styles.sectionText}>Algorithm: {config?.algorithmMode}</Text>
          <Text style={styles.sectionText}>Asset Focus: {config?.assetFocus}</Text>
          <Text style={styles.sectionText}>Strictness: {config?.strictness}</Text>
        </TouchableOpacity>

        <View style={styles.card}>
          <Text style={styles.cardTitle}>Trading Wallet</Text>
          {bot.agentWallet ? (
            <>
              <Text style={styles.walletAddress}>{bot.agentWallet}</Text>
              <TouchableOpacity
                style={styles.secondaryButton}
                onPress={async () => {
                  await Clipboard.setStringAsync(bot.agentWallet || '');
                  Alert.alert('Copied', 'Wallet address copied.');
                }}
              >
                <Text style={styles.secondaryButtonText}>Copy Wallet Address</Text>
              </TouchableOpacity>
            </>
          ) : (
            <Text style={styles.sectionText}>Wallet is still being provisioned.</Text>
          )}
        </View>

        <View style={styles.card}>
          <Text style={styles.cardTitle}>Actions</Text>
          {isActionLoading ? (
            <ActivityIndicator color={lightTheme.colors.primary[700]} />
          ) : (
            <View style={styles.actionRow}>
              {bot.status === 'online' && (
                <TouchableOpacity style={styles.actionButtonWarn} onPress={() => handleAction('pause')}>
                  <Text style={styles.actionButtonText}>Pause</Text>
                </TouchableOpacity>
              )}
              {bot.status === 'paused' && (
                <TouchableOpacity style={styles.actionButtonOk} onPress={() => handleAction('resume')}>
                  <Text style={styles.actionButtonText}>Resume</Text>
                </TouchableOpacity>
              )}
              <TouchableOpacity style={styles.actionButtonNeutral} onPress={() => handleAction('redeploy')}>
                <Text style={styles.actionButtonText}>Redeploy</Text>
              </TouchableOpacity>
              <TouchableOpacity style={styles.actionButtonDanger} onPress={() => handleAction('destroy')}>
                <Text style={styles.actionButtonText}>Destroy</Text>
              </TouchableOpacity>
            </View>
          )}
        </View>

        <View style={styles.card}>
          <Text style={styles.cardTitle}>Trade History</Text>
          {tradeEvents.length === 0 ? (
            <Text style={styles.sectionText}>No trades yet.</Text>
          ) : (
            tradeEvents.slice(0, 12).map((event) => (
              <View key={event.id} style={styles.historyItem}>
                <Text style={styles.historyType}>{event.type === 'trade_opened' ? 'OPEN' : 'CLOSE'}</Text>
                <Text style={styles.historyMessage}>{event.message}</Text>
                <Text style={styles.historyTime}>{new Date(event.timestamp).toLocaleString()}</Text>
              </View>
            ))
          )}
        </View>

        <View style={styles.card}>
          <Text style={styles.cardTitle}>Chat History</Text>
          {messages.length === 0 ? (
            <Text style={styles.sectionText}>No conversation yet. Ask this bot about its plan.</Text>
          ) : (
            messages.slice(-30).map((message) => (
              <View
                key={message.id}
                style={[styles.chatBubble, message.role === 'user' ? styles.userBubble : styles.assistantBubble]}
              >
                <Text style={styles.chatRole}>{message.role === 'user' ? 'You' : 'Bot'}</Text>
                <Text style={styles.chatText}>{message.content}</Text>
                <Text style={styles.chatTime}>{new Date(message.timestamp).toLocaleTimeString()}</Text>
              </View>
            ))
          )}

          <View style={styles.chatComposer}>
            <TextInput
              value={draftMessage}
              onChangeText={setDraftMessage}
              placeholder="Ask this bot about strategy, risk, or recent behavior..."
              placeholderTextColor={lightTheme.colors.wave[400]}
              style={styles.chatInput}
              multiline
            />
            <TouchableOpacity
              style={[styles.sendButton, isSending && styles.sendButtonDisabled]}
              onPress={sendMessage}
              disabled={isSending}
            >
              <Text style={styles.sendButtonText}>{isSending ? '...' : 'Send'}</Text>
            </TouchableOpacity>
          </View>
        </View>
      </ScrollView>
    </OceanBackground>
  );
}

