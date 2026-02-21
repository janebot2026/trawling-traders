import { create } from 'zustand';
import { persist, createJSONStorage, type StateStorage } from 'zustand/middleware';
import AsyncStorage from '@react-native-async-storage/async-storage';
import * as SecureStore from 'expo-secure-store';
import type { User, LlmProvider, LlmModel } from '@trawling-traders/types';

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
