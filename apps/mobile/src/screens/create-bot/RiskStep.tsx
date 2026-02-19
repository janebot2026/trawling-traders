import React from 'react';
import { Text, TextInput, View } from 'react-native';
import type { Strictness } from '@trawling-traders/types';
import { createBotWizardStyles as styles } from './CreateBotWizard.styles';
import { renderChip } from './WizardChip';
import type { Option } from './wizardShared';

export type RiskStepProps = {
  maxPositionSize: string;
  setMaxPositionSize: (value: string) => void;
  maxTradesPerDay: string;
  setMaxTradesPerDay: (value: string) => void;
  maxDailyLoss: string;
  setMaxDailyLoss: (value: string) => void;
  maxDrawdown: string;
  setMaxDrawdown: (value: string) => void;
  strictness: Strictness;
  setStrictness: (value: Strictness) => void;
  strictnessOptions: Option<Strictness>[];
};

export function RiskStep({
  maxPositionSize,
  setMaxPositionSize,
  maxTradesPerDay,
  setMaxTradesPerDay,
  maxDailyLoss,
  setMaxDailyLoss,
  maxDrawdown,
  setMaxDrawdown,
  strictness,
  setStrictness,
  strictnessOptions,
}: RiskStepProps) {
  return (
    <View>
      <Text style={styles.sectionLabel}>Risk Caps</Text>
      <View style={styles.row}>
        <View style={styles.half}>
          <TextInput
            style={styles.input}
            value={maxPositionSize}
            onChangeText={setMaxPositionSize}
            keyboardType="numeric"
            placeholder="Max Position %"
          />
          <Text style={styles.helperText}>Default: 5%</Text>
        </View>
        <View style={styles.half}>
          <TextInput
            style={styles.input}
            value={maxTradesPerDay}
            onChangeText={setMaxTradesPerDay}
            keyboardType="numeric"
            placeholder="Max Trades/Day"
          />
          <Text style={styles.helperText}>Default: 5</Text>
        </View>
      </View>
      <View style={styles.row}>
        <View style={styles.half}>
          <TextInput
            style={styles.input}
            value={maxDailyLoss}
            onChangeText={setMaxDailyLoss}
            keyboardType="numeric"
            placeholder="Daily Loss USD"
          />
          <Text style={styles.helperText}>Default: $50</Text>
        </View>
        <View style={styles.half}>
          <TextInput
            style={styles.input}
            value={maxDrawdown}
            onChangeText={setMaxDrawdown}
            keyboardType="numeric"
            placeholder="Max Drawdown %"
          />
          <Text style={styles.helperText}>Default: 10%</Text>
        </View>
      </View>
      <Text style={styles.sectionLabel}>Strictness</Text>
      {renderChip(strictnessOptions, strictness, setStrictness)}
      <View style={styles.instructionBox}>
        <Text style={styles.instructionTitle}>How Strictness Works</Text>
        <Text style={styles.instructionStep}>
          Strictness works with your risk caps and other execution thresholds to control how much confirmation is required before a trade.
        </Text>
        <Text style={styles.instructionStep}>
          Example: a conservative setup can wait for signal stability over a longer look-back window (such as ~15 minutes) instead of trading the first second RSI flips.
        </Text>
        <Text style={styles.instructionStep}>
          Your parameters are always respected. Strictness changes confidence requirements and stability checks, not whether your bot ignores your settings.
        </Text>
      </View>
    </View>
  );
}
