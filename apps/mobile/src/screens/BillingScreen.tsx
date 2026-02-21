import React, { useCallback, useEffect, useMemo, useState } from 'react';
import {
  ActivityIndicator,
  ImageBackground,
  Linking,
  RefreshControl,
  ScrollView,
  StyleSheet,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { API_URL } from '../config/api';
import type { RootStackParamList } from '../navigation/AppNavigator';
import type { BillingSummary } from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';
import { lightTheme } from '../theme';

type BillingScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'Billing'>;
const ENGINE_ROOM_BG = require('../../../../assets/branding/tt-engine-room.png');
const HEADER_HEIGHT = 56;

function formatPlanName(planCode: string): string {
  const normalized = planCode.toLowerCase();
  if (normalized.includes('enterprise')) return 'Enterprise';
  if (normalized.includes('pro')) return 'Trader Pro';
  return 'Free';
}

export function BillingScreen() {
  const navigation = useNavigation<BillingScreenNavigationProp>();
  const insets = useSafeAreaInsets();
  const contentTopPadding = insets.top + HEADER_HEIGHT + 10;
  const [billing, setBilling] = useState<BillingSummary | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadBilling = useCallback(async () => {
    setError(null);
    try {
      const response = await api.user.getBillingSummary();
      setBilling(response);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load billing');
      setBilling(null);
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, []);

  useEffect(() => {
    loadBilling();
  }, [loadBilling]);

  const usagePercent = useMemo(() => {
    if (!billing) return 0;
    if (billing.maxBots <= 0) return 0;
    return Math.min((billing.botCount / billing.maxBots) * 100, 100);
  }, [billing]);

  const onRefresh = () => {
    setIsRefreshing(true);
    loadBilling();
  };

  const openCheckout = () => {
    const url = `${API_URL}/paywall/v1/shop`;
    // MB-003: validate URL starts with https:// before opening to prevent open-redirect abuse
    if (!url.startsWith('https://')) {
      console.warn('[BillingScreen] Blocked non-https URL from Linking.openURL:', url);
      return;
    }
    Linking.openURL(url);
  };

  if (isLoading) {
    return (
      <ImageBackground source={ENGINE_ROOM_BG} style={styles.bgFill} resizeMode="cover">
        <View style={styles.centered}>
          <ActivityIndicator size="large" color={lightTheme.colors.primary[700]} />
        </View>
      </ImageBackground>
    );
  }

  return (
    <ImageBackground source={ENGINE_ROOM_BG} style={styles.bgFill} resizeMode="cover">
      <ScrollView
        contentContainerStyle={[styles.content, { paddingTop: contentTopPadding }]}
        refreshControl={
          <RefreshControl
            refreshing={isRefreshing}
            onRefresh={onRefresh}
            tintColor={lightTheme.colors.primary[700]}
          />
        }
      >
        <Text style={styles.title}>Billing</Text>
        <Text style={styles.subtitle}>Subscription status, usage, and billing controls.</Text>

        {error && (
          <View style={styles.errorCard}>
            <Text style={styles.errorTitle}>Unable to load billing</Text>
            <Text style={styles.errorText}>{error}</Text>
          </View>
        )}

        <View style={styles.card}>
          <Text style={styles.sectionTitle}>Current Plan</Text>
          <View style={styles.row}>
            <Text style={styles.label}>Plan</Text>
            <Text style={styles.value}>{formatPlanName(billing?.planCode || 'free')}</Text>
          </View>
          <View style={styles.row}>
            <Text style={styles.label}>Status</Text>
            <Text style={styles.value}>{(billing?.status || 'inactive').toUpperCase()}</Text>
          </View>
          <View style={styles.row}>
            <Text style={styles.label}>Renews</Text>
            <Text style={styles.value}>
              {billing?.currentPeriodEnd
                ? new Date(billing.currentPeriodEnd).toLocaleDateString()
                : 'N/A'}
            </Text>
          </View>
        </View>

        <View style={styles.card}>
          <Text style={styles.sectionTitle}>Usage</Text>
          <View style={styles.row}>
            <Text style={styles.label}>Bots Used</Text>
            <Text style={styles.value}>
              {billing?.botCount ?? 0} / {billing?.maxBots ?? 1}
            </Text>
          </View>
          <View style={styles.progressTrack}>
            <View
              style={[
                styles.progressFill,
                {
                  width: `${usagePercent}%`,
                  backgroundColor:
                    usagePercent > 80
                      ? lightTheme.colors.lobster[500]
                      : lightTheme.colors.bullish[500],
                },
              ]}
            />
          </View>
          <Text style={styles.helperText}>
            Keep usage below your plan limit to avoid bot creation failures.
          </Text>
        </View>

        <View style={styles.card}>
          <Text style={styles.sectionTitle}>Billing Actions</Text>
          <TouchableOpacity style={styles.primaryButton} onPress={openCheckout}>
            <Text style={styles.primaryButtonText}>Manage Subscription</Text>
          </TouchableOpacity>
          <TouchableOpacity
            style={styles.secondaryButton}
            onPress={() => navigation.navigate('Subscribe')}
          >
            <Text style={styles.secondaryButtonText}>Change Plan</Text>
          </TouchableOpacity>
        </View>
      </ScrollView>
    </ImageBackground>
  );
}

const styles = StyleSheet.create({
  bgFill: {
    flex: 1,
  },
  content: {
    padding: 16,
    paddingBottom: 28,
  },
  centered: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
  },
  title: {
    fontSize: 30,
    fontWeight: '700',
    color: lightTheme.colors.wave[900],
    fontFamily: lightTheme.typography.families.display,
  },
  subtitle: {
    marginTop: 8,
    fontSize: 14,
    color: lightTheme.colors.wave[600],
  },
  card: {
    marginTop: 14,
    borderRadius: 16,
    borderWidth: 1,
    borderColor: lightTheme.colors.cardBorder,
    backgroundColor: lightTheme.colors.surface,
    padding: 14,
  },
  sectionTitle: {
    fontSize: 17,
    fontWeight: '700',
    color: lightTheme.colors.wave[900],
    marginBottom: 10,
  },
  row: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 8,
  },
  label: {
    fontSize: 13,
    color: lightTheme.colors.wave[600],
  },
  value: {
    fontSize: 14,
    fontWeight: '700',
    color: lightTheme.colors.wave[900],
  },
  progressTrack: {
    marginTop: 4,
    height: 10,
    borderRadius: 999,
    backgroundColor: lightTheme.colors.wave[200],
    overflow: 'hidden',
  },
  progressFill: {
    height: '100%',
  },
  helperText: {
    marginTop: 8,
    fontSize: 12,
    color: lightTheme.colors.wave[500],
  },
  primaryButton: {
    borderRadius: 10,
    backgroundColor: lightTheme.colors.primary[700],
    paddingVertical: 11,
    alignItems: 'center',
  },
  primaryButtonText: {
    color: '#fff',
    fontSize: 14,
    fontWeight: '700',
  },
  secondaryButton: {
    marginTop: 10,
    borderRadius: 10,
    borderWidth: 1,
    borderColor: lightTheme.colors.primary[700],
    paddingVertical: 11,
    alignItems: 'center',
    backgroundColor: '#fff',
  },
  secondaryButtonText: {
    color: lightTheme.colors.primary[700],
    fontSize: 14,
    fontWeight: '700',
  },
  errorCard: {
    marginTop: 14,
    borderRadius: 12,
    borderWidth: 1,
    borderColor: lightTheme.colors.lobster[300],
    backgroundColor: lightTheme.colors.lobster[50],
    padding: 12,
  },
  errorTitle: {
    fontSize: 14,
    fontWeight: '700',
    color: lightTheme.colors.lobster[700],
  },
  errorText: {
    marginTop: 4,
    color: lightTheme.colors.lobster[700],
    fontSize: 12,
  },
});
