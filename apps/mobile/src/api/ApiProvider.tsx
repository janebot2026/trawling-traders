import React from 'react';
import { useCedrosLogin } from '@cedros/login-react-native';
import { setAuthProvider } from '@trawling-traders/api-client';

interface ApiProviderProps {
  children: React.ReactNode;
}

export function ApiProvider({ children }: ApiProviderProps) {
  const { getAccessToken, logout } = useCedrosLogin();

  React.useEffect(() => {
    const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

    const waitForToken = async (attempts: number, delayMs: number): Promise<string | null> => {
      for (let i = 0; i < attempts; i += 1) {
        const token = getAccessToken();
        if (token) {
          return token;
        }
        await sleep(delayMs);
      }
      return null;
    };

    setAuthProvider({
      getToken: async () => waitForToken(4, 120),
      // SDK token manager refreshes internally; this retries token acquisition
      // across short post-login races before surfacing auth-expired errors.
      refreshToken: async () => waitForToken(8, 150),
      clearAuth: async () => {
        await logout();
      },
    });
  }, [getAccessToken, logout]);

  return <>{children}</>;
}
