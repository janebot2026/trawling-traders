import React, { useState, useEffect, useCallback } from 'react';
import {
  View,
  Text,
  StyleSheet,
  ScrollView,
  TextInput,
  Button,
  TouchableOpacity,
  Switch,
  ActivityIndicator,
  Alert,
} from 'react-native';
import { useNavigation, useRoute, RouteProp } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';
import type { Bot, BotConfig, Persona, AlgorithmMode, AssetFocus, Strictness, TradingMode, LlmProvider } from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';

type BotSettingsScreenRouteProp = RouteProp<RootStackParamList, 'BotSettings'>;
type BotSettingsScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'BotSettings'>;

const PERSONAS: { value: Persona; label: string; description: string }[] = [
  { value: 'beginner', label: 'Set & Forget', description: 'Blue-chip crypto + tokenized equities/metals' },
  { value: 'tweaker', label: 'Hands-on', description: 'Tune assets including xStocks, risk controls' },
  { value: 'quant-lite', label: 'Power User', description: 'Signal knobs, custom baskets, full control' },
];

const ALGORITHMS: { value: AlgorithmMode; label: string; description: string }[] = [
  { value: 'trend', label: 'Trend', description: 'Ride momentum with confirmations' },
  { value: 'mean-reversion', label: 'Mean Reversion', description: 'Fade extremes, frequent trades' },
  { value: 'breakout', label: 'Breakout', description: 'Trade breakouts with volume' },
];

const ASSET_FOCUSES: { value: AssetFocus; label: string; description: string; tier: 'core' | 'quality' | 'speculative' }[] = [
  { value: 'majors', label: 'Crypto Majors', description: 'BTC, ETH, SOL', tier: 'core' },
  { value: 'tokenized-equities', label: 'Tokenized Stocks', description: 'AAPL, TSLA, SPY on Solana', tier: 'quality' },
  { value: 'tokenized-metals', label: 'Tokenized Metals', description: 'Gold, silver (ORO)', tier: 'quality' },
  { value: 'custom', label: 'Custom Basket', description: 'Build your own selection', tier: 'quality' },
  { value: 'memes', label: 'Meme Coins', description: 'High risk - Not recommended', tier: 'speculative' },
];

export function BotSettingsScreen() {
  const route = useRoute<BotSettingsScreenRouteProp>();
  const navigation = useNavigation<BotSettingsScreenNavigationProp>();
  const { botId } = route.params;

  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
  const [bot, setBot] = useState<Bot | null>(null);

  // Bot state
  const [name, setName] = useState('');
  const [persona, setPersona] = useState<Persona>('beginner');
  const [assetFocus, setAssetFocus] = useState<AssetFocus>('tokenized-equities');
  const [algorithmMode, setAlgorithmMode] = useState<AlgorithmMode>('trend');
  const [strictness, setStrictness] = useState<Strictness>('high');
  const [tradingMode, setTradingMode] = useState<TradingMode>('paper');
  const [llmProvider, setLlmProvider] = useState<LlmProvider>('openai');
  const [llmApiKey, setLlmApiKey] = useState('');

  // Risk caps
  const [maxPositionSize, setMaxPositionSize] = useState('5');
  const [maxDailyLoss, setMaxDailyLoss] = useState('50');
  const [maxDrawdown, setMaxDrawdown] = useState('10');
  const [maxTradesPerDay, setMaxTradesPerDay] = useState('5');

  // Signal knobs (Quant-lite only)
  const [volumeConfirmation, setVolumeConfirmation] = useState(true);
  const [volatilityBrake, setVolatilityBrake] = useState(true);
  const [liquidityFilter, setLiquidityFilter] = useState<'low' | 'medium' | 'high'>('high');
  const [correlationBrake, setCorrelationBrake] = useState(true);

  const fetchBotConfig = useCallback(async () => {
    try {
      const response = await api.bot.getBot(botId);
      setBot(response.bot);
      
      // Populate form with current config
      if (response.config) {
        setName(response.config.name || '');
        setPersona(response.config.persona);
        setAssetFocus(response.config.assetFocus);
        setAlgorithmMode(response.config.algorithmMode);
        setStrictness(response.config.strictness);
        setTradingMode(response.config.tradingMode);
        setLlmProvider(response.config.llmProvider);
        setLlmApiKey(response.config.llmApiKey || '');
        setMaxPositionSize(response.config.riskCaps?.maxPositionSizePercent?.toString() || '5');
        setMaxDailyLoss(response.config.riskCaps?.maxDailyLossUsd?.toString() || '50');
        setMaxDrawdown(response.config.riskCaps?.maxDrawdownPercent?.toString() || '10');
        setMaxTradesPerDay(response.config.riskCaps?.maxTradesPerDay?.toString() || '5');
        
        if (response.config.signalKnobs) {
          setVolumeConfirmation(response.config.signalKnobs.volumeConfirmation);
          setVolatilityBrake(response.config.signalKnobs.volatilityBrake);
          setLiquidityFilter(response.config.signalKnobs.liquidityFilter);
          setCorrelationBrake(response.config.signalKnobs.correlationBrake);
        }
      }
    } catch (error) {
      console.error('Failed to fetch bot config:', error);
      Alert.alert('Error', 'Failed to load bot configuration');
    } finally {
      setIsLoading(false);
    }
  }, [botId]);

  useEffect(() => {
    fetchBotConfig();
  }, [fetchBotConfig]);

  const handleSave = async () => {
    if (!name.trim()) {
      Alert.alert('Error', 'Please enter a bot name');
      return;
    }
    if (!llmApiKey.trim()) {
      Alert.alert('Error', 'Please enter your LLM API key');
      return;
    }

    setIsSaving(true);
    try {
      await api.bot.updateBotConfig(botId, {
        config: {
          name: name.trim(),
          persona,
          assetFocus,
          algorithmMode,
          strictness,
          riskCaps: {
            maxPositionSizePercent: parseInt(maxPositionSize) || 5,
            maxDailyLossUsd: parseInt(maxDailyLoss) || 50,
            maxDrawdownPercent: parseInt(maxDrawdown) || 10,
            maxTradesPerDay: parseInt(maxTradesPerDay) || 5,
          },
          tradingMode,
          llmProvider,
          llmApiKey: llmApiKey.trim(),
          signalKnobs: persona === 'quant-lite' ? {
            volumeConfirmation,
            volatilityBrake,
            liquidityFilter,
            correlationBrake,
          } : undefined,
        },
      });

      setHasChanges(false);
      Alert.alert(
        'Success', 
        'Configuration updated! The bot will receive the new config within 30 seconds.',
        [{ text: 'OK', onPress: () => navigation.goBack() }]
      );
    } catch (error) {
      console.error('Failed to save config:', error);
      Alert.alert('Error', 'Failed to save configuration. Please try again.');
    } finally {
      setIsSaving(false);
    }
  };

  const handleDiscard = () => {
    if (hasChanges) {
      Alert.alert(
        'Discard Changes?',
        'You have unsaved changes. Are you sure you want to discard them?',
        [
          { text: 'Cancel', style: 'cancel' },
          { text: 'Discard', style: 'destructive', onPress: () => navigation.goBack() },
        ]
      );
    } else {
      navigation.goBack();
    }
  };

  const onChange = <T, > setter: React.Dispatch<React.SetStateAction<T>>) => {
    return (value: T) => {
      setter(value);
      setHasChanges(true);
    };
  };

  if (isLoading) {
    return (
      <View style={styles.centered}>
        <ActivityIndicator size="large" />
      </View>
    );
  }

  const coreAssets = ASSET_FOCUSES.filter(a => a.tier === 'core');
  const qualityAssets = ASSET_FOCUSES.filter(a => a.tier === 'quality');
  const speculativeAssets = ASSET_FOCUSES.filter(a => a.tier === 'speculative');

  return (
    <ScrollView style={styles.container}>
      <View style={styles.header}>
        <Text style={styles.headerTitle}>Bot Settings</Text>
        <Text style={styles.headerSubtitle}>Changes apply within 30 seconds</Text>
        {bot?.configStatus === 'pending' && (
          <View style={styles.pendingBadge}>
            <Text style={styles.pendingText}>⏳ Config Update Pending</Text>
          </View>
        )}
      </View>

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Identity</Text>
        
        <Text style={styles.label}>Name</Text>
        <TextInput
          style={styles.input}
          value={name}
          onChangeText={onChange(setName)}
          placeholder="Bot name"
          editable={!isSaving}
        />
      </View>

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Trading Persona</Text>
        
        {PERSONAS.map((p) => (
          <TouchableOpacity
            key={p.value}
            style={[styles.optionCard, persona === p.value && styles.selectedCard]}
            onPress={() => !isSaving && onChange(setPersona)(p.value)}
            disabled={isSaving}
          >
            <Text style={styles.optionTitle}>{p.label}</Text>
            <Text style={styles.optionDescription}>{p.description}</Text>
          </TouchableOpacity>
        ))}
      </View>

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Asset Focus</Text>

        <Text style={styles.tierLabel}>Core Crypto</Text>
        <View style={styles.chipRow}>
          {coreAssets.map((af) => (
            <TouchableOpacity
              key={af.value}
              style={[styles.chip, assetFocus === af.value && styles.selectedChip]}
              onPress={() => !isSaving && onChange(setAssetFocus)(af.value)}
              disabled={isSaving}
            >
              <Text style={[styles.chipText, assetFocus === af.value && styles.selectedChipText]}>
                {af.label}
              </Text>
            </TouchableOpacity>
          ))}
        </View>

        <Text style={styles.tierLabel}>Tokenized Assets ⭐</Text>
        {qualityAssets.map((af) => (
          <TouchableOpacity
            key={af.value}
            style={[
              styles.qualityCard,
              assetFocus === af.value && styles.selectedQualityCard
            ]}
            onPress={() => !isSaving && onChange(setAssetFocus)(af.value)}
            disabled={isSaving}
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

        <View style={styles.speculativeSection}>
          <Text style={styles.speculativeLabel}>⚠️ Speculative</Text>
          {speculativeAssets.map((af) => (
            <TouchableOpacity
              key={af.value}
              style={[styles.speculativeCard, assetFocus === af.value && styles.selectedSpeculativeCard]}
              onPress={() => !isSaving && onChange(setAssetFocus)(af.value)}
              disabled={isSaving}
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
            onPress={() => !isSaving && onChange(setAlgorithmMode)(alg.value)}
            disabled={isSaving}
          >
            <Text style={styles.optionTitle}>{alg.label}</Text>
            <Text style={styles.optionDescription}>{alg.description}</Text>
          </TouchableOpacity>
        ))}
      </View>

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Risk Management</Text>
        
        <View style={styles.inputRow}>
          <View style={styles.inputGroup}>
            <Text style={styles.label}>Max Position %</Text>
            <TextInput
              style={styles.smallInput}
              value={maxPositionSize}
              onChangeText={onChange(setMaxPositionSize)}
              keyboardType="numeric"
              editable={!isSaving}
            />
          </View>
          
          <View style={styles.inputGroup}>
            <Text style={styles.label}>Max Daily Loss ($)</Text>
            <TextInput
              style={styles.smallInput}
              value={maxDailyLoss}
              onChangeText={onChange(setMaxDailyLoss)}
              keyboardType="numeric"
              editable={!isSaving}
            />
          </View>
        </View>

        <View style={styles.inputRow}>
          <View style={styles.inputGroup}>
            <Text style={styles.label}>Max Drawdown %</Text>
            <TextInput
              style={styles.smallInput}
              value={maxDrawdown}
              onChangeText={onChange(setMaxDrawdown)}
              keyboardType="numeric"
              editable={!isSaving}
            />
          </View>
          
          <View style={styles.inputGroup}>
            <Text style={styles.label}>Max Trades/Day</Text>
            <TextInput
              style={styles.smallInput}
              value={maxTradesPerDay}
              onChangeText={onChange(setMaxTradesPerDay)}
              keyboardType="numeric"
              editable={!isSaving}
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
            onValueChange={(v) => onChange(setTradingMode)(v ? 'live' : 'paper')}
            disabled={isSaving}
          />
        </View>
        
        {tradingMode === 'live' && (
          <Text style={styles.warning}>⚠️ Live trading uses real funds</Text>
        )}
      </View>

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>LLM Configuration</Text>
        
        <Text style={styles.label}>Provider</Text>
        <View style={styles.chipRow}>
          {(['openai', 'anthropic', 'venice', 'openrouter'] as LlmProvider[]).map((provider) => (
            <TouchableOpacity
              key={provider}
              style={[styles.chip, llmProvider === provider && styles.selectedChip]}
              onPress={() => !isSaving && onChange(setLlmProvider)(provider)}
              disabled={isSaving}
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
          onChangeText={onChange(setLlmApiKey)}
          placeholder="sk-..."
          secureTextEntry
          editable={!isSaving}
        />
      </View>

      {persona === 'quant-lite' && (
        <View style={styles.section}>
          <Text style={styles.sectionTitle}>Signal Knobs</Text>
          
          <View style={styles.switchRow}>
            <Text>Volume Confirmation</Text>
            <Switch value={volumeConfirmation} onValueChange={onChange(setVolumeConfirmation)} disabled={isSaving} />
          </View>
          
          <View style={styles.switchRow}>
            <Text>Volatility Brake</Text>
            <Switch value={volatilityBrake} onValueChange={onChange(setVolatilityBrake)} disabled={isSaving} />
          </View>
          
          <View style={styles.switchRow}>
            <Text>Correlation Brake</Text>
            <Switch value={correlationBrake} onValueChange={onChange(setCorrelationBrake)} disabled={isSaving} />
          </View>
          
          <Text style={styles.label}>Liquidity Filter</Text>
          <View style={styles.chipRow}>
            {(['low', 'medium', 'high'] as const).map((level) => (
              <TouchableOpacity
                key={level}
                style={[styles.chip, liquidityFilter === level && styles.selectedChip]}
                onPress={() => !isSaving && onChange(setLiquidityFilter)(level)}
                disabled={isSaving}
              >
                <Text style={[styles.chipText, liquidityFilter === level && styles.selectedChipText]}>
                  {level}
                </Text>
              </TouchableOpacity>
            ))}
          </View>
        </View>
      )}

      <View style={styles.actions}>
        {isSaving ? (
          <ActivityIndicator size="large" />
        ) : (
          <>
            <Button
              title="Save Changes"
              onPress={handleSave}
              disabled={!hasChanges}
            />
            <View style={styles.buttonSpacer} />
            
            <Button
              title="Discard Changes"
              onPress={handleDiscard}
              color="#666"
            />
          </>
        )}
      </View>
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#f5f5f5',
  },
  centered: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  header: {
    backgroundColor: '#fff',
    padding: 20,
    borderBottomWidth: 1,
    borderBottomColor: '#e0e0e0',
  },
  headerTitle: {
    fontSize: 24,
    fontWeight: 'bold',
  },
  headerSubtitle: {
    fontSize: 14,
    color: '#666',
    marginTop: 4,
  },
  pendingBadge: {
    backgroundColor: '#FFF3E0',
    paddingHorizontal: 12,
    paddingVertical: 6,
    borderRadius: 8,
    marginTop: 12,
    alignSelf: 'flex-start',
  },
  pendingText: {
    fontSize: 12,
    color: '#E65100',
    fontWeight: '600',
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
  tierLabel: {
    fontSize: 12,
    fontWeight: '700',
    color: '#666',
    marginTop: 12,
    marginBottom: 8,
    textTransform: 'uppercase',
  },
  chipRow: {
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
  qualityCard: {
    borderWidth: 2,
    borderColor: '#22c55e',
    borderRadius: 8,
    padding: 12,
    marginBottom: 8,
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
    marginBottom: 12,
  },
  warning: {
    color: '#F44336',
    marginTop: 8,
    fontSize: 14,
  },
  actions: {
    margin: 16,
    marginTop: 8,
    marginBottom: 32,
  },
  buttonSpacer: {
    height: 12,
  },
});
