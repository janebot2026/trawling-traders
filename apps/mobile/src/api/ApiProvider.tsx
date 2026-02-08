import React, { useEffect } from 'react';
import { configureApi, setAuthProvider, AuthExpiredError } from '@trawling-traders/api-client';
import { API_URL } from '../config/api';
import { useUserStore } from '../store';

// Dynamically import Cedros to avoid issues if not available
async function getCedrosTokenManager() {
  try {
    const cedros = await import('@cedros/login-react-native');
    return cedros.TokenManager;
  } catch {
    return null;
  }
}

interface ApiProviderProps {
  children: React.ReactNode;
}

export function ApiProvider({ children }: ApiProviderProps) {
  const logout = useUserStore((state) => state.logout);

  useEffect(() => {
    // Configure API client
    configureApi({
      baseUrl: API_URL,
      dataApiUrl: API_URL.replace(':3000', ':8080'), // Data service typically on 8080
      timeoutMs: 30000,
      maxRetries: 3,
    });

    // Set up auth provider
    setAuthProvider({
      getToken: async () => {
        const TokenManager = await getCedrosTokenManager();
        if (TokenManager) {
          return await TokenManager.getAccessToken();
        }
        return null;
      },
      refreshToken: async () => {
        const TokenManager = await getCedrosTokenManager();
        if (TokenManager && typeof TokenManager.refreshAccessToken === 'function') {
          return await TokenManager.refreshAccessToken();
        }
        return null;
      },
      clearAuth: async () => {
        const TokenManager = await getCedrosTokenManager();
        if (TokenManager && typeof TokenManager.clearTokens === 'function') {
          await TokenManager.clearTokens();
        }
        logout();
      },
    });
  }, [logout]);

  return <>{children}</>;
}
