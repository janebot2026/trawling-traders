import { useAppStore } from '../index';

beforeEach(() => {
  useAppStore.setState({ isOnline: true });
});

describe('useAppStore', () => {
  it('defaults to online', () => {
    expect(useAppStore.getState().isOnline).toBe(true);
  });

  it('setIsOnline(false) marks offline', () => {
    useAppStore.getState().setIsOnline(false);
    expect(useAppStore.getState().isOnline).toBe(false);
  });

  it('setIsOnline(true) marks online again', () => {
    useAppStore.getState().setIsOnline(false);
    useAppStore.getState().setIsOnline(true);
    expect(useAppStore.getState().isOnline).toBe(true);
  });

  it('has no persistence middleware', () => {
    // useAppStore is a plain store without persist
    const hasPersist = !!(useAppStore as any).persist;
    expect(hasPersist).toBe(false);
  });
});
