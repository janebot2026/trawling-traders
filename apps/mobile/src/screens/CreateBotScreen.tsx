import React, { useState, useRef, useEffect } from 'react';
import {
  View,
  Text,
  StyleSheet,
  ScrollView,
  TextInput,
  TouchableOpacity,
  Switch,
  ActivityIndicator,
  Alert,
  Animated,
} from 'react-native';
import { useNavigation } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';
import type { Persona, AlgorithmMode, AssetFocus, TradingMode, Strictness, LlmProvider } from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';
import { OceanBackground } from '../components/OceanBackground';
import { lightTheme } from '../theme';

// Validation helper for numeric inputs
function validateNumericInput(
  value: string,
  fieldName: string,
  min: number,
  max: number
): { valid: boolean; value: number; error?: string } {
  const trimmed = value.trim();
  if (!trimmed) {
    return { valid: false, value: 0, error: `${fieldName} is required` };
  }
  const num = parseInt(trimmed, 10);
  if (isNaN(num)) {
    return { valid: false, value: 0, error: `${fieldName} must be a number` };
  }
  if (num < min || num > max) {
    return { valid: false, value: num, error: `${fieldName} must be between ${min} and ${max}` };
  }
  return { valid: true, value: num };
}

type CreateBotScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'CreateBot'>;

const PERSONAS: { value: Persona; label: string; description: string }[] = [
  { value: 'beginner', label: 'Set & Forget', description: 'Blue-chip crypto + xStocks/metals, strict safety' },
  { value: 'tweaker', label: 'Hands-on', description: 'Tune assets, risk controls, see trade reasons' },
  { value: 'quant-lite', label: 'Power User', description: 'Signal knobs, custom baskets, full control' },
];

const ALGORITHMS: { value: AlgorithmMode; label: string; description: string }[] = [
  { value: 'trend', label: 'Trend', description: 'Ride momentum with confirmations' },
  { value: 'mean-reversion', label: 'Mean Reversion', description: 'Fade extremes, frequent trades' },
  { value: 'breakout', label: 'Breakout', description: 'Trade breakouts with volume' },
];

const ASSET_FOCUSES: { value: AssetFocus; label: string; description: string; tier: 'core' | 'quality' | 'speculative' }[] = [
  { value: 'majors', label: 'Crypto Majors', description: 'BTC, ETH, SOL', tier: 'core' },
  { value: 'tokenized-equities', label: 'xStocks', description: 'AAPL, TSLA, SPY on Solana', tier: 'quality' },
  { value: 'tokenized-metals', label: 'Metals', description: 'Gold, silver (ORO)', tier: 'quality' },
  { value: 'custom', label: 'Custom', description: 'Build your own basket', tier: 'quality' },
  { value: 'memes', label: 'Memes ⚠️', description: 'High risk', tier: 'speculative' },
];

export function CreateBotScreen() {
  const navigation = useNavigation<CreateBotScreenNavigationProp>();
  const [isLoading, setIsLoading] = useState(false);

  // Animation
  const fadeAnim = useRef(new Animated.Value(0)).current;
  const slideAnim = useRef(new Animated.Value(20)).current;

  useEffect(() => {
    Animated.parallel([
      Animated.timing(fadeAnim, { toValue: 1, duration: 400, useNativeDriver: true }),
      Animated.timing(slideAnim, { toValue: 0, duration: 400, useNativeDriver: true }),
    ]).start();
  }, []);

  // Form state
  const [name, setName] = useState('');
  const [persona, setPersona] = useState<Persona>('beginner');
  const [assetFocus, setAssetFocus] = useState<AssetFocus>('tokenized-equities');
  const [algorithmMode, setAlgorithmMode] = useState<AlgorithmMode>('trend');
  const [strictness, setStrictness] = useState<Strictness>('high');
  const [tradingMode, setTradingMode] = useState<TradingMode>('paper');
  const [llmProvider, setLlmProvider] = useState<LlmProvider>('openai');
  const [llmApiKey, setLlmApiKey] = useState('');
  const [maxPositionSize, setMaxPositionSize] = useState('5');
  const [maxDailyLoss, setMaxDailyLoss] = useState('50');
  const [maxDrawdown, setMaxDrawdown] = useState('10');
  const [maxTradesPerDay, setMaxTradesPerDay] = useState('5');
  const [volumeConfirmation, setVolumeConfirmation] = useState(true);
  const [volatilityBrake, setVolatilityBrake] = useState(true);
  const [liquidityFilter, setLiquidityFilter] = useState<'low' | 'medium' | 'high'>('high');
  const [correlationBrake, setCorrelationBrake] = useState(true);

  const handleCreate = async () => {
    if (!name.trim()) {
      Alert.alert('Error', 'Please enter a bot name');
      return;
    }
    if (!llmApiKey.trim()) {
      Alert.alert('Error', 'Please enter your LLM API key');
      return;
    }

    // Validate numeric inputs
    const positionValidation = validateNumericInput(maxPositionSize, 'Max Position Size', 1, 50);
    if (!positionValidation.valid) {
      Alert.alert('Validation Error', positionValidation.error);
      return;
    }

    const dailyLossValidation = validateNumericInput(maxDailyLoss, 'Max Daily Loss', 1, 100000);
    if (!dailyLossValidation.valid) {
      Alert.alert('Validation Error', dailyLossValidation.error);
      return;
    }

    const drawdownValidation = validateNumericInput(maxDrawdown, 'Max Drawdown', 1, 50);
    if (!drawdownValidation.valid) {
      Alert.alert('Validation Error', drawdownValidation.error);
      return;
    }

    const tradesValidation = validateNumericInput(maxTradesPerDay, 'Max Trades Per Day', 1, 100);
    if (!tradesValidation.valid) {
      Alert.alert('Validation Error', tradesValidation.error);
      return;
    }

    setIsLoading(true);
    try {
      await api.bot.createBot({
        name: name.trim(),
        persona,
        assetFocus,
        algorithmMode,
        strictness,
        riskCaps: {
          maxPositionSizePercent: positionValidation.value,
          maxDailyLossUsd: dailyLossValidation.value,
          maxDrawdownPercent: drawdownValidation.value,
          maxTradesPerDay: tradesValidation.value,
        },
        tradingMode,
        llmProvider,
        llmApiKey: llmApiKey.trim(),
      });
      Alert.alert('Success', 'Trawler deployed!');
      navigation.navigate('Main');
    } catch (error) {
      Alert.alert('Error', 'Failed to deploy trawler');
    } finally {
      setIsLoading(false);
    }
  };

  const renderSection = (title: string, children: React.ReactNode) => (
    <View style={styles.card}>
      <Text style={styles.sectionTitle}>{title}</Text>
      {children}
    </View>
  );

  return (
    <OceanBackground>
      <Animated.ScrollView
        style={[styles.container, { opacity: fadeAnim }]}
        contentContainerStyle={{ paddingBottom: 40 }}
      >
        {/* Header */}
        <View style={styles.header}>
          <Text style={styles.headerTitle}>Deploy New Trawler</Text>
          <Text style={styles.headerSubtitle}>Configure your LOB trading agent</Text>
        </View>

        {renderSection('Identity',
          <>
            <Text style={styles.label}>Name</Text>
            <TextInput
              style={styles.input}
              value={name}
              onChangeText={setName}
              placeholder="e.g., Trend Hunter #1"
              placeholderTextColor={lightTheme.colors.wave[400]}
            />
          </>
        )}

        {renderSection('Trading Persona',
          <>
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
          </>
        )}

        {renderSection('Asset Focus',
          <>
            <View style={styles.assetGrid}>
              {ASSET_FOCUSES.filter(a => a.tier !== 'speculative').map((af) => (
                <TouchableOpacity
                  key={af.value}
                  style={[
                    styles.assetChip,
                    assetFocus === af.value && styles.assetChipSelected,
                    { borderColor: af.tier === 'quality' ? lightTheme.colors.bullish[500] : lightTheme.colors.primary[500] },
                  ]}
                  onPress={() => setAssetFocus(af.value)}
                >
                  <Text style={[styles.assetChipText, assetFocus === af.value && styles.assetChipTextSelected]}>
                    {af.label}
                  </Text>
                  <Text style={styles.assetChipSub}>{af.description}</Text>
                </TouchableOpacity>
              ))}
            </View>

            <View style={styles.speculativeSection}>
              <Text style={styles.speculativeLabel}>⚠️ Speculative (Not Recommended)</Text>
              <TouchableOpacity
                style={[styles.assetChip, assetFocus === 'memes' && styles.assetChipSpecSelected]}
                onPress={() => setAssetFocus('memes')}
              >
                <Text style={styles.assetChipText}>Meme Coins</Text>
              </TouchableOpacity>
            </View>
          </>
        )}

        {renderSection('Algorithm',
          <>
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
          </>
        )}

        {renderSection('Risk Management',
          <>
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
                <Text style={styles.label}>Daily Loss ($)</Text>
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
                <Text style={styles.label}>Trades/Day</Text>
                <TextInput
                  style={styles.smallInput}
                  value={maxTradesPerDay}
                  onChangeText={setMaxTradesPerDay}
                  keyboardType="numeric"
                />
              </View>
            </View>
          </>
        )}

        {renderSection('Trading Mode',
          <>
            <View style={styles.switchRow}>
              <View>
                <Text style={styles.switchLabel}>
                  {tradingMode === 'paper' ? '📄 Paper Trading' : '💰 Live Trading'}
                </Text>
                <Text style={styles.switchSub}>
                  {tradingMode === 'paper' ? 'Test with fake money' : 'Real funds at risk'}
                </Text>
              </View>
              <Switch
                value={tradingMode === 'live'}
                onValueChange={(v) => setTradingMode(v ? 'live' : 'paper')}
                trackColor={{ false: lightTheme.colors.wave[300], true: lightTheme.colors.lobster[400] }}
                thumbColor={tradingMode === 'live' ? lightTheme.colors.lobster[600] : '#fff'}
              />
            </View>
            
            {tradingMode === 'live' && (
              <View style={styles.warningBox}>
                <Text style={styles.warningText}>⚠️ Live trading uses real funds. Start with paper first.</Text>
              </View>
            )}
          </>
        )}

        {renderSection('LLM Configuration',
          <>
            <Text style={styles.label}>Provider</Text>
            <View style={styles.providerRow}>
              {(['openai', 'anthropic', 'venice', 'openrouter'] as LlmProvider[]).map((provider) => (
                <TouchableOpacity
                  key={provider}
                  style={[styles.providerChip, llmProvider === provider && styles.providerChipSelected]}
                  onPress={() => setLlmProvider(provider)}
                >
                  <Text style={[styles.providerText, llmProvider === provider && styles.providerTextSelected]}>
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
              placeholderTextColor={lightTheme.colors.wave[400]}
              secureTextEntry
            />
          </>
        )}

        {persona === 'quant-lite' && renderSection('Signal Knobs',
          <>
            {[
              { label: 'Volume Confirmation', value: volumeConfirmation, setter: setVolumeConfirmation },
              { label: 'Volatility Brake', value: volatilityBrake, setter: setVolatilityBrake },
              { label: 'Correlation Brake', value: correlationBrake, setter: setCorrelationBrake },
            ].map((knob) => (
              <View key={knob.label} style={styles.switchRow}>
                <Text style={styles.switchLabel}>{knob.label}</Text>
                <Switch
                  value={knob.value}
                  onValueChange={knob.setter}
                  trackColor={{ false: lightTheme.colors.wave[300], true: lightTheme.colors.bullish[400] }}
                />
              </View>
            ))}
            
            <Text style={styles.label}>Liquidity Filter</Text>
            <View style={styles.providerRow}>
              {(['low', 'medium', 'high'] as const).map((level) => (
                <TouchableOpacity
                  key={level}
                  style={[styles.providerChip, liquidityFilter === level && styles.providerChipSelected]}
                  onPress={() => setLiquidityFilter(level)}
                >
                  <Text style={[styles.providerText, liquidityFilter === level && styles.providerTextSelected]}>
                    {level}
                  </Text>
                </TouchableOpacity>
              ))}
            </View>
          </>
        )}

        {/* Deploy Button */}
        <View style={styles.card}>
          {isLoading ? (
            <ActivityIndicator size="large" color={lightTheme.colors.primary[700]} />
          ) : (
            <TouchableOpacity
              style={styles.deployButton}
              onPress={handleCreate}
              disabled={!name.trim() || !llmApiKey.trim()}
            >
              <Text style={styles.deployButtonText}>🚀 Deploy Trawler</Text>
            </TouchableOpacity>
          )}
        </View>
      </Animated.ScrollView>
    </OceanBackground>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
  },
  header: {
    padding: 20,
    paddingTop: 60,
    backgroundColor: 'rgba(255, 255, 255, 0.95)',
    borderBottomLeftRadius: 24,
    borderBottomRightRadius: 24,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 8,
    elevation: 3,
  },
  headerTitle: {
    fontSize: 28,
    fontWeight: 'bold',
    color: lightTheme.colors.wave[900],
  },
  headerSubtitle: {
    fontSize: 16,
    color: lightTheme.colors.wave[500],
    marginTop: 4,
  },
  card: {
    backgroundColor: 'rgba(255, 255, 255, 0.95)',
    margin: 16,
    marginBottom: 8,
    padding: 20,
    borderRadius: 16,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.1,
    shadowRadius: 12,
    elevation: 4,
  },
  sectionTitle: {
    fontSize: 18,
    fontWeight: 'bold',
    color: lightTheme.colors.wave[900],
    marginBottom: 16,
  },
  label: {
    fontSize: 14,
    fontWeight: '600',
    color: lightTheme.colors.wave[700],
    marginBottom: 8,
  },
  input: {
    borderWidth: 1,
    borderColor: lightTheme.colors.wave[200],
    borderRadius: 10,
    padding: 12,
    fontSize: 16,
    backgroundColor: lightTheme.colors.wave[50],
    color: lightTheme.colors.wave[900],
  },
  smallInput: {
    borderWidth: 1,
    borderColor: lightTheme.colors.wave[200],
    borderRadius: 10,
    padding: 12,
    fontSize: 16,
    backgroundColor: lightTheme.colors.wave[50],
    width: 100,
    color: lightTheme.colors.wave[900],
  },
  inputRow: {
    flexDirection: 'row',
    gap: 16,
  },
  inputGroup: {
    flex: 1,
  },
  optionCard: {
    borderWidth: 2,
    borderColor: lightTheme.colors.wave[200],
    borderRadius: 12,
    padding: 16,
    marginBottom: 10,
    backgroundColor: lightTheme.colors.wave[50],
  },
  selectedCard: {
    borderColor: lightTheme.colors.primary[600],
    backgroundColor: lightTheme.colors.primary[50],
  },
  optionTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: lightTheme.colors.wave[900],
    marginBottom: 4,
  },
  optionDescription: {
    fontSize: 13,
    color: lightTheme.colors.wave[500],
  },
  assetGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 10,
  },
  assetChip: {
    borderWidth: 2,
    borderRadius: 12,
    padding: 12,
    minWidth: '30%',
    backgroundColor: lightTheme.colors.wave[50],
  },
  assetChipSelected: {
    backgroundColor: lightTheme.colors.bullish[50],
  },
  assetChipSpecSelected: {
    backgroundColor: lightTheme.colors.lobster[50],
    borderColor: lightTheme.colors.lobster[400],
  },
  assetChipText: {
    fontSize: 14,
    fontWeight: '600',
    color: lightTheme.colors.wave[800],
  },
  assetChipTextSelected: {
    color: lightTheme.colors.bullish[700],
  },
  assetChipSub: {
    fontSize: 11,
    color: lightTheme.colors.wave[500],
    marginTop: 2,
  },
  speculativeSection: {
    marginTop: 16,
    paddingTop: 16,
    borderTopWidth: 1,
    borderTopColor: lightTheme.colors.wave[200],
  },
  speculativeLabel: {
    fontSize: 12,
    fontWeight: '700',
    color: lightTheme.colors.lobster[600],
    marginBottom: 10,
  },
  switchRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  switchLabel: {
    fontSize: 16,
    fontWeight: '600',
    color: lightTheme.colors.wave[900],
  },
  switchSub: {
    fontSize: 13,
    color: lightTheme.colors.wave[500],
    marginTop: 2,
  },
  warningBox: {
    backgroundColor: lightTheme.colors.lobster[100],
    padding: 12,
    borderRadius: 10,
    marginTop: 8,
  },
  warningText: {
    color: lightTheme.colors.lobster[700],
    fontSize: 13,
  },
  providerRow: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 8,
  },
  providerChip: {
    borderWidth: 1,
    borderColor: lightTheme.colors.wave[300],
    borderRadius: 20,
    paddingHorizontal: 14,
    paddingVertical: 8,
    backgroundColor: lightTheme.colors.wave[50],
  },
  providerChipSelected: {
    backgroundColor: lightTheme.colors.primary[600],
    borderColor: lightTheme.colors.primary[600],
  },
  providerText: {
    fontSize: 14,
    color: lightTheme.colors.wave[700],
  },
  providerTextSelected: {
    color: '#fff',
    fontWeight: '600',
  },
  deployButton: {
    backgroundColor: lightTheme.colors.primary[700],
    paddingVertical: 16,
    borderRadius: 12,
    alignItems: 'center',
    shadowColor: lightTheme.colors.primary[700],
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.3,
    shadowRadius: 8,
    elevation: 4,
  },
  deployButtonText: {
    color: '#fff',
    fontSize: 18,
    fontWeight: 'bold',
  },
});
