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
  { value: 'beginner', label: 'Set & Forget', description: 'Blue-chip crypto + tokenized equities/metals, strict safety' },
  { value: 'tweaker', label: 'Hands-on', description: 'Tune assets including xStocks, risk controls, see trade reasons' },
  { value: 'quant-lite', label: 'Power User', description: 'Signal knobs, custom baskets, full control' },
];

const ALGORITHMS: { value: AlgorithmMode; label: string; description: string }[] = [
  { value: 'trend', label: 'Trend', description: 'Ride momentum with confirmations' },
  { value: 'mean-reversion', label: 'Mean Reversion', description: 'Fade extremes, frequent trades' },
  { value: 'breakout', label: 'Breakout', description: 'Trade breakouts with volume' },
];

// ASSET FOCUS: Quality assets first, memes require explicit opt-in
const ASSET_FOCUSES: { value: AssetFocus; label: string; description: string; tier: 'core' | 'quality' | 'speculative' }[] = [
  { 
    value: 'majors', 
    label: 'Crypto Majors', 
    description: 'BTC, ETH, SOL - Blue chip cryptocurrencies',
    tier: 'core'
  },
  { 
    value: 'tokenized-equities', 
    label: 'Tokenized Stocks (xStocks)', 
    description: 'Stock exposures on Solana - AAPL, TSLA, SPY, etc.',
    tier: 'quality'
  },
  { 
    value: 'tokenized-metals', 
    label: 'Tokenized Metals', 
    description: 'Gold, silver - ORO and similar assets',
    tier: 'quality'
  },
  { 
    value: 'custom', 
    label: 'Custom Basket', 
    description: 'Build your own asset selection',
    tier: 'quality'
  },
  { 
    value: 'memes', 
    label: 'Meme Coins ⚠️', 
    description: 'High volatility, high risk - Not recommended for serious capital',
    tier: 'speculative'
  },
];

export function CreateBotScreen() {
  const navigation = useNavigation<CreateBotScreenNavigationProp>();
  const [isLoading, setIsLoading] = useState(false);

  // Form state - DEFAULT to quality assets, not memes
  const [name, setName] = useState('');
  const [persona, setPersona] = useState<Persona>('beginner');
  const [assetFocus, setAssetFocus] = useState<AssetFocus>('tokenized-equities'); // Default to xStocks
  const [algorithmMode, setAlgorithmMode] = useState<AlgorithmMode>('trend');
  const [strictness, setStrictness] = useState<Strictness>('high'); // Default strict for safety
  const [tradingMode, setTradingMode] = useState<TradingMode>('paper');
  const [llmProvider, setLlmProvider] = useState<LlmProvider>('openai');
  const [llmApiKey, setLlmApiKey] = useState('');

  // Risk caps - Conservative defaults
  const [maxPositionSize, setMaxPositionSize] = useState('5'); // 5% default (conservative)
  const [maxDailyLoss, setMaxDailyLoss] = useState('50');
  const [maxDrawdown, setMaxDrawdown] = useState('10'); // 10% max drawdown
  const [maxTradesPerDay, setMaxTradesPerDay] = useState('5'); // Fewer trades for quality

  // Signal knobs (Quant-lite only)
  const [volumeConfirmation, setVolumeConfirmation] = useState(true);
  const [volatilityBrake, setVolatilityBrake] = useState(true);
  const [liquidityFilter, setLiquidityFilter] = useState<'low' | 'medium' | 'high'>('high'); // High liquidity default
  const [correlationBrake, setCorrelationBrake] = useState(true);

  const handleCreate = async () => {
    setIsLoading(true);
    try {
      // TODO: Call API to create bot
      navigation.navigate('Main');
    } catch (error) {
      console.error('Failed to create bot:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const isValid = name.trim() && llmApiKey.trim();

  // Filter asset focuses by tier
  const coreAssets = ASSET_FOCUSES.filter(a => a.tier === 'core');
  const qualityAssets = ASSET_FOCUSES.filter(a => a.tier === 'quality');
  const speculativeAssets = ASSET_FOCUSES.filter(a => a.tier === 'speculative');

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
        <Text style={styles.sectionSubtitle}>Solana-powered trading in quality assets</Text>

        {/* Core Assets */}
        <Text style={styles.tierLabel}>Core Crypto</Text>
        <View style={styles.optionsRow}>
          {coreAssets.map((af) => (
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

        {/* Quality Assets - xStocks and Metals */}
        <Text style={styles.tierLabel}>Tokenized Assets ⭐</Text>
        <View style={styles.qualityAssetsContainer}>
          {qualityAssets.map((af) => (
            <TouchableOpacity
              key={af.value}
              style={[
                styles.qualityCard, 
                assetFocus === af.value && styles.selectedQualityCard
              ]}
              onPress={() => setAssetFocus(af.value)}
            >
              <Text style={[
                styles.qualityTitle, 
                assetFocus === af.value && styles.selectedQualityTitle
              ]}>
                {af.label}
              </Text>
              <Text style={styles.qualityDescription}>{af.description}</Text>
            </TouchableOpacity>
          ))}
        </View>

        {/* Speculative - Memes with warning */}
        <View style={styles.speculativeSection}>
          <Text style={styles.speculativeLabel}>⚠️ Speculative (Not Recommended)</Text>
          {speculativeAssets.map((af) => (
            <TouchableOpacity
              key={af.value}
              style={[styles.speculativeCard, assetFocus === af.value && styles.selectedSpeculativeCard]}
              onPress={() => setAssetFocus(af.value)}
            >
              <Text style={styles.speculativeTitle}>{af.label}</Text>
              <Text style={styles.speculativeDescription}>{af.description}</Text>
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
    marginBottom: 8,
  },
  sectionSubtitle: {
    fontSize: 14,
    color: '#666',
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
  tierLabel: {
    fontSize: 12,
    fontWeight: '700',
    color: '#666',
    marginTop: 12,
    marginBottom: 8,
    textTransform: 'uppercase',
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
  // Quality assets styling
  qualityAssetsContainer: {
    gap: 8,
  },
  qualityCard: {
    borderWidth: 2,
    borderColor: '#22c55e',
    borderRadius: 8,
    padding: 12,
    backgroundColor: '#f0fdf4',
  },
  selectedQualityCard: {
    backgroundColor: '#22c55e',
  },
  qualityTitle: {
    fontSize: 15,
    fontWeight: '600',
    color: '#15803d',
    marginBottom: 2,
  },
  selectedQualityTitle: {
    color: '#fff',
  },
  qualityDescription: {
    fontSize: 12,
    color: '#666',
  },
  // Speculative section
  speculativeSection: {
    marginTop: 16,
    paddingTop: 16,
    borderTopWidth: 1,
    borderTopColor: '#e5e5e5',
  },
  speculativeLabel: {
    fontSize: 12,
    fontWeight: '700',
    color: '#dc2626',
    marginBottom: 8,
  },
  speculativeCard: {
    borderWidth: 1,
    borderColor: '#fca5a5',
    borderRadius: 8,
    padding: 12,
    backgroundColor: '#fef2f2',
    opacity: 0.8,
  },
  selectedSpeculativeCard: {
    borderColor: '#dc2626',
    backgroundColor: '#fee2e2',
    opacity: 1,
  },
  speculativeTitle: {
    fontSize: 14,
    fontWeight: '600',
    color: '#dc2626',
  },
  speculativeDescription: {
    fontSize: 12,
    color: '#991b1b',
    marginTop: 2,
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
