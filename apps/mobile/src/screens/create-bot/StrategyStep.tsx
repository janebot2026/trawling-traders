import React from 'react';
import { Text, View } from 'react-native';
import { createBotWizardStyles as styles } from './CreateBotWizard.styles';
import { renderChip } from './WizardChip';
import type { Option, StrategyType } from './wizardShared';

export type StrategyStepProps = {
  strategyOptions: Option<StrategyType>[];
  strategyType: StrategyType;
  setStrategyType: (value: StrategyType) => void;
};

export function StrategyStep({ strategyOptions, strategyType, setStrategyType }: StrategyStepProps) {
  return (
    <View>
      <Text style={styles.sectionLabel}>Strategy Type</Text>
      {renderChip(strategyOptions, strategyType, setStrategyType)}
      <Text style={styles.helperText}>
        This chooses the style of trading logic for your selected assets.
      </Text>
    </View>
  );
}
