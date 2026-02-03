import React, { useEffect, useState } from 'react';
import {
  View,
  Text,
  StyleSheet,
  ScrollView,
  Button,
  ActivityIndicator,
  TouchableOpacity,
} from 'react-native';
import { useNavigation, useRoute, RouteProp } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';
import type { Bot, BotEvent } from '@trawling-traders/types';

type BotDetailScreenRouteProp = RouteProp<RootStackParamList, 'BotDetail'>;
type BotDetailScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'BotDetail'>;

export function BotDetailScreen() {
  const route = useRoute<BotDetailScreenRouteProp>();
  const navigation = useNavigation<BotDetailScreenNavigationProp>();
  const { botId } = route.params;

  const [bot, setBot] = useState<Bot | null>(null);
  const [events, setEvents] = useState<BotEvent[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isActionLoading, setIsActionLoading] = useState(false);

  useEffect(() => {
    fetchBotDetails();
  }, [botId]);

  const fetchBotDetails = async () => {
    try {
      // TODO: Call API
      // const botResponse = await getApi().getBot(botId);
      // const eventsResponse = await getApi().getEvents(botId);
      // setBot(botResponse.bot);
      // setEvents(eventsResponse.events);

      // Placeholder
      setBot({
        id: botId,
        userId: 'user1',
        name: 'Trend Hunter',
        status: 'online',
        persona: 'tweaker',
        region: 'nyc1',
        dropletId: '123456',
        ipAddress: '192.168.1.100',
        desiredVersionId: 'v1',
        appliedVersionId: 'v1',
        configStatus: 'applied',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        lastHeartbeatAt: new Date().toISOString(),
        todayPnl: 12.45,
        totalPnl: 145.20,
      });

      setEvents([
        {
          id: '1',
          botId,
          type: 'trade_opened',
          timestamp: new Date().toISOString(),
          message: 'Opened long BTC position @ $42,500',
        },
        {
          id: '2',
          botId,
          type: 'config_applied',
          timestamp: new Date(Date.now() - 3600000).toISOString(),
          message: 'Configuration applied successfully',
        },
      ]);
    } catch (error) {
      console.error('Failed to fetch bot details:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleAction = async (action: 'pause' | 'resume' | 'redeploy' | 'destroy') => {
    setIsActionLoading(true);
    try {
      // TODO: Call API
      // await getApi().botAction(botId, action);
      
      if (action === 'destroy') {
        navigation.navigate('Main');
        return;
      }
      
      fetchBotDetails();
    } catch (error) {
      console.error(`Failed to ${action} bot:`, error);
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

  return (
    <ScrollView style={styles.container}>
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
                <Text style={styles.pendingBadge}>CONFIG PENDING</Text>
              )}
            </View>
          </View>
          
          <TouchableOpacity style={styles.settingsButton} onPress={handleSettings}>
            <Text style={styles.settingsText}>⚙️ Settings</Text>
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
              {bot.todayPnl >= 0 ? '+' : ''}
              {bot.todayPnl?.toFixed(2) || '0.00'}
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
              {bot.totalPnl >= 0 ? '+' : ''}
              {bot.totalPnl?.toFixed(2) || '0.00'}
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
          <Text style={styles.chartSubtext}>(Integration with react-native-chart-kit)</Text>
        </View>
      </View>

      {/* Actions */}
      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Actions</Text>

        <View style={styles.actionsRow}>
          {bot.status === 'online' ? (
            <View style={styles.actionButton}>
              <Button
                title="⏸️ Pause"
                onPress={() => handleAction('pause')}
                disabled={isActionLoading}
                color="#FFC107"
              />
            </View>
          ) : bot.status === 'paused' ? (
            <View style={styles.actionButton}>
              <Button
                title="▶️ Resume"
                onPress={() => handleAction('resume')}
                disabled={isActionLoading}
                color="#4CAF50"
              />
            </View>
          ) : null}

          <View style={styles.actionButton}>
            <Button
              title="🔄 Redeploy"
              onPress={() => handleAction('redeploy')}
              disabled={isActionLoading}
            />
          </View>

          <View style={styles.actionButton}>
            <Button
              title="🗑️ Destroy"
              onPress={() => handleAction('destroy')}
              disabled={isActionLoading}
              color="#F44336"
            />
          </View>
        </View>
      </View>

      {/* Events */}
      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Recent Events</Text>

        {events.length === 0 ? (
          <Text style={styles.emptyText}>No events yet</Text>
        ) : (
          events.map((event) => (
            <View key={event.id} style={styles.eventItem}>
              <View style={styles.eventHeader}>
                <Text style={styles.eventType}>{event.type}</Text>
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
    fontSize: 10,
    backgroundColor: '#FFA500',
    color: '#fff',
    paddingHorizontal: 6,
    paddingVertical: 2,
    borderRadius: 4,
  },
  settingsButton: {
    padding: 8,
  },
  settingsText: {
    fontSize: 16,
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
  actionsRow: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 12,
  },
  actionButton: {
    flex: 1,
    minWidth: 100,
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
    textTransform: 'uppercase',
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
