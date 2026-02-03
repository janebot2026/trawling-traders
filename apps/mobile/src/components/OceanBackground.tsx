import React from 'react';
import {
  View,
  ImageBackground,
  StyleSheet,
  useColorScheme,
  ViewStyle,
} from 'react-native';

const OCEAN_LIGHT = require('../../assets/bg-ocean-light.png');
const OCEAN_DARK = require('../../assets/bg-ocean-dark.png');

interface OceanBackgroundProps {
  children: React.ReactNode;
  style?: ViewStyle;
}

export function OceanBackground({ children, style }: OceanBackgroundProps) {
  const colorScheme = useColorScheme();
  const isDark = colorScheme === 'dark';

  return (
    <ImageBackground
      source={isDark ? OCEAN_DARK : OCEAN_LIGHT}
      style={[styles.background, style]}
      resizeMode="cover"
    >
      <View style={styles.overlay}>
        {children}
      </View>
    </ImageBackground>
  );
}

const styles = StyleSheet.create({
  background: {
    flex: 1,
  },
  overlay: {
    flex: 1,
    backgroundColor: 'rgba(255, 255, 255, 0.85)', // Semi-transparent white for light
  },
});

// Dark mode overlay variant
export function OceanBackgroundDark({ children, style }: OceanBackgroundProps) {
  return (
    <ImageBackground
      source={OCEAN_DARK}
      style={[styles.background, style]}
      resizeMode="cover"
    >
      <View style={styles.overlayDark}>
        {children}
      </View>
    </ImageBackground>
  );
}

const stylesDark = StyleSheet.create({
  ...styles,
  overlayDark: {
    flex: 1,
    backgroundColor: 'rgba(15, 23, 42, 0.85)', // Semi-transparent navy for dark
  },
});
