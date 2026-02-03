import React, { useEffect, useState, useCallback } from 'react';
import {
  View,
  Text,
  StyleSheet,
  ScrollView,
  Button,
  ActivityIndicator,
  TouchableOpacity,
  Alert,
  RefreshControl,
} from 'react-native';
import { useNavigation, useRoute, RouteProp } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';
import type { Bot, BotEvent, BotConfig } from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';

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
        'Destroy Bot?',
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
      Alert.alert('Success', `Bot ${actionLabels[action]}ed successfully`);
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
      <View style={styles.centered}>
        <ActivityIndicator size="large" />
      </View>
    );
  }

  const getStatusColor = (status: Bot['status']) => {
    const colors: Record<Bot['status'], string> = {
      provisioning: '#FFA500',
      online: '#4CAF50',
      offline: '#9E9E9E',
      paused: '#FFC107',
      error: '#F44336',
      destroying: '#9C27B0',
    };
    return colors[status];
  };

  const formatEventType = (type: string) => {
    return type.replace(/_/g, ' ').toUpperCase();
  };

  return (
    <ScrollView 
      style={styles.container}
      refreshControl={
        <RefreshControl refreshing={isRefreshing} onRefresh={handleRefresh} />
      }
    >
      {/* Header */}
      <View style={styles.header}>
        <View style={styles.headerTop}>
          <View>
            <Text style={styles.botName}>{bot.name}</Text>
            <View style={styles.statusRow}>
              <View
                style={[
                  styles.statusDot,
                  { backgroundColor: getStatusColor(bot.status) },
                ]}
              />
              <Text style={styles.statusText}>{bot.status.toUpperCase()}</Text>
              {bot.configStatus === 'pending' && (
                <View style={styles.pendingBadge}>
                  <Text style={styles.pendingText}>CONFIG PENDING</Text>
                </View>
              )}
            </View>
          </View>
          
          <TouchableOpacity style={styles.settingsButton} onPress={handleSettings}>
            <Text style={styles.settingsText}>⚙️</Text>
          </TouchableOpacity>
        </View>

        {bot.lastHeartbeatAt && (
          <Text style={styles.heartbeat}>
            Last seen: {new Date(bot.lastHeartbeatAt).toLocaleString()}
          </Text>
        )}
      </View>

      {/* Performance */}
      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Performance</Text>
        
        <View style={styles.metricsRow}>
          <View style={styles.metric}>
            <Text style={styles.metricLabel}>Today</Text>
            <Text
              style={[
                styles.metricValue,
                (bot.todayPnl || 0) >= 0 ? styles.positive : styles.negative,
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
                (bot.totalPnl || 0) >= 0 ? styles.positive : styles.negative,
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
        <View style={styles.chartPlaceholder}>
          <Text style={styles.chartText}>📈 Equity Chart</Text>
          <Text style={styles.chartSubtext}>(Integration with victory-native-xl)</Text>
        </View>
      </View>

      {/* Configuration Summary */}
      {config && (
        <View style={styles.section}>
          <Text style={styles.sectionTitle}>Configuration</Text>
          
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
            <Text style={[
              styles.configValue,
              config.tradingMode === 'live' && styles.liveMode
            ]}>
              {config.tradingMode === 'live' ? '🔴 LIVE' : '📄 Paper'}
            </Text>
          </View>
          
          <View style={styles.configRow}>
            <Text style={styles.configLabel}>Max Position</Text>
            <Text style={styles.configValue}>{config.riskCaps?.maxPositionSizePercent || 5}%</Text>
          </View>
          
          <View style={styles.configRow}>
            <Text style={styles.configLabel}>Max Daily Loss</Text>
            <Text style={styles.configValue}>${config.riskCaps?.maxDailyLossUsd || 50}</Text>
          </View>
        </View>
      )}

      {/* Actions */}
      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Actions</Text>

        {isActionLoading ? (
          <ActivityIndicator style={styles.actionLoading} />
        ) : (
          <View style={styles.actionsRow}>
            {bot.status === 'online' ? (
              <View style={styles.actionButton}>
                <Button
                  title="⏸️ Pause"
                  onPress={() => handleAction('pause')}
                  color="#FFC107"
                />
              </View>
            ) : bot.status === 'paused' ? (
              <View style={styles.actionButton}>
                <Button
                  title="▶️ Resume"
                  onPress={() => handleAction('resume')}
                  color="#4CAF50"
                />
              </View>
            ) : null}

            <View style={styles.actionButton}>
              <Button
                title="🔄 Redeploy"
                onPress={() => handleAction('redeploy')}
              />
            </View>

            <View style={styles.actionButton}>
              <Button
                title="🗑️ Destroy"
                onPress={() => handleAction('destroy')}
                color="#F44336"
              />
            </View>
          </View>
        )}
      </View>

      {/* Events */}
      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Recent Events</Text>

        {events.length === 0 ? (
          <Text style={styles.emptyText}>No events yet</Text>
        ) : (
          events.slice(0, 10).map((event) => (
            <View key={event.id} style={styles.eventItem}>
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
        <View style={styles.section}>
          <Text style={styles.sectionTitle}>Infrastructure</Text>
          
          <View style={styles.infoRow}>
            <Text style={styles.infoLabel}>Region:</Text>
            <Text style={styles.infoValue}>{bot.region}</Text>
          </View>
          
          <View style={styles.infoRow}>
            <Text style={styles.infoLabel}>Droplet ID:</Text>
            <Text style={styles.infoValue}>{bot.dropletId}</Text>
          </View>
          
          {bot.ipAddress && (
            <View style={styles.infoRow}>
              <Text style={styles.infoLabel}>IP Address:</Text>
              <Text style={styles.infoValue}>{bot.ipAddress}</Text>
            </View>
          )}
        </View>
      )}
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#f5f5f5',
  },
  centered: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  header: {
    backgroundColor: '#fff',
    padding: 20,
    borderBottomWidth: 1,
    borderBottomColor: '#e0e0e0',
  },
  headerTop: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'flex-start',
    marginBottom: 8,
  },
  botName: {
    fontSize: 24,
    fontWeight: 'bold',
  },
  statusRow: {
    flexDirection: 'row',
    alignItems: 'center',
    marginTop: 4,
    gap: 8,
  },
  statusDot: {
    width: 8,
    height: 8,
    borderRadius: 4,
  },
  statusText: {
    fontSize: 14,
    fontWeight: '600',
  },
  pendingBadge: {
    backgroundColor: '#FFA500',
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
  },
  settingsText: {
    fontSize: 20,
  },
  heartbeat: {
    fontSize: 12,
    color: '#666',
  },
  section: {
    backgroundColor: '#fff',
    margin: 16,
    marginBottom: 8,
    padding: 16,
    borderRadius: 12,
  },
  sectionTitle: {
    fontSize: 18,
    fontWeight: 'bold',
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
    color: '#666',
    marginBottom: 4,
  },
  metricValue: {
    fontSize: 24,
    fontWeight: 'bold',
  },
  positive: {
    color: '#4CAF50',
  },
  negative: {
    color: '#F44336',
  },
  chartPlaceholder: {
    height: 200,
    backgroundColor: '#f5f5f5',
    borderRadius: 8,
    justifyContent: 'center',
    alignItems: 'center',
  },
  chartText: {
    fontSize: 18,
    color: '#666',
  },
  chartSubtext: {
    fontSize: 12,
    color: '#999',
    marginTop: 4,
  },
  configRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    paddingVertical: 8,
    borderBottomWidth: 1,
    borderBottomColor: '#f0f0f0',
  },
  configLabel: {
    fontSize: 14,
    color: '#666',
  },
  configValue: {
    fontSize: 14,
    fontWeight: '600',
    textTransform: 'capitalize',
  },
  liveMode: {
    color: '#F44336',
  },
  actionsRow: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 12,
  },
  actionButton: {
    flex: 1,
    minWidth: 100,
  },
  actionLoading: {
    padding: 20,
  },
  eventItem: {
    paddingVertical: 12,
    borderBottomWidth: 1,
    borderBottomColor: '#f0f0f0',
  },
  eventHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    marginBottom: 4,
  },
  eventType: {
    fontSize: 12,
    fontWeight: '600',
    color: '#666',
  },
  eventTime: {
    fontSize: 12,
    color: '#999',
  },
  eventMessage: {
    fontSize: 14,
  },
  emptyText: {
    textAlign: 'center',
    color: '#999',
    padding: 20,
  },
  infoRow: {
    flexDirection: 'row',
    marginBottom: 8,
  },
  infoLabel: {
    width: 100,
    fontSize: 14,
    color: '#666',
  },
  infoValue: {
    flex: 1,
    fontSize: 14,
    fontFamily: 'monospace',
  },
});
