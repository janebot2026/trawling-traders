import { useEffect, useRef, useCallback } from 'react';
import { Animated, Easing } from 'react-native';

// Animation configuration types
export interface AnimationConfig {
  duration?: number;
  delay?: number;
  easing?: (value: number) => number;
}

// Animation result with cleanup
export interface AnimationResult {
  value: Animated.Value;
  start: () => void;
  stop: () => void;
}

/**
 * Hook for fade-in animation with automatic cleanup on unmount
 */
export function useFadeIn(config: AnimationConfig = {}): AnimationResult {
  const { duration = 300, delay = 0, easing = Easing.ease } = config;
  const animatedValue = useRef(new Animated.Value(0)).current;
  const animationRef = useRef<Animated.CompositeAnimation | null>(null);

  const stop = useCallback(() => {
    animationRef.current?.stop();
  }, []);

  const start = useCallback(() => {
    animationRef.current = Animated.timing(animatedValue, {
      toValue: 1,
      duration,
      delay,
      useNativeDriver: true,
      easing,
    });
    animationRef.current.start();
  }, [animatedValue, duration, delay, easing]);

  useEffect(() => {
    return () => {
      stop();
    };
  }, [stop]);

  return { value: animatedValue, start, stop };
}

/**
 * Hook for slide animation with automatic cleanup
 */
export function useSlideIn(
  initialValue: number,
  config: AnimationConfig = {}
): AnimationResult {
  const { duration = 250, easing = Easing.out(Easing.cubic) } = config;
  const animatedValue = useRef(new Animated.Value(initialValue)).current;
  const animationRef = useRef<Animated.CompositeAnimation | null>(null);

  const stop = useCallback(() => {
    animationRef.current?.stop();
  }, []);

  const start = useCallback(() => {
    animationRef.current = Animated.timing(animatedValue, {
      toValue: 0,
      duration,
      useNativeDriver: true,
      easing,
    });
    animationRef.current.start();
  }, [animatedValue, duration, easing]);

  useEffect(() => {
    return () => {
      stop();
    };
  }, [stop]);

  return { value: animatedValue, start, stop };
}

/**
 * Hook for pulse animation (looping) with automatic cleanup
 */
export function usePulseAnimation(
  minScale = 1,
  maxScale = 1.2,
  duration = 500
): AnimationResult {
  const animatedValue = useRef(new Animated.Value(minScale)).current;
  const animationRef = useRef<Animated.CompositeAnimation | null>(null);

  const stop = useCallback(() => {
    animationRef.current?.stop();
    animatedValue.setValue(minScale);
  }, [animatedValue, minScale]);

  const start = useCallback(() => {
    animationRef.current = Animated.loop(
      Animated.sequence([
        Animated.timing(animatedValue, {
          toValue: maxScale,
          duration,
          useNativeDriver: true,
          easing: Easing.inOut(Easing.ease),
        }),
        Animated.timing(animatedValue, {
          toValue: minScale,
          duration,
          useNativeDriver: true,
          easing: Easing.inOut(Easing.ease),
        }),
      ])
    );
    animationRef.current.start();
  }, [animatedValue, minScale, maxScale, duration]);

  useEffect(() => {
    return () => {
      stop();
    };
  }, [stop]);

  return { value: animatedValue, start, stop };
}

/**
 * Hook for shimmer/loading animation with automatic cleanup
 */
export function useShimmerAnimation(width: number, duration = 1500): AnimationResult {
  const animatedValue = useRef(new Animated.Value(-width)).current;
  const animationRef = useRef<Animated.CompositeAnimation | null>(null);

  const stop = useCallback(() => {
    animationRef.current?.stop();
    animatedValue.setValue(-width);
  }, [animatedValue, width]);

  const start = useCallback(() => {
    animationRef.current = Animated.loop(
      Animated.timing(animatedValue, {
        toValue: width,
        duration,
        useNativeDriver: true,
        easing: Easing.linear,
      })
    );
    animationRef.current.start();
  }, [animatedValue, width, duration]);

  useEffect(() => {
    return () => {
      stop();
    };
  }, [stop]);

  return { value: animatedValue, start, stop };
}

/**
 * Hook for press scale effect
 */
export function usePressScale(minScale = 0.98): {
  value: Animated.Value;
  onPressIn: () => void;
  onPressOut: () => void;
} {
  const animatedValue = useRef(new Animated.Value(1)).current;
  const animationRef = useRef<Animated.CompositeAnimation | null>(null);

  const onPressIn = useCallback(() => {
    animationRef.current?.stop();
    animationRef.current = Animated.spring(animatedValue, {
      toValue: minScale,
      useNativeDriver: true,
      friction: 5,
      tension: 300,
    });
    animationRef.current.start();
  }, [animatedValue, minScale]);

  const onPressOut = useCallback(() => {
    animationRef.current?.stop();
    animationRef.current = Animated.spring(animatedValue, {
      toValue: 1,
      useNativeDriver: true,
      friction: 5,
      tension: 300,
    });
    animationRef.current.start();
  }, [animatedValue]);

  useEffect(() => {
    return () => {
      animationRef.current?.stop();
    };
  }, []);

  return { value: animatedValue, onPressIn, onPressOut };
}

/**
 * Hook for staggered list item animations with cleanup
 */
export function useStaggeredFadeIn(
  itemCount: number,
  baseDelay = 50,
  duration = 300
): {
  values: Animated.Value[];
  start: () => void;
  stop: () => void;
  reset: () => void;
} {
  const valuesRef = useRef<Animated.Value[]>([]);
  const animationRef = useRef<Animated.CompositeAnimation | null>(null);

  // Initialize values array
  if (valuesRef.current.length !== itemCount) {
    valuesRef.current = Array.from({ length: itemCount }, () => new Animated.Value(0));
  }

  const stop = useCallback(() => {
    animationRef.current?.stop();
  }, []);

  const reset = useCallback(() => {
    stop();
    valuesRef.current.forEach(val => val.setValue(0));
  }, [stop]);

  const start = useCallback(() => {
    const animations = valuesRef.current.map((item, index) =>
      Animated.timing(item, {
        toValue: 1,
        duration,
        useNativeDriver: true,
        delay: index * baseDelay,
        easing: Easing.ease,
      })
    );
    animationRef.current = Animated.stagger(baseDelay, animations);
    animationRef.current.start();
  }, [baseDelay, duration]);

  useEffect(() => {
    return () => {
      stop();
    };
  }, [stop]);

  return { values: valuesRef.current, start, stop, reset };
}

// Common animation presets (unchanged)
export const AnimationPresets = {
  cardPress: {
    scale: { min: 0.98, max: 1 },
    duration: 100,
  },
  screenTransition: {
    duration: 250,
    easing: Easing.out(Easing.cubic),
  },
  listItem: {
    duration: 300,
    stagger: 50,
  },
  spinner: {
    duration: 1000,
    easing: Easing.linear,
  },
};

// Legacy exports for backwards compatibility (deprecated)

/** @deprecated Use useFadeIn hook instead */
export function fadeIn(duration = 300) {
  const opacity = new Animated.Value(0);
  const animation = Animated.timing(opacity, {
    toValue: 1,
    duration,
    useNativeDriver: true,
    easing: Easing.ease,
  });
  animation.start();
  return opacity;
}

/** @deprecated Use useSlideIn hook instead */
export function slideInFromRight(translateX: Animated.Value, duration = 250) {
  Animated.timing(translateX, {
    toValue: 0,
    duration,
    useNativeDriver: true,
    easing: Easing.out(Easing.cubic),
  }).start();
}

/** @deprecated Use usePressScale hook instead */
export function pressScale(animatedValue: Animated.Value, pressed: boolean) {
  Animated.spring(animatedValue, {
    toValue: pressed ? 0.98 : 1,
    useNativeDriver: true,
    friction: 5,
    tension: 300,
  }).start();
}

/** @deprecated Use usePulseAnimation hook instead */
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

/** @deprecated Use useStaggeredFadeIn hook instead */
export function staggerFadeIn(items: Animated.Value[], baseDelay = 50) {
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

/** @deprecated Use useShimmerAnimation hook instead */
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

/** @deprecated Use standard Animated.spring with cleanup */
export function bounce(animatedValue: Animated.Value) {
  return Animated.spring(animatedValue, {
    toValue: 1,
    useNativeDriver: true,
    friction: 3,
    tension: 40,
  });
}
