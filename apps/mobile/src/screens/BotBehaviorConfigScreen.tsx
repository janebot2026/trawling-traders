import React, { useCallback, useEffect, useState } from 'react';
import {
  ActivityIndicator,
  Alert,
  ScrollView,
  StyleSheet,
  Switch,
  Text,
  TextInput,
  TouchableOpacity,
  View,
} from 'react-native';
import { RouteProp, useNavigation, useRoute } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { BotConfig, TradingMode } from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';
import { OceanBackground } from '../components/OceanBackground';
import type { RootStackParamList } from '../navigation/AppNavigator';
import { lightTheme } from '../theme';

type BotBehaviorRoute = RouteProp<RootStackParamList, 'BotBehaviorConfig'>;
type BotBehaviorNav = NativeStackNavigationProp<RootStackParamList, 'BotBehaviorConfig'>;

export function BotBehaviorConfigScreen() {
  const navigation = useNavigation<BotBehaviorNav>();
  const route = useRoute<BotBehaviorRoute>();
  const { botId } = route.params;

  const [config, setConfig] = useState<BotConfig | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);

  const fetchConfig = useCallback(async () => {
    try {
      const response = await api.bot.getBot(botId);
      setConfig(response.config);
    } catch {
      Alert.alert('Error', 'Failed to load behavior settings.');
    } finally {
      setIsLoading(false);
    }
  }, [botId]);

  useEffect(() => {
    fetchConfig();
  }, [fetchConfig]);

  const updateTradingMode = (liveEnabled: boolean) => {
    const mode: TradingMode = liveEnabled ? 'live' : 'paper';
    setConfig((prev) => (prev ? { ...prev, tradingMode: mode } : prev));
  };

  const save = async () => {
    if (!config) return;
    setIsSaving(true);

    try {
      await api.bot.updateBotConfig(botId, {
        config: {
          name: config.name,
          assistantStyle: config.assistantStyle,
          assetFocus: config.assetFocus,
          algorithmMode: config.algorithmMode,
          strictness: config.strictness,
          tradingMode: config.tradingMode,
          llmProvider: config.llmProvider,
          // R5-MB-001: Don't send llmApiKey — the server returns a masked value
          // (e.g., "sk-...a1b2") which would overwrite the real key.
          signalKnobs: config.signalKnobs,
          riskCaps: config.riskCaps,
        },
      });
      Alert.alert('Saved', 'Behavior settings updated.');
      navigation.goBack();
    } catch {
      Alert.alert('Error', 'Failed to save behavior settings.');
    } finally {
      setIsSaving(false);
    }
  };

  if (isLoading || !config) {
    return (
      <OceanBackground>
        <View style={styles.centered}>
          <ActivityIndicator size="large" color={lightTheme.colors.primary[700]} />
        </View>
      </OceanBackground>
    );
  }

  return (
    <OceanBackground>
      <ScrollView contentContainerStyle={styles.content}>
        <Text style={styles.title}>Behavior Configuration</Text>

        <View style={styles.card}>
          <Text style={styles.sectionLabel}>Trading Mode</Text>
          <View style={styles.switchRow}>
            <Text style={styles.valueLabel}>{config.tradingMode === 'live' ? 'Live Trading' : 'Paper Trading'}</Text>
            <Switch
              value={config.tradingMode === 'live'}
              onValueChange={updateTradingMode}
              trackColor={{ false: lightTheme.colors.wave[300], true: lightTheme.colors.lobster[400] }}
            />
          </View>
          {config.tradingMode === 'live' && (
            <Text style={styles.warning}>Live mode uses real funds and should include conservative stops.</Text>
          )}
        </View>

        <View style={styles.card}>
          <Text style={styles.sectionLabel}>Risk Tolerance</Text>
          <View style={styles.inputRow}>
            <Text style={styles.inputLabel}>Max Position %</Text>
            <TextInput
              value={String(config.riskCaps.maxPositionSizePercent)}
              keyboardType="numeric"
              style={styles.input}
              onChangeText={(value) =>
                setConfig((prev) =>
                  prev
                    ? {
                        ...prev,
                        riskCaps: {
                          ...prev.riskCaps,
                          maxPositionSizePercent: Number(value) || prev.riskCaps.maxPositionSizePercent,
                        },
                      }
                    : prev
                )
              }
            />
          </View>
          <View style={styles.inputRow}>
            <Text style={styles.inputLabel}>Max Daily Loss (USD)</Text>
            <TextInput
              value={String(config.riskCaps.maxDailyLossUsd)}
              keyboardType="numeric"
              style={styles.input}
              onChangeText={(value) =>
                setConfig((prev) =>
                  prev
                    ? {
                        ...prev,
                        riskCaps: {
                          ...prev.riskCaps,
                          maxDailyLossUsd: Number(value) || prev.riskCaps.maxDailyLossUsd,
                        },
                      }
                    : prev
                )
              }
            />
          </View>
          <View style={styles.inputRow}>
            <Text style={styles.inputLabel}>Max Drawdown %</Text>
            <TextInput
              value={String(config.riskCaps.maxDrawdownPercent)}
              keyboardType="numeric"
              style={styles.input}
              onChangeText={(value) =>
                setConfig((prev) =>
                  prev
                    ? {
                        ...prev,
                        riskCaps: {
                          ...prev.riskCaps,
                          maxDrawdownPercent: Number(value) || prev.riskCaps.maxDrawdownPercent,
                        },
                      }
                    : prev
                )
              }
            />
          </View>
          <View style={styles.inputRow}>
            <Text style={styles.inputLabel}>Max Trades / Day</Text>
            <TextInput
              value={String(config.riskCaps.maxTradesPerDay)}
              keyboardType="numeric"
              style={styles.input}
              onChangeText={(value) =>
                setConfig((prev) =>
                  prev
                    ? {
                        ...prev,
                        riskCaps: {
                          ...prev.riskCaps,
                          maxTradesPerDay: Number(value) || prev.riskCaps.maxTradesPerDay,
                        },
                      }
                    : prev
                )
              }
            />
          </View>
        </View>

        <TouchableOpacity style={styles.saveButton} onPress={save} disabled={isSaving}>
          <Text style={styles.saveButtonText}>{isSaving ? 'Saving...' : 'Save Behavior'}</Text>
        </TouchableOpacity>
      </ScrollView>
    </OceanBackground>
  );
}

const styles = StyleSheet.create({
  content: {
    padding: 16,
    paddingBottom: 32,
  },
  centered: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  title: {
    fontSize: 26,
    fontWeight: '700',
    color: lightTheme.colors.wave[900],
    fontFamily: lightTheme.typography.families.display,
    marginBottom: 14,
  },
  card: {
    borderRadius: 14,
    borderWidth: 1,
    borderColor: lightTheme.colors.cardBorder,
    backgroundColor: lightTheme.colors.surface,
    padding: 14,
    marginBottom: 12,
  },
  sectionLabel: {
    fontSize: 14,
    fontWeight: '700',
    color: lightTheme.colors.wave[700],
    marginBottom: 10,
  },
  switchRow: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  valueLabel: {
    fontSize: 16,
    color: lightTheme.colors.wave[900],
  },
  warning: {
    marginTop: 10,
    fontSize: 12,
    color: lightTheme.colors.lobster[600],
  },
  inputRow: {
    marginBottom: 10,
  },
  inputLabel: {
    fontSize: 12,
    color: lightTheme.colors.wave[500],
    marginBottom: 6,
  },
  input: {
    borderWidth: 1,
    borderColor: lightTheme.colors.wave[300],
    borderRadius: 10,
    backgroundColor: lightTheme.colors.wave[50],
    color: lightTheme.colors.wave[900],
    paddingHorizontal: 10,
    paddingVertical: 9,
  },
  saveButton: {
    marginTop: 16,
    borderRadius: 18,
    backgroundColor: lightTheme.colors.accent,
    alignItems: 'center',
    paddingVertical: 14,
  },
  saveButtonText: {
    color: '#fff',
    fontWeight: '700',
    fontSize: 16,
  },
});
