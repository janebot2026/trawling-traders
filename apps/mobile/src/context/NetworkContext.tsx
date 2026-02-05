import React, { createContext, useContext, useEffect, useState, useCallback, useRef } from 'react';
import { View, Text, StyleSheet, Animated } from 'react-native';
import { lightTheme } from '../theme';

interface NetworkContextValue {
  isOnline: boolean;
  checkConnection: () => Promise<boolean>;
}

const NetworkContext = createContext<NetworkContextValue>({
  isOnline: true,
  checkConnection: async () => true,
});

export function useNetworkStatus(): NetworkContextValue {
  return useContext(NetworkContext);
}

/**
 * Component that displays an offline banner when network is unavailable.
 * Automatically shows/hides with animation.
 */
function OfflineBanner({ isOnline }: { isOnline: boolean }) {
  const slideAnim = useRef(new Animated.Value(-50)).current;

  useEffect(() => {
    Animated.timing(slideAnim, {
      toValue: isOnline ? -50 : 0,
      duration: 300,
      useNativeDriver: true,
    }).start();
  }, [isOnline, slideAnim]);

  if (isOnline) {
    return null;
  }

  return (
    <Animated.View style={[styles.banner, { transform: [{ translateY: slideAnim }] }]}>
      <Text style={styles.bannerText}>No Internet Connection</Text>
      <Text style={styles.bannerSubtext}>Please check your network settings</Text>
    </Animated.View>
  );
}

/**
 * Provider that monitors network connectivity and provides status to children.
 * Uses a simple fetch-based approach for network detection that works reliably
 * across Expo and React Native without additional native dependencies.
 */
export function NetworkProvider({ children }: { children: React.ReactNode }) {
  const [isOnline, setIsOnline] = useState(true);
  const checkIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Simple connectivity check using fetch to a reliable endpoint
  const checkConnection = useCallback(async (): Promise<boolean> => {
    try {
      // Use a tiny request to Google's generate_204 endpoint (returns 204, no content)
      // This is commonly used for captive portal detection and is very reliable
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 5000);

      const response = await fetch('https://www.google.com/generate_204', {
        method: 'HEAD',
        signal: controller.signal,
        cache: 'no-store',
      });

      clearTimeout(timeout);
      const online = response.status === 204 || response.ok;
      setIsOnline(online);
      return online;
    } catch {
      setIsOnline(false);
      return false;
    }
  }, []);

  // Initial check and periodic monitoring
  useEffect(() => {
    // Check immediately on mount
    checkConnection();

    // Check every 30 seconds while app is active
    checkIntervalRef.current = setInterval(checkConnection, 30000);

    return () => {
      if (checkIntervalRef.current) {
        clearInterval(checkIntervalRef.current);
      }
    };
  }, [checkConnection]);

  return (
    <NetworkContext.Provider value={{ isOnline, checkConnection }}>
      <View style={styles.container}>
        <OfflineBanner isOnline={isOnline} />
        {children}
      </View>
    </NetworkContext.Provider>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  banner: {
    position: 'absolute',
    top: 0,
    left: 0,
    right: 0,
    backgroundColor: lightTheme.colors.lobster[600],
    paddingTop: 50, // Account for status bar
    paddingBottom: 12,
    paddingHorizontal: 16,
    zIndex: 9999,
    elevation: 10,
  },
  bannerText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: 'bold',
    textAlign: 'center',
  },
  bannerSubtext: {
    color: 'rgba(255, 255, 255, 0.8)',
    fontSize: 12,
    textAlign: 'center',
    marginTop: 2,
  },
});

export default NetworkProvider;
