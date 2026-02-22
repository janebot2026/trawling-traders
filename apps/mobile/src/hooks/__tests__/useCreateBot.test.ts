import { renderHook, act } from '@testing-library/react-native';
import { botApi } from '@trawling-traders/api-client';
import { useCreateBot } from '../useBots';
import { buildBot } from '../../test-utils/factories';

const mockCreateBot = botApi.createBot as jest.Mock;

beforeEach(() => {
  jest.clearAllMocks();
});

const createRequest = {
  name: 'Test Trawler',
  assetFocus: 'majors' as const,
  algorithmMode: 'trend' as const,
  strictness: 'high' as const,
  tradingMode: 'paper' as const,
  llmProvider: 'openai' as const,
  llmModel: 'gpt-4o' as const,
  llmApiKey: 'sk-test',
  riskCaps: {
    maxPositionSizePercent: 10,
    maxDailyLossUsd: 500,
    maxDrawdownPercent: 15,
    maxTradesPerDay: 20,
  },
};

describe('useCreateBot', () => {
  it('calls createBot API and returns created bot', async () => {
    const created = buildBot({ name: 'Test Trawler' });
    mockCreateBot.mockResolvedValue(created);

    const { result } = renderHook(() => useCreateBot());

    let returnedBot: any;
    await act(async () => {
      returnedBot = await result.current.createBot(createRequest);
    });

    expect(returnedBot).toEqual(created);
    expect(result.current.error).toBeNull();
    expect(mockCreateBot).toHaveBeenCalledWith(createRequest);
  });

  it('sets loading during creation', async () => {
    let resolveFn: (value: any) => void;
    mockCreateBot.mockReturnValue(new Promise((resolve) => { resolveFn = resolve; }));

    const { result } = renderHook(() => useCreateBot());

    act(() => {
      result.current.createBot(createRequest).catch(() => {});
    });

    // loading should be true while promise is pending
    expect(result.current.loading).toBe(true);

    await act(async () => {
      resolveFn!(buildBot());
    });

    expect(result.current.loading).toBe(false);
  });

  it('re-throws errors for caller to handle', async () => {
    const error = new Error('Quota exceeded');
    mockCreateBot.mockRejectedValue(error);

    const { result } = renderHook(() => useCreateBot());

    await expect(
      act(async () => {
        await result.current.createBot(createRequest);
      })
    ).rejects.toThrow('Quota exceeded');

    expect(result.current.error?.message).toBe('Quota exceeded');
  });
});
