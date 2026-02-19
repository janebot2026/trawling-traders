import React, { useEffect, useRef, useState } from 'react';
import {
  Image,
  ScrollView,
  Text,
  TouchableOpacity,
  View,
  type NativeSyntheticEvent,
  type NativeScrollEvent,
} from 'react-native';
import { Ionicons } from '@expo/vector-icons';
import type { AIAssistantOption, Persona } from '@trawling-traders/types';
import { lightTheme } from '../../theme';
import { createBotWizardStyles as styles } from './CreateBotWizard.styles';
import { imageForCaptainKey } from './wizardShared';

export type PersonaStepProps = {
  assistantStyle: Persona;
  setAssistantStyle: (value: Persona) => void;
  assistantOptions: AIAssistantOption[];
  assistantOptionsLoading: boolean;
};

export function PersonaStep({
  assistantStyle,
  setAssistantStyle,
  assistantOptions,
  assistantOptionsLoading,
}: PersonaStepProps) {
  const [captainViewportWidth, setCaptainViewportWidth] = useState(280);
  const captainScrollRef = useRef<ScrollView | null>(null);

  useEffect(() => {
    if (!captainScrollRef.current || captainViewportWidth <= 0 || assistantOptions.length === 0) {
      return;
    }
    const selectedIndex = Math.max(
      0,
      assistantOptions.findIndex((option) => option.assistantStyle === assistantStyle)
    );
    captainScrollRef.current.scrollTo({ x: selectedIndex * captainViewportWidth, animated: true });
  }, [assistantOptions, assistantStyle, captainViewportWidth]);

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

  const activeCaptainIndex = Math.max(
    0,
    assistantOptions.findIndex((option) => option.assistantStyle === assistantStyle)
  );

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
