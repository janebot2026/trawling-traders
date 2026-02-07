import React from 'react';
import {
  View,
  ImageBackground,
  StyleSheet,
  useColorScheme,
  ViewStyle,
} from 'react-native';

// Ocean background images not yet added - using solid color fallback
// const OCEAN_LIGHT = require('../../assets/bg-ocean-light.png');
// const OCEAN_DARK = require('../../assets/bg-ocean-dark.png');

interface OceanBackgroundProps {
  children: React.ReactNode;
  style?: ViewStyle;
}

export function OceanBackground({ children, style }: OceanBackgroundProps) {
  const colorScheme = useColorScheme();
  const isDark = colorScheme === 'dark';

  return (
    <View style={[styles.background, { backgroundColor: isDark ? '#0a2540' : '#e0f2fe' }, style]}>
      <View style={styles.overlay}>
        {children}
      </View>
    </View>
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
