import React from 'react';
import { useCedrosLogin } from '@cedros/login-react-native';
import { setAuthProvider } from '@trawling-traders/api-client';

interface ApiProviderProps {
  children: React.ReactNode;
}

/** Seconds before expiry at which a token is considered near-expired. */
const TOKEN_EXPIRY_BUFFER_SECONDS = 60;

/**
 * Decodes the expiry from a JWT without verifying the signature.
 * Returns the `exp` claim in seconds, or null if the token is malformed.
 */
function getTokenExpiry(token: string): number | null {
  try {
    const parts = token.split('.');
    if (parts.length !== 3) return null;
    const payload = JSON.parse(atob(parts[1]));
    return typeof payload.exp === 'number' ? payload.exp : null;
  } catch {
    return null;
  }
}

/**
 * Returns true when the token has expired or is within
 * TOKEN_EXPIRY_BUFFER_SECONDS of expiry.
 */
function isTokenExpiredOrNearExpiry(token: string): boolean {
  const exp = getTokenExpiry(token);
  if (exp === null) return false; // Cannot determine — treat as valid
  const nowSeconds = Date.now() / 1000;
  return exp - nowSeconds < TOKEN_EXPIRY_BUFFER_SECONDS;
}

export function ApiProvider({ children }: ApiProviderProps) {
  const { getAccessToken, logout } = useCedrosLogin();

  // MB-001: singleton promise ref prevents concurrent callers from each
  // triggering a separate token fetch during startup.
  const pendingTokenPromiseRef = React.useRef<Promise<string | null> | null>(null);

  React.useEffect(() => {
    const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

    const fetchToken = async (attempts: number, delayMs: number): Promise<string | null> => {
      for (let i = 0; i < attempts; i += 1) {
        const token = getAccessToken();
        // MB-010: skip tokens that are expired or near-expired so the caller
        // is forced to refresh rather than receiving a stale credential.
        if (token && !isTokenExpiredOrNearExpiry(token)) {
          return token;
        }
        await sleep(delayMs);
      }
      return null;
    };

    // MB-001: wrap fetch in a deduplicating singleton so concurrent callers
    // share the same in-flight request instead of issuing multiple fetches.
    const deduplicatedFetch = (attempts: number, delayMs: number): Promise<string | null> => {
      if (pendingTokenPromiseRef.current) {
        return pendingTokenPromiseRef.current;
      }
      pendingTokenPromiseRef.current = fetchToken(attempts, delayMs).finally(() => {
        pendingTokenPromiseRef.current = null;
      });
      return pendingTokenPromiseRef.current;
    };

    setAuthProvider({
      getToken: () => deduplicatedFetch(4, 120),
      // SDK token manager refreshes internally; this retries token acquisition
      // across short post-login races before surfacing auth-expired errors.
      refreshToken: () => deduplicatedFetch(8, 150),
      clearAuth: async () => {
        await logout();
      },
    });
  }, [getAccessToken, logout]);

  return <>{children}</>;
}
