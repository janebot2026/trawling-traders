import React from 'react';
import { View, Text, StyleSheet, ScrollView, Image } from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';
import { SubscribeButton } from '@cedros/pay-react-native';
import { OceanBackground } from '../components/OceanBackground';
import { lightTheme } from '../theme';

const LOB_AVATAR = require('../../assets/lob-avatar.png');

type SubscribeScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'Subscribe'>;

export function SubscribeScreen() {
  const navigation = useNavigation<SubscribeScreenNavigationProp>();

  const handleSubscribeSuccess = (sessionId: string) => {
    console.log('Subscription initiated:', sessionId);
    // After successful Stripe checkout redirect, user lands back in app
    // Navigation to Main happens via deep link or success callback
    navigation.navigate('Main');
  };

  const handleSubscribeError = (error: string) => {
    console.error('Subscription error:', error);
  };

  return (
    <OceanBackground>
      <ScrollView contentContainerStyle={styles.scrollContainer}>
        <View style={styles.container}>
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
              <SubscribeButton
                resource="trader-pro-monthly"
                interval="month"
                label="Subscribe with Stripe"
                onSuccess={handleSubscribeSuccess}
                onError={handleSubscribeError}
                style={styles.subscribeButton}
                textStyle={styles.subscribeButtonText}
              />
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
    paddingTop: 40,
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
    borderColor: lightTheme.colors.primary[700],
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
    backgroundColor: 'rgba(255, 255, 255, 0.95)',
    borderRadius: 20,
    padding: 28,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 8 },
    shadowOpacity: 0.15,
    shadowRadius: 16,
    elevation: 8,
    borderWidth: 2,
    borderColor: lightTheme.colors.primary[200],
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
    color: lightTheme.colors.wave[900],
    marginBottom: 8,
  },
  planPrice: {
    fontSize: 36,
    fontWeight: 'bold',
    color: lightTheme.colors.primary[700],
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
  subscribeButton: {
    backgroundColor: lightTheme.colors.primary[700],
    paddingVertical: 16,
    borderRadius: 12,
    shadowColor: lightTheme.colors.primary[700],
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
