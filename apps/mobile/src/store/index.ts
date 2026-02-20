import { create } from 'zustand';
import { persist, createJSONStorage, type StateStorage } from 'zustand/middleware';
import AsyncStorage from '@react-native-async-storage/async-storage';
import * as SecureStore from 'expo-secure-store';
import type { Bot, User, BotConfig, LlmProvider, LlmModel } from '@trawling-traders/types';

/** Zustand-compatible storage backed by expo-secure-store (encrypted at rest). */
const secureStorage: StateStorage = {
  getItem: async (name: string) => {
    return SecureStore.getItemAsync(name);
  },
  setItem: async (name: string, value: string) => {
    await SecureStore.setItemAsync(name, value);
  },
  removeItem: async (name: string) => {
    await SecureStore.deleteItemAsync(name);
  },
};

interface BotsState {
  bots: Bot[];
  selectedBotId: string | null;
  isLoading: boolean;
  error: string | null;
  setBots: (bots: Bot[]) => void;
  addBot: (bot: Bot) => void;
  updateBot: (botId: string, updates: Partial<Bot>) => void;
  removeBot: (botId: string) => void;
  selectBot: (botId: string | null) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
}

export const useBotsStore = create<BotsState>()(
  persist(
    (set) => ({
      bots: [],
      selectedBotId: null,
      isLoading: false,
      error: null,
      setBots: (bots) => set({ bots }),
      addBot: (bot) => set((state) => ({ bots: [...state.bots, bot] })),
      updateBot: (botId, updates) =>
        set((state) => ({
          bots: state.bots.map((b) =>
            b.id === botId ? { ...b, ...updates } : b
          ),
        })),
      removeBot: (botId) =>
        set((state) => ({
          bots: state.bots.filter((b) => b.id !== botId),
          selectedBotId: state.selectedBotId === botId ? null : state.selectedBotId,
        })),
      selectBot: (botId) => set({ selectedBotId: botId }),
      setLoading: (isLoading) => set({ isLoading }),
      setError: (error) => set({ error }),
    }),
    {
      name: 'bots-storage',
      storage: createJSONStorage(() => AsyncStorage),
      partialize: (state) => ({ bots: state.bots, selectedBotId: state.selectedBotId }),
    }
  )
);

interface UserState {
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  setUser: (user: User | null) => void;
  setLoading: (loading: boolean) => void;
  logout: () => void;
}

export const useUserStore = create<UserState>()(
  persist(
    (set) => ({
      user: null,
      isAuthenticated: false,
      isLoading: true,
      setUser: (user) =>
        set({
          user,
          isAuthenticated: !!user,
          isLoading: false,
        }),
      setLoading: (isLoading) => set({ isLoading }),
      logout: () => set({ user: null, isAuthenticated: false }),
    }),
    {
      name: 'user-storage',
      storage: createJSONStorage(() => AsyncStorage),
      partialize: (state) => ({ user: state.user, isAuthenticated: state.isAuthenticated }),
    }
  )
);

interface PricesState {
  prices: Record<string, { price: string; timestamp: string; source: string }>;
  lastUpdated: Record<string, number>;
  setPrice: (symbol: string, data: { price: string; timestamp: string; source: string }) => void;
  getPrice: (symbol: string) => { price: string; timestamp: string; source: string } | undefined;
  isStale: (symbol: string, maxAgeMs?: number) => boolean;
}

const PRICE_STALE_THRESHOLD = 60000; // 60 seconds

export const usePricesStore = create<PricesState>()((set, get) => ({
  prices: {},
  lastUpdated: {},
  setPrice: (symbol, data) =>
    set((state) => ({
      prices: { ...state.prices, [symbol]: data },
      lastUpdated: { ...state.lastUpdated, [symbol]: Date.now() },
    })),
  getPrice: (symbol) => get().prices[symbol],
  isStale: (symbol, maxAgeMs = PRICE_STALE_THRESHOLD) => {
    const lastUpdate = get().lastUpdated[symbol];
    if (!lastUpdate) return true;
    return Date.now() - lastUpdate > maxAgeMs;
  },
}));

interface SettingsState {
  apiKeys: Partial<Record<LlmProvider, string>>;
  preferredModels: Partial<Record<LlmProvider, LlmModel>>;
  disabledCustodians: string[];
  setApiKey: (provider: LlmProvider, key: string) => void;
  removeApiKey: (provider: LlmProvider) => void;
  clearApiKeys: () => void;
  setPreferredModel: (provider: LlmProvider, model: LlmModel) => void;
  toggleCustodian: (custodian: string) => void;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      apiKeys: {},
      preferredModels: {},
      disabledCustodians: [],
      setApiKey: (provider, key) =>
        set((state) => ({ apiKeys: { ...state.apiKeys, [provider]: key } })),
      removeApiKey: (provider) =>
        set((state) => {
          const next = { ...state.apiKeys };
          delete next[provider];
          return { apiKeys: next };
        }),
      clearApiKeys: () => set({ apiKeys: {} }),
      setPreferredModel: (provider, model) =>
        set((state) => ({ preferredModels: { ...state.preferredModels, [provider]: model } })),
      toggleCustodian: (custodian) =>
        set((state) => ({
          disabledCustodians: state.disabledCustodians.includes(custodian)
            ? state.disabledCustodians.filter((c) => c !== custodian)
            : [...state.disabledCustodians, custodian],
        })),
    }),
    {
      name: 'settings-preferences',
      storage: createJSONStorage(() => secureStorage),
      // apiKeys contain sensitive credentials and must not be persisted
      partialize: (state) => ({
        preferredModels: state.preferredModels,
        disabledCustodians: state.disabledCustodians,
      }),
    }
  )
);

interface AppState {
  isOnline: boolean;
  setIsOnline: (online: boolean) => void;
}

export const useAppStore = create<AppState>()((set) => ({
  isOnline: true,
  setIsOnline: (isOnline) => set({ isOnline }),
}));
