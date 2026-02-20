import React, { useCallback, useEffect, useMemo, useState } from 'react';
import {
  ActivityIndicator,
  Alert,
  ImageBackground,
  RefreshControl,
  ScrollView,
  StyleSheet,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import { CommonActions, useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { useCedrosLogin } from '@cedros/login-react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import type { UserSettings } from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';
import type { RootStackParamList } from '../navigation/AppNavigator';
import { lightTheme } from '../theme';
import { useSettingsStore } from '../store';
const PROFILE_BG = require('../../../../assets/branding/tt-head.png');
const HEADER_HEIGHT = 56;

type ProfileScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'Profile'>;

export function ProfileScreen() {
  const navigation = useNavigation<ProfileScreenNavigationProp>();
  const insets = useSafeAreaInsets();
  const contentTopPadding = insets.top + HEADER_HEIGHT + 10;
  const { logout } = useCedrosLogin();
  const clearApiKeys = useSettingsStore((s) => s.clearApiKeys);
  const [settings, setSettings] = useState<UserSettings | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isLoggingOut, setIsLoggingOut] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadProfile = useCallback(async () => {
    setError(null);
    try {
      const settingsResponse = await api.user.getSettings();
      setSettings(settingsResponse);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load profile');
      setSettings(null);
    } finally {
      setIsLoading(false);
      setIsRefreshing(false);
    }
  }, []);

  useEffect(() => {
    loadProfile();
  }, [loadProfile]);

  const onRefresh = () => {
    setIsRefreshing(true);
    loadProfile();
  };

  const displayName = useMemo(() => {
    const name = settings?.displayName?.trim();
    if (name) return name;
    return settings?.email ?? 'Trader';
  }, [settings?.displayName, settings?.email]);

  const handleLogout = () => {
    Alert.alert('Sign Out', 'Are you sure you want to sign out?', [
      { text: 'Cancel', style: 'cancel' },
      {
        text: 'Sign Out',
        style: 'destructive',
        onPress: async () => {
          setIsLoggingOut(true);
          clearApiKeys();
          try {
            await logout();
          } catch {
            // fallback navigation still applies below
          } finally {
            setIsLoggingOut(false);
            navigation.dispatch(
              CommonActions.reset({
                index: 0,
                routes: [{ name: 'Auth' }],
              })
            );
          }
        },
      },
    ]);
  };

  if (isLoading) {
    return (
      <ImageBackground source={PROFILE_BG} style={styles.bgFill} resizeMode="cover">
        <View style={styles.centered}>
          <ActivityIndicator size="large" color={lightTheme.colors.primary[700]} />
        </View>
      </ImageBackground>
    );
  }

  return (
    <ImageBackground source={PROFILE_BG} style={styles.bgFill} resizeMode="cover">
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
        {error && (
          <View style={styles.errorCard}>
            <Text style={styles.errorTitle}>Unable to load profile</Text>
            <Text style={styles.errorText}>{error}</Text>
          </View>
        )}

        <View style={styles.card}>
          <Text style={styles.sectionTitle}>Account</Text>
          <View style={styles.row}>
            <Text style={styles.label}>Display Name</Text>
            <Text style={styles.value}>{displayName}</Text>
          </View>
          <View style={styles.row}>
            <Text style={styles.label}>Email</Text>
            <Text style={styles.value}>{settings?.email || 'No email set'}</Text>
          </View>
          <View style={styles.row}>
            <Text style={styles.label}>Google Sign-In</Text>
            <Text style={styles.value}>{settings?.authMethods.google ? 'Connected' : 'Not connected'}</Text>
          </View>
          <View style={styles.row}>
            <Text style={styles.label}>Apple Sign-In</Text>
            <Text style={styles.value}>{settings?.authMethods.apple ? 'Connected' : 'Not connected'}</Text>
          </View>

          <TouchableOpacity style={styles.primaryButton} onPress={() => navigation.navigate('Settings')}>
            <Text style={styles.primaryButtonText}>Edit Settings</Text>
          </TouchableOpacity>
        </View>

        <View style={styles.card}>
          <Text style={styles.sectionTitle}>Session</Text>
          <TouchableOpacity
            style={[styles.dangerButton, isLoggingOut && styles.dangerButtonDisabled]}
            onPress={handleLogout}
            disabled={isLoggingOut}
          >
            <Text style={styles.dangerButtonText}>{isLoggingOut ? 'Signing out...' : 'Sign Out'}</Text>
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
  card: {
    marginTop: 14,
    borderRadius: 16,
    borderWidth: 1,
    borderColor: '#9ed6ff',
    backgroundColor: 'rgba(246, 251, 255, 0.92)',
    padding: 16,
    shadowColor: '#063f6c',
    shadowOpacity: 0.14,
    shadowRadius: 8,
    shadowOffset: { width: 0, height: 4 },
    elevation: 2,
  },
  sectionTitle: {
    fontSize: 17,
    fontWeight: '700',
    color: lightTheme.colors.wave[900],
    marginBottom: 12,
  },
  row: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 10,
  },
  label: {
    fontSize: 13,
    color: lightTheme.colors.wave[700],
  },
  value: {
    fontSize: 14,
    fontWeight: '700',
    color: lightTheme.colors.wave[900],
  },
  primaryButton: {
    marginTop: 12,
    borderRadius: 10,
    backgroundColor: lightTheme.colors.primary[700],
    paddingVertical: 11,
    alignItems: 'center',
    borderWidth: 1,
    borderColor: lightTheme.colors.primary[800],
  },
  primaryButtonText: {
    color: '#fff',
    fontSize: 14,
    fontWeight: '700',
  },
  secondaryButton: {
    marginTop: 12,
    borderRadius: 10,
    borderWidth: 1,
    borderColor: '#5ca8e4',
    paddingVertical: 11,
    alignItems: 'center',
    backgroundColor: 'rgba(255,255,255,0.9)',
  },
  secondaryButtonText: {
    color: lightTheme.colors.primary[700],
    fontSize: 14,
    fontWeight: '700',
  },
  dangerButton: {
    borderRadius: 10,
    backgroundColor: lightTheme.colors.lobster[600],
    paddingVertical: 11,
    alignItems: 'center',
  },
  dangerButtonDisabled: {
    opacity: 0.6,
  },
  dangerButtonText: {
    color: '#fff',
    fontSize: 14,
    fontWeight: '700',
  },
  errorCard: {
    marginTop: 14,
    borderRadius: 12,
    borderWidth: 1,
    borderColor: lightTheme.colors.lobster[300],
    backgroundColor: 'rgba(255, 236, 236, 0.94)',
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
