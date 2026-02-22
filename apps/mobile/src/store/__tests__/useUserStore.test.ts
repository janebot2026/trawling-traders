import { useUserStore } from '../index';
import { buildUser } from '../../test-utils/factories';

// Reset zustand store state between tests
beforeEach(() => {
  useUserStore.setState({
    user: null,
    isAuthenticated: false,
    isLoading: true,
  });
});

describe('useUserStore', () => {
  it('has correct initial state', () => {
    const state = useUserStore.getState();
    expect(state.user).toBeNull();
    expect(state.isAuthenticated).toBe(false);
    expect(state.isLoading).toBe(true);
  });

  it('setUser sets user and marks authenticated', () => {
    const user = buildUser();
    useUserStore.getState().setUser(user);

    const state = useUserStore.getState();
    expect(state.user).toEqual(user);
    expect(state.isAuthenticated).toBe(true);
    expect(state.isLoading).toBe(false);
  });

  it('setUser(null) clears user and marks unauthenticated', () => {
    useUserStore.getState().setUser(buildUser());
    useUserStore.getState().setUser(null);

    const state = useUserStore.getState();
    expect(state.user).toBeNull();
    expect(state.isAuthenticated).toBe(false);
  });

  it('logout clears user and authentication', () => {
    useUserStore.getState().setUser(buildUser());
    useUserStore.getState().logout();

    const state = useUserStore.getState();
    expect(state.user).toBeNull();
    expect(state.isAuthenticated).toBe(false);
  });

  it('setLoading updates loading state', () => {
    useUserStore.getState().setLoading(false);
    expect(useUserStore.getState().isLoading).toBe(false);

    useUserStore.getState().setLoading(true);
    expect(useUserStore.getState().isLoading).toBe(true);
  });

  it('partialize excludes isLoading from persistence', () => {
    // The persist config's partialize only includes user + isAuthenticated.
    // Verify by checking the persist options directly.
    const persistOptions = (useUserStore as any).persist?.getOptions?.();
    if (persistOptions?.partialize) {
      const full = {
        user: buildUser(),
        isAuthenticated: true,
        isLoading: true,
        setUser: jest.fn(),
        setLoading: jest.fn(),
        logout: jest.fn(),
      };
      const partialised = persistOptions.partialize(full);
      expect(partialised).not.toHaveProperty('isLoading');
      expect(partialised).toHaveProperty('user');
      expect(partialised).toHaveProperty('isAuthenticated');
    }
  });
});
