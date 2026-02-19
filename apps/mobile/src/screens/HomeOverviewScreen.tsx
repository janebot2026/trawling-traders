import React, { useCallback, useState } from 'react';
import {
  ActivityIndicator,
  FlatList,
  ImageBackground,
  RefreshControl,
  ScrollView,
  StyleSheet,
  Text,
  useColorScheme,
  View,
} from 'react-native';
import { useFocusEffect } from '@react-navigation/native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import type { Bot, BotEvent, MetricPoint } from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';
import { AuthExpiredError, NetworkError, ServerError } from '@trawling-traders/api-client';

const HOME_BG_LIGHT = require('../../../../assets/branding/tt-home-light.png');
const HOME_BG_DARK = require('../../../../assets/branding/tt-home-dark.png');
import { useBotAction } from '../hooks/useBots';
import { lightTheme, colors, spacing } from '../theme';
import { KpiStrip } from './home/KpiStrip';
import { BotFleetCard } from './home/BotFleetCard';
import { OnboardingSection } from './home/OnboardingSection';
import { TodayInsights } from './home/TodayInsights';
import { AlertsPanel } from './home/AlertsPanel';

interface OverviewStats {
  totalTrades: number;
  openTrades: number;
}

const HEADER_HEIGHT = 56;

export function HomeOverviewScreen() {
  const insets = useSafeAreaInsets();
  const colorScheme = useColorScheme();
  const homeBackground = colorScheme === 'dark' ? HOME_BG_DARK : HOME_BG_LIGHT;
  const { performAction } = useBotAction();
  const contentTopPadding = insets.top + HEADER_HEIGHT;

  const [bots, setBots] = useState<Bot[]>([]);
  const [allMetrics, setAllMetrics] = useState<MetricPoint[]>([]);
  const [allEvents, setAllEvents] = useState<BotEvent[]>([]);
  const [stats, setStats] = useState<OverviewStats>({
    totalTrades: 0,
    openTrades: 0,
  });
  const [botsLoaded, setBotsLoaded] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    setError(null);
    try {
      const botsResponse = await api.bot.listBots();
      setBots(botsResponse.bots);
      setBotsLoaded(true);

      // No bots → nothing else to fetch
      if (botsResponse.bots.length === 0) return;

      // Fetch metrics + events in background (UI already visible)
      const [metricsByBot, eventsByBot] = await Promise.all([
        Promise.all(
          botsResponse.bots.map(async (bot) => {
            try {
              const response = await api.bot.getMetrics(bot.id);
              return response.metrics.slice(-30);
            } catch {
              return [];
            }
          })
        ),
        Promise.all(
          botsResponse.bots.map(async (bot) => {
            try {
              const response = await api.bot.getEvents(bot.id);
              return response.events;
            } catch {
              return [];
            }
          })
        ),
      ]);

      setAllMetrics(metricsByBot.flat());
      setAllEvents(eventsByBot.flat());

      const flatEvents = eventsByBot.flat();
      const openedTrades = flatEvents.filter((e) => e.type === 'trade_opened').length;
      const closedTrades = flatEvents.filter((e) => e.type === 'trade_closed').length;

      setStats({
        totalTrades: flatEvents.filter(
          (e) => e.type === 'trade_opened' || e.type === 'trade_closed'
        ).length,
        openTrades: Math.max(openedTrades - closedTrades, 0),
      });
    } catch (loadErr) {
      if (__DEV__) {
        console.error('Overview load failed:', loadErr);
      }
      setBotsLoaded(true);
      if (loadErr instanceof AuthExpiredError) {
        setError('Session expired. Please log in again.');
      } else if (loadErr instanceof NetworkError) {
        setError('You appear offline. Pull to refresh.');
      } else if (loadErr instanceof ServerError) {
        setError(null);
      } else {
        setError('Unable to refresh overview right now.');
      }
    } finally {
      setRefreshing(false);
    }
  }, []);

  useFocusEffect(
    useCallback(() => {
      let cancelled = false;

      const run = async () => {
        setError(null);
        try {
          const botsResponse = await api.bot.listBots();
          if (cancelled) return;
          setBots(botsResponse.bots);
          setBotsLoaded(true);

          if (botsResponse.bots.length === 0) return;

          const [metricsByBot, eventsByBot] = await Promise.all([
            Promise.all(
              botsResponse.bots.map(async (bot) => {
                try {
                  const response = await api.bot.getMetrics(bot.id);
                  return response.metrics.slice(-30);
                } catch {
                  return [];
                }
              })
            ),
            Promise.all(
              botsResponse.bots.map(async (bot) => {
                try {
                  const response = await api.bot.getEvents(bot.id);
                  return response.events;
                } catch {
                  return [];
                }
              })
            ),
          ]);

          if (cancelled) return;

          setAllMetrics(metricsByBot.flat());
          setAllEvents(eventsByBot.flat());

          const flatEvents = eventsByBot.flat();
          const openedTrades = flatEvents.filter((e) => e.type === 'trade_opened').length;
          const closedTrades = flatEvents.filter((e) => e.type === 'trade_closed').length;

          setStats({
            totalTrades: flatEvents.filter(
              (e) => e.type === 'trade_opened' || e.type === 'trade_closed'
            ).length,
            openTrades: Math.max(openedTrades - closedTrades, 0),
          });
        } catch (loadErr) {
          if (cancelled) return;
          if (__DEV__) {
            console.error('Overview load failed:', loadErr);
          }
          setBotsLoaded(true);
          if (loadErr instanceof AuthExpiredError) {
            setError('Session expired. Please log in again.');
          } else if (loadErr instanceof NetworkError) {
            setError('You appear offline. Pull to refresh.');
          } else if (loadErr instanceof ServerError) {
            setError(null);
          } else {
            setError('Unable to refresh overview right now.');
          }
        } finally {
          if (!cancelled) {
            setRefreshing(false);
          }
        }
      };

      run();

      return () => {
        cancelled = true;
      };
    }, [])
  );

  const onRefresh = useCallback(async () => {
    setRefreshing(true);
    await loadData();
  }, [loadData]);

  const handlePauseResume = useCallback(
    async (botId: string, action: 'pause' | 'resume') => {
      try {
        await performAction(botId, action);
        loadData();
      } catch {
        /* useBotAction sets its own error */
      }
    },
    [performAction, loadData]
  );

  const hasActiveBots = bots.length > 0;

  if (!botsLoaded) {
    return (
      <ImageBackground source={homeBackground} style={styles.bgFill} resizeMode="cover">
        <View style={styles.centered}>
          <ActivityIndicator size="large" color={colors.primary[700]} />
        </View>
      </ImageBackground>
    );
  }

  if (!hasActiveBots) {
    return (
      <ImageBackground source={homeBackground} style={styles.bgFill} resizeMode="cover">
        <ScrollView
          contentContainerStyle={[styles.content, { paddingTop: contentTopPadding + spacing.md }]}
          showsVerticalScrollIndicator={false}
          refreshControl={
            <RefreshControl refreshing={refreshing} onRefresh={onRefresh} tintColor="#fff" />
          }
        >
          {error && <Text style={styles.errorText}>{error}</Text>}
          <OnboardingSection hasBots={false} hasFundedBot={false} />
        </ScrollView>
      </ImageBackground>
    );
  }

  const listHeader = (
    <>
      {error && <Text style={styles.errorText}>{error}</Text>}
      <KpiStrip
        bots={bots}
        openTrades={stats.openTrades}
        totalTrades={stats.totalTrades}
      />
      <TodayInsights bots={bots} allMetrics={allMetrics} />
      <Text style={styles.sectionTitle}>Your Fleet</Text>
    </>
  );

  return (
    <ImageBackground source={homeBackground} style={styles.bgFill} resizeMode="cover">
      <FlatList
        data={bots}
        keyExtractor={(bot) => bot.id}
        renderItem={({ item, index }) => (
          <BotFleetCard
            bot={item}
            index={index}
            onPauseResume={handlePauseResume}
          />
        )}
        ListHeaderComponent={listHeader}
        ListFooterComponent={<AlertsPanel events={allEvents} />}
        contentContainerStyle={[styles.content, { paddingTop: contentTopPadding + spacing.md }]}
        showsVerticalScrollIndicator={false}
        refreshControl={
          <RefreshControl
            refreshing={refreshing}
            onRefresh={onRefresh}
            tintColor={colors.primary[700]}
          />
        }
      />
    </ImageBackground>
  );
}

const styles = StyleSheet.create({
  bgFill: {
    flex: 1,
  },
  content: {
    paddingHorizontal: spacing.md,
    paddingBottom: 36,
  },
  centered: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  errorText: {
    color: colors.lobster[600],
    fontSize: 13,
    marginBottom: spacing.sm,
    textAlign: 'center',
  },
  sectionTitle: {
    fontSize: 18,
    fontWeight: '700',
    color: lightTheme.colors.text,
    fontFamily: lightTheme.typography.families.display,
    marginBottom: spacing.sm,
  },
});
