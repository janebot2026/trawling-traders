import { renderHook, waitFor } from '@testing-library/react-native';
import { botApi } from '@trawling-traders/api-client';
import { useBotMetrics } from '../useBots';
import { buildMetrics } from '../../test-utils/factories';

const mockGetMetrics = botApi.getMetrics as jest.Mock;

beforeEach(() => {
  jest.clearAllMocks();
});

describe('useBotMetrics', () => {
  it('fetches metrics for given botId', async () => {
    const metrics = buildMetrics(7);
    mockGetMetrics.mockResolvedValue({ metrics, range: '7d' });

    const { result } = renderHook(() => useBotMetrics({ botId: 'bot-1' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.metrics).toEqual(metrics);
    expect(result.current.error).toBeNull();
    expect(mockGetMetrics).toHaveBeenCalledWith('bot-1');
  });

  it('skips fetch when botId is empty', async () => {
    const { result } = renderHook(() => useBotMetrics({ botId: '' }));

    // Should not call API and should stay in non-loading state
    expect(mockGetMetrics).not.toHaveBeenCalled();
    expect(result.current.metrics).toEqual([]);
  });

  it('handles error state', async () => {
    mockGetMetrics.mockRejectedValue(new Error('Metrics unavailable'));

    const { result } = renderHook(() => useBotMetrics({ botId: 'bot-1' }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error?.message).toBe('Metrics unavailable');
    expect(result.current.metrics).toEqual([]);
  });
});
