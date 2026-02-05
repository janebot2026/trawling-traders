import React from 'react';
import { StatusBar } from 'expo-status-bar';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { CedrosLoginProvider } from '@cedros/login-react-native';
import { CedrosProvider } from '@cedros/pay-react-native';
import { AppNavigator } from './src/navigation/AppNavigator';
import { CEDROS_CONFIG, CEDROS_PAY_CONFIG } from './src/config/api';
import { ErrorBoundary } from './src/components/ErrorBoundary';

export default function App() {
  return (
    <ErrorBoundary>
      <CedrosLoginProvider config={CEDROS_CONFIG}>
        <CedrosProvider config={CEDROS_PAY_CONFIG}>
          <SafeAreaProvider>
            <AppNavigator />
            <StatusBar style="auto" />
          </SafeAreaProvider>
        </CedrosProvider>
      </CedrosLoginProvider>
    </ErrorBoundary>
  );
}
