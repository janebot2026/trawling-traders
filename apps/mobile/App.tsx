import React, { useEffect, useState, useCallback, useRef } from 'react';
import { StatusBar } from 'expo-status-bar';
import { ActivityIndicator, Text, TouchableOpacity, View } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { useFonts } from 'expo-font';
import * as SplashScreen from 'expo-splash-screen';
import { CedrosLoginProvider } from '@cedros/login-react-native';
import { CedrosProvider } from '@cedros/pay-react-native';
import { AppNavigator } from './src/navigation/AppNavigator';
import { CEDROS_CONFIG, fetchCedrosPayConfig, type CedrosPayConfig } from './src/config/api';
import { ErrorBoundary } from './src/components/ErrorBoundary';
import { NetworkProvider } from './src/context/NetworkContext';
import { PaymentsProvider } from './src/context/PaymentsContext';
import { ApiProvider } from './src/api';

SplashScreen.preventAutoHideAsync();

export default function App() {
  const [fontsLoaded] = useFonts({
    BNRumble: require('./assets/fonts/BNRumble.otf'),
  });
  const [payConfig, setPayConfig] = useState<CedrosPayConfig | null>(null);
  const [configError, setConfigError] = useState<string | null>(null);
  const payConfigSeqRef = useRef(0);

  const loadPayConfig = React.useCallback(() => {
    const seq = ++payConfigSeqRef.current;
    setConfigError(null);
    fetchCedrosPayConfig()
      .then((config) => {
        if (seq === payConfigSeqRef.current) setPayConfig(config);
      })
      .catch((err) => {
        const message = err instanceof Error ? err.message : String(err);
        if (seq === payConfigSeqRef.current) setConfigError(message);
      });
    return () => {
      // Invalidate this attempt so its result is ignored if a newer one is in-flight
      payConfigSeqRef.current = seq + 1;
    };
  }, []);

  useEffect(() => {
    return loadPayConfig();
  }, [loadPayConfig]);

  const onLayoutRootView = useCallback(async () => {
    if (fontsLoaded) {
      await SplashScreen.hideAsync();
    }
  }, [fontsLoaded]);

  if (!fontsLoaded) {
    return null;
  }

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
      <View style={{ flex: 1 }} onLayout={onLayoutRootView}>
      <CedrosLoginProvider config={CEDROS_CONFIG}>
        <PaymentsProvider value={{ isPaymentsEnabled: !!payConfig, paymentConfigError: configError }}>
          {payConfig ? (
            <CedrosProvider config={payConfig}>{content}</CedrosProvider>
          ) : (
            <>
              {content}
              <View
                style={{
                  position: 'absolute',
                  bottom: 20,
                  left: 16,
                  right: 16,
                  borderRadius: 12,
                  padding: 12,
                  backgroundColor: '#fff8e1',
                  borderWidth: 1,
                  borderColor: '#fbbf24',
                  shadowColor: '#000',
                  shadowOffset: { width: 0, height: 2 },
                  shadowOpacity: 0.1,
                  shadowRadius: 4,
                  elevation: 2,
                }}
              >
                {configError ? (
                  <>
                    <Text style={{ textAlign: 'center', fontWeight: '600' }}>
                      Payments unavailable in this session.
                    </Text>
                    <Text style={{ marginTop: 4, textAlign: 'center', opacity: 0.75 }}>
                      {configError}
                    </Text>
                    <TouchableOpacity
                      style={{
                        marginTop: 10,
                        backgroundColor: '#0f172a',
                        paddingHorizontal: 16,
                        paddingVertical: 10,
                        borderRadius: 8,
                        alignSelf: 'center',
                      }}
                      onPress={loadPayConfig}
                    >
                      <Text style={{ color: '#fff', fontWeight: '600' }}>Retry payments init</Text>
                    </TouchableOpacity>
                  </>
                ) : (
                  <View style={{ alignItems: 'center', justifyContent: 'center', flexDirection: 'row' }}>
                    <ActivityIndicator size="small" />
                    <Text style={{ marginLeft: 8 }}>Loading payment configuration…</Text>
                  </View>
                )}
              </View>
            </>
          )}
        </PaymentsProvider>
      </CedrosLoginProvider>
      </View>
    </ErrorBoundary>
  );
}
