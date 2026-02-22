const path = require('path');

// react-native lives in apps/mobile/node_modules, jest-expo in root node_modules.
// Build the preset manually to bridge the monorepo split.
const rnPreset = require(path.resolve(__dirname, 'node_modules/react-native/jest-preset'));
const expoAssetTransformer = path.resolve(
  __dirname,
  '../../node_modules/jest-expo/src/preset/assetFileTransformer.js'
);

/** @type {import('jest').Config} */
module.exports = {
  ...rnPreset,
  // Let Jest resolve modules from both local and root node_modules
  modulePaths: [
    path.resolve(__dirname, 'node_modules'),
    path.resolve(__dirname, '../../node_modules'),
  ],
  transform: {
    '\\.[jt]sx?$': 'babel-jest',
    '^.+\\.(bmp|gif|jpg|jpeg|mp4|png|psd|svg|webp|ttf|otf|m4v|mov|mpeg|mpg|webm|aac|aiff|caf|m4a|mp3|wav|html|pdf|obj)$':
      expoAssetTransformer,
  },
  setupFiles: [
    ...(rnPreset.setupFiles || []),
    path.resolve(__dirname, '../../node_modules/jest-expo/src/preset/setup.js'),
    './jest.setupFiles.ts',
  ],
  globals: {
    __DEV__: true,
  },
  transformIgnorePatterns: [
    'node_modules/(?!(' +
      'jest-)?react-native' +
      '|@react-native(-community)?' +
      '|expo(nent)?' +
      '|@expo(nent)?/.*' +
      '|@expo-google-fonts/.*' +
      '|react-navigation' +
      '|@react-navigation/.*' +
      '|@unimodules/.*' +
      '|unimodules' +
      '|sentry-expo' +
      '|native-base' +
      '|react-native-svg' +
      '|react-native-reanimated' +
      '|react-native-gesture-handler' +
      '|react-native-screens' +
      '|react-native-safe-area-context' +
      '|react-native-get-random-values' +
      '|react-native-chart-kit' +
      '|@trawling-traders/.*' +
      '|@cedros/.*' +
      '|@solana/.*' +
      '|@stripe/.*' +
      '|zustand' +
      ')',
  ],
  moduleNameMapper: {
    ...rnPreset.moduleNameMapper,
    // Stub image/asset imports
    '\\.(png|jpg|jpeg|gif|webp|svg)$': '<rootDir>/__mocks__/fileMock.js',
    // Stub font files
    '\\.(otf|ttf|woff|woff2)$': '<rootDir>/__mocks__/fileMock.js',
  },
  moduleFileExtensions: ['ts', 'tsx', 'js', 'jsx', 'json'],
  testPathIgnorePatterns: ['/node_modules/', '/dist/'],
};
