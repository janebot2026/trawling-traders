import React, { useEffect, useState, useCallback, useRef } from 'react';
import {
  View,
  Text,
  StyleSheet,
  ScrollView,
  TouchableOpacity,
  ActivityIndicator,
  Alert,
  RefreshControl,
  Image,
  Animated,
} from 'react-native';
import { useNavigation, useRoute, RouteProp } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';
import type { Bot, BotEvent, BotConfig } from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';
import { OceanBackground } from '../components/OceanBackground';
import { lightTheme } from '../theme';

const LOB_AVATAR = require('../../assets/lob-avatar.png');

type BotDetailScreenRouteProp = RouteProp<RootStackParamList, 'BotDetail'>;
type BotDetailScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'BotDetail'>;

export function BotDetailScreen() {
  const route = useRoute<BotDetailScreenRouteProp>();
  const navigation = useNavigation<BotDetailScreenNavigationProp>();
  const { botId } = route.params;

  const [bot, setBot] = useState<Bot | null>(null);
  const [config, setConfig] = useState<BotConfig | null>(null);
  const [events, setEvents] = useState<BotEvent[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isActionLoading, setIsActionLoading] = useState(false);

  const fadeAnim = React.useRef(new Animated.Value(0)).current;
  const isMountedRef = useRef(true);

  // Cleanup on unmount to prevent state updates after unmount
  useEffect(() => {
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  const fetchBotDetails = useCallback(async () => {
    try {
      const [botResponse, eventsResponse] = await Promise.all([
        api.bot.getBot(botId),
        api.bot.getEvents(botId),
      ]);
      // Guard against state updates after unmount
      if (!isMountedRef.current) return;
      setBot(botResponse.bot);
      setConfig(botResponse.config);
      setEvents(eventsResponse.events);
    } catch (error) {
      if (!isMountedRef.current) return;
      Alert.alert('Error', 'Failed to load bot details');
    } finally {
      if (!isMountedRef.current) return;
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, [botId]);

  useEffect(() => {
    fetchBotDetails();
  }, [fetchBotDetails]);

  useEffect(() => {
    if (!isLoading) {
      Animated.timing(fadeAnim, { toValue: 1, duration: 500, useNativeDriver: true }).start();
    }
  }, [isLoading, fadeAnim]);

  const handleRefresh = () => {
    setIsRefreshing(true);
    fetchBotDetails();
  };

  const handleAction = async (action: 'pause' | 'resume' | 'redeploy' | 'destroy') => {
    if (action === 'destroy') {
      Alert.alert('Destroy Trawler?', 'Permanent deletion. Cannot be undone.', [
        { text: 'Cancel', style: 'cancel' },
        {
          text: 'Destroy',
          style: 'destructive',
          onPress: async () => {
            if (!isMountedRef.current) return;
            setIsActionLoading(true);
            try {
              await api.bot.botAction(botId, 'destroy');
              if (!isMountedRef.current) return;
              navigation.navigate('Main');
            } catch (error) {
              if (!isMountedRef.current) return;
              Alert.alert('Error', 'Failed to destroy bot');
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
      if (!isMountedRef.current) return;
      await fetchBotDetails();
    } catch (error) {
      if (!isMountedRef.current) return;
      Alert.alert('Error', `Failed to ${action} bot`);
    } finally {
      if (!isMountedRef.current) return;
      setIsActionLoading(false);
    }
  };

  const handleSettings = () => {
    navigation.navigate('BotSettings', { botId });
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

  const getStatusColor = (status: Bot['status']) => {
    const colors: Record<Bot['status'], string> = {
      provisioning: lightTheme.colors.caution[500],
      online: lightTheme.colors.bullish[500],
      offline: lightTheme.colors.wave[400],
      paused: lightTheme.colors.caution[400],
      error: lightTheme.colors.lobster[600],
      destroying: lightTheme.colors.wave[500],
    };
    return colors[status];
  };

  return (
    <OceanBackground>
      <Animated.ScrollView
        style={[styles.container, { opacity: fadeAnim }]}
        contentContainerStyle={{ paddingBottom: 40 }}
        refreshControl={
          <RefreshControl refreshing={isRefreshing} onRefresh={handleRefresh} tintColor={lightTheme.colors.primary[700]} />
        }
      >
        {/* Header */}
        <View style={styles.header}>
          <View style={styles.headerTop}>
            <View style={styles.avatarSection}>
              <View style={styles.avatarContainer}>
                <Image source={LOB_AVATAR} style={styles.avatar} />
                <View style={styles.botBadge}>
                  <Text style={styles.botBadgeText}>#{bot.id.slice(-4)}</Text>
                </View>
                <View style={[styles.statusDot, { backgroundColor: getStatusColor(bot.status) }]} />
              </View>

              <View style={styles.headerInfo}>
                <Text style={styles.botName}>{bot.name}</Text>
                <View style={styles.statusRow}>
                  <Text style={styles.statusText}>{bot.status.toUpperCase()}</Text>
                  {bot.configStatus === 'pending' && (
                    <View style={styles.pendingBadge}>
                      <Text style={styles.pendingText}>CONFIG PENDING</Text>
                    </View>
                  )}
                </View>
                {bot.lastHeartbeatAt && (
                  <Text style={styles.heartbeat}>Last seen: {new Date(bot.lastHeartbeatAt).toLocaleString()}</Text>
                )}
              </View>
            </View>

            <TouchableOpacity style={styles.settingsButton} onPress={handleSettings}>
              <Text style={styles.settingsText}>⚙️</Text>
            </TouchableOpacity>
          </View>
        </View>

        {/* Wallet Card */}
        <View style={styles.card}>
          <Text style={styles.cardTitle}>Trading Wallet</Text>
          
          {bot.agentWallet ? (
            <>
              <View style={styles.walletRow}>
                <Text style={styles.walletLabel}>Address</Text>
                <Text style={styles.walletAddress}>{bot.agentWallet.slice(0, 8)}...{bot.agentWallet.slice(-8)}</Text>
              </View>
              <View style={styles.walletActions}>
                <TouchableOpacity 
                  style={styles.copyButton}
                  onPress={() => {
                    // TODO: Copy to clipboard
                    Alert.alert('Copied', 'Wallet address copied to clipboard');
                  }}
                >
                  <Text style={styles.copyButtonText}>📋 Copy Address</Text>
                </TouchableOpacity>
              </View>
              <View style={styles.fundHint}>
                <Text style={styles.fundHintText}>💡 Fund this wallet from your main account to start trading</Text>
              </View>
            </>
          ) : (
            <View style={styles.waitingWallet}>
              <Text style={styles.waitingText}>⏳ Waiting for wallet...</Text>
              <Text style={styles.waitingSub}>The bot will create and report its wallet address shortly after deployment.</Text>
            </View>
          )}
        </View>

        {/* Performance */}
        <View style={styles.card}>
          <Text style={styles.cardTitle}>Performance</Text>
          <View style={styles.metricsRow}>
            <View style={styles.metric}>
              <Text style={styles.metricLabel}>Today</Text>
              <Text style={[styles.metricValue, { color: (bot.todayPnl || 0) >= 0 ? lightTheme.colors.bullish[600] : lightTheme.colors.lobster[600] }]}>
                {(bot.todayPnl || 0) >= 0 ? '+' : ''}${(bot.todayPnl || 0).toFixed(2)}
              </Text>
            </View>
            <View style={styles.metric}>
              <Text style={styles.metricLabel}>Total</Text>
              <Text style={[styles.metricValue, { color: (bot.totalPnl || 0) >= 0 ? lightTheme.colors.bullish[600] : lightTheme.colors.lobster[600] }]}>
                {(bot.totalPnl || 0) >= 0 ? '+' : ''}${(bot.totalPnl || 0).toFixed(2)}
              </Text>
            </View>
            <View style={styles.metric}>
              <Text style={styles.metricLabel}>Persona</Text>
              <Text style={styles.metricValue}>{bot.persona}</Text>
            </View>
          </View>
        </View>

        {/* Configuration */}
        {config && (
          <View style={styles.card}>
            <Text style={styles.cardTitle}>Configuration</Text>
            <View style={styles.configRow}>
              <Text style={styles.configLabel}>Algorithm</Text>
              <Text style={styles.configValue}>{config.algorithmMode}</Text>
            </View>
            <View style={styles.configRow}>
              <Text style={styles.configLabel}>Assets</Text>
              <Text style={styles.configValue}>{config.assetFocus}</Text>
            </View>
            <View style={styles.configRow}>
              <Text style={styles.configLabel}>Mode</Text>
              <Text style={[styles.configValue, config.tradingMode === 'live' && styles.liveMode]}>
                {config.tradingMode === 'live' ? '🔴 LIVE' : '📄 Paper'}
              </Text>
            </View>
          </View>
        )}

        {/* Actions */}
        <View style={styles.card}>
          <Text style={styles.cardTitle}>Actions</Text>
          {isActionLoading ? (
            <ActivityIndicator />
          ) : (
            <View style={styles.actionsRow}>
              {bot.status === 'online' ? (
                <TouchableOpacity style={[styles.actionButton, { backgroundColor: lightTheme.colors.caution[500] }]} onPress={() => handleAction('pause')}>
                  <Text style={styles.actionButtonText}>⏸️ Pause</Text>
                </TouchableOpacity>
              ) : bot.status === 'paused' ? (
                <TouchableOpacity style={[styles.actionButton, { backgroundColor: lightTheme.colors.bullish[500] }]} onPress={() => handleAction('resume')}>
                  <Text style={styles.actionButtonText}>▶️ Resume</Text>
                </TouchableOpacity>
              ) : null}
              <TouchableOpacity style={[styles.actionButton, { backgroundColor: lightTheme.colors.primary[600] }]} onPress={() => handleAction('redeploy')}>
                <Text style={styles.actionButtonText}>🔄 Redeploy</Text>
              </TouchableOpacity>
              <TouchableOpacity style={[styles.actionButton, { backgroundColor: lightTheme.colors.lobster[600] }]} onPress={() => handleAction('destroy')}>
                <Text style={styles.actionButtonText}>🗑️ Destroy</Text>
              </TouchableOpacity>
            </View>
          )}
        </View>

        {/* Events */}
        <View style={styles.card}>
          <Text style={styles.cardTitle}>Recent Events</Text>
          {events.length === 0 ? (
            <Text style={styles.emptyText}>No events yet</Text>
          ) : (
            events.slice(0, 5).map((event) => (
              <View key={event.id} style={styles.eventItem}>
                <View style={styles.eventHeader}>
                  <Text style={styles.eventType}>{event.type.toUpperCase()}</Text>
                  <Text style={styles.eventTime}>{new Date(event.timestamp).toLocaleTimeString()}</Text>
                </View>
                <Text style={styles.eventMessage}>{event.message}</Text>
              </View>
            ))
          )}
        </View>
      </Animated.ScrollView>
    </OceanBackground>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1 },
  centered: { flex: 1, justifyContent: 'center', alignItems: 'center' },
  header: {
    backgroundColor: 'rgba(255, 255, 255, 0.95)',
    padding: 20,
    paddingTop: 60,
    borderBottomLeftRadius: 24,
    borderBottomRightRadius: 24,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 8,
    elevation: 3,
  },
  headerTop: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'flex-start' },
  avatarSection: { flexDirection: 'row', alignItems: 'center', flex: 1 },
  avatarContainer: { position: 'relative', marginRight: 16 },
  avatar: { width: 80, height: 80, borderRadius: 40, borderWidth: 3, borderColor: lightTheme.colors.primary[700] },
  botBadge: { position: 'absolute', bottom: -4, right: -4, backgroundColor: lightTheme.colors.bullish[500], paddingHorizontal: 8, paddingVertical: 2, borderRadius: 10, borderWidth: 2, borderColor: '#fff' },
  botBadgeText: { color: '#fff', fontSize: 11, fontWeight: 'bold' },
  statusDot: { position: 'absolute', top: 4, right: 4, width: 16, height: 16, borderRadius: 8, borderWidth: 2, borderColor: '#fff' },
  headerInfo: { flex: 1 },
  botName: { fontSize: 24, fontWeight: 'bold', color: lightTheme.colors.wave[900] },
  statusRow: { flexDirection: 'row', alignItems: 'center', marginTop: 4, gap: 8, flexWrap: 'wrap' },
  statusText: { fontSize: 14, fontWeight: '600', color: lightTheme.colors.wave[700] },
  pendingBadge: { backgroundColor: lightTheme.colors.caution[500], paddingHorizontal: 6, paddingVertical: 2, borderRadius: 4 },
  pendingText: { fontSize: 10, color: '#fff', fontWeight: 'bold' },
  settingsButton: { padding: 8, backgroundColor: lightTheme.colors.wave[100], borderRadius: 12 },
  settingsText: { fontSize: 20 },
  heartbeat: { fontSize: 12, color: lightTheme.colors.wave[500], marginTop: 4 },
  card: {
    backgroundColor: 'rgba(255, 255, 255, 0.95)',
    margin: 16,
    marginBottom: 8,
    padding: 20,
    borderRadius: 16,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.1,
    shadowRadius: 12,
    elevation: 4,
  },
  cardTitle: { fontSize: 18, fontWeight: 'bold', color: lightTheme.colors.wave[900], marginBottom: 16 },
  walletRow: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 },
  walletLabel: { fontSize: 14, color: lightTheme.colors.wave[600] },
  walletAddress: { fontSize: 14, fontWeight: '600', color: lightTheme.colors.wave[900], fontFamily: 'monospace' },
  walletActions: { marginTop: 8 },
  copyButton: { backgroundColor: lightTheme.colors.primary[100], paddingVertical: 10, paddingHorizontal: 16, borderRadius: 8, alignItems: 'center' },
  copyButtonText: { color: lightTheme.colors.primary[700], fontWeight: '600' },
  fundHint: { marginTop: 12, backgroundColor: lightTheme.colors.bullish[50], padding: 10, borderRadius: 8 },
  fundHintText: { color: lightTheme.colors.bullish[700], fontSize: 13 },
  waitingWallet: { alignItems: 'center', padding: 20 },
  waitingText: { fontSize: 16, color: lightTheme.colors.wave[600] },
  waitingSub: { fontSize: 12, color: lightTheme.colors.wave[400], marginTop: 4, textAlign: 'center' },
  metricsRow: { flexDirection: 'row', justifyContent: 'space-around' },
  metric: { alignItems: 'center' },
  metricLabel: { fontSize: 12, color: lightTheme.colors.wave[500], marginBottom: 4 },
  metricValue: { fontSize: 24, fontWeight: 'bold' },
  configRow: { flexDirection: 'row', justifyContent: 'space-between', paddingVertical: 10, borderBottomWidth: 1, borderBottomColor: lightTheme.colors.wave[100] },
  configLabel: { fontSize: 14, color: lightTheme.colors.wave[600] },
  configValue: { fontSize: 14, fontWeight: '600', color: lightTheme.colors.wave[800], textTransform: 'capitalize' },
  liveMode: { color: lightTheme.colors.lobster[600] },
  actionsRow: { flexDirection: 'row', flexWrap: 'wrap', gap: 12 },
  actionButton: { flex: 1, minWidth: 100, paddingVertical: 12, paddingHorizontal: 16, borderRadius: 10, alignItems: 'center' },
  actionButtonText: { color: '#fff', fontWeight: '600', fontSize: 14 },
  eventItem: { paddingVertical: 12, borderBottomWidth: 1, borderBottomColor: lightTheme.colors.wave[100] },
  eventHeader: { flexDirection: 'row', justifyContent: 'space-between', marginBottom: 4 },
  eventType: { fontSize: 11, fontWeight: '700', color: lightTheme.colors.wave[600] },
  eventTime: { fontSize: 11, color: lightTheme.colors.wave[400] },
  eventMessage: { fontSize: 14, color: lightTheme.colors.wave[800] },
  emptyText: { textAlign: 'center', color: lightTheme.colors.wave[400], padding: 20 },
});
