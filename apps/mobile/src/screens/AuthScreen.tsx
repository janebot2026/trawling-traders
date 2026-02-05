import React, { useEffect, useRef } from 'react';
import {
  View,
  Text,
  StyleSheet,
  ActivityIndicator,
  Image,
  ScrollView,
  Animated,
  Dimensions,
} from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';
import { OceanBackground } from '../components/OceanBackground';
import { lightTheme } from '../theme';
import {
  useCedrosLogin,
  EmailLoginForm,
  GoogleLoginButton,
} from '@cedros/login-react-native';
import { api } from '@trawling-traders/api-client';

const { width } = Dimensions.get('window');

// LOB Avatar - our lobster mascot
const LOB_AVATAR = require('../../assets/lob-avatar.png');

type AuthScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'Auth'>;

export function AuthScreen() {
  const navigation = useNavigation<AuthScreenNavigationProp>();
  const { isAuthenticated, isLoading: authLoading, user } = useCedrosLogin();

  // Animation values
  const fadeAnim = React.useRef(new Animated.Value(0)).current;
  const slideAnim = React.useRef(new Animated.Value(50)).current;
  const scaleAnim = React.useRef(new Animated.Value(0.8)).current;
  const featureAnim = React.useRef(new Animated.Value(0)).current;

  // Track if navigation is in progress to prevent race condition
  const isNavigatingRef = useRef(false);

  // Navigate when authenticated, checking subscription status first
  useEffect(() => {
    if (isAuthenticated && user && !isNavigatingRef.current) {
      isNavigatingRef.current = true;

      // Check subscription status before navigating
      api.getMe()
        .then((userData) => {
          const subscription = userData.subscription;
          const isActive = subscription?.status === 'active';

          if (isActive) {
            navigation.navigate('Main');
          } else {
            // No active subscription - go to subscribe screen
            navigation.navigate('Subscribe');
          }
        })
        .catch((error) => {
          if (__DEV__) {
            console.error('Failed to check subscription status:', error);
          }
          // On error, default to Main (subscription middleware will handle)
          navigation.navigate('Main');
        })
        .finally(() => {
          isNavigatingRef.current = false;
        });
    }
  }, [isAuthenticated, user, navigation]);

  useEffect(() => {
    // Start entrance animations
    Animated.sequence([
      Animated.parallel([
        Animated.timing(fadeAnim, {
          toValue: 1,
          duration: 800,
          useNativeDriver: true,
        }),
        Animated.spring(scaleAnim, {
          toValue: 1,
          friction: 6,
          tension: 40,
          useNativeDriver: true,
        }),
        Animated.timing(slideAnim, {
          toValue: 0,
          duration: 600,
          useNativeDriver: true,
        }),
      ]),
      Animated.timing(featureAnim, {
        toValue: 1,
        duration: 500,
        useNativeDriver: true,
      }),
    ]).start();
  }, []);

  if (authLoading) {
    return (
      <OceanBackground>
        <View style={styles.loadingContainer}>
          <Animated.Image
            source={LOB_AVATAR}
            style={[
              styles.loadingAvatar,
              {
                transform: [{ scale: scaleAnim }],
              },
            ]}
          />
          <View style={styles.loadingTextContainer}>
            <Text style={styles.loadingText}>Trawling Traders</Text>
            <ActivityIndicator
              size="small"
              color={lightTheme.colors.primary[700]}
              style={styles.loadingIndicator}
            />
          </View>
        </View>
      </OceanBackground>
    );
  }

  return (
    <OceanBackground>
      <ScrollView
        contentContainerStyle={styles.scrollContainer}
        showsVerticalScrollIndicator={false}
      >
        <Animated.View
          style={[
            styles.container,
            {
              opacity: fadeAnim,
              transform: [{ translateY: slideAnim }],
            },
          ]}
        >
          {/* LOB Mascot */}
          <View style={styles.mascotContainer}>
            <Image source={LOB_AVATAR} style={styles.avatar} />
            <View style={styles.badge}>
              <Text style={styles.badgeText}>#4821</Text>
            </View>
            <View style={styles.statusIndicator} />
          </View>

          <Text style={styles.greeting}>Ahoy, Trader! 🦞</Text>
          <Text style={styles.title}>Trawling Traders</Text>
          <Text style={styles.subtitle}>
            Deploy AI-powered trading bots that trawl the markets 24/7
          </Text>

          {/* Feature highlights */}
          <Animated.View
            style={[
              styles.features,
              { opacity: featureAnim },
            ]}
          >
            <View style={styles.feature}>
              <View style={[styles.featureIcon, { backgroundColor: lightTheme.colors.bullish[100] }]}>
                <Text style={[styles.featureEmoji, { color: lightTheme.colors.bullish[600] }]}>🤖</Text>
              </View>
              <Text style={styles.featureText}>Up to 4 bots</Text>
            </View>

            <View style={styles.feature}>
              <View style={[styles.featureIcon, { backgroundColor: lightTheme.colors.primary[100] }]}>
                <Text style={[styles.featureEmoji, { color: lightTheme.colors.primary[600] }]}>📈</Text>
              </View>
              <Text style={styles.featureText}>xStocks & crypto</Text>
            </View>

            <View style={styles.feature}>
              <View style={[styles.featureIcon, { backgroundColor: lightTheme.colors.lobster[100] }]}>
                <Text style={[styles.featureEmoji, { color: lightTheme.colors.lobster[600] }]}>🔒</Text>
              </View>
              <Text style={styles.featureText}>Your own VPS</Text>
            </View>
          </Animated.View>

          {/* Cedros Login Form */}
          <Animated.View style={[styles.authContainer, { opacity: featureAnim }]}>
            <EmailLoginForm
              onSubmit={async (email, _password) => {
                // CedrosLoginProvider handles the auth state
                // Navigation happens automatically via useEffect above
                if (__DEV__) {
                  console.log('Login attempt:', email);
                }
              }}
              isLoading={authLoading}
            />
          </Animated.View>

          <Text style={styles.hint}>Secure authentication powered by Cedros</Text>

          {/* Lobster footer */}
          <View style={styles.footer}>
            <Text style={styles.footerText}>Meet LOB — your trading companion</Text>
            <Text style={styles.footerSub}>Always trawling, never sleeping</Text>
          </View>
        </Animated.View>
      </ScrollView>
    </OceanBackground>
  );
}

const styles = StyleSheet.create({
  scrollContainer: {
    flexGrow: 1,
  },
  loadingContainer: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  loadingAvatar: {
    width: 140,
    height: 140,
    borderRadius: 70,
    borderWidth: 4,
    borderColor: lightTheme.colors.primary[700],
  },
  loadingTextContainer: {
    marginTop: 24,
    alignItems: 'center',
  },
  loadingText: {
    fontSize: 24,
    fontWeight: 'bold',
    color: lightTheme.colors.wave[800],
    marginBottom: 12,
  },
  loadingIndicator: {
    marginTop: 8,
  },
  container: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    padding: 24,
    paddingTop: 60,
    paddingBottom: 40,
  },
  mascotContainer: {
    position: 'relative',
    marginBottom: 24,
  },
  avatar: {
    width: 160,
    height: 160,
    borderRadius: 80,
    borderWidth: 4,
    borderColor: lightTheme.colors.primary[700],
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 8 },
    shadowOpacity: 0.2,
    shadowRadius: 16,
    elevation: 8,
  },
  badge: {
    position: 'absolute',
    bottom: 8,
    right: 8,
    backgroundColor: lightTheme.colors.bullish[500],
    paddingHorizontal: 14,
    paddingVertical: 6,
    borderRadius: 16,
    borderWidth: 3,
    borderColor: '#fff',
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.2,
    shadowRadius: 4,
    elevation: 4,
  },
  badgeText: {
    color: '#fff',
    fontWeight: 'bold',
    fontSize: 14,
  },
  statusIndicator: {
    position: 'absolute',
    top: 12,
    right: 12,
    width: 20,
    height: 20,
    borderRadius: 10,
    backgroundColor: lightTheme.colors.bullish[500],
    borderWidth: 3,
    borderColor: '#fff',
  },
  greeting: {
    fontSize: 28,
    fontWeight: '700',
    color: lightTheme.colors.primary[700],
    marginBottom: 8,
  },
  title: {
    fontSize: 36,
    fontWeight: 'bold',
    color: lightTheme.colors.wave[900],
    marginBottom: 12,
  },
  subtitle: {
    fontSize: 16,
    color: lightTheme.colors.wave[600],
    textAlign: 'center',
    marginBottom: 32,
    maxWidth: 320,
    lineHeight: 22,
  },
  features: {
    flexDirection: 'row',
    marginBottom: 32,
    gap: 16,
  },
  feature: {
    alignItems: 'center',
    backgroundColor: 'rgba(255, 255, 255, 0.9)',
    padding: 16,
    borderRadius: 16,
    minWidth: 90,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 8,
    elevation: 3,
  },
  featureIcon: {
    width: 48,
    height: 48,
    borderRadius: 24,
    justifyContent: 'center',
    alignItems: 'center',
    marginBottom: 8,
  },
  featureEmoji: {
    fontSize: 24,
  },
  featureText: {
    fontSize: 12,
    fontWeight: '600',
    color: lightTheme.colors.wave[700],
  },
  authContainer: {
    width: '100%',
    maxWidth: 340,
    marginBottom: 16,
  },
  hint: {
    fontSize: 13,
    color: lightTheme.colors.wave[500],
    marginBottom: 40,
  },
  footer: {
    alignItems: 'center',
    marginTop: 'auto',
    paddingTop: 20,
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
