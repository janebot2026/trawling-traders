import React from 'react';
import { View, Text, TouchableOpacity } from 'react-native';
import { createBotWizardStyles as styles } from './CreateBotWizard.styles';
import type { Option } from './wizardShared';

export function renderChip<T extends string>(
  options: Option<T>[],
  selected: T,
  onSelect: (value: T) => void
) {
  return (
    <View style={styles.chipRow}>
      {options.map((item) => (
        <TouchableOpacity
          key={item.value}
          style={[styles.chip, selected === item.value && styles.chipActive]}
          onPress={() => onSelect(item.value)}
        >
          <Text style={[styles.chipText, selected === item.value && styles.chipTextActive]}>
            {item.label}
          </Text>
        </TouchableOpacity>
      ))}
    </View>
  );
}
