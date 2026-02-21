import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  ActivityIndicator,
  Alert,
  Image,
  ImageBackground,
  KeyboardAvoidingView,
  Linking,
  Platform,
  Pressable,
  ScrollView,
  Text,
  TextInput,
  useColorScheme,
  View,
} from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';
import { useCedrosLogin, useEmailAuth, useOrgs, GoogleLoginButton } from '@cedros/login-react-native';
import { api } from '@trawling-traders/api-client';
import { CEDROS_CONFIG } from '../config/api';
import { authScreenStyles as styles } from './auth/AuthScreen.styles';

const LOB_AVATAR = require('../../assets/lob-avatar.png');
const OCEAN_LIGHT = require('../../../../assets/branding/bg-ocean-light.png');
const OCEAN_DARK = require('../../../../assets/branding/bg-ocean-dark.png');

type AuthScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'Auth'>;
type AuthMode = 'login' | 'register';
type LoginStep = 'email' | 'password';

const EMAIL_REGEX = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export function AuthScreen() {
  const navigation = useNavigation<AuthScreenNavigationProp>();
  const insets = useSafeAreaInsets();
  const colorScheme = useColorScheme();
  const { isAuthenticated, isLoading: authLoading } = useCedrosLogin();
  const { login, register, isLoading: emailAuthLoading, error, clearError } = useEmailAuth();
  const { activeOrg } = useOrgs();
  const isDark = colorScheme === 'dark';
  const backgroundAsset = isDark ? OCEAN_DARK : OCEAN_LIGHT;

  const [mode, setMode] = useState<AuthMode>('login');
  const [loginStep, setLoginStep] = useState<LoginStep>('email');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [emailError, setEmailError] = useState<string | undefined>();
  const [passwordError, setPasswordError] = useState<string | undefined>();
  const [displayNameError, setDisplayNameError] = useState<string | undefined>();
  const [showLoginPassword, setShowLoginPassword] = useState(false);
  const [showRegisterPassword, setShowRegisterPassword] = useState(false);

  const emailRef = useRef<TextInput>(null);
  const passwordRef = useRef<TextInput>(null);
  const nameRef = useRef<TextInput>(null);
  const isNavigatingRef = useRef(false);

  const trimmedEmail = email.trim();
  const emailIsValid = useMemo(() => EMAIL_REGEX.test(trimmedEmail), [trimmedEmail]);
  const canContinueEmail = trimmedEmail.length > 0 && emailIsValid;
  const canSubmitLogin = canContinueEmail && password.length >= 8 && !emailAuthLoading;
  const canSubmitRegister =
    canContinueEmail && password.length >= 8 && displayName.trim().length >= 2 && !emailAuthLoading;

  const resetErrors = useCallback(() => {
    setEmailError(undefined);
    setPasswordError(undefined);
    setDisplayNameError(undefined);
    clearError();
  }, [clearError]);

  const routeAfterAuth = useCallback(() => {
    if (isNavigatingRef.current) {
      return;
    }
    isNavigatingRef.current = true;
    navigation.reset({ index: 0, routes: [{ name: 'Main' }] });
    // Reset guard after navigation has committed so it does not block future calls
    // (e.g. if the component remounts after a logout).
    setTimeout(() => {
      isNavigatingRef.current = false;
    }, 500);
  }, [navigation]);

  useEffect(() => {
    if (isAuthenticated) {
      routeAfterAuth();
    }
  }, [isAuthenticated, routeAfterAuth]);

  // Subscription gating: waits for activeOrg to load before deciding.
  // orgsLoading starts false (before fetch), so we can't rely on it alone.
  // Instead, wait for activeOrg to be non-null (cedros-login always creates one).
  const subscriptionCheckedRef = useRef(false);
  useEffect(() => {
    if (!isAuthenticated || subscriptionCheckedRef.current) return;

    // Admin/owner org members skip subscription checks entirely
    const orgRole = activeOrg?.membership?.role;
    if (orgRole === 'owner' || orgRole === 'admin') {
      subscriptionCheckedRef.current = true;
      return;
    }

    // activeOrg not yet loaded — wait for the orgs fetch to complete
    if (!activeOrg) return;

    // Non-admin member — check billing
    let mounted = true;
    (async () => {
      try {
        const billing = await api.user.getBillingSummary();
        if (!mounted) return;
        subscriptionCheckedRef.current = true;
        const status = String((billing as { status?: string }).status || '').toLowerCase();
        if (status !== 'active') {
          navigation.reset({ index: 0, routes: [{ name: 'Subscribe' }] });
        }
      } catch (err) {
        if (!mounted) return;
        if (__DEV__) {
          console.warn('Subscription check failed:', err);
        }
        // Do not set subscriptionCheckedRef so the check can be retried
        // on the next render cycle when conditions change.
      }
    })();
    return () => { mounted = false; };
  }, [isAuthenticated, activeOrg, navigation]);

  useEffect(() => {
    emailRef.current?.focus();
  }, []);

  useEffect(() => {
    if (mode === 'login' && loginStep === 'password') {
      passwordRef.current?.focus();
    }
  }, [mode, loginStep]);

  useEffect(() => {
    if (mode === 'register') {
      nameRef.current?.focus();
    }
  }, [mode]);

  const handleContinueToPassword = useCallback(() => {
    resetErrors();
    if (!trimmedEmail) {
      setEmailError('Email is required');
      return;
    }
    if (!emailIsValid) {
      setEmailError('Enter a valid email address');
      return;
    }
    setLoginStep('password');
  }, [emailIsValid, resetErrors, trimmedEmail]);

  const handleLogin = useCallback(async () => {
    resetErrors();
    if (!canContinueEmail) {
      setEmailError('Enter a valid email address');
      return;
    }
    if (password.length < 8) {
      setPasswordError('Password must be at least 8 characters');
      return;
    }

    try {
      await login(trimmedEmail, password);
      routeAfterAuth();
    } catch {
      // Hook error is rendered inline
    }
  }, [canContinueEmail, login, password, resetErrors, routeAfterAuth, trimmedEmail]);

  const handleRegister = useCallback(async () => {
    resetErrors();
    const nameValue = displayName.trim();
    if (nameValue.length < 2) {
      setDisplayNameError('Display name must be at least 2 characters');
      return;
    }
    if (!canContinueEmail) {
      setEmailError('Enter a valid email address');
      return;
    }
    if (password.length < 8) {
      setPasswordError('Password must be at least 8 characters');
      return;
    }

    try {
      await register(trimmedEmail, password, nameValue);
      routeAfterAuth();
    } catch {
      // Hook error is rendered inline
    }
  }, [canContinueEmail, displayName, password, register, resetErrors, routeAfterAuth, trimmedEmail]);

  const handleSwitchMode = useCallback((nextMode: AuthMode) => {
    setMode(nextMode);
    setLoginStep('email');
    setPassword('');
    resetErrors();
  }, [resetErrors]);

  const formError = error?.message;

  if (authLoading) {
    return (
      <View style={styles.loadingContainer}>
        <Image source={LOB_AVATAR} style={styles.loadingAvatar} />
        <Text style={styles.loadingText}>Preparing secure trading console...</Text>
        <ActivityIndicator size="small" color="#0c64b5" style={styles.loadingSpinner} />
      </View>
    );
  }

  return (
    <ImageBackground source={backgroundAsset} style={styles.screen} resizeMode="cover">
      <View style={styles.overlay} />
      <KeyboardAvoidingView
        style={styles.keyboardContainer}
        behavior={Platform.OS === 'ios' ? 'padding' : undefined}
      >
        <ScrollView
          contentContainerStyle={[styles.scrollContent, { paddingTop: insets.top + 18, paddingBottom: insets.bottom + 12 }]}
          keyboardShouldPersistTaps="handled"
          showsVerticalScrollIndicator={false}
        >
          <View style={styles.heroSection}>
            <View style={styles.brandRow}>
              <Image source={LOB_AVATAR} style={styles.brandMark} />
              <Text style={styles.brandTitle}>Trawling Traders</Text>
            </View>
          </View>

          <View style={styles.sheet}>
            <View style={styles.modeRow}>
              <Pressable
                style={[styles.modePill, mode === 'login' ? styles.modePillActive : undefined]}
                onPress={() => handleSwitchMode('login')}
              >
                <Text style={[styles.modePillText, mode === 'login' ? styles.modePillTextActive : undefined]}>
                  Sign in
                </Text>
              </Pressable>
              <Pressable
                style={[styles.modePill, mode === 'register' ? styles.modePillActive : undefined]}
                onPress={() => handleSwitchMode('register')}
              >
                <Text style={[styles.modePillText, mode === 'register' ? styles.modePillTextActive : undefined]}>
                  Sign up
                </Text>
              </Pressable>
            </View>

            {mode === 'login' ? (
              <View>
                <View style={styles.fieldGroup}>
                  <Text style={styles.fieldLabel}>Email</Text>
                  <TextInput
                    ref={emailRef}
                    value={email}
                    onChangeText={(value) => {
                      setEmail(value);
                      setEmailError(undefined);
                    }}
                    autoCapitalize="none"
                    autoCorrect={false}
                    keyboardType="email-address"
                    textContentType="emailAddress"
                    autoFocus={loginStep === 'email'}
                    returnKeyType={loginStep === 'email' ? 'next' : 'done'}
                    onSubmitEditing={() => {
                      if (loginStep === 'email') {
                        handleContinueToPassword();
                      } else {
                        handleLogin();
                      }
                    }}
                    placeholder="you@company.com"
                    placeholderTextColor="#6d86a6"
                    style={styles.input}
                  />
                  {emailError ? <Text style={styles.fieldError}>{emailError}</Text> : null}
                </View>

                {loginStep === 'password' ? (
                  <View style={styles.fieldGroup}>
                    <Text style={styles.fieldLabel}>Password</Text>
                    <TextInput
                      ref={passwordRef}
                      value={password}
                      onChangeText={(value) => {
                        setPassword(value);
                        setPasswordError(undefined);
                      }}
                      secureTextEntry={!showLoginPassword}
                      textContentType="password"
                      autoCapitalize="none"
                      autoCorrect={false}
                      returnKeyType="go"
                      onSubmitEditing={handleLogin}
                      placeholder="Enter your password"
                      placeholderTextColor="#6d86a6"
                      style={styles.input}
                    />
                    <Pressable style={styles.passwordToggle} onPress={() => setShowLoginPassword((prev) => !prev)}>
                      <Text style={styles.passwordToggleText}>{showLoginPassword ? 'Hide' : 'Show'}</Text>
                    </Pressable>
                    {passwordError ? <Text style={styles.fieldError}>{passwordError}</Text> : null}
                  </View>
                ) : null}

                {formError ? <Text style={styles.fieldError}>{formError}</Text> : null}

                <Pressable
                  style={[
                    styles.primaryButton,
                    (loginStep === 'email' ? !canContinueEmail : !canSubmitLogin) ? styles.primaryButtonDisabled : undefined,
                  ]}
                  onPress={loginStep === 'email' ? handleContinueToPassword : handleLogin}
                  disabled={loginStep === 'email' ? !canContinueEmail : !canSubmitLogin}
                >
                  <Text style={styles.primaryButtonText}>
                    {emailAuthLoading ? 'Working...' : loginStep === 'email' ? 'Continue' : 'Sign in'}
                  </Text>
                </Pressable>

                {loginStep === 'password' ? (
                  <Pressable
                    style={styles.inlineLink}
                    onPress={() =>
                      Linking.openURL(`${CEDROS_CONFIG.serverUrl}/auth/forgot-password`)
                    }
                  >
                    <Text style={styles.inlineLinkText}>Forgot password?</Text>
                  </Pressable>
                ) : null}
              </View>
            ) : (
              <View>
                <View style={styles.fieldGroup}>
                  <Text style={styles.fieldLabel}>Display Name</Text>
                  <TextInput
                    ref={nameRef}
                    value={displayName}
                    onChangeText={(value) => {
                      setDisplayName(value);
                      setDisplayNameError(undefined);
                    }}
                    autoCapitalize="words"
                    returnKeyType="next"
                    onSubmitEditing={() => emailRef.current?.focus()}
                    placeholder="Your name"
                    placeholderTextColor="#6d86a6"
                    style={styles.input}
                  />
                  {displayNameError ? <Text style={styles.fieldError}>{displayNameError}</Text> : null}
                </View>
                <View style={styles.fieldGroup}>
                  <Text style={styles.fieldLabel}>Email</Text>
                  <TextInput
                    ref={emailRef}
                    value={email}
                    onChangeText={(value) => {
                      setEmail(value);
                      setEmailError(undefined);
                    }}
                    autoCapitalize="none"
                    autoCorrect={false}
                    keyboardType="email-address"
                    textContentType="emailAddress"
                    returnKeyType="next"
                    onSubmitEditing={() => passwordRef.current?.focus()}
                    placeholder="you@company.com"
                    placeholderTextColor="#6d86a6"
                    style={styles.input}
                  />
                  {emailError ? <Text style={styles.fieldError}>{emailError}</Text> : null}
                </View>
                <View style={styles.fieldGroup}>
                  <Text style={styles.fieldLabel}>Password</Text>
                  <TextInput
                    ref={passwordRef}
                    value={password}
                    onChangeText={(value) => {
                      setPassword(value);
                      setPasswordError(undefined);
                    }}
                    secureTextEntry={!showRegisterPassword}
                    textContentType="newPassword"
                    autoCapitalize="none"
                    autoCorrect={false}
                    returnKeyType="go"
                    onSubmitEditing={handleRegister}
                    placeholder="At least 8 characters"
                    placeholderTextColor="#6d86a6"
                    style={styles.input}
                  />
                  <Pressable style={styles.passwordToggle} onPress={() => setShowRegisterPassword((prev) => !prev)}>
                    <Text style={styles.passwordToggleText}>{showRegisterPassword ? 'Hide' : 'Show'}</Text>
                  </Pressable>
                  {passwordError ? <Text style={styles.fieldError}>{passwordError}</Text> : null}
                </View>

                {formError ? <Text style={styles.fieldError}>{formError}</Text> : null}

                <Pressable
                  style={[styles.primaryButton, !canSubmitRegister ? styles.primaryButtonDisabled : undefined]}
                  onPress={handleRegister}
                  disabled={!canSubmitRegister}
                >
                  <Text style={styles.primaryButtonText}>{emailAuthLoading ? 'Working...' : 'Create account'}</Text>
                </Pressable>
              </View>
            )}

            <View style={styles.orRow}>
              <View style={styles.orLine} />
              <Text style={styles.orText}>or</Text>
              <View style={styles.orLine} />
            </View>

            <GoogleLoginButton
              onRequestToken={async () => {
                Alert.alert(
                  'Google Sign-In',
                  'Google native sign-in is not configured in this build yet. Use email sign-in for now.'
                );
                throw new Error('Google sign-in unavailable in this build');
              }}
              onSuccess={routeAfterAuth}
              style={styles.googleButton}
            />

            <View style={styles.proofRow}>
              <Text style={styles.proofItem}>🤖 Up to 4 bots</Text>
              <Text style={styles.proofDot}>•</Text>
              <Text style={styles.proofItem}>📈 Stocks + crypto</Text>
              <Text style={styles.proofDot}>•</Text>
              <Text style={styles.proofItem}>🖥️ Your VPS</Text>
            </View>

            <Text style={styles.reassurance}>Your bots run on your infrastructure. We don&apos;t touch your keys.</Text>

            <View style={styles.metaLinksRow}>
              <Pressable onPress={() => Linking.openURL('https://trawlingtraders.com/terms')}>
                <Text style={styles.metaLink}>Terms</Text>
              </Pressable>
              <Text style={styles.metaDivider}>•</Text>
              <Pressable onPress={() => Linking.openURL('https://trawlingtraders.com/privacy')}>
                <Text style={styles.metaLink}>Privacy</Text>
              </Pressable>
              <Text style={styles.metaDivider}>•</Text>
              <Pressable onPress={() => Linking.openURL('https://trawlingtraders.com/security')}>
                <Text style={styles.metaLink}>Security</Text>
              </Pressable>
            </View>
          </View>
        </ScrollView>
      </KeyboardAvoidingView>
    </ImageBackground>
  );
}
