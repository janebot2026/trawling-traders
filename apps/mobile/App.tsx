import React, { useEffect, useState } from 'react';
import { StatusBar } from 'expo-status-bar';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { CedrosLoginProvider } from '@cedros/login-react-native';
import { CedrosProvider } from '@cedros/pay-react-native';
import { AppNavigator } from './src/navigation/AppNavigator';
import { CEDROS_CONFIG, fetchCedrosPayConfig, type CedrosPayConfig } from './src/config/api';
import { ErrorBoundary } from './src/components/ErrorBoundary';
import { NetworkProvider } from './src/context/NetworkContext';
import { ApiProvider } from './src/api';

export default function App() {
  const [payConfig, setPayConfig] = useState<CedrosPayConfig | null>(null);

  useEffect(() => {
    let cancelled = false;
    fetchCedrosPayConfig()
      .then((config) => { if (!cancelled) setPayConfig(config); })
      .catch((err) => console.warn('Pay config unavailable, payments disabled:', err));
    return () => { cancelled = true; };
  }, []);

  const content = (
    <ApiProvider>
      <SafeAreaProvider>
        <NetworkProvider>
          <AppNavigator />
          <StatusBar style="auto" />
        </NetworkProvider>
      </SafeAreaProvider>
    </ApiProvider>
  );

  return (
    <ErrorBoundary>
      <CedrosLoginProvider config={CEDROS_CONFIG}>
        {payConfig ? (
          <CedrosProvider config={payConfig}>{content}</CedrosProvider>
        ) : (
          content
        )}
      </CedrosLoginProvider>
    </ErrorBoundary>
  );
}
