import React, { useEffect, useMemo, useRef, useState } from 'react';
import {
  Image,
  Modal,
  ScrollView,
  Text,
  TouchableOpacity,
  View,
  type NativeSyntheticEvent,
  type NativeScrollEvent,
} from 'react-native';
import { Ionicons } from '@expo/vector-icons';
import type { AssetFocus, TradeableAsset } from '@trawling-traders/types';
import { useSettingsStore } from '../../store';
import { lightTheme } from '../../theme';
import { createBotWizardStyles as styles } from './CreateBotWizard.styles';
import {
  type AssetSelectionMode,
  type Option,
  imageForCategory,
  hasSameTokens,
} from './wizardShared';

export type AssetsStepProps = {
  assetFocus: AssetFocus;
  setAssetFocus: (value: AssetFocus) => void;
  tradeableAssets: TradeableAsset[];
  selectedAssets: string[];
  setSelectedAssets: (value: string[]) => void;
  assetsLoading: boolean;
  assetChoices: Option<AssetFocus>[];
};

export function AssetsStep({
  assetFocus,
  setAssetFocus,
  tradeableAssets,
  selectedAssets,
  setSelectedAssets,
  assetsLoading,
  assetChoices,
}: AssetsStepProps) {
  const disabledCustodians = useSettingsStore((s) => s.disabledCustodians);
  const [categoryViewportWidth, setCategoryViewportWidth] = useState(280);
  const categoryScrollRef = useRef<ScrollView | null>(null);
  const [assetSelectionMode, setAssetSelectionMode] = useState<AssetSelectionMode>('all');
  const [customAssetsByFocus, setCustomAssetsByFocus] = useState<Partial<Record<AssetFocus, string[]>>>({});
  const [assetPickerOpen, setAssetPickerOpen] = useState(false);
  const [assetPickerDraftSelection, setAssetPickerDraftSelection] = useState<string[]>([]);

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
    if (!categoryScrollRef.current || categoryViewportWidth <= 0) {
      return;
    }
    const selectedIndex = Math.max(
      0,
      assetChoices.findIndex((choice) => choice.value === assetFocus)
    );
    categoryScrollRef.current.scrollTo({ x: selectedIndex * categoryViewportWidth, animated: true });
  }, [assetChoices, assetFocus, categoryViewportWidth]);

  useEffect(() => {
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
  ]);

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
          <Text style={styles.captainName}>{activeCategory?.label}</Text>
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
