/**
 * In-memory SecureStore mock for tests.
 * Mirrors expo-secure-store API surface.
 */
const store = new Map<string, string>();

export const getItemAsync = jest.fn(async (key: string) => store.get(key) ?? null);
export const setItemAsync = jest.fn(async (key: string, value: string) => { store.set(key, value); });
export const deleteItemAsync = jest.fn(async (key: string) => { store.delete(key); });

/** Exposed for test assertions — clear between tests with `__store.clear()`. */
export const __store = store;
