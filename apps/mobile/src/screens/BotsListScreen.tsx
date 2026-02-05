import React, { useState, useCallback } from 'react';
import {
  View,
  Text,
  StyleSheet,
  FlatList,
  TouchableOpacity,
  ActivityIndicator,
  RefreshControl,
  Image,
  Animated,
  Alert,
} from 'react-native';
import { useNavigation, useFocusEffect } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';
import type { Bot } from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';
import { OceanBackground } from '../components/OceanBackground';
import { lightTheme } from '../theme';

// LOB Avatar - default bot image
const LOB_AVATAR = require('../../assets/lob-avatar.png');

type BotsListScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'Main'>;

// Animated bot card
function BotCard({ bot, onPress, index }: { bot: Bot; onPress: () => void; index: number }) {
  const fadeAnim = React.useRef(new Animated.Value(0)).current;
  const slideAnim = React.useRef(new Animated.Value(30)).current;

  React.useEffect(() => {
    Animated.parallel([
      Animated.timing(fadeAnim, {
        toValue: 1,
        duration: 500,
        delay: index * 100,
        useNativeDriver: true,
      }),
      Animated.timing(slideAnim, {
        toValue: 0,
        duration: 500,
        delay: index * 100,
        useNativeDriver: true,
      }),
    ]).start();
  }, [index]);

  const pnl = bot.todayPnl || 0;
  const totalPnl = bot.totalPnl || 0;

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
    <Animated.View
      style={[
        styles.cardWrapper,
        {
          opacity: fadeAnim,
          transform: [{ translateY: slideAnim }],
        },
      ]}
    >
      <TouchableOpacity style={styles.card} onPress={onPress} activeOpacity={0.9}>
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
              <View style={[styles.badge, { backgroundColor: getStatusColor(bot.status) }]}>
                <Text style={styles.badgeText}>{bot.status.toUpperCase()}</Text>
              </View>
            </View>

            <View style={styles.cardBody}>
              <Text style={styles.persona}>{bot.persona}</Text>
              <View style={styles.pnlRow}>
                <View style={styles.pnlItem}>
                  <Text style={styles.pnlLabel}>Today</Text>
                  <Text
                    style={[
                      styles.pnlValue,
                      { color: pnl >= 0 ? lightTheme.colors.bullish[600] : lightTheme.colors.lobster[600] },
                    ]}
                  >
                    {pnl >= 0 ? '+' : ''}${pnl.toFixed(2)}
                  </Text>
                </View>

                <View style={styles.pnlItem}>
                  <Text style={styles.pnlLabel}>Total</Text>
                  <Text
                    style={[
                      styles.pnlValue,
                      { color: totalPnl >= 0 ? lightTheme.colors.bullish[600] : lightTheme.colors.lobster[600] },
                    ]}
                  >
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
    </Animated.View>
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

  // useFocusEffect handles both initial load and refocus
  // (removed duplicate useEffect to prevent double fetching)
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
      <OceanBackground>
        <View style={styles.centered}>
          <ActivityIndicator size="large" color={lightTheme.colors.primary[700]} />
        </View>
      </OceanBackground>
    );
  }

  return (
    <OceanBackground>
      <View style={styles.container}>
        <View style={styles.header}>
          <View style={styles.headerRow}>
            <Image source={LOB_AVATAR} style={styles.headerAvatar} />
            <View>
              <Text style={styles.title}>My Trawlers</Text>
              <Text style={styles.subtitle}>
                {botCount} / {maxBots} deployed
              </Text>
            </View>
          </View>
        </View>

        <View style={styles.progressContainer}>
          <View style={styles.progressBar}>
            <View
              style={[
                styles.progressFill,
                {
                  width: `${Math.min((botCount / maxBots) * 100, 100)}%`,
                  backgroundColor: botCount >= maxBots ? lightTheme.colors.lobster[500] : lightTheme.colors.bullish[500],
                },
              ]}
            />
          </View>
        </View>

        {botCount < maxBots && (
          <TouchableOpacity style={styles.createButton} onPress={handleCreateBot}>
            <Text style={styles.createButtonText}>+ Deploy New Trawler</Text>
          </TouchableOpacity>
        )}

        <FlatList
          data={bots}
          keyExtractor={(item) => item.id}
          renderItem={({ item, index }) => (
            <BotCard
              bot={item}
              index={index}
              onPress={() => handleBotPress(item.id)}
            />
          )}
          contentContainerStyle={styles.list}
          refreshControl={
            <RefreshControl
              refreshing={isRefreshing}
              onRefresh={handleRefresh}
              tintColor={lightTheme.colors.primary[700]}
            />
          }
          ListEmptyComponent={
            <View style={styles.empty}>
              <Image source={LOB_AVATAR} style={styles.emptyAvatar} />
              <Text style={styles.emptyText}>No trawlers deployed yet</Text>
              <Text style={styles.emptySubtext}>
                Deploy your first LOB to start trawling the markets
              </Text>
              <TouchableOpacity style={styles.emptyButton} onPress={handleCreateBot}>
                <Text style={styles.emptyButtonText}>Deploy First Trawler</Text>
              </TouchableOpacity>
            </View>
          }
        />
      </View>
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
    padding: 20,
    paddingTop: 60,
    backgroundColor: 'rgba(255, 255, 255, 0.9)',
    borderBottomLeftRadius: 24,
    borderBottomRightRadius: 24,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 8,
    elevation: 3,
  },
  headerRow: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  headerAvatar: {
    width: 56,
    height: 56,
    borderRadius: 28,
    marginRight: 16,
    borderWidth: 3,
    borderColor: lightTheme.colors.primary[700],
  },
  title: {
    fontSize: 28,
    fontWeight: 'bold',
    color: lightTheme.colors.wave[900],
  },
  subtitle: {
    fontSize: 16,
    color: lightTheme.colors.wave[500],
    marginTop: 4,
  },
  progressContainer: {
    paddingHorizontal: 20,
    paddingTop: 12,
  },
  progressBar: {
    height: 6,
    backgroundColor: lightTheme.colors.wave[200],
    borderRadius: 3,
    overflow: 'hidden',
  },
  progressFill: {
    height: '100%',
    borderRadius: 3,
  },
  createButton: {
    margin: 16,
    backgroundColor: lightTheme.colors.primary[700],
    paddingVertical: 14,
    paddingHorizontal: 24,
    borderRadius: 12,
    alignItems: 'center',
    shadowColor: lightTheme.colors.primary[700],
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.3,
    shadowRadius: 8,
    elevation: 4,
  },
  createButtonText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: '600',
  },
  list: {
    padding: 16,
    paddingTop: 8,
  },
  cardWrapper: {
    marginBottom: 12,
  },
  card: {
    backgroundColor: 'rgba(255, 255, 255, 0.95)',
    borderRadius: 16,
    padding: 16,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.1,
    shadowRadius: 12,
    elevation: 4,
    borderWidth: 1,
    borderColor: 'rgba(255, 255, 255, 0.5)',
  },
  cardContent: {
    flexDirection: 'row',
  },
  avatarContainer: {
    position: 'relative',
    marginRight: 16,
  },
  avatar: {
    width: 64,
    height: 64,
    borderRadius: 32,
    borderWidth: 3,
    borderColor: lightTheme.colors.primary[700],
  },
  botNumberBadge: {
    position: 'absolute',
    bottom: -4,
    right: -4,
    backgroundColor: lightTheme.colors.bullish[500],
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
    marginBottom: 4,
  },
  botName: {
    fontSize: 18,
    fontWeight: 'bold',
    color: lightTheme.colors.wave[900],
    flex: 1,
  },
  badge: {
    paddingHorizontal: 8,
    paddingVertical: 4,
    borderRadius: 6,
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
    color: lightTheme.colors.wave[500],
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
    color: lightTheme.colors.wave[400],
  },
  pnlValue: {
    fontSize: 16,
    fontWeight: 'bold',
  },
  heartbeat: {
    fontSize: 11,
    color: lightTheme.colors.wave[400],
    marginTop: 4,
  },
  pendingBadge: {
    backgroundColor: lightTheme.colors.caution[100],
    paddingHorizontal: 8,
    paddingVertical: 4,
    borderRadius: 6,
    marginTop: 6,
    alignSelf: 'flex-start',
  },
  pendingText: {
    fontSize: 11,
    color: lightTheme.colors.caution[700],
    fontWeight: '500',
  },
  empty: {
    alignItems: 'center',
    padding: 40,
    marginTop: 40,
  },
  emptyAvatar: {
    width: 100,
    height: 100,
    borderRadius: 50,
    marginBottom: 16,
    opacity: 0.5,
  },
  emptyText: {
    fontSize: 20,
    fontWeight: '600',
    color: lightTheme.colors.wave[700],
  },
  emptySubtext: {
    fontSize: 14,
    color: lightTheme.colors.wave[500],
    marginTop: 8,
    textAlign: 'center',
    maxWidth: 280,
  },
  emptyButton: {
    marginTop: 24,
    backgroundColor: lightTheme.colors.primary[700],
    paddingVertical: 14,
    paddingHorizontal: 32,
    borderRadius: 12,
  },
  emptyButtonText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: '600',
  },
});
