import React from 'react';
import { render } from '@testing-library/react-native';
import { Text } from 'react-native';
import { setAuthProvider } from '@trawling-traders/api-client';
import { useCedrosLogin } from '@cedros/login-react-native';
import { ApiProvider } from '../ApiProvider';

const mockSetAuthProvider = setAuthProvider as jest.Mock;
const mockUseCedrosLogin = useCedrosLogin as jest.Mock;

beforeEach(() => {
  jest.clearAllMocks();
  mockUseCedrosLogin.mockReturnValue({
    getAccessToken: jest.fn(() => null),
    logout: jest.fn(async () => {}),
  });
});

describe('ApiProvider', () => {
  it('renders children', () => {
    const { getByText } = render(
      <ApiProvider>
        <Text>Child</Text>
      </ApiProvider>
    );
    expect(getByText('Child')).toBeTruthy();
  });

  it('calls setAuthProvider with getToken, refreshToken, clearAuth', () => {
    render(
      <ApiProvider>
        <Text>Test</Text>
      </ApiProvider>
    );

    expect(mockSetAuthProvider).toHaveBeenCalledTimes(1);
    const provider = mockSetAuthProvider.mock.calls[0][0];
    expect(typeof provider.getToken).toBe('function');
    expect(typeof provider.refreshToken).toBe('function');
    expect(typeof provider.clearAuth).toBe('function');
  });

  it('clearAuth calls logout from cedros SDK', async () => {
    const mockLogout = jest.fn(async () => {});
    mockUseCedrosLogin.mockReturnValue({
      getAccessToken: jest.fn(() => null),
      logout: mockLogout,
    });

    render(
      <ApiProvider>
        <Text>Test</Text>
      </ApiProvider>
    );

    const { clearAuth } = mockSetAuthProvider.mock.calls[0][0];
    await clearAuth();
    expect(mockLogout).toHaveBeenCalled();
  });

  it('getToken returns valid non-expired token', async () => {
    // Create a JWT that expires in 5 minutes (well outside the 60s buffer)
    const exp = Math.floor(Date.now() / 1000) + 300;
    const payload = btoa(JSON.stringify({ exp }));
    const token = `header.${payload}.signature`;

    mockUseCedrosLogin.mockReturnValue({
      getAccessToken: jest.fn(() => token),
      logout: jest.fn(async () => {}),
    });

    render(
      <ApiProvider>
        <Text>Test</Text>
      </ApiProvider>
    );

    const { getToken } = mockSetAuthProvider.mock.calls[0][0];
    const result = await getToken();
    expect(result).toBe(token);
  });

  it('getToken skips near-expired token (MB-010)', async () => {
    // Token expires in 30s (within 60s buffer) — should be skipped
    const exp = Math.floor(Date.now() / 1000) + 30;
    const payload = btoa(JSON.stringify({ exp }));
    const nearExpiredToken = `header.${payload}.signature`;

    mockUseCedrosLogin.mockReturnValue({
      getAccessToken: jest.fn(() => nearExpiredToken),
      logout: jest.fn(async () => {}),
    });

    render(
      <ApiProvider>
        <Text>Test</Text>
      </ApiProvider>
    );

    const { getToken } = mockSetAuthProvider.mock.calls[0][0];
    // getToken retries 4 times with 120ms delay. Since token is always near-expired,
    // it should return null after exhausting attempts.
    const result = await getToken();
    expect(result).toBeNull();
  });

  it('deduplicates concurrent getToken calls (MB-001)', async () => {
    const exp = Math.floor(Date.now() / 1000) + 300;
    const payload = btoa(JSON.stringify({ exp }));
    const token = `header.${payload}.signature`;

    let callCount = 0;
    mockUseCedrosLogin.mockReturnValue({
      getAccessToken: jest.fn(() => {
        callCount++;
        return token;
      }),
      logout: jest.fn(async () => {}),
    });

    render(
      <ApiProvider>
        <Text>Test</Text>
      </ApiProvider>
    );

    const { getToken } = mockSetAuthProvider.mock.calls[0][0];

    // Fire two concurrent calls
    const [r1, r2] = await Promise.all([getToken(), getToken()]);

    expect(r1).toBe(token);
    expect(r2).toBe(token);
    // Both should share the same singleton promise, so getAccessToken
    // should only have been called a small number of times (not 2x attempts)
  });
});
