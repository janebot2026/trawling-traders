import React from 'react';
import { Text, View } from 'react-native';
import type {
  AIAssistantOption,
  AlgorithmFactor,
  AssetFocus,
  LlmModel,
  LlmProvider,
  Persona,
  TradingMode,
} from '@trawling-traders/types';
import { createBotWizardStyles as styles } from './CreateBotWizard.styles';
import { type StrategyType, toSubscript } from './wizardShared';

export type SummaryStepProps = {
  name: string;
  assistantStyle: Persona;
  assistantOptions: AIAssistantOption[];
  strategyType: StrategyType;
  strictness: string;
  assetFocus: AssetFocus;
  selectedAssets: string[];
  tradingMode: TradingMode;
  maxPositionSize: string;
  maxDailyLoss: string;
  maxDrawdown: string;
  maxTradesPerDay: string;
  algorithmFactors: AlgorithmFactor[];
  llmProvider: LlmProvider;
  llmModel: LlmModel;
  telegramEnabled: boolean;
  telegramUserId: string;
  telegramPairingCode: string;
};

export function SummaryStep({
  name,
  assistantStyle,
  assistantOptions,
  strategyType,
  strictness,
  assetFocus,
  selectedAssets,
  tradingMode,
  maxPositionSize,
  maxDailyLoss,
  maxDrawdown,
  maxTradesPerDay,
  algorithmFactors,
  llmProvider,
  llmModel,
  telegramEnabled,
  telegramUserId,
  telegramPairingCode,
}: SummaryStepProps) {
  return (
    <View>
      <View style={styles.summaryRow}>
        <Text style={styles.summaryLabel}>Boat</Text>
        <Text style={styles.summaryValue}>{name || 'Unnamed'}</Text>
      </View>
      <View style={styles.summaryRow}>
        <Text style={styles.summaryLabel}>Captain</Text>
        <Text style={styles.summaryValue}>
          {assistantOptions.find((option) => option.assistantStyle === assistantStyle)?.captainName ?? assistantStyle}
        </Text>
      </View>
      <View style={styles.summaryRow}>
        <Text style={styles.summaryLabel}>Persona / Strategy</Text>
        <Text style={styles.summaryValue}>
          {strategyType} • {strictness}
        </Text>
      </View>
      <View style={styles.summaryRow}>
        <Text style={styles.summaryLabel}>Market / Mode</Text>
        <Text style={styles.summaryValue}>
          {assetFocus} • {selectedAssets.length} assets • {tradingMode}
        </Text>
      </View>
      <View style={styles.summaryRow}>
        <Text style={styles.summaryLabel}>Risk Caps</Text>
        <Text style={styles.summaryValue}>
          {maxPositionSize}% pos • ${maxDailyLoss} daily • {maxDrawdown}% drawdown •{' '}
          {maxTradesPerDay}/day
        </Text>
      </View>
      <View style={styles.summaryRow}>
        <Text style={styles.summaryLabel}>Algorithm Formula</Text>
        <Text style={styles.summaryValue}>
          {algorithmFactors.length === 0
            ? 'None'
            : algorithmFactors
                .map((item, index) => `${item.weight.toFixed(2)}x${toSubscript(index + 1)}`)
                .join(' + ')}
        </Text>
      </View>
      <View style={styles.summaryRow}>
        <Text style={styles.summaryLabel}>AI</Text>
        <Text style={styles.summaryValue}>
          {llmProvider} • {llmModel}
        </Text>
      </View>
      <View style={styles.summaryRow}>
        <Text style={styles.summaryLabel}>Telegram</Text>
        <Text style={styles.summaryValue}>
          {telegramEnabled
            ? `Enabled • ID ${telegramUserId || 'pending'} • Code ${telegramPairingCode || 'pending'}`
            : 'Disabled'}
        </Text>
      </View>
    </View>
  );
}
