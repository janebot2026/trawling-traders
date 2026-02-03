import React, { useEffect, useRef } from 'react';
import {
  View,
  Animated,
  StyleSheet,
  ViewStyle,
  TouchableOpacity,
} from 'react-native';
import { lightTheme, darkTheme } from '../theme';

interface AnimatedCardProps {
  children: React.ReactNode;
  style?: ViewStyle;
  delay?: number;
  onPress?: () => void;
  elevated?: boolean;
}

export function AnimatedCard({
  children,
  style,
  delay = 0,
  onPress,
  elevated = true,
}: AnimatedCardProps) {
  const fadeAnim = useRef(new Animated.Value(0)).current;
  const slideAnim = useRef(new Animated.Value(20)).current;
  const scaleAnim = useRef(new Animated.Value(0.95)).current;

  useEffect(() => {
    Animated.parallel([
      Animated.timing(fadeAnim, {
        toValue: 1,
        duration: 400,
        delay,
        useNativeDriver: true,
      }),
      Animated.timing(slideAnim, {
        toValue: 0,
        duration: 400,
        delay,
        useNativeDriver: true,
      }),
      Animated.spring(scaleAnim, {
        toValue: 1,
        friction: 8,
        tension: 40,
        delay,
        useNativeDriver: true,
      }),
    ]).start();
  }, [delay]);

  const animatedStyle = {
    opacity: fadeAnim,
    transform: [
      { translateY: slideAnim },
      { scale: scaleAnim },
    ],
  };

  const CardWrapper = onPress ? TouchableOpacity : View;

  return (
    <Animated.View style={[styles.container, animatedStyle, style]}>
      <CardWrapper
        onPress={onPress}
        activeOpacity={onPress ? 0.9 : 1}
        style={[styles.card, elevated && styles.elevated]}
      >
        {children}
      </CardWrapper>
    </Animated.View>
  );
}

const styles = StyleSheet.create({
  container: {
    marginHorizontal: 16,
    marginBottom: 12,
  },
  card: {
    backgroundColor: 'rgba(255, 255, 255, 0.95)',
    borderRadius: 16,
    padding: 16,
    borderWidth: 1,
    borderColor: 'rgba(255, 255, 255, 0.3)',
  },
  elevated: {
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.15,
    shadowRadius: 12,
    elevation: 5,
  },
});

// Pulse animation for loading states
export function PulseAnimation({ children }: { children: React.ReactNode }) {
  const pulseAnim = useRef(new Animated.Value(1)).current;

  useEffect(() => {
    Animated.loop(
      Animated.sequence([
        Animated.timing(pulseAnim, {
          toValue: 1.05,
          duration: 1000,
          useNativeDriver: true,
        }),
        Animated.timing(pulseAnim, {
          toValue: 1,
          duration: 1000,
          useNativeDriver: true,
        }),
      ])
    ).start();
  }, []);

  return (
    <Animated.View style={{ transform: [{ scale: pulseAnim }] }}>
      {children}
    </Animated.View>
  );
}

// Staggered children animation
interface StaggerContainerProps {
  children: React.ReactNode[];
  staggerDelay?: number;
}

export function StaggerContainer({
  children,
  staggerDelay = 100,
}: StaggerContainerProps) {
  return (
    <>
      {React.Children.map(children, (child, index) => (
        <AnimatedCard key={index} delay={index * staggerDelay}>
          {child}
        </AnimatedCard>
      ))}
    </>
  );
}
