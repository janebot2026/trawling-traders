import 'react-native-get-random-values'; // Must be first - polyfills crypto.getRandomValues
import React, { useEffect, useState } from 'react';
import { StatusBar } from 'expo-status-bar';
import { ActivityIndicator, View } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { CedrosLoginProvider } from '@cedros/login-react-native';
import { CedrosProvider } from '@cedros/pay-react-native';
import { AppNavigator } from './src/navigation/AppNavigator';
import { CEDROS_CONFIG, fetchCedrosPayConfig, type CedrosPayConfig } from './src/config/api';
import { ErrorBoundary } from './src/components/ErrorBoundary';
import { NetworkProvider } from './src/context/NetworkContext';

export default function App() {
  const [payConfig, setPayConfig] = useState<CedrosPayConfig | null>(null);
  const [initError, setInitError] = useState<Error | null>(null);

  useEffect(() => {
    let cancelled = false;

    (async () => {
      try {
        const config = await fetchCedrosPayConfig();
        if (!cancelled) setPayConfig(config);
      } catch (error) {
        if (!cancelled) {
          setInitError(error instanceof Error ? error : new Error(String(error)));
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  if (initError) {
    throw initError;
  }

  if (!payConfig) {
    return (
      <SafeAreaProvider>
        <View style={{ flex: 1, alignItems: 'center', justifyContent: 'center', backgroundColor: '#fff' }}>
          <ActivityIndicator size="large" />
        </View>
      </SafeAreaProvider>
    );
  }

  return (
    <ErrorBoundary>
      <CedrosLoginProvider config={CEDROS_CONFIG}>
        <CedrosProvider config={payConfig}>
          <SafeAreaProvider>
            <NetworkProvider>
              <AppNavigator />
              <StatusBar style="auto" />
            </NetworkProvider>
          </SafeAreaProvider>
        </CedrosProvider>
      </CedrosLoginProvider>
    </ErrorBoundary>
  );
}
