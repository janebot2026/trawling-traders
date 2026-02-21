import React, { useEffect, useRef } from 'react';
import {
  Animated,
  Image,
  StyleSheet,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { DrawerNavigationProp } from '@react-navigation/drawer';
import type { Bot } from '@trawling-traders/types';
import { lightTheme, colors, spacing, shadows } from '../../theme';
import { pressScale } from '../../utils/animations';
import type { RootStackParamList, MainDrawerParamList } from '../../navigation/AppNavigator';

const LOB_AVATAR = require('../../../assets/lob-avatar.png');

interface BotFleetCardProps {
  bot: Bot;
  index: number;
  onPauseResume: (botId: string, action: 'pause' | 'resume') => void;
}

const STATUS_COLORS: Record<Bot['status'], { bg: string; text: string }> = {
  provisioning: { bg: lightTheme.colors.accent, text: '#fff' },
  online: { bg: colors.bullish[500], text: '#fff' },
  offline: { bg: colors.wave[400], text: '#fff' },
  paused: { bg: colors.caution[500], text: colors.wave[900] },
  error: { bg: colors.lobster[500], text: '#fff' },
  destroying: { bg: colors.wave[500], text: '#fff' },
};

function relativeTime(dateStr?: string): string {
  if (!dateStr) return 'No activity';
  const diffMs = Date.now() - new Date(dateStr).getTime();
  const mins = Math.floor(diffMs / 60_000);
  if (mins < 1) return 'Just now';
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  return `${Math.floor(hrs / 24)}d ago`;
}

export function BotFleetCard({ bot, index, onPauseResume }: BotFleetCardProps) {
  const navigation = useNavigation<NativeStackNavigationProp<RootStackParamList>>();
  const drawerNav = useNavigation<DrawerNavigationProp<MainDrawerParamList>>();
  const scaleAnim = useRef(new Animated.Value(1)).current;
  const fadeAnim = useRef(new Animated.Value(0)).current;
  const translateY = useRef(new Animated.Value(20)).current;

  // Staggered entrance
  useEffect(() => {
    Animated.parallel([
      Animated.timing(fadeAnim, {
        toValue: 1,
        duration: 300,
        delay: index * 60,
        useNativeDriver: true,
      }),
      Animated.timing(translateY, {
        toValue: 0,
        duration: 300,
        delay: index * 60,
        useNativeDriver: true,
      }),
    ]).start();
  }, [index, fadeAnim, translateY]);

  const pnl = bot.todayPnl ?? 0;
  const statusColor = STATUS_COLORS[bot.status];
  const isPausable = bot.status === 'online';
  const isResumable = bot.status === 'paused';

  return (
    <TouchableOpacity
      activeOpacity={0.9}
      onPressIn={() => pressScale(scaleAnim, true)}
      onPressOut={() => pressScale(scaleAnim, false)}
      onPress={() => navigation.navigate('BotDetail', { botId: bot.id })}
    >
      <Animated.View
        style={[
          styles.card,
          {
            opacity: fadeAnim,
            transform: [{ scale: scaleAnim }, { translateY }],
          },
        ]}
      >
        {/* Top row: avatar + name + status */}
        <View style={styles.topRow}>
          <Image source={LOB_AVATAR} style={styles.avatar} />
          <View style={styles.nameCol}>
            <Text style={styles.botName} numberOfLines={1}>
              {bot.name}
            </Text>
            <Text style={styles.persona}>{bot.assistantStyle}</Text>
          </View>
          <View style={[styles.badge, { backgroundColor: statusColor.bg }]}>
            <Text style={[styles.badgeText, { color: statusColor.text }]}>
              {bot.status.toUpperCase()}
            </Text>
          </View>
        </View>

        {/* Info row: activity + P&L */}
        <View style={styles.infoRow}>
          <Text style={styles.activity}>
            Last trade {relativeTime(bot.lastHeartbeatAt)}
          </Text>
          <Text
            style={[
              styles.pnl,
              { color: pnl >= 0 ? colors.bullish[600] : colors.lobster[600] },
            ]}
          >
            {pnl >= 0 ? '+' : ''}${pnl.toFixed(2)}
          </Text>
        </View>

        {/* Quick actions */}
        <View style={styles.actions}>
          <TouchableOpacity
            style={styles.actionBtn}
            onPress={() => drawerNav.navigate('Chat')}
          >
            <Text style={styles.actionIcon}>💬</Text>
            <Text style={styles.actionLabel}>Chat</Text>
          </TouchableOpacity>

          {(isPausable || isResumable) && (
            <TouchableOpacity
              style={styles.actionBtn}
              onPress={() =>
                onPauseResume(bot.id, isPausable ? 'pause' : 'resume')
              }
            >
              <Text style={styles.actionIcon}>
                {isPausable ? '⏸' : '▶'}
              </Text>
              <Text style={styles.actionLabel}>
                {isPausable ? 'Pause' : 'Resume'}
              </Text>
            </TouchableOpacity>
          )}

          <TouchableOpacity
            style={styles.actionBtn}
            onPress={() => navigation.navigate('BotDetail', { botId: bot.id })}
          >
            <Text style={styles.actionIcon}>›</Text>
            <Text style={styles.actionLabel}>Details</Text>
          </TouchableOpacity>
        </View>
      </Animated.View>
    </TouchableOpacity>
  );
}

const styles = StyleSheet.create({
  card: {
    backgroundColor: lightTheme.colors.surface,
    borderRadius: 18,
    borderWidth: 1,
    borderColor: lightTheme.colors.cardBorder,
    padding: 14,
    marginBottom: 12,
    ...shadows.md,
  },
  topRow: {
    flexDirection: 'row',
    alignItems: 'center',
    marginBottom: 10,
  },
  avatar: {
    width: 40,
    height: 40,
    borderRadius: 20,
    marginRight: 10,
  },
  nameCol: {
    flex: 1,
  },
  botName: {
    fontSize: 16,
    fontWeight: '700',
    color: lightTheme.colors.text,
    fontFamily: lightTheme.typography.families.display,
  },
  persona: {
    fontSize: 12,
    color: colors.wave[500],
    textTransform: 'capitalize',
  },
  badge: {
    paddingHorizontal: 10,
    paddingVertical: 4,
    borderRadius: 6,
    minWidth: 70,
    alignItems: 'center',
  },
  badgeText: {
    fontSize: 10,
    fontWeight: '800',
    letterSpacing: 0.5,
  },
  infoRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 10,
  },
  activity: {
    fontSize: 12,
    color: colors.wave[400],
  },
  pnl: {
    fontSize: 20,
    fontWeight: '700',
  },
  actions: {
    flexDirection: 'row',
    gap: 12,
    borderTopWidth: 1,
    borderTopColor: lightTheme.colors.border,
    paddingTop: 10,
  },
  actionBtn: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 4,
    paddingVertical: 4,
    paddingHorizontal: 8,
    borderRadius: 8,
    backgroundColor: colors.wave[50],
  },
  actionIcon: {
    fontSize: 14,
  },
  actionLabel: {
    fontSize: 12,
    fontWeight: '600',
    color: colors.wave[600],
  },
});
