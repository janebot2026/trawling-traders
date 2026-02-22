import { renderHook, waitFor, act } from '@testing-library/react-native';
import { botApi } from '@trawling-traders/api-client';
import { useBot } from '../useBots';
import { buildBot, buildBotConfig } from '../../test-utils/factories';

const mockGetBot = botApi.getBot as jest.Mock;

beforeEach(() => {
  jest.clearAllMocks();
});

describe('useBot', () => {
  it('fetches bot and config by ID', async () => {
    const bot = buildBot({ id: 'bot-1' });
    const config = buildBotConfig({ botId: 'bot-1' });
    mockGetBot.mockResolvedValue({ bot, config });

    const { result } = renderHook(() => useBot({ botId: 'bot-1' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.bot).toEqual(bot);
    expect(result.current.config).toEqual(config);
    expect(result.current.error).toBeNull();
    expect(mockGetBot).toHaveBeenCalledWith('bot-1');
  });

  it('skips fetch when botId is empty', async () => {
    const { result } = renderHook(() => useBot({ botId: '' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.bot).toBeNull();
    expect(mockGetBot).not.toHaveBeenCalled();
  });

  it('handles fetch error', async () => {
    mockGetBot.mockRejectedValue(new Error('Not found'));

    const { result } = renderHook(() => useBot({ botId: 'bot-1' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error?.message).toBe('Not found');
    expect(result.current.bot).toBeNull();
  });

  it('wraps non-Error throws in Error', async () => {
    mockGetBot.mockRejectedValue('string error');

    const { result } = renderHook(() => useBot({ botId: 'bot-1' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error?.message).toBe('Failed to fetch bot');
  });

  it('refetch reloads data', async () => {
    const bot = buildBot({ id: 'bot-1' });
    mockGetBot.mockResolvedValue({ bot, config: null });

    const { result } = renderHook(() => useBot({ botId: 'bot-1' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    const updated = buildBot({ id: 'bot-1', name: 'Updated' });
    mockGetBot.mockResolvedValue({ bot: updated, config: null });

    await act(async () => {
      await result.current.refetch();
    });

    expect(result.current.bot?.name).toBe('Updated');
  });

  it('sets up refresh interval when specified', async () => {
    jest.useFakeTimers();
    mockGetBot.mockResolvedValue({ bot: buildBot(), config: null });

    renderHook(() => useBot({ botId: 'bot-1', refreshInterval: 3000 }));

    await waitFor(() => {
      expect(mockGetBot).toHaveBeenCalledTimes(1);
    });

    act(() => {
      jest.advanceTimersByTime(3000);
    });

    await waitFor(() => {
      expect(mockGetBot).toHaveBeenCalledTimes(2);
    });

    jest.useRealTimers();
  });
});
