import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import AsyncStorage from '@react-native-async-storage/async-storage';
import type { Bot, User, BotConfig } from '@trawling-traders/types';

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

interface AppState {
  isOnline: boolean;
  setIsOnline: (online: boolean) => void;
}

export const useAppStore = create<AppState>()((set) => ({
  isOnline: true,
  setIsOnline: (isOnline) => set({ isOnline }),
}));
