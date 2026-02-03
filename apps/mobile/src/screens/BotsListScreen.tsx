import React, { useEffect, useState } from 'react';
import {
  View,
  Text,
  StyleSheet,
  FlatList,
  TouchableOpacity,
  Button,
  ActivityIndicator,
  RefreshControl,
} from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';
import type { Bot } from '@trawling-traders/types';

type BotsListScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'Main'>;

// Bot status badge component
function StatusBadge({ status }: { status: Bot['status'] }) {
  const colors: Record<Bot['status'], string> = {
    provisioning: '#FFA500',
    online: '#4CAF50',
    offline: '#9E9E9E',
    paused: '#FFC107',
    error: '#F44336',
    destroying: '#9C27B0',
  };

  return (
    <View style={[styles.badge, { backgroundColor: colors[status] }]}>
      <Text style={styles.badgeText}>{status.toUpperCase()}</Text>
    </View>
  );
}

// Bot card component
function BotCard({ bot, onPress }: { bot: Bot; onPress: () => void }) {
  return (
    <TouchableOpacity style={styles.card} onPress={onPress}>
      <View style={styles.cardHeader}>
        <Text style={styles.botName}>{bot.name}</Text>
        <StatusBadge status={bot.status} />
      </View>
      
      <View style={styles.cardBody}>
        <Text style={styles.persona}>{bot.persona}</Text>
        {bot.todayPnl !== undefined && (
          <Text
            style={[
              styles.pnl,
              bot.todayPnl >= 0 ? styles.positive : styles.negative,
            ]}
          >
            {bot.todayPnl >= 0 ? '+' : ''}
            {bot.todayPnl.toFixed(2)} today
          </Text>
        )}
        {bot.lastHeartbeatAt && (
          <Text style={styles.heartbeat}>
            Last seen: {new Date(bot.lastHeartbeatAt).toLocaleTimeString()}
          </Text>
        )}
      </View>
    </TouchableOpacity>
  );
}

export function BotsListScreen() {
  const navigation = useNavigation<BotsListScreenNavigationProp>();
  const [bots, setBots] = useState<Bot[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [botCount, setBotCount] = useState(0);
  const MAX_BOTS = 4;

  const fetchBots = async () => {
    try {
      // TODO: Call API
      // const response = await getApi().listBots();
      // setBots(response.bots);
      // setBotCount(response.total);
      
      // Placeholder data for now
      setBots([
        {
          id: '1',
          userId: 'user1',
          name: 'Trend Hunter',
          status: 'online',
          persona: 'tweaker',
          region: 'nyc1',
          desiredVersionId: 'v1',
          appliedVersionId: 'v1',
          configStatus: 'applied',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
          lastHeartbeatAt: new Date().toISOString(),
          todayPnl: 12.45,
        },
      ]);
      setBotCount(1);
    } catch (error) {
      console.error('Failed to fetch bots:', error);
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  };

  useEffect(() => {
    fetchBots();
  }, []);

  const handleRefresh = () => {
    setIsRefreshing(true);
    fetchBots();
  };

  const handleCreateBot = () => {
    navigation.navigate('CreateBot');
  };

  const handleBotPress = (botId: string) => {
    navigation.navigate('BotDetail', { botId });
  };

  if (isLoading) {
    return (
      <View style={styles.centered}>
        <ActivityIndicator size="large" />
      </View>
    );
  }

  return (
    <View style={styles.container}>
      <View style={styles.header}>
        <Text style={styles.title}>My Bots</Text>
        <Text style={styles.subtitle}>
          {botCount} / {MAX_BOTS} bots
        </Text>
      </View>

      {botCount < MAX_BOTS && (
        <View style={styles.createButton}>
          <Button title="+ Create New Bot" onPress={handleCreateBot} />
        </View>
      )}

      <FlatList
        data={bots}
        keyExtractor={(item) => item.id}
        renderItem={({ item }) => (
          <BotCard bot={item} onPress={() => handleBotPress(item.id)} />
        )}
        contentContainerStyle={styles.list}
        refreshControl={
          <RefreshControl refreshing={isRefreshing} onRefresh={handleRefresh} />
        }
        ListEmptyComponent={
          <View style={styles.empty}>
            <Text style={styles.emptyText}>No bots yet</Text>
            <Text style={styles.emptySubtext}>Create your first trading bot to get started</Text>
          </View>
        }
      />
    </View>
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
    padding: 20,
    backgroundColor: '#fff',
    borderBottomWidth: 1,
    borderBottomColor: '#e0e0e0',
  },
  title: {
    fontSize: 28,
    fontWeight: 'bold',
  },
  subtitle: {
    fontSize: 16,
    color: '#666',
    marginTop: 4,
  },
  createButton: {
    margin: 20,
  },
  list: {
    padding: 16,
  },
  card: {
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 16,
    marginBottom: 12,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 4,
    elevation: 3,
  },
  cardHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  botName: {
    fontSize: 20,
    fontWeight: 'bold',
  },
  badge: {
    paddingHorizontal: 8,
    paddingVertical: 4,
    borderRadius: 4,
  },
  badgeText: {
    color: '#fff',
    fontSize: 10,
    fontWeight: 'bold',
  },
  cardBody: {
    gap: 4,
  },
  persona: {
    fontSize: 14,
    color: '#666',
    textTransform: 'capitalize',
  },
  pnl: {
    fontSize: 16,
    fontWeight: '600',
  },
  positive: {
    color: '#4CAF50',
  },
  negative: {
    color: '#F44336',
  },
  heartbeat: {
    fontSize: 12,
    color: '#999',
  },
  empty: {
    alignItems: 'center',
    padding: 40,
  },
  emptyText: {
    fontSize: 18,
    fontWeight: '600',
    color: '#666',
  },
  emptySubtext: {
    fontSize: 14,
    color: '#999',
    marginTop: 8,
  },
});
