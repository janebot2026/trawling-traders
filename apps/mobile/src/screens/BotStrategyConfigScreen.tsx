import React, { useCallback, useEffect, useState } from 'react';
import {
  ActivityIndicator,
  Alert,
  ScrollView,
  StyleSheet,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import { RouteProp, useNavigation, useRoute } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { AlgorithmMode, AssetFocus, BotConfig, Strictness } from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';
import { OceanBackground } from '../components/OceanBackground';
import type { RootStackParamList } from '../navigation/AppNavigator';
import { lightTheme } from '../theme';

type BotStrategyRoute = RouteProp<RootStackParamList, 'BotStrategyConfig'>;
type BotStrategyNav = NativeStackNavigationProp<RootStackParamList, 'BotStrategyConfig'>;

const ALGORITHMS: AlgorithmMode[] = ['trend', 'mean_reversion', 'breakout'];
const ASSETS: AssetFocus[] = ['majors', 'tokenized_equities', 'tokenized_metals', 'finance_2', 'custom', 'memes'];
const STRICTNESS: Strictness[] = ['low', 'medium', 'high'];

export function BotStrategyConfigScreen() {
  const navigation = useNavigation<BotStrategyNav>();
  const route = useRoute<BotStrategyRoute>();
  const { botId } = route.params;

  const [config, setConfig] = useState<BotConfig | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);

  const fetchConfig = useCallback(async () => {
    try {
      const response = await api.bot.getBot(botId);
      setConfig(response.config);
    } catch {
      Alert.alert('Error', 'Failed to load strategy settings.');
    } finally {
      setIsLoading(false);
    }
  }, [botId]);

  useEffect(() => {
    fetchConfig();
  }, [fetchConfig]);

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
          riskCaps: config.riskCaps,
          tradingMode: config.tradingMode,
          llmProvider: config.llmProvider,
          llmApiKey: undefined,
          signalKnobs: config.signalKnobs,
        },
      });
      Alert.alert('Saved', 'Strategy updated.');
      navigation.goBack();
    } catch {
      Alert.alert('Error', 'Failed to save strategy settings.');
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
        <Text style={styles.title}>Strategy Configuration</Text>

        <Text style={styles.sectionLabel}>Algorithm</Text>
        <View style={styles.row}>
          {ALGORITHMS.map((value) => (
            <TouchableOpacity
              key={value}
              style={[styles.chip, config.algorithmMode === value && styles.chipSelected]}
              onPress={() => setConfig((prev) => (prev ? { ...prev, algorithmMode: value } : prev))}
            >
              <Text style={[styles.chipText, config.algorithmMode === value && styles.chipTextSelected]}>{value}</Text>
            </TouchableOpacity>
          ))}
        </View>

        <Text style={styles.sectionLabel}>Asset Focus</Text>
        <View style={styles.row}>
          {ASSETS.map((value) => (
            <TouchableOpacity
              key={value}
              style={[styles.chip, config.assetFocus === value && styles.chipSelected]}
              onPress={() => setConfig((prev) => (prev ? { ...prev, assetFocus: value } : prev))}
            >
              <Text style={[styles.chipText, config.assetFocus === value && styles.chipTextSelected]}>{value}</Text>
            </TouchableOpacity>
          ))}
        </View>

        <Text style={styles.sectionLabel}>Strictness</Text>
        <View style={styles.row}>
          {STRICTNESS.map((value) => (
            <TouchableOpacity
              key={value}
              style={[styles.chip, config.strictness === value && styles.chipSelected]}
              onPress={() => setConfig((prev) => (prev ? { ...prev, strictness: value } : prev))}
            >
              <Text style={[styles.chipText, config.strictness === value && styles.chipTextSelected]}>{value}</Text>
            </TouchableOpacity>
          ))}
        </View>

        <TouchableOpacity style={styles.saveButton} onPress={save} disabled={isSaving}>
          <Text style={styles.saveButtonText}>{isSaving ? 'Saving...' : 'Save Strategy'}</Text>
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
  },
  sectionLabel: {
    marginTop: 20,
    marginBottom: 10,
    fontSize: 14,
    fontWeight: '600',
    color: lightTheme.colors.wave[700],
  },
  row: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 8,
  },
  chip: {
    borderWidth: 1,
    borderColor: lightTheme.colors.wave[300],
    borderRadius: 16,
    paddingVertical: 8,
    paddingHorizontal: 12,
    backgroundColor: lightTheme.colors.surface,
  },
  chipSelected: {
    borderColor: lightTheme.colors.primary[600],
    backgroundColor: lightTheme.colors.primary[50],
  },
  chipText: {
    color: lightTheme.colors.wave[700],
    textTransform: 'capitalize',
  },
  chipTextSelected: {
    color: lightTheme.colors.primary[700],
    fontWeight: '600',
  },
  saveButton: {
    marginTop: 28,
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
