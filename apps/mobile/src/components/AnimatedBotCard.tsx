import React, { useRef, useEffect } from 'react';
import {
  View,
  Text,
  StyleSheet,
  TouchableOpacity,
  Animated,
} from 'react-native';
import type { Bot } from '@trawling-traders/types';
import { pressScale } from '../utils/animations';

interface AnimatedBotCardProps {
  bot: Bot;
  onPress: () => void;
  index?: number;
}

function StatusBadge({ status }: { status: Bot['status'] }) {
  const pulseAnim = useRef(new Animated.Value(1)).current;
  
  const colors: Record<Bot['status'], { bg: string; text: string }> = {
    provisioning: { bg: '#FFA500', text: '#fff' },
    online: { bg: '#4CAF50', text: '#fff' },
    offline: { bg: '#9E9E9E', text: '#fff' },
    paused: { bg: '#FFC107', text: '#000' },
    error: { bg: '#F44336', text: '#fff' },
    destroying: { bg: '#9C27B0', text: '#fff' },
  };
  
  const color = colors[status];
  
  // Pulse animation for provisioning/loading states
  useEffect(() => {
    if (status === 'provisioning' || status === 'destroying') {
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
      return () => pulse.stop();
    }
  }, [status]);
  
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
  
  // Entrance animation
  useEffect(() => {
    Animated.parallel([
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
    ]).start();
  }, [index]);
  
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
            <Text style={styles.persona}>{bot.persona}</Text>
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
            <Text style={styles.pendingText}>⏳ Config update pending</Text>
          </View>
        )}
      </Animated.View>
    </TouchableOpacity>
  );
}

const styles = StyleSheet.create({
  card: {
    backgroundColor: '#fff',
    borderRadius: 16,
    padding: 20,
    marginBottom: 16,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.08,
    shadowRadius: 8,
    elevation: 4,
    borderWidth: 1,
    borderColor: '#f0f0f0',
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
    color: '#1a1a1a',
    marginBottom: 4,
  },
  persona: {
    fontSize: 14,
    color: '#666',
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
    color: '#999',
    fontWeight: '500',
  },
  positive: {
    color: '#22c55e',
  },
  negative: {
    color: '#ef4444',
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
    backgroundColor: '#22c55e',
  },
  heartbeat: {
    fontSize: 12,
    color: '#999',
    fontWeight: '500',
  },
  pendingBanner: {
    marginTop: 12,
    paddingVertical: 8,
    paddingHorizontal: 12,
    backgroundColor: '#fffbeb',
    borderRadius: 8,
    borderWidth: 1,
    borderColor: '#fcd34d',
  },
  pendingText: {
    fontSize: 13,
    color: '#92400e',
    fontWeight: '500',
  },
});
