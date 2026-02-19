import React from 'react';
import { View, Text, StyleSheet, ScrollView, Image } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';
import { SubscribeButton } from '@cedros/pay-react-native';
import { OceanBackground } from '../components/OceanBackground';
import { lightTheme } from '../theme';
import { usePayments } from '../context/PaymentsContext';

const LOB_AVATAR = require('../../assets/lob-avatar.png');

type SubscribeScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'Subscribe'>;

export function SubscribeScreen() {
  const navigation = useNavigation<SubscribeScreenNavigationProp>();
  const insets = useSafeAreaInsets();
  const { isPaymentsEnabled, paymentConfigError } = usePayments();

  const handleSubscribeSuccess = (sessionId: string) => {
    if (__DEV__) {
      console.log('Subscription initiated:', sessionId);
    }
    // After successful Stripe checkout redirect, user lands back in app
    // Navigation to Main happens via deep link or success callback
    navigation.navigate('Main');
  };

  const handleSubscribeError = (error: string) => {
    if (__DEV__) {
      console.error('Subscription error:', error);
    }
  };

  return (
    <OceanBackground>
      <ScrollView contentContainerStyle={styles.scrollContainer}>
        <View style={[styles.container, { paddingTop: insets.top + 16 }]}>
          {/* LOB Mascot */}
          <View style={styles.mascotContainer}>
            <Image source={LOB_AVATAR} style={styles.avatar} />
            <View style={styles.badge}>
              <Text style={styles.badgeText}>#4821</Text>
            </View>
          </View>

          <Text style={styles.title}>Choose Your Plan</Text>
          <Text style={styles.subtitle}>Start trawling the markets today</Text>

          <View style={styles.planCard}>
            <View style={styles.planHeader}>
              <Text style={styles.planName}>Trader Pro</Text>
              <Text style={styles.planPrice}>$29/month</Text>
            </View>

            <View style={styles.features}>
              <View style={styles.featureRow}>
                <Text style={styles.featureIcon}>🤖</Text>
                <Text style={styles.featureText}>Up to 4 trading bots</Text>
              </View>
              <View style={styles.featureRow}>
                <Text style={styles.featureIcon}>📈</Text>
                <Text style={styles.featureText}>Paper & Live trading</Text>
              </View>
              <View style={styles.featureRow}>
                <Text style={styles.featureIcon}>📊</Text>
                <Text style={styles.featureText}>Real-time metrics</Text>
              </View>
              <View style={styles.featureRow}>
                <Text style={styles.featureIcon}>🦞</Text>
                <Text style={styles.featureText}>Priority support</Text>
              </View>
            </View>

            <View style={styles.buttonContainer}>
              {isPaymentsEnabled ? (
                <SubscribeButton
                  resource="trader-pro-monthly"
                  interval="monthly"
                  label="Subscribe with Stripe"
                  onSuccess={handleSubscribeSuccess}
                  onError={handleSubscribeError}
                  style={styles.subscribeButton}
                  textStyle={styles.subscribeButtonText}
                />
              ) : (
                <View style={styles.unavailableBox}>
                  <Text style={styles.unavailableTitle}>Payments are temporarily unavailable.</Text>
                  {paymentConfigError ? (
                    <Text style={styles.unavailableMessage}>{paymentConfigError}</Text>
                  ) : null}
                </View>
              )}
            </View>
          </View>

          <View style={styles.secureContainer}>
            <Text style={styles.secureText}>🔒 Secure payment via Cedros Pay</Text>
            <Text style={styles.secureSubtext}>Cancel anytime. 30-day money-back guarantee.</Text>
          </View>

          {/* Lobster footer */}
          <View style={styles.footer}>
            <Text style={styles.footerText}>LOB is waiting to start trawling</Text>
            <Text style={styles.footerSub}>Your personal trading companion</Text>
          </View>
        </View>
      </ScrollView>
    </OceanBackground>
  );
}

const styles = StyleSheet.create({
  scrollContainer: {
    flexGrow: 1,
  },
  container: {
    flex: 1,
    padding: 24,
  },
  mascotContainer: {
    alignSelf: 'center',
    position: 'relative',
    marginBottom: 24,
  },
  avatar: {
    width: 120,
    height: 120,
    borderRadius: 60,
    borderWidth: 4,
    borderColor: lightTheme.colors.cardBorder,
  },
  badge: {
    position: 'absolute',
    bottom: 0,
    right: 0,
    backgroundColor: lightTheme.colors.bullish[500],
    paddingHorizontal: 12,
    paddingVertical: 4,
    borderRadius: 12,
    borderWidth: 3,
    borderColor: '#fff',
  },
  badgeText: {
    color: '#fff',
    fontWeight: 'bold',
    fontSize: 12,
  },
  title: {
    fontSize: 32,
    fontWeight: 'bold',
    fontFamily: lightTheme.typography.families.display,
    textAlign: 'center',
    color: lightTheme.colors.wave[900],
    marginBottom: 8,
  },
  subtitle: {
    fontSize: 16,
    textAlign: 'center',
    color: lightTheme.colors.wave[600],
    marginBottom: 32,
  },
  planCard: {
    backgroundColor: lightTheme.colors.surface,
    borderRadius: 24,
    padding: 28,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 8 },
    shadowOpacity: 0.15,
    shadowRadius: 16,
    elevation: 8,
    borderWidth: 2,
    borderColor: lightTheme.colors.cardBorder,
  },
  planHeader: {
    alignItems: 'center',
    marginBottom: 24,
    paddingBottom: 20,
    borderBottomWidth: 2,
    borderBottomColor: lightTheme.colors.wave[100],
  },
  planName: {
    fontSize: 28,
    fontWeight: 'bold',
    fontFamily: lightTheme.typography.families.display,
    color: lightTheme.colors.wave[900],
    marginBottom: 8,
  },
  planPrice: {
    fontSize: 36,
    fontWeight: 'bold',
    fontFamily: lightTheme.typography.families.display,
    color: lightTheme.colors.accent,
  },
  features: {
    marginBottom: 24,
    gap: 16,
  },
  featureRow: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 12,
  },
  featureIcon: {
    fontSize: 24,
    width: 32,
    textAlign: 'center',
  },
  featureText: {
    fontSize: 16,
    color: lightTheme.colors.wave[700],
    fontWeight: '500',
  },
  buttonContainer: {
    marginTop: 8,
  },
  unavailableBox: {
    borderRadius: 14,
    borderWidth: 1,
    borderColor: lightTheme.colors.cardBorder,
    backgroundColor: lightTheme.colors.wave[50],
    padding: 14,
  },
  unavailableTitle: {
    fontSize: 15,
    fontWeight: '700',
    color: lightTheme.colors.wave[900],
    textAlign: 'center',
  },
  unavailableMessage: {
    marginTop: 6,
    fontSize: 12,
    color: lightTheme.colors.wave[600],
    textAlign: 'center',
  },
  subscribeButton: {
    backgroundColor: lightTheme.colors.accent,
    paddingVertical: 16,
    borderRadius: 20,
    shadowColor: lightTheme.colors.accent,
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.3,
    shadowRadius: 12,
    elevation: 5,
  },
  subscribeButtonText: {
    fontSize: 18,
    fontWeight: 'bold',
    color: '#fff',
  },
  secureContainer: {
    alignItems: 'center',
    marginTop: 24,
    marginBottom: 32,
  },
  secureText: {
    fontSize: 14,
    color: lightTheme.colors.wave[600],
  },
  secureSubtext: {
    fontSize: 12,
    color: lightTheme.colors.wave[400],
    marginTop: 4,
  },
  footer: {
    alignItems: 'center',
    marginTop: 'auto',
  },
  footerText: {
    fontSize: 14,
    color: lightTheme.colors.wave[600],
    fontStyle: 'italic',
  },
  footerSub: {
    fontSize: 12,
    color: lightTheme.colors.wave[400],
    marginTop: 4,
  },
});
