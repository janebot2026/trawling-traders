import { renderHook, waitFor } from '@testing-library/react-native';
import { userApi, AuthExpiredError } from '@trawling-traders/api-client';
import { useUser } from '../useBots';
import { buildUser } from '../../test-utils/factories';

const mockGetCurrentUser = userApi.getCurrentUser as jest.Mock;

beforeEach(() => {
  jest.clearAllMocks();
});

describe('useUser', () => {
  it('fetches current user on mount', async () => {
    const user = buildUser();
    mockGetCurrentUser.mockResolvedValue(user);

    const { result } = renderHook(() => useUser());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.user).toEqual(user);
    expect(result.current.error).toBeNull();
  });

  it('handles generic error', async () => {
    mockGetCurrentUser.mockRejectedValue(new Error('Server error'));

    const { result } = renderHook(() => useUser());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error?.message).toBe('Server error');
  });

  it('clears user on AuthExpiredError', async () => {
    mockGetCurrentUser.mockRejectedValue(new AuthExpiredError());

    const { result } = renderHook(() => useUser());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.user).toBeNull();
    expect(result.current.error).toBeInstanceOf(AuthExpiredError);
  });

  it('wraps non-Error throws in Error', async () => {
    mockGetCurrentUser.mockRejectedValue('unexpected');

    const { result } = renderHook(() => useUser());

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.error?.message).toBe('Failed to fetch user');
  });
});
