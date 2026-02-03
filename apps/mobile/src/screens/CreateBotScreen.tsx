import React, { useState } from 'react';
import {
  View,
  Text,
  StyleSheet,
  ScrollView,
  TextInput,
  Button,
  TouchableOpacity,
  Switch,
} from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';
import type { Persona, AlgorithmMode, AssetFocus, TradingMode, Strictness, LlmProvider } from '@trawling-traders/types';

type CreateBotScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'CreateBot'>;

const PERSONAS: { value: Persona; label: string; description: string }[] = [
  { value: 'beginner', label: 'Set & Forget', description: 'Simple defaults, paper trading, strict safety' },
  { value: 'tweaker', label: 'Hands-on', description: 'Tune assets, risk, and see trade reasons' },
  { value: 'quant-lite', label: 'Power User', description: 'Signal knobs and advanced controls' },
];

const ALGORITHMS: { value: AlgorithmMode; label: string; description: string }[] = [
  { value: 'trend', label: 'Trend', description: 'Ride momentum with confirmations' },
  { value: 'mean-reversion', label: 'Mean Reversion', description: 'Fade extremes, frequent trades' },
  { value: 'breakout', label: 'Breakout', description: 'Trade breakouts with volume' },
];

const ASSET_FOCUSES: { value: AssetFocus; label: string }[] = [
  { value: 'majors', label: 'Majors (BTC, ETH, etc.)' },
  { value: 'memes', label: 'Meme Coins' },
  { value: 'custom', label: 'Custom Selection' },
];

export function CreateBotScreen() {
  const navigation = useNavigation<CreateBotScreenNavigationProp>();
  const [isLoading, setIsLoading] = useState(false);

  // Form state
  const [name, setName] = useState('');
  const [persona, setPersona] = useState<Persona>('beginner');
  const [assetFocus, setAssetFocus] = useState<AssetFocus>('majors');
  const [algorithmMode, setAlgorithmMode] = useState<AlgorithmMode>('trend');
  const [strictness, setStrictness] = useState<Strictness>('medium');
  const [tradingMode, setTradingMode] = useState<TradingMode>('paper');
  const [llmProvider, setLlmProvider] = useState<LlmProvider>('openai');
  const [llmApiKey, setLlmApiKey] = useState('');

  // Risk caps
  const [maxPositionSize, setMaxPositionSize] = useState('10');
  const [maxDailyLoss, setMaxDailyLoss] = useState('100');
  const [maxDrawdown, setMaxDrawdown] = useState('20');
  const [maxTradesPerDay, setMaxTradesPerDay] = useState('10');

  // Signal knobs (Quant-lite only)
  const [volumeConfirmation, setVolumeConfirmation] = useState(true);
  const [volatilityBrake, setVolatilityBrake] = useState(true);
  const [liquidityFilter, setLiquidityFilter] = useState<'low' | 'medium' | 'high'>('medium');
  const [correlationBrake, setCorrelationBrake] = useState(true);

  const handleCreate = async () => {
    setIsLoading(true);
    try {
      // TODO: Call API to create bot
      // const request: CreateBotRequest = {
      //   name,
      //   persona,
      //   assetFocus,
      //   algorithmMode,
      //   strictness,
      //   riskCaps: {
      //     maxPositionSizePercent: parseInt(maxPositionSize),
      //     maxDailyLossUsd: parseInt(maxDailyLoss),
      //     maxDrawdownPercent: parseInt(maxDrawdown),
      //     maxTradesPerDay: parseInt(maxTradesPerDay),
      //   },
      //   tradingMode,
      //   llmProvider,
      //   llmApiKey,
      // };
      // await getApi().createBot(request);

      navigation.navigate('Main');
    } catch (error) {
      console.error('Failed to create bot:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const isValid = name.trim() && llmApiKey.trim();

  return (
    <ScrollView style={styles.container}>
      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Bot Identity</Text>
        
        <Text style={styles.label}>Name</Text>
        <TextInput
          style={styles.input}
          value={name}
          onChangeText={setName}
          placeholder="My Trading Bot"
          autoCapitalize="words"
        />
      </View>

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Trading Persona</Text>
        
        {PERSONAS.map((p) => (
          <TouchableOpacity
            key={p.value}
            style={[styles.optionCard, persona === p.value && styles.selectedCard]}
            onPress={() => setPersona(p.value)}
          >
            <Text style={styles.optionTitle}>{p.label}</Text>
            <Text style={styles.optionDescription}>{p.description}</Text>
          </TouchableOpacity>
        ))}
      </View>

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Asset Focus</Text>
        
        <View style={styles.optionsRow}>
          {ASSET_FOCUSES.map((af) => (
            <TouchableOpacity
              key={af.value}
              style={[styles.chip, assetFocus === af.value && styles.selectedChip]}
              onPress={() => setAssetFocus(af.value)}
            >
              <Text style={[styles.chipText, assetFocus === af.value && styles.selectedChipText]}>
                {af.label}
              </Text>
            </TouchableOpacity>
          ))}
        </View>
      </View>

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Algorithm</Text>
        
        {ALGORITHMS.map((alg) => (
          <TouchableOpacity
            key={alg.value}
            style={[styles.optionCard, algorithmMode === alg.value && styles.selectedCard]}
            onPress={() => setAlgorithmMode(alg.value)}
          >
            <Text style={styles.optionTitle}>{alg.label}</Text>
            <Text style={styles.optionDescription}>{alg.description}</Text>
          </TouchableOpacity>
        ))}
      </View>

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Risk Caps</Text>
        
        <View style={styles.inputRow}>
          <View style={styles.inputGroup}>
            <Text style={styles.label}>Max Position %</Text>
            <TextInput
              style={styles.smallInput}
              value={maxPositionSize}
              onChangeText={setMaxPositionSize}
              keyboardType="numeric"
            />
          </View>
          
          <View style={styles.inputGroup}>
            <Text style={styles.label}>Max Daily Loss ($)</Text>
            <TextInput
              style={styles.smallInput}
              value={maxDailyLoss}
              onChangeText={setMaxDailyLoss}
              keyboardType="numeric"
            />
          </View>
        </View>

        <View style={styles.inputRow}>
          <View style={styles.inputGroup}>
            <Text style={styles.label}>Max Drawdown %</Text>
            <TextInput
              style={styles.smallInput}
              value={maxDrawdown}
              onChangeText={setMaxDrawdown}
              keyboardType="numeric"
            />
          </View>
          
          <View style={styles.inputGroup}>
            <Text style={styles.label}>Max Trades/Day</Text>
            <TextInput
              style={styles.smallInput}
              value={maxTradesPerDay}
              onChangeText={setMaxTradesPerDay}
              keyboardType="numeric"
            />
          </View>
        </View>
      </View>

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Trading Mode</Text>
        
        <View style={styles.switchRow}>
          <Text>{tradingMode === 'paper' ? '📄 Paper Trading' : '💰 Live Trading'}</Text>
          <Switch
            value={tradingMode === 'live'}
            onValueChange={(v) => setTradingMode(v ? 'live' : 'paper')}
          />
        </View>
        
        {tradingMode === 'live' && (
          <Text style={styles.warning}>⚠️ Live trading uses real funds. Start with paper first.</Text>
        )}
      </View>

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>LLM Configuration</Text>
        
        <Text style={styles.label}>Provider</Text>
        <View style={styles.optionsRow}>
          {(['openai', 'anthropic', 'venice', 'openrouter'] as LlmProvider[]).map((provider) => (
            <TouchableOpacity
              key={provider}
              style={[styles.chip, llmProvider === provider && styles.selectedChip]}
              onPress={() => setLlmProvider(provider)}
            >
              <Text style={[styles.chipText, llmProvider === provider && styles.selectedChipText]}>
                {provider}
              </Text>
            </TouchableOpacity>
          ))}
        </View>

        <Text style={styles.label}>API Key</Text>
        <TextInput
          style={styles.input}
          value={llmApiKey}
          onChangeText={setLlmApiKey}
          placeholder="sk-..."
          secureTextEntry
        />
      </View>

      <View style={styles.buttonContainer}>
        <Button
          title={isLoading ? 'Creating...' : 'Create Bot'}
          onPress={handleCreate}
          disabled={!isValid || isLoading}
        />
      </View>
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#f5f5f5',
  },
  section: {
    backgroundColor: '#fff',
    margin: 16,
    marginBottom: 8,
    padding: 16,
    borderRadius: 12,
  },
  sectionTitle: {
    fontSize: 18,
    fontWeight: 'bold',
    marginBottom: 16,
  },
  label: {
    fontSize: 14,
    fontWeight: '600',
    marginBottom: 8,
    color: '#333',
  },
  input: {
    borderWidth: 1,
    borderColor: '#ddd',
    borderRadius: 8,
    padding: 12,
    fontSize: 16,
    backgroundColor: '#fafafa',
  },
  smallInput: {
    borderWidth: 1,
    borderColor: '#ddd',
    borderRadius: 8,
    padding: 12,
    fontSize: 16,
    backgroundColor: '#fafafa',
    width: 100,
  },
  optionCard: {
    borderWidth: 1,
    borderColor: '#ddd',
    borderRadius: 8,
    padding: 16,
    marginBottom: 8,
  },
  selectedCard: {
    borderColor: '#007AFF',
    backgroundColor: '#F0F8FF',
  },
  optionTitle: {
    fontSize: 16,
    fontWeight: '600',
    marginBottom: 4,
  },
  optionDescription: {
    fontSize: 14,
    color: '#666',
  },
  optionsRow: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 8,
  },
  chip: {
    borderWidth: 1,
    borderColor: '#ddd',
    borderRadius: 16,
    paddingHorizontal: 12,
    paddingVertical: 6,
  },
  selectedChip: {
    backgroundColor: '#007AFF',
    borderColor: '#007AFF',
  },
  chipText: {
    fontSize: 14,
    color: '#333',
  },
  selectedChipText: {
    color: '#fff',
  },
  inputRow: {
    flexDirection: 'row',
    gap: 16,
  },
  inputGroup: {
    flex: 1,
  },
  switchRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  warning: {
    color: '#F44336',
    marginTop: 8,
    fontSize: 14,
  },
  buttonContainer: {
    margin: 16,
    marginTop: 8,
    marginBottom: 32,
  },
});
