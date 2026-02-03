import React, { useEffect, useState, useCallback } from 'react';
import {
  View,
  Text,
  StyleSheet,
  ScrollView,
  TouchableOpacity,
  Button,
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

// LOB Avatar - default bot image
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

  // Animation values
  const fadeAnim = React.useRef(new Animated.Value(0)).current;
  const slideAnim = React.useRef(new Animated.Value(30)).current;

  const fetchBotDetails = useCallback(async () => {
    try {
      const [botResponse, eventsResponse] = await Promise.all([
        api.bot.getBot(botId),
        api.bot.getEvents(botId),
      ]);
      setBot(botResponse.bot);
      setConfig(botResponse.config);
      setEvents(eventsResponse.events);
    } catch (error) {
      console.error('Failed to fetch bot details:', error);
      Alert.alert('Error', 'Failed to load bot details');
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, [botId]);

  useEffect(() => {
    fetchBotDetails();
  }, [fetchBotDetails]);

  useEffect(() => {
    if (!isLoading) {
      Animated.parallel([
        Animated.timing(fadeAnim, {
          toValue: 1,
          duration: 500,
          useNativeDriver: true,
        }),
        Animated.timing(slideAnim, {
          toValue: 0,
          duration: 500,
          useNativeDriver: true,
        }),
      ]).start();
    }
  }, [isLoading]);

  const handleRefresh = () => {
    setIsRefreshing(true);
    fetchBotDetails();
  };

  const handleAction = async (action: 'pause' | 'resume' | 'redeploy' | 'destroy') => {
    const actionLabels: Record<string, string> = {
      pause: 'pause',
      resume: 'resume',
      redeploy: 'redeploy',
      destroy: 'destroy',
    };

    if (action === 'destroy') {
      Alert.alert(
        'Destroy Trawler?',
        'This will permanently delete the bot and all its data. This cannot be undone.',
        [
          { text: 'Cancel', style: 'cancel' },
          {
            text: 'Destroy',
            style: 'destructive',
            onPress: async () => {
              setIsActionLoading(true);
              try {
                await api.bot.botAction(botId, 'destroy');
                navigation.navigate('Main');
              } catch (error) {
                Alert.alert('Error', 'Failed to destroy bot');
                setIsActionLoading(false);
              }
            },
          },
        ]
      );
      return;
    }

    setIsActionLoading(true);
    try {
      await api.bot.botAction(botId, action);
      await fetchBotDetails();
      Alert.alert('Success', `Trawler ${actionLabels[action]}ed successfully`);
    } catch (error) {
      Alert.alert('Error', `Failed to ${actionLabels[action]} bot`);
    } finally {
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

  const formatEventType = (type: string) => {
    return type.replace(/_/g, ' ').toUpperCase();
  };

  return (
    <OceanBackground>
      <Animated.ScrollView
        style={[styles.container, { opacity: fadeAnim }]}
        contentContainerStyle={{ paddingBottom: 40 }}
        refreshControl={
          <RefreshControl
            refreshing={isRefreshing}
            onRefresh={handleRefresh}
            tintColor={lightTheme.colors.primary[700]}
          />
        }
      >
        {/* Header with LOB Avatar */}
        <View style={styles.header}>
          <View style={styles.headerTop}>
            <View style={styles.avatarSection}>
              <View style={styles.avatarContainer}>
                <Image source={LOB_AVATAR} style={styles.avatar} />
                <View style={styles.botBadge}>
                  <Text style={styles.botBadgeText}>#{bot.id.slice(-4)}</Text>
                </View>
                <View
                  style={[
                    styles.statusDot,
                    { backgroundColor: getStatusColor(bot.status) },
                  ]}
                />
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
                  <Text style={styles.heartbeat}>
                    Last seen: {new Date(bot.lastHeartbeatAt).toLocaleString()}
                  </Text>
                )}
              </View>
            </View>

            <TouchableOpacity style={styles.settingsButton} onPress={handleSettings}>
              <Text style={styles.settingsText}>⚙️</Text>
            </TouchableOpacity>
          </View>
        </View>

        {/* Performance */}
        <Animated.View
          style={[
            styles.card,
            { transform: [{ translateY: slideAnim }] },
          ]}
        >
          <Text style={styles.cardTitle}>Performance</Text>

          <View style={styles.metricsRow}>
            <View style={styles.metric}>
              <Text style={styles.metricLabel}>Today</Text>
              <Text
                style={[
                  styles.metricValue,
                  {
                    color:
                      (bot.todayPnl || 0) >= 0
                        ? lightTheme.colors.bullish[600]
                        : lightTheme.colors.lobster[600],
                  },
                ]}
              >
                {(bot.todayPnl || 0) >= 0 ? '+' : ''}
                ${(bot.todayPnl || 0).toFixed(2)}
              </Text>
            </View>

            <View style={styles.metric}>
              <Text style={styles.metricLabel}>Total</Text>
              <Text
                style={[
                  styles.metricValue,
                  {
                    color:
                      (bot.totalPnl || 0) >= 0
                        ? lightTheme.colors.bullish[600]
                        : lightTheme.colors.lobster[600],
                  },
                ]}
              >
                {(bot.totalPnl || 0) >= 0 ? '+' : ''}
                ${(bot.totalPnl || 0).toFixed(2)}
              </Text>
            </View>

            <View style={styles.metric}>
              <Text style={styles.metricLabel}>Persona</Text>
              <Text style={styles.metricValue}>{bot.persona}</Text>
            </View>
          </View>

          {/* Chart placeholder */}
          <View style={styles.chartContainer}>
            <View style={styles.chartPlaceholder}>
              <Text style={styles.chartText}>📈 Equity Curve</Text>
              <Text style={styles.chartSubtext}>Coming soon</Text>
            </View>
          </View>
        </Animated.View>

        {/* Configuration Summary */}
        {config && (
          <View style={styles.card}>
            <Text style={styles.cardTitle}>Configuration</Text>

            <View style={styles.configRow}>
              <Text style={styles.configLabel}>Algorithm</Text>
              <Text style={styles.configValue}>{config.algorithmMode}</Text>
            </View>

            <View style={styles.configRow}>
              <Text style={styles.configLabel}>Asset Focus</Text>
              <Text style={styles.configValue}>{config.assetFocus}</Text>
            </View>

            <View style={styles.configRow}>
              <Text style={styles.configLabel}>Trading Mode</Text>
              <Text
                style={[
                  styles.configValue,
                  config.tradingMode === 'live' && styles.liveMode,
                ]}
              >
                {config.tradingMode === 'live' ? '🔴 LIVE' : '📄 Paper'}
              </Text>
            </View>

            <View style={styles.configRow}>
              <Text style={styles.configLabel}>Max Position</Text>
              <Text style={styles.configValue}>
                {config.riskCaps?.maxPositionSizePercent || 5}%
              </Text>
            </View>

            <View style={styles.configRow}>
              <Text style={styles.configLabel}>Max Daily Loss</Text>
              <Text style={styles.configValue}>
                ${config.riskCaps?.maxDailyLossUsd || 50}
              </Text>
            </View>
          </View>
        )}

        {/* Actions */}
        <View style={styles.card}>
          <Text style={styles.cardTitle}>Actions</Text>

          {isActionLoading ? (
            <ActivityIndicator style={styles.actionLoading} />
          ) : (
            <View style={styles.actionsRow}>
              {bot.status === 'online' ? (
                <TouchableOpacity
                  style={[styles.actionButton, { backgroundColor: lightTheme.colors.caution[500] }]}
                  onPress={() => handleAction('pause')}
                >
                  <Text style={styles.actionButtonText}>⏸️ Pause</Text>
                </TouchableOpacity>
              ) : bot.status === 'paused' ? (
                <TouchableOpacity
                  style={[styles.actionButton, { backgroundColor: lightTheme.colors.bullish[500] }]}
                  onPress={() => handleAction('resume')}
                >
                  <Text style={styles.actionButtonText}>▶️ Resume</Text>
                </TouchableOpacity>
              ) : null}

              <TouchableOpacity
                style={[styles.actionButton, { backgroundColor: lightTheme.colors.primary[600] }]}
                onPress={() => handleAction('redeploy')}
              >
                <Text style={styles.actionButtonText}>🔄 Redeploy</Text>
              </TouchableOpacity>

              <TouchableOpacity
                style={[styles.actionButton, { backgroundColor: lightTheme.colors.lobster[600] }]}
                onPress={() => handleAction('destroy')}
              >
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
            events.slice(0, 10).map((event, index) => (
              <View
                key={event.id}
                style={[
                  styles.eventItem,
                  index === events.length - 1 && { borderBottomWidth: 0 },
                ]}
              >
                <View style={styles.eventHeader}>
                  <Text style={styles.eventType}>{formatEventType(event.type)}</Text>
                  <Text style={styles.eventTime}>
                    {new Date(event.timestamp).toLocaleTimeString()}
                  </Text>
                </View>
                <Text style={styles.eventMessage}>{event.message}</Text>
              </View>
            ))
          )}
        </View>

        {/* Infrastructure */}
        {bot.dropletId && (
          <View style={styles.card}>
            <Text style={styles.cardTitle}>Infrastructure</Text>

            <View style={styles.configRow}>
              <Text style={styles.configLabel}>Region</Text>
              <Text style={styles.configValue}>{bot.region}</Text>
            </View>

            <View style={styles.configRow}>
              <Text style={styles.configLabel}>Droplet ID</Text>
              <Text style={[styles.configValue, styles.mono]}>{bot.dropletId}</Text>
            </View>

            {bot.ipAddress && (
              <View style={styles.configRow}>
                <Text style={styles.configLabel}>IP Address</Text>
                <Text style={[styles.configValue, styles.mono]}>{bot.ipAddress}</Text>
              </View>
            )}
          </View>
        )}
      </Animated.ScrollView>
    </OceanBackground>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  centered: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
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
  headerTop: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'flex-start',
  },
  avatarSection: {
    flexDirection: 'row',
    alignItems: 'center',
    flex: 1,
  },
  avatarContainer: {
    position: 'relative',
    marginRight: 16,
  },
  avatar: {
    width: 80,
    height: 80,
    borderRadius: 40,
    borderWidth: 3,
    borderColor: lightTheme.colors.primary[700],
  },
  botBadge: {
    position: 'absolute',
    bottom: -4,
    right: -4,
    backgroundColor: lightTheme.colors.bullish[500],
    paddingHorizontal: 8,
    paddingVertical: 2,
    borderRadius: 10,
    borderWidth: 2,
    borderColor: '#fff',
  },
  botBadgeText: {
    color: '#fff',
    fontSize: 11,
    fontWeight: 'bold',
  },
  statusDot: {
    position: 'absolute',
    top: 4,
    right: 4,
    width: 16,
    height: 16,
    borderRadius: 8,
    borderWidth: 2,
    borderColor: '#fff',
  },
  headerInfo: {
    flex: 1,
  },
  botName: {
    fontSize: 24,
    fontWeight: 'bold',
    color: lightTheme.colors.wave[900],
  },
  statusRow: {
    flexDirection: 'row',
    alignItems: 'center',
    marginTop: 4,
    gap: 8,
    flexWrap: 'wrap',
  },
  statusText: {
    fontSize: 14,
    fontWeight: '600',
    color: lightTheme.colors.wave[700],
  },
  pendingBadge: {
    backgroundColor: lightTheme.colors.caution[500],
    paddingHorizontal: 6,
    paddingVertical: 2,
    borderRadius: 4,
  },
  pendingText: {
    fontSize: 10,
    color: '#fff',
    fontWeight: 'bold',
  },
  settingsButton: {
    padding: 8,
    backgroundColor: lightTheme.colors.wave[100],
    borderRadius: 12,
  },
  settingsText: {
    fontSize: 20,
  },
  heartbeat: {
    fontSize: 12,
    color: lightTheme.colors.wave[500],
    marginTop: 4,
  },
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
    borderWidth: 1,
    borderColor: 'rgba(255, 255, 255, 0.5)',
  },
  cardTitle: {
    fontSize: 18,
    fontWeight: 'bold',
    color: lightTheme.colors.wave[900],
    marginBottom: 16,
  },
  metricsRow: {
    flexDirection: 'row',
    justifyContent: 'space-around',
    marginBottom: 20,
  },
  metric: {
    alignItems: 'center',
  },
  metricLabel: {
    fontSize: 12,
    color: lightTheme.colors.wave[500],
    marginBottom: 4,
  },
  metricValue: {
    fontSize: 24,
    fontWeight: 'bold',
  },
  chartContainer: {
    marginTop: 8,
  },
  chartPlaceholder: {
    height: 180,
    backgroundColor: lightTheme.colors.wave[100],
    borderRadius: 12,
    justifyContent: 'center',
    alignItems: 'center',
    borderWidth: 2,
    borderColor: lightTheme.colors.wave[200],
    borderStyle: 'dashed',
  },
  chartText: {
    fontSize: 18,
    color: lightTheme.colors.wave[600],
    fontWeight: '600',
  },
  chartSubtext: {
    fontSize: 12,
    color: lightTheme.colors.wave[400],
    marginTop: 4,
  },
  configRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    paddingVertical: 10,
    borderBottomWidth: 1,
    borderBottomColor: lightTheme.colors.wave[100],
  },
  configLabel: {
    fontSize: 14,
    color: lightTheme.colors.wave[600],
  },
  configValue: {
    fontSize: 14,
    fontWeight: '600',
    color: lightTheme.colors.wave[800],
    textTransform: 'capitalize',
  },
  liveMode: {
    color: lightTheme.colors.lobster[600],
  },
  mono: {
    fontFamily: 'monospace',
    fontSize: 13,
  },
  actionsRow: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 12,
  },
  actionButton: {
    flex: 1,
    minWidth: 100,
    paddingVertical: 12,
    paddingHorizontal: 16,
    borderRadius: 10,
    alignItems: 'center',
  },
  actionButtonText: {
    color: '#fff',
    fontWeight: '600',
    fontSize: 14,
  },
  actionLoading: {
    padding: 20,
  },
  eventItem: {
    paddingVertical: 12,
    borderBottomWidth: 1,
    borderBottomColor: lightTheme.colors.wave[100],
  },
  eventHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    marginBottom: 4,
  },
  eventType: {
    fontSize: 11,
    fontWeight: '700',
    color: lightTheme.colors.wave[600],
  },
  eventTime: {
    fontSize: 11,
    color: lightTheme.colors.wave[400],
  },
  eventMessage: {
    fontSize: 14,
    color: lightTheme.colors.wave[800],
  },
  emptyText: {
    textAlign: 'center',
    color: lightTheme.colors.wave[400],
    padding: 20,
  },
});
