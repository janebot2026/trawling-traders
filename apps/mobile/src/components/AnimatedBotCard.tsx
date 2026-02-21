import React, { useRef, useEffect } from 'react';
import {
  View,
  Text,
  StyleSheet,
  TouchableOpacity,
  Animated,
} from 'react-native';
import type { Bot } from '@trawling-traders/types';
import { lightTheme } from '../theme';
import { pressScale } from '../utils/animations';

interface AnimatedBotCardProps {
  bot: Bot;
  onPress: () => void;
  index?: number;
}

function StatusBadge({ status }: { status: Bot['status'] }) {
  const pulseAnim = useRef(new Animated.Value(1)).current;
  // Guard against stacking animation loops on rapid re-renders
  const isAnimatingRef = useRef(false);

  const statusColors: Record<Bot['status'], { bg: string; text: string }> = {
    provisioning: { bg: lightTheme.colors.accent, text: '#fff' },
    online: { bg: lightTheme.colors.bullish[500], text: '#fff' },
    offline: { bg: lightTheme.colors.wave[400], text: '#fff' },
    paused: { bg: lightTheme.colors.caution[500], text: lightTheme.colors.wave[900] },
    error: { bg: lightTheme.colors.lobster[500], text: '#fff' },
    destroying: { bg: lightTheme.colors.wave[500], text: '#fff' },
  };

  const color = statusColors[status];

  // Pulse animation for provisioning/loading states
  useEffect(() => {
    if (status === 'provisioning' || status === 'destroying') {
      // Skip if a loop is already running to prevent stacking on rapid re-renders
      if (isAnimatingRef.current) return;
      isAnimatingRef.current = true;

      const pulse = Animated.loop(
        Animated.sequence([
          Animated.timing(pulseAnim, {
            toValue: 1.1,
            duration: 600,
            useNativeDriver: true,
          }),
          Animated.timing(pulseAnim, {
            toValue: 1,
            duration: 600,
            useNativeDriver: true,
          }),
        ])
      );
      pulse.start();
      return () => {
        pulse.stop();
        isAnimatingRef.current = false;
      };
    } else {
      // Reset scale when transitioning away from animated states
      isAnimatingRef.current = false;
      pulseAnim.setValue(1);
    }
  }, [status, pulseAnim]);

  return (
    <Animated.View
      style={[
        styles.badge,
        {
          backgroundColor: color.bg,
          transform: [{ scale: pulseAnim }],
        },
      ]}
    >
      <Text style={[styles.badgeText, { color: color.text }]}>
        {status.toUpperCase()}
      </Text>
    </Animated.View>
  );
}

function PnLDisplay({ value }: { value?: number }) {
  if (value === undefined) return null;

  const isPositive = value >= 0;
  const formatted = `${isPositive ? '+' : ''}${value.toFixed(2)}`;

  return (
    <View style={styles.pnlContainer}>
      <Text style={[styles.pnlLabel, isPositive ? styles.positive : styles.negative]}>
        {formatted}
      </Text>
      <Text style={styles.pnlSuffix}>today</Text>
    </View>
  );
}

export function AnimatedBotCard({ bot, onPress, index = 0 }: AnimatedBotCardProps) {
  const scaleAnim = useRef(new Animated.Value(1)).current;
  const fadeAnim = useRef(new Animated.Value(0)).current;
  const translateY = useRef(new Animated.Value(20)).current;

  // Entrance animation — stopped on unmount to prevent updates on an
  // unmounted component when the list re-renders rapidly.
  useEffect(() => {
    const entrance = Animated.parallel([
      Animated.timing(fadeAnim, {
        toValue: 1,
        duration: 300,
        delay: index * 50,
        useNativeDriver: true,
      }),
      Animated.timing(translateY, {
        toValue: 0,
        duration: 300,
        delay: index * 50,
        useNativeDriver: true,
        easing: (x) => x < 0.5 ? 2 * x * x : 1 - Math.pow(-2 * x + 2, 2) / 2,
      }),
    ]);
    entrance.start();
    return () => entrance.stop();
  }, [index, fadeAnim, translateY]);

  const handlePressIn = () => {
    pressScale(scaleAnim, true);
  };

  const handlePressOut = () => {
    pressScale(scaleAnim, false);
  };

  return (
    <TouchableOpacity
      onPress={onPress}
      onPressIn={handlePressIn}
      onPressOut={handlePressOut}
      activeOpacity={0.9}
    >
      <Animated.View
        style={[
          styles.card,
          {
            opacity: fadeAnim,
            transform: [
              { scale: scaleAnim },
              { translateY },
            ],
          },
        ]}
      >
        <View style={styles.cardHeader}>
          <View style={styles.nameContainer}>
            <Text style={styles.botName}>{bot.name}</Text>
            <Text style={styles.persona}>{bot.assistantStyle}</Text>
          </View>
          <StatusBadge status={bot.status} />
        </View>

        <View style={styles.cardBody}>
          <PnLDisplay value={bot.todayPnl} />

          {bot.lastHeartbeatAt && (
            <View style={styles.heartbeatContainer}>
              <View style={styles.heartbeatDot} />
              <Text style={styles.heartbeat}>
                Last seen {new Date(bot.lastHeartbeatAt).toLocaleTimeString([], {
                  hour: 'numeric',
                  minute: '2-digit',
                })}
              </Text>
            </View>
          )}
        </View>

        {bot.configStatus === 'pending' && (
          <View style={styles.pendingBanner}>
            <Text style={styles.pendingText}>Config update pending</Text>
          </View>
        )}
      </Animated.View>
    </TouchableOpacity>
  );
}

const styles = StyleSheet.create({
  card: {
    backgroundColor: lightTheme.colors.surface,
    borderRadius: 20,
    padding: 20,
    marginBottom: 16,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.08,
    shadowRadius: 8,
    elevation: 4,
    borderWidth: 1,
    borderColor: lightTheme.colors.cardBorder,
  },
  cardHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'flex-start',
    marginBottom: 16,
  },
  nameContainer: {
    flex: 1,
  },
  botName: {
    fontSize: 22,
    fontWeight: '700',
    fontFamily: lightTheme.typography.families.display,
    color: lightTheme.colors.text,
    marginBottom: 4,
  },
  persona: {
    fontSize: 14,
    color: lightTheme.colors.textSecondary,
    textTransform: 'capitalize',
    fontWeight: '500',
  },
  badge: {
    paddingHorizontal: 10,
    paddingVertical: 5,
    borderRadius: 6,
    minWidth: 80,
    alignItems: 'center',
  },
  badgeText: {
    fontSize: 10,
    fontWeight: '800',
    letterSpacing: 0.5,
  },
  cardBody: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  pnlContainer: {
    flexDirection: 'row',
    alignItems: 'baseline',
    gap: 6,
  },
  pnlLabel: {
    fontSize: 28,
    fontWeight: '700',
  },
  pnlSuffix: {
    fontSize: 14,
    color: lightTheme.colors.textMuted,
    fontWeight: '500',
  },
  positive: {
    color: lightTheme.colors.bullish[500],
  },
  negative: {
    color: lightTheme.colors.lobster[500],
  },
  heartbeatContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 6,
  },
  heartbeatDot: {
    width: 6,
    height: 6,
    borderRadius: 3,
    backgroundColor: lightTheme.colors.bullish[500],
  },
  heartbeat: {
    fontSize: 12,
    color: lightTheme.colors.textMuted,
    fontWeight: '500',
  },
  pendingBanner: {
    marginTop: 12,
    paddingVertical: 8,
    paddingHorizontal: 12,
    backgroundColor: lightTheme.colors.caution[50],
    borderRadius: 8,
    borderWidth: 1,
    borderColor: lightTheme.colors.caution[300],
  },
  pendingText: {
    fontSize: 13,
    color: lightTheme.colors.caution[700],
    fontWeight: '500',
  },
});
