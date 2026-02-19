import React, { useMemo, useState } from 'react';
import { Text, TextInput, TouchableOpacity, View } from 'react-native';
import type { AlgorithmFactor } from '@trawling-traders/types';
import { createBotWizardStyles as styles } from './CreateBotWizard.styles';
import { type Option, type StrategyType, toSubscript } from './wizardShared';

export type AlgorithmStepProps = {
  strategyType: StrategyType;
  factorCatalog: Option<string>[];
  algorithmFactors: AlgorithmFactor[];
  setAlgorithmFactors: (value: AlgorithmFactor[]) => void;
};

export function AlgorithmStep({
  strategyType,
  factorCatalog,
  algorithmFactors,
  setAlgorithmFactors,
}: AlgorithmStepProps) {
  const [activeDropdownRow, setActiveDropdownRow] = useState<number | null>(null);

  const usedFactorSet = useMemo(
    () => new Set(algorithmFactors.map((factor) => factor.factor)),
    [algorithmFactors]
  );

  if (strategyType !== 'macro') {
    return (
      <View>
        <View style={styles.instructionBox}>
          <Text style={styles.instructionTitle}>Coming Soon</Text>
          <Text style={styles.instructionStep}>
            Algorithm customization for {strategyType} is coming soon.
          </Text>
          <Text style={styles.instructionStep}>
            We are shipping Macro end-to-end first, then enabling full controls for other strategies.
          </Text>
        </View>
      </View>
    );
  }

  const coefficientSum = algorithmFactors.reduce((sum, factor) => sum + factor.weight, 0);
  const canAddAnother = algorithmFactors.length < factorCatalog.length;

  const updateWeight = (factorKey: string, weightInput: string) => {
    const parsed = Number.parseFloat(weightInput);
    const safeWeight = Number.isFinite(parsed)
      ? Math.max(-1, Math.min(1, parsed))
      : 0;
    setAlgorithmFactors(
      algorithmFactors.map((item) =>
        item.factor === factorKey ? { ...item, weight: safeWeight } : item
      )
    );
  };

  const addAnotherFactor = () => {
    const nextFactor = factorCatalog.find((entry) => !usedFactorSet.has(entry.value));
    if (!nextFactor) {
      return;
    }
    setAlgorithmFactors([...algorithmFactors, { factor: nextFactor.value, weight: 0.2 }]);
  };

  const updateRowFactor = (rowIndex: number, factorKey: string) => {
    const existing = algorithmFactors.find((factor) => factor.factor === factorKey);
    if (existing && algorithmFactors[rowIndex]?.factor !== factorKey) {
      return;
    }
    setAlgorithmFactors(
      algorithmFactors.map((item, index) =>
        index === rowIndex ? { ...item, factor: factorKey } : item
      )
    );
    setActiveDropdownRow(null);
  };

  const removeFactor = (rowIndex: number) => {
    setAlgorithmFactors(algorithmFactors.filter((_, index) => index !== rowIndex));
    setActiveDropdownRow(null);
  };

  return (
    <View>
      <Text style={styles.sectionLabel}>Formula</Text>
      <Text style={styles.formulaPreview}>
        y ={' '}
        {algorithmFactors.length === 0
          ? '0'
          : algorithmFactors
              .map(
                (item, index) =>
                  `${item.weight.toFixed(2)}x${toSubscript(index + 1)}`
              )
              .join(' + ')}
      </Text>
      {coefficientSum > 1 ? (
        <Text style={styles.coefficientWarning}>
          Coefficient sum is {coefficientSum.toFixed(2)} ({'>'} 1.00). Consider lowering weights.
        </Text>
      ) : (
        <Text style={styles.helperText}>
          Coefficient sum: {coefficientSum.toFixed(2)}
        </Text>
      )}

      <Text style={styles.sectionLabel}>Factors</Text>
      {algorithmFactors.map((factor, rowIndex) => {
        const currentFactorMeta = factorCatalog.find(
          (entry) => entry.value === factor.factor
        );
        return (
          <View key={factor.factor} style={styles.factorBlock}>
            <View style={styles.factorRow}>
              <TouchableOpacity
                style={styles.factorSelectButton}
                onPress={() =>
                  setActiveDropdownRow((prev) => (prev === rowIndex ? null : rowIndex))
                }
              >
                <Text style={styles.factorSelectText}>
                  {currentFactorMeta?.label || factor.factor}
                </Text>
                <Text style={styles.factorSelectChevron}>▼</Text>
              </TouchableOpacity>
              <TextInput
                style={styles.factorWeightInput}
                value={String(factor.weight)}
                onChangeText={(value) => updateWeight(factor.factor, value)}
                keyboardType="decimal-pad"
                placeholder="0.20"
              />
              <TouchableOpacity
                style={styles.factorRemoveButton}
                onPress={() => removeFactor(rowIndex)}
              >
                <Text style={styles.factorRemoveText}>Remove</Text>
              </TouchableOpacity>
            </View>

            {activeDropdownRow === rowIndex ? (
              <View style={styles.factorDropdownMenu}>
                {factorCatalog.map((entry) => {
                  const disabled =
                    usedFactorSet.has(entry.value) && entry.value !== factor.factor;
                  return (
                    <TouchableOpacity
                      key={entry.value}
                      style={[
                        styles.factorDropdownItem,
                        disabled ? styles.factorDropdownItemDisabled : undefined,
                      ]}
                      disabled={disabled}
                      onPress={() => updateRowFactor(rowIndex, entry.value)}
                    >
                      <Text
                        style={[
                          styles.factorDropdownText,
                          disabled ? styles.factorDropdownTextDisabled : undefined,
                        ]}
                      >
                        {entry.label}
                      </Text>
                    </TouchableOpacity>
                  );
                })}
              </View>
            ) : null}
          </View>
        );
      })}

      <TouchableOpacity
        style={[
          styles.addFactorButton,
          !canAddAnother ? styles.addFactorButtonDisabled : undefined,
        ]}
        disabled={!canAddAnother}
        onPress={addAnotherFactor}
      >
        <Text style={styles.addFactorButtonText}>
          {canAddAnother ? 'Add Another Factor' : 'All Factors Added'}
        </Text>
      </TouchableOpacity>
    </View>
  );
}
