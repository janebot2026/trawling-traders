import React, { useState, useEffect, useCallback, useRef } from 'react';
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
import { useNavigation, useRoute, RouteProp } from '@react-navigation/native';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';
import type { Bot, BotConfig, Persona, AlgorithmMode, AssetFocus, Strictness, TradingMode, LlmProvider } from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';
import { OceanBackground } from '../components/OceanBackground';
import { lightTheme } from '../theme';

type BotSettingsScreenRouteProp = RouteProp<RootStackParamList, 'BotSettings'>;
type BotSettingsScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'BotSettings'>;

const PERSONAS: { value: Persona; label: string; description: string }[] = [
  { value: 'beginner', label: 'Set & Forget', description: 'Blue-chip crypto + xStocks/metals' },
  { value: 'tweaker', label: 'Hands-on', description: 'Tune assets, risk controls' },
  { value: 'quant-lite', label: 'Power User', description: 'Signal knobs, full control' },
];

const ALGORITHMS: { value: AlgorithmMode; label: string; description: string }[] = [
  { value: 'trend', label: 'Trend', description: 'Ride momentum' },
  { value: 'mean-reversion', label: 'Mean Reversion', description: 'Fade extremes' },
  { value: 'breakout', label: 'Breakout', description: 'Volume breakouts' },
];

const ASSET_FOCUSES: { value: AssetFocus; label: string; description: string; tier: 'core' | 'quality' | 'speculative' }[] = [
  { value: 'majors', label: 'Crypto Majors', description: 'BTC, ETH, SOL', tier: 'core' },
  { value: 'tokenized-equities', label: 'xStocks', description: 'AAPL, TSLA, SPY', tier: 'quality' },
  { value: 'tokenized-metals', label: 'Metals', description: 'Gold, silver', tier: 'quality' },
  { value: 'custom', label: 'Custom', description: 'Your basket', tier: 'quality' },
  { value: 'memes', label: 'Memes ⚠️', description: 'High risk', tier: 'speculative' },
];

export function BotSettingsScreen() {
  const route = useRoute<BotSettingsScreenRouteProp>();
  const navigation = useNavigation<BotSettingsScreenNavigationProp>();
  const { botId } = route.params;

  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);
  const [bot, setBot] = useState<Bot | null>(null);

  // Animation
  const fadeAnim = useRef(new Animated.Value(0)).current;

  // Bot state
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

  const fetchBotConfig = useCallback(async () => {
    try {
      const response = await api.bot.getBot(botId);
      setBot(response.bot);
      
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
      Alert.alert('Error', 'Failed to load configuration');
    } finally {
      setIsLoading(false);
      Animated.timing(fadeAnim, { toValue: 1, duration: 400, useNativeDriver: true }).start();
    }
  }, [botId, fadeAnim]);

  useEffect(() => {
    fetchBotConfig();
  }, [fetchBotConfig]);

  const handleSave = async () => {
    if (!name.trim()) {
      Alert.alert('Error', 'Please enter a bot name');
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
      Alert.alert('Success', 'Configuration updated!');
      navigation.goBack();
    } catch (error) {
      Alert.alert('Error', 'Failed to save configuration');
    } finally {
      setIsSaving(false);
    }
  };

  const handleDiscard = () => {
    if (hasChanges) {
      Alert.alert('Discard Changes?', 'Unsaved changes will be lost.', [
        { text: 'Cancel', style: 'cancel' },
        { text: 'Discard', style: 'destructive', onPress: () => navigation.goBack() },
      ]);
    } else {
      navigation.goBack();
    }
  };

  const onChange = <T, >(setter: React.Dispatch<React.SetStateAction<T>>) => {
    return (value: T) => {
      setter(value);
      setHasChanges(true);
    };
  };

  const renderSection = (title: string, children: React.ReactNode) => (
    <View style={styles.card}>
      <Text style={styles.sectionTitle}>{title}</Text>
      {children}
    </View>
  );

  if (isLoading) {
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
      <Animated.ScrollView
        style={[styles.container, { opacity: fadeAnim }]}
        contentContainerStyle={{ paddingBottom: 40 }}
      >
        {/* Header */}
        <View style={styles.header}>
          <Text style={styles.headerTitle}>Bot Settings</Text>
          <Text style={styles.headerSubtitle}>Changes apply within 30 seconds</Text>
          {bot?.configStatus === 'pending' && (
            <View style={styles.pendingBadge}>
              <Text style={styles.pendingText}>⏳ Config Update Pending</Text>
            </View>
          )}
        </View>

        {renderSection('Identity',
          <>
            <Text style={styles.label}>Name</Text>
            <TextInput
              style={styles.input}
              value={name}
              onChangeText={onChange(setName)}
              placeholder="Bot name"
              placeholderTextColor={lightTheme.colors.wave[400]}
            />
          </>
        )}

        {renderSection('Persona',
          <>
            {PERSONAS.map((p) => (
              <TouchableOpacity
                key={p.value}
                style={[styles.optionCard, persona === p.value && styles.selectedCard]}
                onPress={() => onChange(setPersona)(p.value)}
              >
                <Text style={styles.optionTitle}>{p.label}</Text>
                <Text style={styles.optionDescription}>{p.description}</Text>
              </TouchableOpacity>
            ))}
          </>
        )}

        {renderSection('Assets',
          <>
            <View style={styles.assetRow}>
              {ASSET_FOCUSES.filter(a => a.tier !== 'speculative').map((af) => (
                <TouchableOpacity
                  key={af.value}
                  style={[
                    styles.assetChip,
                    assetFocus === af.value && styles.assetChipSelected,
                  ]}
                  onPress={() => onChange(setAssetFocus)(af.value)}
                >
                  <Text style={[styles.assetChipText, assetFocus === af.value && styles.assetChipTextSelected]}>
                    {af.label}
                  </Text>
                </TouchableOpacity>
              ))}
            </View>

            <View style={styles.speculativeSection}>
              <Text style={styles.speculativeLabel}>⚠️ Speculative</Text>
              <TouchableOpacity
                style={[styles.assetChip, assetFocus === 'memes' && styles.assetChipSpecSelected]}
                onPress={() => onChange(setAssetFocus)('memes')}
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
                onPress={() => onChange(setAlgorithmMode)(alg.value)}
              >
                <Text style={styles.optionTitle}>{alg.label}</Text>
                <Text style={styles.optionDescription}>{alg.description}</Text>
              </TouchableOpacity>
            ))}
          </>
        )}

        {renderSection('Risk',
          <>
            <View style={styles.inputRow}>
              <View style={styles.inputGroup}>
                <Text style={styles.label}>Max Position %</Text>
                <TextInput
                  style={styles.smallInput}
                  value={maxPositionSize}
                  onChangeText={onChange(setMaxPositionSize)}
                  keyboardType="numeric"
                />
              </View>
              <View style={styles.inputGroup}>
                <Text style={styles.label}>Daily Loss ($)</Text>
                <TextInput
                  style={styles.smallInput}
                  value={maxDailyLoss}
                  onChangeText={onChange(setMaxDailyLoss)}
                  keyboardType="numeric"
                />
              </View>
            </View>
            <View style={styles.inputRow}>
              <View style={styles.inputGroup}>
                <Text style={styles.label}>Drawdown %</Text>
                <TextInput
                  style={styles.smallInput}
                  value={maxDrawdown}
                  onChangeText={onChange(setMaxDrawdown)}
                  keyboardType="numeric"
                />
              </View>
              <View style={styles.inputGroup}>
                <Text style={styles.label}>Trades/Day</Text>
                <TextInput
                  style={styles.smallInput}
                  value={maxTradesPerDay}
                  onChangeText={onChange(setMaxTradesPerDay)}
                  keyboardType="numeric"
                />
              </View>
            </View>
          </>
        )}

        {renderSection('Trading',
          <>
            <View style={styles.switchRow}>
              <View>
                <Text style={styles.switchLabel}>
                  {tradingMode === 'paper' ? '📄 Paper' : '💰 Live'}
                </Text>
                <Text style={styles.switchSub}>
                  {tradingMode === 'paper' ? 'Test mode' : 'Real funds'}
                </Text>
              </View>
              <Switch
                value={tradingMode === 'live'}
                onValueChange={(v) => onChange(setTradingMode)(v ? 'live' : 'paper')}
                trackColor={{ false: lightTheme.colors.wave[300], true: lightTheme.colors.lobster[400] }}
              />
            </View>
            
            {tradingMode === 'live' && (
              <View style={styles.warningBox}>
                <Text style={styles.warningText}>⚠️ Live trading uses real funds</Text>
              </View>
            )}
          </>
        )}

        {renderSection('LLM',
          <>
            <Text style={styles.label}>Provider</Text>
            <View style={styles.providerRow}>
              {(['openai', 'anthropic', 'venice', 'openrouter'] as LlmProvider[]).map((p) => (
                <TouchableOpacity
                  key={p}
                  style={[styles.providerChip, llmProvider === p && styles.providerChipSelected]}
                  onPress={() => onChange(setLlmProvider)(p)}
                >
                  <Text style={[styles.providerText, llmProvider === p && styles.providerTextSelected]}>{p}</Text>
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
            />
          </>
        )}

        {persona === 'quant-lite' && renderSection('Signal Knobs',
          <>
            {[
              { label: 'Volume Confirmation', value: volumeConfirmation, setter: setVolumeConfirmation },
              { label: 'Volatility Brake', value: volatilityBrake, setter: setVolatilityBrake },
              { label: 'Correlation Brake', value: correlationBrake, setter: setCorrelationBrake },
            ].map((k) => (
              <View key={k.label} style={styles.switchRow}>
                <Text style={styles.switchLabel}>{k.label}</Text>
                <Switch value={k.value} onValueChange={k.setter} />
              </View>
            ))}
            
            <Text style={styles.label}>Liquidity</Text>
            <View style={styles.providerRow}>
              {(['low', 'medium', 'high'] as const).map((l) => (
                <TouchableOpacity
                  key={l}
                  style={[styles.providerChip, liquidityFilter === l && styles.providerChipSelected]}
                  onPress={() => onChange(setLiquidityFilter)(l)}
                >
                  <Text style={[styles.providerText, liquidityFilter === l && styles.providerTextSelected]}>{l}</Text>
                </TouchableOpacity>
              ))}
            </View>
          </>
        )}

        {/* Actions */}
        <View style={styles.card}>
          {isSaving ? (
            <ActivityIndicator size="large" color={lightTheme.colors.primary[700]} />
          ) : (
            <>
              <TouchableOpacity
                style={styles.saveButton}
                onPress={handleSave}
                disabled={!hasChanges}
              >
                <Text style={styles.saveButtonText}>Save Changes</Text>
              </TouchableOpacity>
              
              <TouchableOpacity
                style={styles.discardButton}
                onPress={handleDiscard}
              >
                <Text style={styles.discardButtonText}>Discard</Text>
              </TouchableOpacity>
            </>
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
  centered: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
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
    fontSize: 14,
    color: lightTheme.colors.wave[500],
    marginTop: 4,
  },
  pendingBadge: {
    backgroundColor: lightTheme.colors.caution[100],
    paddingHorizontal: 12,
    paddingVertical: 6,
    borderRadius: 8,
    marginTop: 12,
    alignSelf: 'flex-start',
  },
  pendingText: {
    fontSize: 12,
    color: lightTheme.colors.caution[700],
    fontWeight: '600',
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
  assetRow: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 10,
  },
  assetChip: {
    borderWidth: 2,
    borderColor: lightTheme.colors.wave[200],
    borderRadius: 10,
    paddingHorizontal: 14,
    paddingVertical: 8,
    backgroundColor: lightTheme.colors.wave[50],
  },
  assetChipSelected: {
    backgroundColor: lightTheme.colors.bullish[50],
    borderColor: lightTheme.colors.bullish[500],
  },
  assetChipSpecSelected: {
    backgroundColor: lightTheme.colors.lobster[50],
    borderColor: lightTheme.colors.lobster[400],
  },
  assetChipText: {
    fontSize: 14,
    fontWeight: '600',
    color: lightTheme.colors.wave[700],
  },
  assetChipTextSelected: {
    color: lightTheme.colors.bullish[700],
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
  saveButton: {
    backgroundColor: lightTheme.colors.primary[700],
    paddingVertical: 16,
    borderRadius: 12,
    alignItems: 'center',
    marginBottom: 12,
  },
  saveButtonText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: 'bold',
  },
  discardButton: {
    backgroundColor: lightTheme.colors.wave[200],
    paddingVertical: 14,
    borderRadius: 12,
    alignItems: 'center',
  },
  discardButtonText: {
    color: lightTheme.colors.wave[700],
    fontSize: 16,
  },
});
