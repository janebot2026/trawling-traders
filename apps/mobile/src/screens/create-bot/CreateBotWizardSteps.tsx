import React, { useEffect, useMemo, useRef, useState } from 'react';
import {
  ActivityIndicator,
  Image,
  Modal,
  ScrollView,
  Switch,
  Text,
  TextInput,
  TouchableOpacity,
  View,
  type NativeSyntheticEvent,
  type NativeScrollEvent,
} from 'react-native';
import { Ionicons } from '@expo/vector-icons';
import type {
  AlgorithmFactor,
  AlgorithmMode,
  AIAssistantOption,
  AssetFocus,
  LlmModel,
  LlmProvider,
  NameAvailability,
  Persona,
  Strictness,
  TradeableAsset,
  TradingMode,
} from '@trawling-traders/types';
import { useSettingsStore } from '../../store';
import { lightTheme } from '../../theme';
import { createBotWizardStyles as styles } from './CreateBotWizard.styles';

export type WizardStep = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8;
type StrategyType = 'macro' | 'event-driven' | 'smart-money' | 'range';

type Option<T extends string> = {
  value: T;
  label: string;
  description?: string;
  recommended?: boolean;
};

type CreateBotWizardStepsProps = {
  step: WizardStep;
  name: string;
  setName: (value: string) => void;
  nameAvailability: NameAvailability | null;
  nameCheckLoading: boolean;
  assistantStyle: Persona;
  setAssistantStyle: (value: Persona) => void;
  assistantOptions: AIAssistantOption[];
  assistantOptionsLoading: boolean;
  assetFocus: AssetFocus;
  setAssetFocus: (value: AssetFocus) => void;
  tradeableAssets: TradeableAsset[];
  selectedAssets: string[];
  setSelectedAssets: (value: string[]) => void;
  assetsLoading: boolean;
  strategyOptions: Option<StrategyType>[];
  strategyType: StrategyType;
  setStrategyType: (value: StrategyType) => void;
  algorithmMode: AlgorithmMode;
  strictness: Strictness;
  setStrictness: (value: Strictness) => void;
  factorCatalog: Option<string>[];
  algorithmFactors: AlgorithmFactor[];
  setAlgorithmFactors: (value: AlgorithmFactor[]) => void;
  tradingMode: TradingMode;
  setTradingMode: (value: TradingMode) => void;
  maxPositionSize: string;
  setMaxPositionSize: (value: string) => void;
  maxTradesPerDay: string;
  setMaxTradesPerDay: (value: string) => void;
  maxDailyLoss: string;
  setMaxDailyLoss: (value: string) => void;
  maxDrawdown: string;
  setMaxDrawdown: (value: string) => void;
  llmProvider: LlmProvider;
  setLlmProvider: (value: LlmProvider) => void;
  llmModel: LlmModel;
  setLlmModel: (value: LlmModel) => void;
  llmApiKey: string;
  setLlmApiKey: (value: string) => void;
  telegramEnabled: boolean;
  setTelegramEnabled: (value: boolean) => void;
  telegramBotToken: string;
  setTelegramBotToken: (value: string) => void;
  telegramUserId: string;
  setTelegramUserId: (value: string) => void;
  telegramPairingCode: string;
  setTelegramPairingCode: (value: string) => void;
  modelsForProvider: { value: LlmModel; label: string }[];
  llmModels: Record<LlmProvider, { value: LlmModel; label: string }[]>;
  assetChoices: Option<AssetFocus>[];
  strictnessOptions: Option<Strictness>[];
};

const SUBSCRIPT_DIGITS: Record<string, string> = {
  '0': '₀',
  '1': '₁',
  '2': '₂',
  '3': '₃',
  '4': '₄',
  '5': '₅',
  '6': '₆',
  '7': '₇',
  '8': '₈',
  '9': '₉',
};

function toSubscript(value: number): string {
  return String(value)
    .split('')
    .map((char) => SUBSCRIPT_DIGITS[char] || char)
    .join('');
}

function renderChip<T extends string>(
  options: Option<T>[],
  selected: T,
  onSelect: (value: T) => void
) {
  return (
    <View style={styles.chipRow}>
      {options.map((item) => (
        <TouchableOpacity
          key={item.value}
          style={[styles.chip, selected === item.value && styles.chipActive]}
          onPress={() => onSelect(item.value)}
        >
          <Text style={[styles.chipText, selected === item.value && styles.chipTextActive]}>
            {item.label}
          </Text>
        </TouchableOpacity>
      ))}
    </View>
  );
}

const CAPTAIN_IMAGES = {
  trader: require('../../../../../assets/branding/tt-trader-captain.png'),
  sea: require('../../../../../assets/branding/tt-sea-captain.png'),
  rocky: require('../../../../../assets/branding/tt-rocky-captain.png'),
} as const;

const BOAT_IMAGES = {
  live: require('../../../../../assets/branding/tt-boat-side.png'),
  paper: require('../../../../../assets/branding/tt-toy-side.png'),
} as const;

const CATEGORY_IMAGES: Record<AssetFocus, number> = {
  'tokenized_equities': require('../../../../../assets/branding/tt-stocks.png'),
  'tokenized_metals': require('../../../../../assets/branding/tt-commodities.png'),
  majors: require('../../../../../assets/branding/tt-crypto-majors.png'),
  'finance_2': require('../../../../../assets/branding/tt-finance-2.png'),
  memes: require('../../../../../assets/branding/tt-memecoins.png'),
  custom: require('../../../../../assets/branding/tt-finance-2.png'),
};

const CATEGORY_COPY: Record<AssetFocus, string> = {
  'tokenized_equities': 'Global stock exposure for broad directional and rotational setups.',
  'tokenized_metals': 'Hard-asset markets for inflation and macro cycle positioning.',
  majors: 'Large-cap crypto pairs with deeper liquidity and tighter structure.',
  'finance_2': 'On-chain finance leaders where narratives can shift quickly.',
  memes: 'High-volatility memecoin markets for aggressive momentum trawling.',
  custom: 'Custom portfolio scope for a manually curated trading universe.',
};

type AssetSelectionMode = 'all' | 'custom';

function imageForCaptainKey(imageKey: string) {
  return CAPTAIN_IMAGES[imageKey as keyof typeof CAPTAIN_IMAGES] ?? CAPTAIN_IMAGES.trader;
}

function imageForCategory(value: AssetFocus) {
  return CATEGORY_IMAGES[value] ?? CATEGORY_IMAGES['tokenized_equities'];
}

function displayBoatName(value: string): string {
  return value
    .split('-')
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

function hasSameTokens(left: string[], right: string[]): boolean {
  if (left.length !== right.length) {
    return false;
  }
  const rightSet = new Set(right);
  return left.every((token) => rightSet.has(token));
}

export function CreateBotWizardSteps(props: CreateBotWizardStepsProps) {
  const {
    step,
    name,
    setName,
    nameAvailability,
    nameCheckLoading,
    assistantStyle,
    setAssistantStyle,
    assistantOptions,
    assistantOptionsLoading,
    assetChoices,
    assetFocus,
    setAssetFocus,
    tradeableAssets,
    selectedAssets,
    setSelectedAssets,
    assetsLoading,
    strategyOptions,
    strategyType,
    setStrategyType,
    algorithmMode,
    strictnessOptions,
    strictness,
    setStrictness,
    factorCatalog,
    algorithmFactors,
    setAlgorithmFactors,
    tradingMode,
    setTradingMode,
    maxPositionSize,
    setMaxPositionSize,
    maxTradesPerDay,
    setMaxTradesPerDay,
    maxDailyLoss,
    setMaxDailyLoss,
    maxDrawdown,
    setMaxDrawdown,
    llmProvider,
    setLlmProvider,
    llmModel,
    setLlmModel,
    llmApiKey,
    setLlmApiKey,
    modelsForProvider,
    llmModels,
    telegramEnabled,
    setTelegramEnabled,
    telegramBotToken,
    setTelegramBotToken,
    telegramUserId,
    setTelegramUserId,
    telegramPairingCode,
    setTelegramPairingCode,
  } = props;
  const disabledCustodians = useSettingsStore((s) => s.disabledCustodians);
  const [activeDropdownRow, setActiveDropdownRow] = useState<number | null>(null);
  const [captainViewportWidth, setCaptainViewportWidth] = useState(280);
  const captainScrollRef = useRef<ScrollView | null>(null);
  const [boatViewportWidth, setBoatViewportWidth] = useState(280);
  const boatScrollRef = useRef<ScrollView | null>(null);
  const [categoryViewportWidth, setCategoryViewportWidth] = useState(280);
  const categoryScrollRef = useRef<ScrollView | null>(null);
  const [assetSelectionMode, setAssetSelectionMode] = useState<AssetSelectionMode>('all');
  const [customAssetsByFocus, setCustomAssetsByFocus] = useState<Partial<Record<AssetFocus, string[]>>>({});
  const [assetPickerOpen, setAssetPickerOpen] = useState(false);
  const [assetPickerDraftSelection, setAssetPickerDraftSelection] = useState<string[]>([]);

  const usedFactorSet = useMemo(
    () => new Set(algorithmFactors.map((factor) => factor.factor)),
    [algorithmFactors]
  );
  const assetsForFocus = useMemo(
    () =>
      tradeableAssets.filter(
        (asset) => asset.assetFocus === assetFocus && !disabledCustodians.includes(asset.custodian)
      ),
    [assetFocus, disabledCustodians, tradeableAssets]
  );
  const assetsForFocusTokenAddresses = useMemo(
    () => assetsForFocus.map((asset) => asset.tokenAddress),
    [assetsForFocus]
  );
  const customSelectionForFocus = useMemo(() => {
    const rawSelection = customAssetsByFocus[assetFocus] ?? [];
    return rawSelection.filter((token) => assetsForFocusTokenAddresses.includes(token));
  }, [assetFocus, assetsForFocusTokenAddresses, customAssetsByFocus]);

  useEffect(() => {
    if (
      step !== 1 ||
      !captainScrollRef.current ||
      captainViewportWidth <= 0 ||
      assistantOptions.length === 0
    ) {
      return;
    }
    const selectedIndex = Math.max(
      0,
      assistantOptions.findIndex((option) => option.assistantStyle === assistantStyle)
    );
    captainScrollRef.current.scrollTo({ x: selectedIndex * captainViewportWidth, animated: true });
  }, [assistantOptions, assistantStyle, captainViewportWidth, step]);

  useEffect(() => {
    if (step !== 0 || !boatScrollRef.current || boatViewportWidth <= 0) {
      return;
    }
    const x = tradingMode === 'live' ? 0 : boatViewportWidth;
    boatScrollRef.current.scrollTo({ x, animated: true });
  }, [boatViewportWidth, step, tradingMode]);

  useEffect(() => {
    if (step !== 2 || !categoryScrollRef.current || categoryViewportWidth <= 0) {
      return;
    }
    const selectedIndex = Math.max(
      0,
      assetChoices.findIndex((choice) => choice.value === assetFocus)
    );
    categoryScrollRef.current.scrollTo({ x: selectedIndex * categoryViewportWidth, animated: true });
  }, [assetChoices, assetFocus, categoryViewportWidth, step]);

  useEffect(() => {
    if (step !== 2) {
      return;
    }
    const preferredMode: AssetSelectionMode =
      customSelectionForFocus.length > 0 ? 'custom' : 'all';
    if (assetSelectionMode !== preferredMode) {
      setAssetSelectionMode(preferredMode);
      return;
    }
    if (assetSelectionMode === 'all') {
      if (!hasSameTokens(selectedAssets, assetsForFocusTokenAddresses)) {
        setSelectedAssets(assetsForFocusTokenAddresses);
      }
      return;
    }
    if (!hasSameTokens(selectedAssets, customSelectionForFocus)) {
      setSelectedAssets(customSelectionForFocus);
    }
  }, [
    assetSelectionMode,
    customSelectionForFocus,
    assetsForFocusTokenAddresses,
    selectedAssets,
    setSelectedAssets,
    step,
  ]);

  if (step === 0) {
    const handleBoatSwipeEnd = (event: NativeSyntheticEvent<NativeScrollEvent>) => {
      if (boatViewportWidth <= 0) {
        return;
      }
      const index = Math.round(event.nativeEvent.contentOffset.x / boatViewportWidth);
      setTradingMode(index <= 0 ? 'live' : 'paper');
    };

    return (
      <View>
        <Text style={styles.sectionLabel}>Boat Name</Text>
        <View style={styles.nameInputRow}>
          <TextInput
            style={[styles.input, styles.nameInput]}
            value={name}
            onChangeText={setName}
            placeholder="e.g. Bright Atlantic Trawler"
            placeholderTextColor={lightTheme.colors.wave[400]}
          />
          <View style={styles.nameStatusIconWrap}>
            {nameCheckLoading ? (
              <ActivityIndicator size="small" color={lightTheme.colors.wave[500]} />
            ) : nameAvailability ? (
              <Ionicons
                name={nameAvailability.available ? 'checkmark-circle' : 'close-circle'}
                size={20}
                color={nameAvailability.available ? lightTheme.colors.bullish[600] : lightTheme.colors.lobster[600]}
              />
            ) : null}
          </View>
        </View>
        {!nameCheckLoading && nameAvailability && !nameAvailability.available && nameAvailability.suggestedName ? (
          <TouchableOpacity onPress={() => setName(displayBoatName(nameAvailability.suggestedName || name))}>
            <Text style={styles.useSuggestionText}>
              Use "{displayBoatName(nameAvailability.suggestedName)}"
            </Text>
          </TouchableOpacity>
        ) : null}
        <Text style={styles.sectionLabel}>Trading Mode</Text>
        <View style={styles.switchRow}>
          <View style={styles.switchCopy}>
            <Text style={styles.switchTitle}>
              {tradingMode === 'paper' ? 'Paper Trading (Test Only)' : 'Live Trading'}
            </Text>
            <Text style={styles.switchSubtitle}>
              {tradingMode === 'paper'
                ? 'Recommended to start. Use this mode to validate behavior with no real funds at risk.'
                : 'Live mode uses real funds. Keep paper mode on until you trust your setup.'}
            </Text>
          </View>
          <Switch
            value={tradingMode === 'live'}
            onValueChange={(live) => setTradingMode(live ? 'live' : 'paper')}
            trackColor={{
              false: lightTheme.colors.wave[300],
              true: lightTheme.colors.primary[500],
            }}
            thumbColor={tradingMode === 'live' ? lightTheme.colors.primary[50] : '#ffffff'}
            ios_backgroundColor={lightTheme.colors.wave[300]}
          />
        </View>
        <View
          style={styles.boatModeVisual}
          onLayout={(event) => {
            const width = Math.floor(event.nativeEvent.layout.width);
            if (width > 0 && width !== boatViewportWidth) {
              setBoatViewportWidth(width);
            }
          }}
        >
          <ScrollView
            ref={boatScrollRef}
            horizontal
            pagingEnabled
            showsHorizontalScrollIndicator={false}
            onMomentumScrollEnd={handleBoatSwipeEnd}
            snapToInterval={boatViewportWidth}
            decelerationRate="fast"
            contentContainerStyle={styles.boatCarousel}
          >
            <TouchableOpacity
              style={[styles.boatSlide, { width: boatViewportWidth }]}
              onPress={() => setTradingMode('live')}
              activeOpacity={0.9}
            >
              <Image source={BOAT_IMAGES.live} style={styles.boatImage} resizeMode="contain" />
            </TouchableOpacity>
            <TouchableOpacity
              style={[styles.boatSlide, { width: boatViewportWidth }]}
              onPress={() => setTradingMode('paper')}
              activeOpacity={0.9}
            >
              <Image source={BOAT_IMAGES.paper} style={styles.boatImage} resizeMode="contain" />
            </TouchableOpacity>
          </ScrollView>
        </View>
      </View>
    );
  }

  if (step === 1) {
    if (assistantOptionsLoading) {
      return (
        <View>
          <Text style={styles.helperText}>Loading captains...</Text>
        </View>
      );
    }

    if (assistantOptions.length === 0) {
      return (
        <View>
          <Text style={styles.helperText}>
            Captain options are temporarily unavailable. You can continue with the default captain.
          </Text>
        </View>
      );
    }

    const handleCaptainSwipeEnd = (event: NativeSyntheticEvent<NativeScrollEvent>) => {
      if (captainViewportWidth <= 0) {
        return;
      }
      const index = Math.round(event.nativeEvent.contentOffset.x / captainViewportWidth);
      const selectedOption = assistantOptions[Math.max(0, Math.min(index, assistantOptions.length - 1))];
      if (selectedOption) {
        setAssistantStyle(selectedOption.assistantStyle);
      }
    };
    const activeCaptainIndex = Math.max(
      0,
      assistantOptions.findIndex((option) => option.assistantStyle === assistantStyle)
    );
    const selectCaptainAt = (index: number) => {
      const bounded = Math.max(0, Math.min(index, assistantOptions.length - 1));
      const selectedOption = assistantOptions[bounded];
      if (selectedOption) {
        setAssistantStyle(selectedOption.assistantStyle);
      }
    };

    return (
      <View>
        <Text style={styles.helperText}>Swipe to browse captains, then tap to confirm your choice.</Text>
        <View
          style={styles.captainModeVisual}
          onLayout={(event) => {
            const width = Math.floor(event.nativeEvent.layout.width);
            if (width > 0 && width !== captainViewportWidth) {
              setCaptainViewportWidth(width);
            }
          }}
        >
          <ScrollView
            ref={captainScrollRef}
            horizontal
            pagingEnabled
            showsHorizontalScrollIndicator={false}
            onMomentumScrollEnd={handleCaptainSwipeEnd}
            snapToInterval={captainViewportWidth}
            decelerationRate="fast"
            contentContainerStyle={styles.captainCarousel}
          >
            {assistantOptions.map((option) => (
              <TouchableOpacity
                key={option.id}
                style={[styles.captainSlide, { width: captainViewportWidth }]}
                onPress={() => setAssistantStyle(option.assistantStyle)}
                activeOpacity={0.9}
              >
                <View style={styles.captainImageFrame}>
                  <Image
                    source={imageForCaptainKey(option.imageKey)}
                    style={[styles.captainImage, { height: Math.round((captainViewportWidth - 8) * 1.5) }]}
                    resizeMode="stretch"
                  />
                </View>
                <View style={styles.captainInfoRow}>
                  <TouchableOpacity
                    style={styles.captainArrowButton}
                    onPress={() => selectCaptainAt(activeCaptainIndex - 1)}
                    disabled={activeCaptainIndex === 0}
                  >
                    <Ionicons
                      name="chevron-back"
                      size={20}
                      color={activeCaptainIndex === 0 ? lightTheme.colors.wave[400] : lightTheme.colors.wave[700]}
                    />
                  </TouchableOpacity>
                  <View style={styles.captainInfoCopy}>
                    <Text style={styles.captainName}>{option.captainName}</Text>
                    <Text style={styles.captainDescription}>{option.personalityDescription}</Text>
                  </View>
                  <TouchableOpacity
                    style={styles.captainArrowButton}
                    onPress={() => selectCaptainAt(activeCaptainIndex + 1)}
                    disabled={activeCaptainIndex === assistantOptions.length - 1}
                  >
                    <Ionicons
                      name="chevron-forward"
                      size={20}
                      color={
                        activeCaptainIndex === assistantOptions.length - 1
                          ? lightTheme.colors.wave[400]
                          : lightTheme.colors.wave[700]
                      }
                    />
                  </TouchableOpacity>
                </View>
              </TouchableOpacity>
            ))}
          </ScrollView>
        </View>
      </View>
    );
  }

  if (step === 2) {
    const activeCategoryIndex = Math.max(
      0,
      assetChoices.findIndex((choice) => choice.value === assetFocus)
    );
    const activeCategory = assetChoices[activeCategoryIndex] ?? assetChoices[0];
    const selectedCountLabel =
      assetSelectionMode === 'custom'
        ? `${selectedAssets.length} selected`
        : `${assetsForFocus.length} total`;
    const openAssetPicker = () => {
      setAssetPickerDraftSelection(customSelectionForFocus);
      setAssetPickerOpen(true);
    };
    const toggleDraftAsset = (tokenAddress: string) => {
      if (assetPickerDraftSelection.includes(tokenAddress)) {
        setAssetPickerDraftSelection(assetPickerDraftSelection.filter((token) => token !== tokenAddress));
        return;
      }
      setAssetPickerDraftSelection([...assetPickerDraftSelection, tokenAddress]);
    };
    const saveAssetPickerSelection = () => {
      const cleanedSelection = assetPickerDraftSelection.filter((token) =>
        assetsForFocusTokenAddresses.includes(token)
      );
      setCustomAssetsByFocus((prev) => ({
        ...prev,
        [assetFocus]: cleanedSelection,
      }));
      setSelectedAssets(cleanedSelection);
      setAssetSelectionMode('custom');
      setAssetPickerOpen(false);
    };
    const handleCategorySwipeEnd = (event: NativeSyntheticEvent<NativeScrollEvent>) => {
      if (categoryViewportWidth <= 0) {
        return;
      }
      const index = Math.round(event.nativeEvent.contentOffset.x / categoryViewportWidth);
      const selectedCategory = assetChoices[Math.max(0, Math.min(index, assetChoices.length - 1))];
      if (selectedCategory) {
        setAssetFocus(selectedCategory.value);
      }
    };
    const selectCategoryAt = (index: number) => {
      const bounded = Math.max(0, Math.min(index, assetChoices.length - 1));
      const category = assetChoices[bounded];
      if (category) {
        setAssetFocus(category.value);
      }
    };

    return (
      <View>
        <View
          style={styles.categoryModeVisual}
          onLayout={(event) => {
            const width = Math.floor(event.nativeEvent.layout.width);
            if (width > 0 && width !== categoryViewportWidth) {
              setCategoryViewportWidth(width);
            }
          }}
        >
          <ScrollView
            ref={categoryScrollRef}
            horizontal
            pagingEnabled
            showsHorizontalScrollIndicator={false}
            onMomentumScrollEnd={handleCategorySwipeEnd}
            snapToInterval={categoryViewportWidth}
            decelerationRate="fast"
            contentContainerStyle={styles.categoryCarousel}
          >
            {assetChoices.map((choice) => (
              <TouchableOpacity
                key={choice.value}
                style={[styles.categorySlide, { width: categoryViewportWidth }]}
                onPress={() => setAssetFocus(choice.value)}
                activeOpacity={0.9}
              >
                <Image
                  source={imageForCategory(choice.value)}
                  style={styles.categoryImage}
                  resizeMode="contain"
                />
              </TouchableOpacity>
            ))}
          </ScrollView>
        </View>
        <View style={styles.categoryInfoRow}>
          <TouchableOpacity
            style={styles.captainArrowButton}
            onPress={() => selectCategoryAt(activeCategoryIndex - 1)}
            disabled={activeCategoryIndex === 0}
          >
            <Ionicons
              name="chevron-back"
              size={20}
              color={activeCategoryIndex === 0 ? lightTheme.colors.wave[400] : lightTheme.colors.wave[700]}
            />
          </TouchableOpacity>
          <View style={styles.captainInfoCopy}>
            <Text style={styles.categoryTitle}>{activeCategory?.label}</Text>
          </View>
          <TouchableOpacity
            style={styles.captainArrowButton}
            onPress={() => selectCategoryAt(activeCategoryIndex + 1)}
            disabled={activeCategoryIndex === assetChoices.length - 1}
          >
            <Ionicons
              name="chevron-forward"
              size={20}
              color={activeCategoryIndex === assetChoices.length - 1 ? lightTheme.colors.wave[400] : lightTheme.colors.wave[700]}
            />
          </TouchableOpacity>
        </View>
        {assetsLoading ? (
          <Text style={styles.helperText}>Loading curated assets...</Text>
        ) : assetsForFocus.length === 0 ? (
          <Text style={styles.helperText}>No assets available for this category yet.</Text>
        ) : (
          <>
            <TouchableOpacity
              style={[styles.optionCard, assetSelectionMode === 'all' && styles.optionCardActive]}
              onPress={() => {
                setAssetSelectionMode('all');
                setSelectedAssets(assetsForFocusTokenAddresses);
              }}
            >
              <View style={styles.optionTitleRow}>
                <Text style={styles.optionTitle}>All assets</Text>
                <Text style={styles.optionCountBadge}>{assetsForFocus.length} total</Text>
              </View>
              <Text style={styles.optionDescription}>
                Trade every available asset in {activeCategory.label}.
              </Text>
            </TouchableOpacity>
            <TouchableOpacity
              style={[styles.optionCard, assetSelectionMode === 'custom' && styles.optionCardActive]}
              onPress={() => {
                setAssetSelectionMode('custom');
                openAssetPicker();
              }}
            >
              <View style={styles.optionTitleRow}>
                <Text style={styles.optionTitle}>Select assets</Text>
                <Text style={styles.optionCountBadge}>{selectedCountLabel}</Text>
              </View>
              <Text style={styles.optionDescription}>
                Choose exactly which assets this boat is allowed to trade.
              </Text>
            </TouchableOpacity>
          </>
        )}
        <Modal
          visible={assetPickerOpen}
          animationType="slide"
          presentationStyle="fullScreen"
          onRequestClose={() => setAssetPickerOpen(false)}
        >
          <View style={styles.assetPickerContainer}>
            <View style={styles.assetPickerHeader}>
              <TouchableOpacity
                style={styles.assetPickerBackButton}
                onPress={() => setAssetPickerOpen(false)}
              >
                <Ionicons name="chevron-back" size={22} color={lightTheme.colors.wave[800]} />
              </TouchableOpacity>
              <Text style={styles.assetPickerHeaderTitle}>Select Assets</Text>
              <View style={styles.assetPickerHeaderSpacer} />
            </View>
            <Text style={styles.assetPickerSubTitle}>
              {activeCategory.label} • {assetPickerDraftSelection.length} selected
            </Text>
            <ScrollView contentContainerStyle={styles.assetPickerList}>
              {assetsForFocus.map((asset) => {
                const selected = assetPickerDraftSelection.includes(asset.tokenAddress);
                return (
                  <TouchableOpacity
                    key={asset.tokenAddress}
                    style={[styles.assetPickerRow, selected && styles.assetPickerRowActive]}
                    onPress={() => toggleDraftAsset(asset.tokenAddress)}
                  >
                    <View style={[styles.assetPickerCheckbox, selected && styles.assetPickerCheckboxActive]}>
                      {selected ? <Text style={styles.assetPickerCheckboxMark}>✓</Text> : null}
                    </View>
                    <View style={styles.assetPickerRowCopy}>
                      <Text style={styles.assetSymbol}>{asset.symbol}</Text>
                      <Text style={styles.assetName}>{asset.name}</Text>
                    </View>
                    <Text style={styles.assetCustodian}>{asset.custodian}</Text>
                  </TouchableOpacity>
                );
              })}
            </ScrollView>
            <TouchableOpacity
              style={[
                styles.assetPickerSaveButton,
                assetPickerDraftSelection.length === 0 && styles.assetPickerSaveButtonDisabled,
              ]}
              onPress={saveAssetPickerSelection}
              disabled={assetPickerDraftSelection.length === 0}
            >
              <Text style={styles.assetPickerSaveButtonText}>Save</Text>
            </TouchableOpacity>
          </View>
        </Modal>
      </View>
    );
  }

  if (step === 3) {
    return (
      <View>
        <Text style={styles.sectionLabel}>Strategy Type</Text>
        {renderChip(strategyOptions, strategyType, setStrategyType)}
        <Text style={styles.helperText}>
          This chooses the style of trading logic for your selected assets.
        </Text>
      </View>
    );
  }

  if (step === 4) {
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
            <View key={`${factor.factor}-${rowIndex}`} style={styles.factorBlock}>
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

  if (step === 5) {
    return (
      <View>
        <Text style={styles.sectionLabel}>Risk Caps</Text>
        <View style={styles.row}>
          <View style={styles.half}>
            <TextInput
              style={styles.input}
              value={maxPositionSize}
              onChangeText={setMaxPositionSize}
              keyboardType="numeric"
              placeholder="Max Position %"
            />
            <Text style={styles.helperText}>Default: 5%</Text>
          </View>
          <View style={styles.half}>
            <TextInput
              style={styles.input}
              value={maxTradesPerDay}
              onChangeText={setMaxTradesPerDay}
              keyboardType="numeric"
              placeholder="Max Trades/Day"
            />
            <Text style={styles.helperText}>Default: 5</Text>
          </View>
        </View>
        <View style={styles.row}>
          <View style={styles.half}>
            <TextInput
              style={styles.input}
              value={maxDailyLoss}
              onChangeText={setMaxDailyLoss}
              keyboardType="numeric"
              placeholder="Daily Loss USD"
            />
            <Text style={styles.helperText}>Default: $50</Text>
          </View>
          <View style={styles.half}>
            <TextInput
              style={styles.input}
              value={maxDrawdown}
              onChangeText={setMaxDrawdown}
              keyboardType="numeric"
              placeholder="Max Drawdown %"
            />
            <Text style={styles.helperText}>Default: 10%</Text>
          </View>
        </View>
        <Text style={styles.sectionLabel}>Strictness</Text>
        {renderChip(strictnessOptions, strictness, setStrictness)}
        <View style={styles.instructionBox}>
          <Text style={styles.instructionTitle}>How Strictness Works</Text>
          <Text style={styles.instructionStep}>
            Strictness works with your risk caps and other execution thresholds to control how much confirmation is required before a trade.
          </Text>
          <Text style={styles.instructionStep}>
            Example: a conservative setup can wait for signal stability over a longer look-back window (such as ~15 minutes) instead of trading the first second RSI flips.
          </Text>
          <Text style={styles.instructionStep}>
            Your parameters are always respected. Strictness changes confidence requirements and stability checks, not whether your bot ignores your settings.
          </Text>
        </View>
      </View>
    );
  }

  if (step === 6) {
    return (
      <View>
        <Text style={styles.sectionLabel}>LLM Provider</Text>
        {renderChip(
          (['openai', 'anthropic', 'venice', 'openrouter'] as LlmProvider[]).map(
            (value) => ({ value, label: value })
          ),
          llmProvider,
          (provider) => {
            setLlmProvider(provider);
            setLlmModel(llmModels[provider][0].value);
          }
        )}

        <Text style={styles.sectionLabel}>Model</Text>
        {renderChip(modelsForProvider, llmModel, setLlmModel)}

        <Text style={styles.sectionLabel}>API Key</Text>
        <TextInput
          style={styles.input}
          value={llmApiKey}
          onChangeText={setLlmApiKey}
          placeholder="sk-..."
          placeholderTextColor={lightTheme.colors.wave[400]}
          secureTextEntry
        />
        <Text style={styles.helperText}>
          Stored securely and only used for this boat.
        </Text>
      </View>
    );
  }

  if (step === 7) {
    return (
      <View>
        <View style={styles.instructionBox}>
          <Text style={styles.instructionTitle}>Setup Guide</Text>
          <Text style={styles.instructionStep}>
            1. In Telegram, open <Text style={styles.inlineCode}>@BotFather</Text> and run{' '}
            <Text style={styles.inlineCode}>/newbot</Text>.
          </Text>
          <Text style={styles.instructionStep}>
            2. Copy the bot token BotFather returns and paste it below.
          </Text>
          <Text style={styles.instructionStep}>
            3. Send your bot a first message (for example:{' '}
            <Text style={styles.inlineCode}>/start</Text>).
          </Text>
          <Text style={styles.instructionStep}>
            4. The bot will reply with your Telegram user ID and a pairing code.
          </Text>
          <Text style={styles.instructionStep}>
            5. Reply with that pairing code in bot chat after deployment to finish linking.
          </Text>
        </View>

        <View style={styles.switchRow}>
          <View style={styles.switchCopy}>
            <Text style={styles.switchTitle}>Enable Telegram</Text>
            <Text style={styles.switchSubtitle}>
              Turn this on only if you want Telegram commands/alerts.
            </Text>
          </View>
          <Switch
            value={telegramEnabled}
            onValueChange={setTelegramEnabled}
            trackColor={{
              false: lightTheme.colors.wave[300],
              true: lightTheme.colors.primary[400],
            }}
          />
        </View>

        {telegramEnabled && (
          <>
            <Text style={styles.sectionLabel}>Telegram Bot Token</Text>
            <TextInput
              style={styles.input}
              value={telegramBotToken}
              onChangeText={setTelegramBotToken}
              placeholder="123456789:ABCdefGHI..."
              placeholderTextColor={lightTheme.colors.wave[400]}
              secureTextEntry
            />
            <Text style={styles.helperText}>
              Keep this private. We encrypt it at rest and only use it for your bot session.
            </Text>

            <Text style={styles.sectionLabel}>Telegram User ID</Text>
            <TextInput
              style={styles.input}
              value={telegramUserId}
              onChangeText={setTelegramUserId}
              placeholder="e.g. 123456789"
              placeholderTextColor={lightTheme.colors.wave[400]}
              keyboardType="number-pad"
            />

            <Text style={styles.sectionLabel}>Pairing Code</Text>
            <TextInput
              style={styles.input}
              value={telegramPairingCode}
              onChangeText={setTelegramPairingCode}
              placeholder="e.g. TRAWL-4821"
              placeholderTextColor={lightTheme.colors.wave[400]}
              autoCapitalize="characters"
              autoCorrect={false}
            />
            <Text style={styles.helperText}>
              We currently keep these values in setup flow for guided pairing.
            </Text>
          </>
        )}
      </View>
    );
  }

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
