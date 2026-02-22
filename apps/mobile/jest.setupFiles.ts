/**
 * Pre-framework setup: polyfills and native module mocks that must be
 * available before React and testing libraries initialise.
 */

// Mock AsyncStorage
jest.mock('@react-native-async-storage/async-storage', () =>
  require('@react-native-async-storage/async-storage/jest/async-storage-mock')
);

// Provide a minimal atob/btoa for ApiProvider's JWT decode
if (typeof globalThis.atob === 'undefined') {
  globalThis.atob = (str: string) => Buffer.from(str, 'base64').toString('binary');
  globalThis.btoa = (str: string) => Buffer.from(str, 'binary').toString('base64');
}

// Mock expo-secure-store: in-memory key-value store
jest.mock('expo-secure-store', () => {
  const store = new Map<string, string>();
  return {
    getItemAsync: jest.fn(async (key: string) => store.get(key) ?? null),
    setItemAsync: jest.fn(async (key: string, value: string) => { store.set(key, value); }),
    deleteItemAsync: jest.fn(async (key: string) => { store.delete(key); }),
    __store: store,
  };
});

// Mock react-native-reanimated
jest.mock('react-native-reanimated', () => {
  const Reanimated = require('react-native-reanimated/mock');
  Reanimated.default.call = () => {};
  return Reanimated;
});

// Mock react-native-safe-area-context
jest.mock('react-native-safe-area-context', () => {
  const inset = { top: 0, right: 0, bottom: 0, left: 0 };
  return {
    SafeAreaProvider: ({ children }: { children: React.ReactNode }) => children,
    SafeAreaView: ({ children }: { children: React.ReactNode }) => children,
    useSafeAreaInsets: () => inset,
    useSafeAreaFrame: () => ({ x: 0, y: 0, width: 390, height: 844 }),
  };
});

// Mock expo-clipboard
jest.mock('expo-clipboard', () => ({
  setStringAsync: jest.fn(async () => true),
  getStringAsync: jest.fn(async () => ''),
}));

// Mock expo-font
jest.mock('expo-font', () => ({
  useFonts: () => [true, null],
  loadAsync: jest.fn(async () => {}),
  isLoaded: jest.fn(() => true),
}));

// Mock expo-splash-screen
jest.mock('expo-splash-screen', () => ({
  preventAutoHideAsync: jest.fn(async () => true),
  hideAsync: jest.fn(async () => true),
}));

// Mock @cedros/login-react-native
jest.mock('@cedros/login-react-native', () => ({
  useCedrosLogin: jest.fn(() => ({
    isAuthenticated: false,
    isLoading: false,
    getAccessToken: jest.fn(() => null),
    logout: jest.fn(async () => {}),
  })),
  useEmailAuth: jest.fn(() => ({
    login: jest.fn(async () => {}),
    register: jest.fn(async () => {}),
    isLoading: false,
    error: null,
    clearError: jest.fn(),
  })),
  useOrgs: jest.fn(() => ({
    activeOrg: null,
    orgs: [],
    orgsLoading: false,
  })),
  CedrosLoginProvider: ({ children }: { children: React.ReactNode }) => children,
  GoogleLoginButton: () => null,
}));

// Mock @cedros/pay-react-native
jest.mock('@cedros/pay-react-native', () => ({
  CedrosProvider: ({ children }: { children: React.ReactNode }) => children,
}));

// Mock @stripe/stripe-react-native
jest.mock('@stripe/stripe-react-native', () => ({
  StripeProvider: ({ children }: { children: React.ReactNode }) => children,
}));

export {};
