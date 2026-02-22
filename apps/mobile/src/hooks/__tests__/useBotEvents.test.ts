import { renderHook, waitFor } from '@testing-library/react-native';
import { botApi } from '@trawling-traders/api-client';
import { useBotEvents } from '../useBots';
import { buildEvent } from '../../test-utils/factories';

const mockGetEvents = botApi.getEvents as jest.Mock;

beforeEach(() => {
  jest.clearAllMocks();
});

describe('useBotEvents', () => {
  it('fetches events for given botId', async () => {
    const events = [
      buildEvent({ type: 'trade_opened' }),
      buildEvent({ type: 'trade_closed' }),
    ];
    mockGetEvents.mockResolvedValue({ events, nextCursor: 'cursor-2' });

    const { result } = renderHook(() => useBotEvents({ botId: 'bot-1' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.events).toEqual(events);
    expect(result.current.nextCursor).toBe('cursor-2');
    expect(result.current.error).toBeNull();
  });

  it('respects limit option by slicing events', async () => {
    const events = Array.from({ length: 10 }, (_, i) =>
      buildEvent({ type: i % 2 === 0 ? 'trade_opened' : 'trade_closed' })
    );
    mockGetEvents.mockResolvedValue({ events, nextCursor: undefined });

    const { result } = renderHook(() => useBotEvents({ botId: 'bot-1', limit: 3 }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.events).toHaveLength(3);
  });

  it('skips fetch when botId is empty', async () => {
    const { result } = renderHook(() => useBotEvents({ botId: '' }));

    expect(mockGetEvents).not.toHaveBeenCalled();
    expect(result.current.events).toEqual([]);
  });

  it('handles error state', async () => {
    mockGetEvents.mockRejectedValue(new Error('Events unavailable'));

    const { result } = renderHook(() => useBotEvents({ botId: 'bot-1' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error?.message).toBe('Events unavailable');
  });
});
