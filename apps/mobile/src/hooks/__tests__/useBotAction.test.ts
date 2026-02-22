import { renderHook, act } from '@testing-library/react-native';
import { botApi } from '@trawling-traders/api-client';
import { useBotAction } from '../useBots';

const mockBotAction = botApi.botAction as jest.Mock;

beforeEach(() => {
  jest.clearAllMocks();
});

describe('useBotAction', () => {
  it('calls botAction API with botId and action', async () => {
    mockBotAction.mockResolvedValue(undefined);

    const { result } = renderHook(() => useBotAction());

    await act(async () => {
      await result.current.performAction('bot-1', 'pause');
    });

    expect(mockBotAction).toHaveBeenCalledWith('bot-1', 'pause');
    expect(result.current.error).toBeNull();
  });

  it('supports all action types', async () => {
    mockBotAction.mockResolvedValue(undefined);

    const { result } = renderHook(() => useBotAction());

    for (const action of ['pause', 'resume', 'redeploy', 'destroy'] as const) {
      await act(async () => {
        await result.current.performAction('bot-1', action);
      });
      expect(mockBotAction).toHaveBeenCalledWith('bot-1', action);
    }

    expect(mockBotAction).toHaveBeenCalledTimes(4);
  });

  it('re-throws errors for caller to handle', async () => {
    const error = new Error('Bot not found');
    mockBotAction.mockRejectedValue(error);

    const { result } = renderHook(() => useBotAction());

    await expect(
      act(async () => {
        await result.current.performAction('bot-1', 'destroy');
      })
    ).rejects.toThrow('Bot not found');

    expect(result.current.error?.message).toBe('Bot not found');
  });
});
