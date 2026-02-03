import { Animated, Easing } from 'react-native';

// Fade in animation for list items
export function fadeIn(duration = 300) {
  const opacity = new Animated.Value(0);
  
  Animated.timing(opacity, {
    toValue: 1,
    duration,
    useNativeDriver: true,
    easing: Easing.ease,
  }).start();
  
  return opacity;
}

// Slide in from right for screen transitions
export function slideInFromRight(translateX: Animated.Value, duration = 250) {
  Animated.timing(translateX, {
    toValue: 0,
    duration,
    useNativeDriver: true,
    easing: Easing.out(Easing.cubic),
  }).start();
}

// Scale press effect for cards/buttons
export function pressScale(animatedValue: Animated.Value, pressed: boolean) {
  Animated.spring(animatedValue, {
    toValue: pressed ? 0.98 : 1,
    useNativeDriver: true,
    friction: 5,
    tension: 300,
  }).start();
}

// Pulsing animation for status badges
export function pulse(animatedValue: Animated.Value) {
  return Animated.loop(
    Animated.sequence([
      Animated.timing(animatedValue, {
        toValue: 1.2,
        duration: 500,
        useNativeDriver: true,
        easing: Easing.inOut(Easing.ease),
      }),
      Animated.timing(animatedValue, {
        toValue: 1,
        duration: 500,
        useNativeDriver: true,
        easing: Easing.inOut(Easing.ease),
      }),
    ])
  );
}

// Stagger animation for lists
export function staggerFadeIn(
  items: Animated.Value[],
  baseDelay = 50
) {
  const animations = items.map((item, index) =>
    Animated.timing(item, {
      toValue: 1,
      duration: 300,
      useNativeDriver: true,
      delay: index * baseDelay,
      easing: Easing.ease,
    })
  );
  
  Animated.stagger(baseDelay, animations).start();
}

// Shimmer effect for loading states
export function shimmer(translateX: Animated.Value, width: number) {
  return Animated.loop(
    Animated.timing(translateX, {
      toValue: width,
      duration: 1500,
      useNativeDriver: true,
      easing: Easing.linear,
    })
  );
}

// Bounce animation for attention
export function bounce(animatedValue: Animated.Value) {
  return Animated.spring(animatedValue, {
    toValue: 1,
    useNativeDriver: true,
    friction: 3,
    tension: 40,
  });
}

// Common animation values
export const AnimationPresets = {
  // Card press
  cardPress: {
    scale: { min: 0.98, max: 1 },
    duration: 100,
  },
  // Screen transition
  screenTransition: {
    duration: 250,
    easing: Easing.out(Easing.cubic),
  },
  // List item entrance
  listItem: {
    duration: 300,
    stagger: 50,
  },
  // Loading spinner
  spinner: {
    duration: 1000,
    easing: Easing.linear,
  },
};
