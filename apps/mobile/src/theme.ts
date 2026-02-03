/**
 * Trawling Traders - Brand Theme
 * Colors extracted from lobster-crew.png brand assets
 */

export const colors = {
  // Primary - Navy/Ocean
  primary: {
    50: '#f0f9ff',
    100: '#e0f2fe',
    200: '#bae6fd',
    300: '#7dd3fc',
    400: '#38bdf8',
    500: '#0ea5e9',
    600: '#0284c7',
    700: '#0369a1',
    800: '#075985',
    900: '#1a365d',  // Navy suit
    950: '#0f172a',  // Deep navy - dark mode base
  },

  // Lobster Red - Accent/Brand
  lobster: {
    50: '#fef2f2',
    100: '#fee2e2',
    200: '#fecaca',
    300: '#fca5a5',
    400: '#f87171',
    500: '#ef4444',
    600: '#dc2626',
    700: '#c53030',  // Main lobster red
    800: '#991b1b',
    900: '#7f1d1d',
  },

  // Bullish Green - Success/Positive
  bullish: {
    50: '#f0fdf4',
    100: '#dcfce7',
    200: '#bbf7d0',
    300: '#86efac',
    400: '#4ade80',
    500: '#22c55e',  // Tie/badge green
    600: '#16a34a',
    700: '#15803d',
    800: '#166534',
    900: '#14532d',
  },

  // Caution Yellow - Warning
  caution: {
    50: '#fefce8',
    100: '#fef9c3',
    200: '#fef08a',
    300: '#fde047',
    400: '#facc15',
    500: '#eab308',  // Railings yellow
    600: '#ca8a04',
    700: '#a16207',
    800: '#854d0e',
    900: '#713f12',
  },

  // Ocean Waves - Neutrals
  wave: {
    50: '#f8fafc',
    100: '#f1f5f9',
    200: '#e2e8f0',
    300: '#cbd5e1',
    400: '#94a3b8',
    500: '#64748b',
    600: '#475569',
    700: '#334155',
    800: '#1e293b',  // Dark mode surfaces
    900: '#0f172a',  // Dark mode background
    950: '#020617',
  },

  // Semantic aliases
  success: '#22c55e',
  error: '#dc2626',
  warning: '#eab308',
  info: '#0ea5e9',

  // Trading specific
  positive: '#22c55e',  // Green for gains
  negative: '#dc2626',  // Red for losses
  neutral: '#64748b',
};

export const spacing = {
  xs: 4,
  sm: 8,
  md: 16,
  lg: 24,
  xl: 32,
  '2xl': 48,
  '3xl': 64,
};

export const borderRadius = {
  sm: 4,
  md: 8,
  lg: 12,
  xl: 16,
  '2xl': 24,
  full: 9999,
};

export const typography = {
  sizes: {
    xs: 12,
    sm: 14,
    md: 16,
    lg: 18,
    xl: 20,
    '2xl': 24,
    '3xl': 30,
    '4xl': 36,
  },
  weights: {
    normal: '400',
    medium: '500',
    semibold: '600',
    bold: '700',
  },
};

export const shadows = {
  sm: {
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.05,
    shadowRadius: 2,
    elevation: 1,
  },
  md: {
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.1,
    shadowRadius: 4,
    elevation: 3,
  },
  lg: {
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 4 },
    shadowOpacity: 0.15,
    shadowRadius: 8,
    elevation: 5,
  },
};

// Light theme
export const lightTheme = {
  colors: {
    background: '#f8fafc',
    surface: '#ffffff',
    surfaceElevated: '#ffffff',
    primary: colors.primary[900],
    primaryLight: colors.primary[700],
    text: colors.wave[900],
    textSecondary: colors.wave[600],
    textMuted: colors.wave[400],
    border: colors.wave[200],
    ...colors,
  },
  spacing,
  borderRadius,
  typography,
  shadows,
};

// Dark theme
export const darkTheme = {
  colors: {
    background: colors.wave[950],
    surface: colors.wave[900],
    surfaceElevated: colors.wave[800],
    primary: colors.primary[400],
    primaryLight: colors.primary[300],
    text: colors.wave[50],
    textSecondary: colors.wave[400],
    textMuted: colors.wave[600],
    border: colors.wave[800],
    ...colors,
  },
  spacing,
  borderRadius,
  typography,
  shadows,
};

export type Theme = typeof lightTheme;
