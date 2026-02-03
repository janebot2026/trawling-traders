import React from 'react';
import { StatusBar } from 'expo-status-bar';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { CedrosLoginProvider } from '@cedros/login-react-native';
import { CedrosProvider } from '@cedros/pay-react-native';
import { AppNavigator } from './src/navigation/AppNavigator';

// API Configuration
const API_URL = 'https://api.trawlingtraders.com';

export default function App() {
  return (
    <CedrosLoginProvider
      config={{
        serverUrl: API_URL,
        timeout: 30000,
        retries: 3,
      }}
    >
      <CedrosProvider
        config={{
          apiUrl: API_URL,
          // Stripe and Solana config left blank for now per instructions
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
