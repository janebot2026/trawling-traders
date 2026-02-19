import React, { useEffect, useRef, useState } from 'react';
import {
  ActivityIndicator,
  Image,
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
import type { NameAvailability, TradingMode } from '@trawling-traders/types';
import { lightTheme } from '../../theme';
import { createBotWizardStyles as styles } from './CreateBotWizard.styles';
import { BOAT_IMAGES, displayBoatName } from './wizardShared';

export type NameStepProps = {
  name: string;
  setName: (value: string) => void;
  nameAvailability: NameAvailability | null;
  nameCheckLoading: boolean;
  tradingMode: TradingMode;
  setTradingMode: (value: TradingMode) => void;
};

export function NameStep({
  name,
  setName,
  nameAvailability,
  nameCheckLoading,
  tradingMode,
  setTradingMode,
}: NameStepProps) {
  const [boatViewportWidth, setBoatViewportWidth] = useState(280);
  const boatScrollRef = useRef<ScrollView | null>(null);

  useEffect(() => {
    if (!boatScrollRef.current || boatViewportWidth <= 0) {
      return;
    }
    const x = tradingMode === 'live' ? 0 : boatViewportWidth;
    boatScrollRef.current.scrollTo({ x, animated: true });
  }, [boatViewportWidth, tradingMode]);

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
