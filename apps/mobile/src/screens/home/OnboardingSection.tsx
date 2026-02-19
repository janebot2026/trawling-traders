import React from 'react';
import { StyleSheet, Text, TouchableOpacity, View } from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { Ionicons } from '@expo/vector-icons';
import { lightTheme, colors, spacing, shadows } from '../../theme';
import type { RootStackParamList } from '../../navigation/AppNavigator';

interface OnboardingSectionProps {
  /** Whether the user has at least one bot (step 2 complete) */
  hasBots: boolean;
  /** Whether any bot has a funded wallet (step 3 complete) */
  hasFundedBot: boolean;
}

interface Step {
  label: string;
  description: string;
  emoji: string;
  done: boolean;
  onPress?: () => void;
}

export function OnboardingSection({ hasBots, hasFundedBot }: OnboardingSectionProps) {
  const navigation = useNavigation<NativeStackNavigationProp<RootStackParamList>>();

  const steps: Step[] = [
    {
      label: 'Create your account',
      description: 'You are in. Welcome to the Trawling League and trading community.',
      emoji: '🎣',
      done: true,
      onPress: () => navigation.navigate('Profile'),
    },
    {
      label: 'Create your first boat',
      description: 'Launch an always-on automated trader that trawls markets around the clock for opportunities.',
      emoji: '🛥️',
      done: hasBots,
      onPress: () => navigation.navigate('CreateBot'),
    },
    {
      label: 'Fuel your fleet',
      description: 'Deposit funds into your fleet so your boats can begin placing live trades.',
      emoji: '⛽',
      done: hasFundedBot,
      onPress: () => navigation.navigate('Deposit'),
    },
    {
      label: 'Track your fleet',
      description: 'This page will show fleet results and let you customize and chat with your captains after your first boat is live.',
      emoji: '📡',
      done: hasFundedBot,
    },
  ];

  const done = steps.filter((s) => s.done).length;
  const progress = done / steps.length;

  return (
    <View style={styles.container}>
      <View style={styles.card}>
        <Text style={styles.title}>Welcome aboard, trader!</Text>
        <View style={styles.progressBar}>
          <View style={[styles.progressFill, { flex: progress }]} />
          <View style={{ flex: 1 - progress }} />
        </View>
        <Text style={styles.progressLabel}>
          {done}/{steps.length} complete
        </Text>

        {steps.map((step, index) => (
          <React.Fragment key={step.label}>
            {index > 0 && <View style={styles.divider} />}
            <TouchableOpacity
              style={styles.stepRow}
              onPress={step.onPress}
              disabled={!step.onPress}
              activeOpacity={0.6}
            >
              <Text style={styles.stepEmoji}>{step.emoji}</Text>
              <View style={styles.stepCopy}>
                <Text
                  style={[styles.stepLabel, step.done && styles.stepLabelDone]}
                >
                  {step.label}
                </Text>
                <Text style={[styles.stepDescription, step.done && styles.stepDescriptionDone]}>
                  {step.description}
                </Text>
              </View>
              {step.onPress ? (
                <Ionicons
                  name="chevron-forward"
                  size={16}
                  color={colors.wave[400]}
                  style={styles.chevron}
                />
              ) : null}
            </TouchableOpacity>
          </React.Fragment>
        ))}
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    marginTop: spacing.sm,
    alignItems: 'center',
  },
  card: {
    backgroundColor: lightTheme.colors.surface,
    borderRadius: 18,
    borderWidth: 1,
    borderColor: lightTheme.colors.cardBorder,
    padding: 16,
    width: '100%',
    maxWidth: 360,
    ...shadows.md,
  },
  title: {
    fontSize: 18,
    fontWeight: '700',
    color: lightTheme.colors.text,
    fontFamily: lightTheme.typography.families.display,
    marginBottom: 12,
  },
  progressBar: {
    flexDirection: 'row',
    height: 6,
    borderRadius: 3,
    backgroundColor: colors.wave[200],
    overflow: 'hidden',
    marginBottom: 6,
  },
  progressFill: {
    backgroundColor: colors.bullish[500],
    borderRadius: 3,
  },
  progressLabel: {
    fontSize: 12,
    color: colors.wave[500],
    marginBottom: 4,
  },
  divider: {
    height: 1,
    backgroundColor: colors.wave[200],
  },
  stepRow: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    paddingVertical: 14,
  },
  stepEmoji: {
    fontSize: 20,
    marginRight: 10,
    marginTop: 2,
  },
  stepCopy: {
    flex: 1,
  },
  stepLabel: {
    fontSize: 14,
    color: lightTheme.colors.text,
    fontWeight: '500',
  },
  stepDescription: {
    marginTop: 2,
    fontSize: 12,
    color: colors.wave[600],
    lineHeight: 17,
  },
  stepLabelDone: {
    textDecorationLine: 'line-through',
    color: colors.wave[400],
  },
  stepDescriptionDone: {
    color: colors.wave[400],
  },
  chevron: {
    marginLeft: 8,
    marginTop: 4,
  },
});
