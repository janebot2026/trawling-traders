import React, { useEffect, useState, useCallback } from 'react';
import {
  View,
  Text,
  StyleSheet,
  FlatList,
  TouchableOpacity,
  Button,
  ActivityIndicator,
  RefreshControl,
  Image,
} from 'react-native';
import { useNavigation, useFocusEffect } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';
import type { Bot } from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';

// LOB Avatar - default bot image
const LOB_AVATAR = require('../../assets/lob-avatar.png');

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

// Bot card component with LOB avatar
function BotCard({ bot, onPress }: { bot: Bot; onPress: () => void }) {
  const pnl = bot.todayPnl || 0;
  const totalPnl = bot.totalPnl || 0;
  
  return (
    <TouchableOpacity style={styles.card} onPress={onPress}>
      <View style={styles.cardContent}>
        {/* LOB Avatar */}
        <View style={styles.avatarContainer}>
          <Image source={LOB_AVATAR} style={styles.avatar} />
          <View style={styles.botNumberBadge}>
            <Text style={styles.botNumberText}>#{bot.id.slice(-4)}</Text>
          </View>
        </View>

        <View style={styles.cardInfo}>
          <View style={styles.cardHeader}>
            <Text style={styles.botName}>{bot.name}</Text>
            <StatusBadge status={bot.status} />
          </View>
          
          <View style={styles.cardBody}>
            <Text style={styles.persona}>{bot.persona}</Text>
            <View style={styles.pnlRow}>
              <View style={styles.pnlItem}>
                <Text style={styles.pnlLabel}>Today</Text>
                <Text style={[styles.pnlValue, pnl >= 0 ? styles.positive : styles.negative]}>
                  {pnl >= 0 ? '+' : ''}${pnl.toFixed(2)}
                </Text>
              </View>
              
              <View style={styles.pnlItem}>
                <Text style={styles.pnlLabel}>Total</Text>
                <Text style={[styles.pnlValue, totalPnl >= 0 ? styles.positive : styles.negative]}>
                  {totalPnl >= 0 ? '+' : ''}${totalPnl.toFixed(2)}
                </Text>
              </View>
            </View>
            
            {bot.lastHeartbeatAt && (
              <Text style={styles.heartbeat}>
                Last seen: {new Date(bot.lastHeartbeatAt).toLocaleTimeString()}
              </Text>
            )}
            
            {bot.configStatus === 'pending' && (
              <View style={styles.pendingBadge}>
                <Text style={styles.pendingText}>⏳ Config update pending</Text>
              </View>
            )}
          </View>
        </View>
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
  const [maxBots, setMaxBots] = useState(4);

  const fetchBots = useCallback(async () => {
    try {
      const response = await api.bot.listBots();
      setBots(response.bots);
      setBotCount(response.total);
    } catch (error) {
      console.error('Failed to fetch bots:', error);
      Alert.alert('Error', 'Failed to load bots. Is the server running?');
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, []);

  // Fetch on mount and when screen comes into focus
  useEffect(() => {
    fetchBots();
  }, [fetchBots]);

  useFocusEffect(
    useCallback(() => {
      fetchBots();
    }, [fetchBots])
  );

  const handleRefresh = () => {
    setIsRefreshing(true);
    fetchBots();
  };

  const handleCreateBot = () => {
    if (botCount >= maxBots) {
      Alert.alert(
        'Bot Limit Reached',
        `You've used ${botCount} of ${maxBots} bots. Upgrade to create more.`
      );
      return;
    }
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
        <View style={styles.headerRow}>
          <Image source={LOB_AVATAR} style={styles.headerAvatar} />
          <View>
            <Text style={styles.title}>My Bots</Text>
            <Text style={styles.subtitle}>
              {botCount} / {maxBots} trawlers deployed
            </Text>
          </View>
        </View>
      </View>

      <View style={styles.progressBar}>
        <View 
          style={[
            styles.progressFill, 
            { width: `${Math.min((botCount / maxBots) * 100, 100)}%` }
          ]} 
        />
      </View>

      {botCount < maxBots && (
        <View style={styles.createButton}>
          <Button title="+ Deploy New Trawler" onPress={handleCreateBot} />
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
            <Image source={LOB_AVATAR} style={styles.emptyAvatar} />
            <Text style={styles.emptyText}>No trawlers deployed yet</Text>
            <Text style={styles.emptySubtext}>
              Deploy your first LOB to start trawling the markets
            </Text>
            <View style={styles.emptyButton}>
              <Button title="Deploy First Trawler" onPress={handleCreateBot} />
            </View>
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
  headerRow: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  headerAvatar: {
    width: 50,
    height: 50,
    borderRadius: 25,
    marginRight: 12,
    borderWidth: 2,
    borderColor: '#007AFF',
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
  progressBar: {
    height: 4,
    backgroundColor: '#e0e0e0',
  },
  progressFill: {
    height: '100%',
    backgroundColor: '#4CAF50',
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
  cardContent: {
    flexDirection: 'row',
  },
  avatarContainer: {
    position: 'relative',
    marginRight: 16,
  },
  avatar: {
    width: 70,
    height: 70,
    borderRadius: 35,
    borderWidth: 3,
    borderColor: '#007AFF',
  },
  botNumberBadge: {
    position: 'absolute',
    bottom: -4,
    right: -4,
    backgroundColor: '#22c55e',
    paddingHorizontal: 6,
    paddingVertical: 2,
    borderRadius: 8,
    borderWidth: 2,
    borderColor: '#fff',
  },
  botNumberText: {
    color: '#fff',
    fontSize: 10,
    fontWeight: 'bold',
  },
  cardInfo: {
    flex: 1,
  },
  cardHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 8,
  },
  botName: {
    fontSize: 18,
    fontWeight: 'bold',
    flex: 1,
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
    gap: 2,
  },
  persona: {
    fontSize: 14,
    color: '#666',
    textTransform: 'capitalize',
  },
  pnlRow: {
    flexDirection: 'row',
    marginTop: 6,
  },
  pnlItem: {
    marginRight: 20,
  },
  pnlLabel: {
    fontSize: 11,
    color: '#999',
  },
  pnlValue: {
    fontSize: 16,
    fontWeight: 'bold',
  },
  positive: {
    color: '#4CAF50',
  },
  negative: {
    color: '#F44336',
  },
  heartbeat: {
    fontSize: 11,
    color: '#999',
    marginTop: 4,
  },
  pendingBadge: {
    backgroundColor: '#FFF3E0',
    paddingHorizontal: 8,
    paddingVertical: 4,
    borderRadius: 4,
    marginTop: 6,
    alignSelf: 'flex-start',
  },
  pendingText: {
    fontSize: 11,
    color: '#E65100',
    fontWeight: '500',
  },
  empty: {
    alignItems: 'center',
    padding: 40,
  },
  emptyAvatar: {
    width: 100,
    height: 100,
    borderRadius: 50,
    marginBottom: 16,
    opacity: 0.5,
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
    textAlign: 'center',
  },
  emptyButton: {
    marginTop: 20,
    width: 200,
  },
});
