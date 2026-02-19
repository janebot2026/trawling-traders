import React, { useEffect, useMemo, useRef, useState } from 'react';
import {
  ActivityIndicator,
  Alert,
  ImageBackground,
  ScrollView,
  Text,
  TouchableOpacity,
  useColorScheme,
  View,
} from 'react-native';
import { useNavigation } from '@react-navigation/native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';
import type { NativeStackNavigationProp } from '@react-navigation/native-stack';
import type { RootStackParamList } from '../navigation/AppNavigator';
import type {
  NameAvailability,
  AlgorithmFactor,
  AlgorithmMode,
  AssetFocus,
  AIAssistantOption,
  LlmModel,
  LlmProvider,
  Persona,
  Strictness,
  TradeableAsset,
  TradingMode,
} from '@trawling-traders/types';
import { api } from '@trawling-traders/api-client';
import { CreateBotWizardSteps } from './create-bot/CreateBotWizardSteps';
import { createBotWizardStyles as styles } from './create-bot/CreateBotWizard.styles';

const SKY_LIGHT = require('../../../../assets/branding/tt-sky-light.png');
const SKY_DARK = require('../../../../assets/branding/tt-sky-dark.png');

type CreateBotScreenNavigationProp = NativeStackNavigationProp<RootStackParamList, 'CreateBot'>;
type WizardStep = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8;
type StrategyType = 'macro' | 'event-driven' | 'smart-money' | 'range';

const STEP_META = [
  { title: 'Basics', description: 'Name your boat and decide if you want paper-only testing.' },
  { title: 'Choose your captain', description: 'Pick the AI captain personality that will command this vessel.' },
  { title: 'Specialty', description: 'Pick a category, then choose the exact assets this boat is allowed to trade.' },
  { title: 'Strategy', description: 'Choose the trading style this boat should apply to your selected assets.' },
  { title: 'Algorithm', description: 'Build your signal formula. Full controls are currently available for Macro strategy.' },
  { title: 'Risk', description: 'Set caps and strictness for how cautiously signals are executed.' },
  { title: 'AI', description: 'Connect the LLM provider and model your boat will use.' },
  { title: 'Telegram', description: 'Optional chat channel for commands, alerts, and pairing.' },
  { title: 'Review', description: 'Double-check configuration and deploy.' },
] as const;

const FACTOR_CATALOG = [
  { key: 'price_momentum', label: 'Price Momentum' },
  { key: 'volume_confirmation', label: 'Volume Confirmation' },
  { key: 'volatility_regime', label: 'Volatility Regime' },
  { key: 'rsi_reversion', label: 'RSI Reversion' },
  { key: 'market_breadth', label: 'Market Breadth' },
  { key: 'news_sentiment', label: 'News Sentiment' },
] as const;

const NAME_ADJECTIVES = [
  'fast',
  'steady',
  'deep',
  'lucky',
  'silent',
  'rugged',
  'swift',
  'keen',
  'mighty',
  'bright',
  'bold',
  'northbound',
  'brisk',
  'stormproof',
  'tireless',
  'fearless',
  'nimble',
  'iron',
  'coastal',
  'offshore',
  'salty',
  'hardy',
  'tidal',
  'driven',
  'resolute',
  'calm',
  'fleet',
  'mariner',
  'farsight',
  'anchored',
  'legendary',
  'starlit',
];

const NAME_WATERS = [
  'atlantic',
  'pacific',
  'river',
  'harbor',
  'delta',
  'inlet',
  'bay',
  'sound',
  'estuary',
  'strait',
  'lagoon',
  'channel',
  'gulf',
  'ocean',
  'sea',
  'fjord',
  'reef',
  'shoal',
  'current',
  'tideway',
  'cove',
  'passage',
  'waterway',
  'basin',
  'bayou',
  'marsh',
  'headwater',
  'wake',
  'undertow',
  'breakwater',
  'narrows',
  'backwater',
];

const NAME_BOATS = [
  'trawler',
  'skiff',
  'schooner',
  'drifter',
  'paddleboat',
  'longliner',
  'cutter',
  'seiner',
  'clipper',
  'raft',
  'dinghy',
  'ferry',
  'catamaran',
  'yawl',
  'ketch',
  'tender',
  'launch',
  'whaler',
  'sloop',
  'brig',
  'barque',
  'canoe',
  'kayak',
  'rowboat',
  'towboat',
  'luggers',
  'dredger',
  'gunwale',
  'coaster',
  'workboat',
  'pilotboat',
  'lifeboat',
];

const ASSET_CHOICES: { value: AssetFocus; label: string; recommended?: boolean }[] = [
  { value: 'tokenized_equities', label: 'Stocks' },
  { value: 'tokenized_metals', label: 'Commodities' },
  { value: 'majors', label: 'Crypto Majors' },
  { value: 'finance_2', label: 'Finance 2.0' },
  { value: 'memes', label: 'Memecoins' },
];

const STRICTNESS_OPTIONS: { value: Strictness; label: string }[] = [
  { value: 'high', label: 'High (Recommended)' },
  { value: 'medium', label: 'Medium' },
  { value: 'low', label: 'Low' },
];

const STRATEGY_OPTIONS: { value: StrategyType; label: string }[] = [
  { value: 'macro', label: 'Macro' },
  { value: 'event-driven', label: 'Event Driven' },
  { value: 'smart-money', label: 'Smart Money' },
  { value: 'range', label: 'Range' },
];

const LLM_MODELS: Record<LlmProvider, { value: LlmModel; label: string }[]> = {
  openai: [
    { value: 'gpt-4o', label: 'GPT-4o (Recommended)' },
    { value: 'gpt-4o-mini', label: 'GPT-4o Mini' },
    { value: 'gpt-4-turbo', label: 'GPT-4 Turbo' },
  ],
  anthropic: [
    { value: 'claude-3-5-sonnet', label: 'Claude 3.5 Sonnet (Recommended)' },
    { value: 'claude-3-opus', label: 'Claude 3 Opus' },
    { value: 'claude-3-haiku', label: 'Claude 3 Haiku' },
  ],
  venice: [{ value: 'llama-3.1-405b', label: 'Llama 3.1 405B' }],
  openrouter: [{ value: 'auto', label: 'Auto (Best Available)' }],
};

const FALLBACK_ASSISTANT_OPTIONS: AIAssistantOption[] = [
  {
    id: 'fallback-beginner',
    assistantStyle: 'beginner',
    captainName: 'Captain Current',
    personalityDescription:
      'Calm and practical. Explains decisions clearly, protects downside first, and keeps the crew focused on disciplined entries.',
    imageKey: 'trader',
    imagePath: '/assets/branding/tt-trader-captain.png',
    sortOrder: 1,
    isActive: true,
  },
  {
    id: 'fallback-tweaker',
    assistantStyle: 'tweaker',
    captainName: 'Captain Helm',
    personalityDescription:
      'Hands-on and tactical. Watches momentum shifts closely, adjusts quickly, and gives direct, execution-focused guidance.',
    imageKey: 'sea',
    imagePath: '/assets/branding/tt-sea-captain.png',
    sortOrder: 2,
    isActive: true,
  },
  {
    id: 'fallback-quant_lite',
    assistantStyle: 'quant_lite',
    captainName: 'Rocky Reef',
    personalityDescription:
      'Signal-driven and analytical. Tracks structure, validates with data, and avoids emotional overtrading during turbulence.',
    imageKey: 'rocky',
    imagePath: '/assets/branding/tt-rocky-captain.png',
    sortOrder: 3,
    isActive: true,
  },
];

function generateFishingName(): string {
  const adjective = NAME_ADJECTIVES[Math.floor(Math.random() * NAME_ADJECTIVES.length)];
  const water = NAME_WATERS[Math.floor(Math.random() * NAME_WATERS.length)];
  const boat = NAME_BOATS[Math.floor(Math.random() * NAME_BOATS.length)];
  return [adjective, water, boat]
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

function sanitizeBoatNameInput(value: string): string {
  const lettersNumbersSpacesOnly = value.replace(/[^a-zA-Z0-9 ]+/g, '');
  return lettersNumbersSpacesOnly.replace(/\s+/g, ' ').trimStart();
}

function displayNameFromNormalized(value: string): string {
  return value
    .split('-')
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

function parseNumberField(value: string, label: string, min: number, max: number): { value: number; error?: string } {
  const trimmed = value.trim();
  const parsed = Number.parseInt(trimmed, 10);
  if (!trimmed) return { value: 0, error: `${label} is required.` };
  if (Number.isNaN(parsed)) return { value: 0, error: `${label} must be a number.` };
  if (parsed < min || parsed > max) return { value: parsed, error: `${label} must be ${min}-${max}.` };
  return { value: parsed };
}

export function CreateBotScreen() {
  const navigation = useNavigation<CreateBotScreenNavigationProp>();
  const insets = useSafeAreaInsets();
  const colorScheme = useColorScheme();
  const skyBg = colorScheme === 'dark' ? SKY_DARK : SKY_LIGHT;
  const headerOffset = insets.top + 56;

  const [step, setStep] = useState<WizardStep>(0);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [inlineError, setInlineError] = useState<string | null>(null);
  const [nameAvailability, setNameAvailability] = useState<NameAvailability | null>(null);
  const [nameCheckLoading, setNameCheckLoading] = useState(false);

  const [name, setName] = useState(generateFishingName);
  const userEditedName = useRef(false);
  const [assetFocus, setAssetFocus] = useState<AssetFocus>('tokenized_equities');
  const [assistantStyle, setAssistantStyle] = useState<Persona>('beginner');
  const [assistantOptions, setAssistantOptions] = useState<AIAssistantOption[]>([]);
  const [assistantOptionsLoading, setAssistantOptionsLoading] = useState(false);
  const [algorithmFactors, setAlgorithmFactors] = useState<AlgorithmFactor[]>([
    { factor: 'price_momentum', weight: 0.4 },
    { factor: 'volume_confirmation', weight: 0.25 },
    { factor: 'volatility_regime', weight: 0.35 },
  ]);
  const [strictness, setStrictness] = useState<Strictness>('high');
  const [tradingMode, setTradingMode] = useState<TradingMode>('live');
  const [tradeableAssets, setTradeableAssets] = useState<TradeableAsset[]>([]);
  const [selectedAssets, setSelectedAssets] = useState<string[]>([]);
  const [assetsLoading, setAssetsLoading] = useState(false);
  const [strategyType, setStrategyType] = useState<StrategyType>('macro');
  const [maxPositionSize, setMaxPositionSize] = useState('5');
  const [maxDailyLoss, setMaxDailyLoss] = useState('50');
  const [maxDrawdown, setMaxDrawdown] = useState('10');
  const [maxTradesPerDay, setMaxTradesPerDay] = useState('5');
  const [llmProvider, setLlmProvider] = useState<LlmProvider>('openai');
  const [llmModel, setLlmModel] = useState<LlmModel>('gpt-4o');
  const [llmApiKey, setLlmApiKey] = useState('');
  const [telegramEnabled, setTelegramEnabled] = useState(false);
  const [telegramBotToken, setTelegramBotToken] = useState('');
  const [telegramUserId, setTelegramUserId] = useState('');
  const [telegramPairingCode, setTelegramPairingCode] = useState('');

  const algorithmMode = useMemo<AlgorithmMode>(() => {
    switch (strategyType) {
      case 'macro':
        return 'trend';
      case 'event-driven':
        return 'breakout';
      case 'smart-money':
        return 'mean_reversion';
      case 'range':
        return 'mean_reversion';
      default:
        return 'trend';
    }
  }, [strategyType]);

  const modelsForProvider = useMemo(() => LLM_MODELS[llmProvider], [llmProvider]);

  // generateFishingName is defined at module scope (above component)

  // The debounced name-check effect (below) handles initial validation.
  // No separate generation effect needed — name is seeded via useState init.

  useEffect(() => {
    let cancelled = false;

    setAssetsLoading(true);
    api.bot.listTradeableAssets().then(
      (assets) => {
        if (!cancelled) {
          setTradeableAssets(assets);
          setAssetsLoading(false);
        }
      },
      () => {
        if (!cancelled) {
          setTradeableAssets([]);
          setAssetsLoading(false);
        }
      }
    );

    setAssistantOptionsLoading(true);
    api.bot.listAssistantOptions().then(
      (options) => {
        if (!cancelled) {
          if (options.length > 0) {
            setAssistantOptions(options);
            setAssistantStyle(options[0].assistantStyle);
          } else {
            setAssistantOptions(FALLBACK_ASSISTANT_OPTIONS);
            setAssistantStyle(FALLBACK_ASSISTANT_OPTIONS[0].assistantStyle);
          }
          setAssistantOptionsLoading(false);
        }
      },
      () => {
        if (!cancelled) {
          setAssistantOptions(FALLBACK_ASSISTANT_OPTIONS);
          setAssistantStyle(FALLBACK_ASSISTANT_OPTIONS[0].assistantStyle);
          setAssistantOptionsLoading(false);
        }
      }
    );

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const currentFocusAssets = new Set(
      tradeableAssets
        .filter((asset) => asset.assetFocus === assetFocus)
        .map((asset) => asset.tokenAddress)
    );
    setSelectedAssets((prev) => prev.filter((token) => currentFocusAssets.has(token)));
  }, [assetFocus, tradeableAssets]);

  useEffect(() => {
    let cancelled = false;
    if (name.trim().length === 0) {
      setNameAvailability(null);
      return;
    }
    setNameCheckLoading(true);
    const delay = userEditedName.current ? 250 : 0;
    const timeout = setTimeout(async () => {
      try {
        const response = await api.bot.checkNameAvailability(name.trim());
        if (cancelled) return;
        setNameAvailability(response);
        // Auto-regenerate on collision only for auto-generated names
        if (!response.available && !userEditedName.current) {
          const next = response.suggestedName ?? generateFishingName();
          setName(displayNameFromNormalized(next));
        }
      } catch {
        if (!cancelled) {
          setNameAvailability(null);
        }
      } finally {
        if (!cancelled) {
          setNameCheckLoading(false);
        }
      }
    }, delay);
    return () => {
      cancelled = true;
      clearTimeout(timeout);
    };
  }, [name]);

  const validateCurrentStep = () => {
    if (step === 0 && !name.trim()) {
      return 'Please give this boat a name.';
    }
    if (step === 2) {
      if (assetsLoading) return 'Loading assets for this category. Please wait a moment.';
      if (selectedAssets.length === 0) return 'Select at least one asset to trade.';
    }
    if (step === 5) {
      const checks = [
        parseNumberField(maxPositionSize, 'Max position %', 1, 50),
        parseNumberField(maxDailyLoss, 'Max daily loss', 1, 100000),
        parseNumberField(maxDrawdown, 'Max drawdown %', 1, 50),
        parseNumberField(maxTradesPerDay, 'Max trades/day', 1, 100),
      ];
      const failed = checks.find((check) => check.error);
      if (failed?.error) return failed.error;
    }
    if (step === 4 && strategyType === 'macro' && algorithmFactors.length === 0) {
      return 'Add at least one algorithm factor.';
    }
    if (step === 6) {
      if (!llmApiKey.trim()) return 'Enter your LLM API key to continue.';
    }
    if (step === 7) {
      if (telegramEnabled && !telegramBotToken.trim()) return 'Enter a Telegram token or disable Telegram.';
      if (telegramEnabled && !telegramUserId.trim()) return 'Enter your Telegram user ID.';
      if (telegramEnabled && !telegramPairingCode.trim()) return 'Enter your Telegram pairing code.';
    }
    return null;
  };

  const onNext = () => {
    setInlineError(null);
    const error = validateCurrentStep();
    if (error) {
      setInlineError(error);
      return;
    }
    if (step < 8) {
      setStep((prev) => (prev + 1) as WizardStep);
    }
  };

  const onBack = () => {
    setInlineError(null);
    if (step > 0) {
      setStep((prev) => (prev - 1) as WizardStep);
    } else {
      navigation.goBack();
    }
  };

  const deployBot = async () => {
    setInlineError(null);
    const error = validateCurrentStep();
    if (error) {
      setInlineError(error);
      return;
    }

    const position = parseNumberField(maxPositionSize, 'Max position %', 1, 50);
    const dailyLoss = parseNumberField(maxDailyLoss, 'Max daily loss', 1, 100000);
    const drawdown = parseNumberField(maxDrawdown, 'Max drawdown %', 1, 50);
    const trades = parseNumberField(maxTradesPerDay, 'Max trades/day', 1, 100);

    if (position.error || dailyLoss.error || drawdown.error || trades.error) {
      setInlineError(position.error || dailyLoss.error || drawdown.error || trades.error || null);
      return;
    }

    setIsSubmitting(true);
    try {
      await api.bot.createBot({
        name: name.trim(),
        assistantStyle,
        assetFocus,
        customAssets: selectedAssets,
        algorithmMode,
        algorithmFactors,
        strictness,
        tradingMode,
        llmProvider,
        llmModel,
        llmApiKey: llmApiKey.trim(),
        telegramEnabled,
        telegramBotToken: telegramEnabled ? telegramBotToken.trim() : undefined,
        riskCaps: {
          maxPositionSizePercent: position.value,
          maxDailyLossUsd: dailyLoss.value,
          maxDrawdownPercent: drawdown.value,
          maxTradesPerDay: trades.value,
        },
      });
      setLlmApiKey('');
      setTelegramBotToken('');
      setTelegramUserId('');
      setTelegramPairingCode('');
      Alert.alert('Boat deployed', 'Your trawler is being provisioned now.');
      navigation.goBack();
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Failed to deploy boat.';
      setInlineError(message);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <ImageBackground source={skyBg} style={styles.container} resizeMode="cover">
      <ScrollView contentContainerStyle={[styles.content, { paddingTop: headerOffset }]}>
        <Text style={styles.headerSubtitle}>Guided setup for a safer, clearer launch.</Text>
        <View style={styles.progressRow}>
          {STEP_META.map((_, index) => (
            <View key={index} style={[styles.progressDot, index <= step && styles.progressDotActive]} />
          ))}
        </View>

        <View style={styles.card}>
          <Text style={styles.stepLabel}>
            Step {step + 1} of {STEP_META.length}
          </Text>
          <Text style={styles.stepTitle}>{STEP_META[step].title}</Text>
          <Text style={styles.stepDescription}>{STEP_META[step].description}</Text>
          {inlineError ? <Text style={styles.inlineError}>{inlineError}</Text> : null}
          <CreateBotWizardSteps
            step={step}
            name={name}
            setName={(v: string) => { userEditedName.current = true; setName(sanitizeBoatNameInput(v)); }}
            nameAvailability={nameAvailability}
            nameCheckLoading={nameCheckLoading}
            assistantStyle={assistantStyle}
            setAssistantStyle={setAssistantStyle}
            assistantOptions={assistantOptions}
            assistantOptionsLoading={assistantOptionsLoading}
            assetChoices={ASSET_CHOICES}
            assetFocus={assetFocus}
            setAssetFocus={setAssetFocus}
            tradeableAssets={tradeableAssets}
            selectedAssets={selectedAssets}
            setSelectedAssets={setSelectedAssets}
            assetsLoading={assetsLoading}
            strategyOptions={STRATEGY_OPTIONS}
            strategyType={strategyType}
            setStrategyType={setStrategyType}
            algorithmMode={algorithmMode}
            strictnessOptions={STRICTNESS_OPTIONS}
            strictness={strictness}
            setStrictness={setStrictness}
            factorCatalog={FACTOR_CATALOG.map((item) => ({ value: item.key, label: item.label }))}
            algorithmFactors={algorithmFactors}
            setAlgorithmFactors={setAlgorithmFactors}
            tradingMode={tradingMode}
            setTradingMode={setTradingMode}
            maxPositionSize={maxPositionSize}
            setMaxPositionSize={setMaxPositionSize}
            maxTradesPerDay={maxTradesPerDay}
            setMaxTradesPerDay={setMaxTradesPerDay}
            maxDailyLoss={maxDailyLoss}
            setMaxDailyLoss={setMaxDailyLoss}
            maxDrawdown={maxDrawdown}
            setMaxDrawdown={setMaxDrawdown}
            llmProvider={llmProvider}
            setLlmProvider={setLlmProvider}
            llmModel={llmModel}
            setLlmModel={setLlmModel}
            llmApiKey={llmApiKey}
            setLlmApiKey={setLlmApiKey}
            modelsForProvider={modelsForProvider}
            llmModels={LLM_MODELS}
            telegramEnabled={telegramEnabled}
            setTelegramEnabled={setTelegramEnabled}
            telegramBotToken={telegramBotToken}
            setTelegramBotToken={setTelegramBotToken}
            telegramUserId={telegramUserId}
            setTelegramUserId={setTelegramUserId}
            telegramPairingCode={telegramPairingCode}
            setTelegramPairingCode={setTelegramPairingCode}
          />
          <View style={styles.footerRow}>
            <TouchableOpacity style={styles.backButton} onPress={onBack} disabled={isSubmitting}>
              <Text style={styles.backButtonText}>{step === 0 ? 'Cancel' : 'Back'}</Text>
            </TouchableOpacity>
            {step < 8 ? (
              <TouchableOpacity style={styles.nextButton} onPress={onNext} disabled={isSubmitting}>
                <Text style={styles.nextButtonText}>Next</Text>
              </TouchableOpacity>
            ) : (
              <TouchableOpacity
                style={[styles.nextButton, isSubmitting && styles.nextButtonDisabled]}
                onPress={deployBot}
                disabled={isSubmitting}
              >
                {isSubmitting ? (
                  <ActivityIndicator size="small" color="#fff" />
                ) : (
                  <Text style={styles.nextButtonText}>Deploy Boat</Text>
                )}
              </TouchableOpacity>
            )}
          </View>
        </View>
      </ScrollView>
    </ImageBackground>
  );
}
