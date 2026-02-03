import React from 'react';
import { StatusBar } from 'expo-status-bar';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { CedrosLoginProvider } from '@cedros/login-react-native';
import { CedrosProvider } from '@cedros/pay-react-native';
import { AppNavigator } from './src/navigation/AppNavigator';

// Control plane API URL - update this to match your deployment
const CONTROL_PLANE_URL = process.env.CONTROL_PLANE_URL || 'http://localhost:3000';

export default function App() {
  return (
    <CedrosLoginProvider
      config={{
        serverUrl: CONTROL_PLANE_URL,
        timeout: 30000,
        retries: 3,
      }}
    >
      <CedrosProvider
        config={{
          apiUrl: CONTROL_PLANE_URL,
          stripePublicKey: process.env.STRIPE_PUBLIC_KEY,
          solanaNetwork: 'mainnet-beta',
        }}
      >
        <SafeAreaProvider>
          <AppNavigator />
          <StatusBar style="auto" />
        </SafeAreaProvider>
      </CedrosProvider>
    </CedrosLoginProvider>
  );
}
