import { renderHook, waitFor, act } from '@testing-library/react-native';
import { botApi } from '@trawling-traders/api-client';
import { useBots } from '../useBots';
import { buildBot } from '../../test-utils/factories';

const mockListBots = botApi.listBots as jest.Mock;

beforeEach(() => {
  jest.clearAllMocks();
  jest.useFakeTimers();
});

afterEach(() => {
  jest.useRealTimers();
});

describe('useBots', () => {
  it('fetches bots on mount', async () => {
    const bots = [buildBot(), buildBot()];
    mockListBots.mockResolvedValue({ bots, total: 2 });

    const { result } = renderHook(() => useBots());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.bots).toEqual(bots);
    expect(result.current.error).toBeNull();
    expect(mockListBots).toHaveBeenCalledTimes(1);
  });

  it('sets error on fetch failure', async () => {
    mockListBots.mockRejectedValue(new Error('Network failure'));

    const { result } = renderHook(() => useBots());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error).toBeInstanceOf(Error);
    expect(result.current.error?.message).toBe('Network failure');
    expect(result.current.bots).toEqual([]);
  });

  it('wraps non-Error throws in Error', async () => {
    mockListBots.mockRejectedValue('string error');

    const { result } = renderHook(() => useBots());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error?.message).toBe('Failed to fetch bots');
  });

  it('refetch re-fetches bots', async () => {
    mockListBots.mockResolvedValue({ bots: [buildBot()], total: 1 });

    const { result } = renderHook(() => useBots());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const newBots = [buildBot(), buildBot(), buildBot()];
    mockListBots.mockResolvedValue({ bots: newBots, total: 3 });

    await act(async () => {
      await result.current.refetch();
    });

    expect(result.current.bots).toEqual(newBots);
    expect(mockListBots).toHaveBeenCalledTimes(2);
  });

  it('sets up refresh interval when specified', async () => {
    mockListBots.mockResolvedValue({ bots: [], total: 0 });

    renderHook(() => useBots({ refreshInterval: 5000 }));

    await waitFor(() => {
      expect(mockListBots).toHaveBeenCalledTimes(1);
    });

    // Advance timer to trigger interval
    act(() => {
      jest.advanceTimersByTime(5000);
    });

    await waitFor(() => {
      expect(mockListBots).toHaveBeenCalledTimes(2);
    });
  });

  it('cleans up interval on unmount', async () => {
    mockListBots.mockResolvedValue({ bots: [], total: 0 });

    const { unmount } = renderHook(() => useBots({ refreshInterval: 5000 }));

    await waitFor(() => {
      expect(mockListBots).toHaveBeenCalledTimes(1);
    });

    unmount();

    act(() => {
      jest.advanceTimersByTime(10000);
    });

    // Should not have been called again after unmount
    expect(mockListBots).toHaveBeenCalledTimes(1);
  });

  it('clears error on successful refetch', async () => {
    mockListBots.mockRejectedValueOnce(new Error('fail'));

    const { result } = renderHook(() => useBots());

    await waitFor(() => {
      expect(result.current.error).toBeTruthy();
    });

    mockListBots.mockResolvedValue({ bots: [buildBot()], total: 1 });

    await act(async () => {
      await result.current.refetch();
    });

    expect(result.current.error).toBeNull();
    expect(result.current.bots).toHaveLength(1);
  });
});
