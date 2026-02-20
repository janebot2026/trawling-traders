import { API_URL } from './config/api';
import {
  ApiError,
  AuthExpiredError,
  ForbiddenError,
  NetworkError,
  RateLimitError,
  ServerError,
  TimeoutError,
} from './errors';

export const API_BASE_URL = `${API_URL}/v1`;
export const DATA_API_URL = process.env.DATA_API_URL || 'http://localhost:8080';

// Pluggable auth provider (configured by host app)
export type AuthTokenProvider = () => Promise<string | null>;
export type TokenRefreshFn = () => Promise<string | null>;
export type ClearAuthFn = () => Promise<void>;

interface AuthConfig {
  getToken: AuthTokenProvider;
  refreshToken?: TokenRefreshFn;
  clearAuth?: ClearAuthFn;
}

let authConfig: AuthConfig | null = null;

export function setAuthProvider(config: AuthConfig): void {
  authConfig = config;
}

async function getAuthToken(): Promise<string | null> {
  if (!authConfig?.getToken) return null;
  return authConfig.getToken();
}

async function refreshAuthToken(): Promise<string | null> {
  if (!authConfig?.refreshToken) return null;
  return authConfig.refreshToken();
}

async function clearAuthState(): Promise<void> {
  if (!authConfig?.clearAuth) return;
  await authConfig.clearAuth();
}

// Default timeout for API requests (30 seconds)
const DEFAULT_TIMEOUT_MS = 30000;

// Retry configuration
const MAX_RETRIES = 3;
const INITIAL_RETRY_DELAY_MS = 1000;

// Helper to sleep for exponential backoff
function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

// Check if status code is retryable (5xx)
function isRetryableError(status: number): boolean {
  return status >= 500 && status < 600;
}

/**
 * Core HTTP client for the control-plane API.
 * Handles auth token injection, automatic token refresh on 401,
 * 30s timeout, and exponential-backoff retry (up to MAX_RETRIES) on
 * network errors and 5xx responses.
 *
 * @param endpoint - path relative to API_BASE_URL (e.g. "/bots")
 * @param options  - standard fetch RequestInit options
 * @param isAuthRetry - internal flag; set true after a 401 token refresh attempt
 * @param retryCount  - current retry attempt count
 * @returns parsed JSON body, or undefined for 204/205 responses
 */
export async function fetchApi(
  endpoint: string,
  options: RequestInit = {},
  isAuthRetry: boolean = false,
  retryCount: number = 0
): Promise<any> {
  const url = `${API_BASE_URL}${endpoint}`;

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...((options.headers as Record<string, string>) || {}),
  };

  // Add auth token from Cedros session
  const token = await getAuthToken();
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  // Create AbortController for timeout
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT_MS);

  // Forward caller-provided AbortSignal to the timeout controller
  if (options.signal) {
    options.signal.addEventListener('abort', () => controller.abort());
  }

  let response: Response;
  try {
    response = await fetch(url, {
      ...options,
      headers,
      signal: controller.signal,
    });
  } catch (error: unknown) {
    clearTimeout(timeoutId);
    const err = error as { name?: string; message?: string };
    if (err.name === 'AbortError') {
      throw new TimeoutError(`Request to ${endpoint} timed out after ${DEFAULT_TIMEOUT_MS}ms`);
    }
    // Retry network errors with exponential backoff
    if (retryCount < MAX_RETRIES) {
      const delay = INITIAL_RETRY_DELAY_MS * Math.pow(2, retryCount);
      await sleep(delay);
      return fetchApi(endpoint, options, isAuthRetry, retryCount + 1);
    }
    // Wrap in NetworkError — redact body to prevent secret leakage
    throw new NetworkError(`Network request to ${endpoint} failed`);
  } finally {
    clearTimeout(timeoutId);
  }

  // Handle 401 Unauthorized - attempt token refresh
  if (response.status === 401 && !isAuthRetry) {
    const newToken = await refreshAuthToken();
    if (newToken) {
      // Retry request with new token
      return fetchApi(endpoint, options, true, 0);
    } else {
      // Refresh failed - clear auth state and throw
      await clearAuthState();
      throw new AuthExpiredError();
    }
  }

  // Retry 5xx errors with exponential backoff
  if (isRetryableError(response.status) && retryCount < MAX_RETRIES) {
    const delay = INITIAL_RETRY_DELAY_MS * Math.pow(2, retryCount);
    await sleep(delay);
    return fetchApi(endpoint, options, isAuthRetry, retryCount + 1);
  }

  if (!response.ok) {
    const error = await response.text();

    // Differentiate error types for better client-side handling
    switch (response.status) {
      case 401:
        throw new AuthExpiredError(error || 'Authentication required');
      case 403:
        throw new ForbiddenError(error || 'Access denied');
      case 429: {
        const retryAfter = parseInt(response.headers.get('retry-after') || '60', 10);
        throw new RateLimitError(retryAfter);
      }
      default:
        if (response.status >= 500) {
          throw new ServerError(response.status, error || 'Server error');
        }
        throw new ApiError(response.status, error || 'API request failed');
    }
  }

  // 204 No Content and 205 Reset Content carry no body — skip JSON parsing
  if (response.status === 204 || response.status === 205) {
    return undefined;
  }

  return response.json();
}

/**
 * HTTP client for the data API (separate service, no auth).
 * Applies a 30s timeout and a single retry with 1s delay on network errors.
 *
 * @param url  - full URL to fetch
 * @param init - standard fetch RequestInit options
 * @returns parsed JSON body
 */
export async function fetchDataApi(
  url: string,
  init: RequestInit = {},
  _retryCount: number = 0,
): Promise<any> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT_MS);

  let response: Response;
  try {
    response = await fetch(url, { ...init, signal: controller.signal });
  } catch (error: unknown) {
    clearTimeout(timeoutId);
    const err = error as { name?: string };
    if (err.name === 'AbortError') {
      throw new TimeoutError(`Data API request to ${url} timed out after ${DEFAULT_TIMEOUT_MS}ms`);
    }
    // Single retry on network error (bounded to prevent infinite recursion)
    if (_retryCount < 1) {
      await sleep(INITIAL_RETRY_DELAY_MS);
      return fetchDataApi(url, init, _retryCount + 1);
    }
    throw new NetworkError(`Data API request to ${url} failed`);
  } finally {
    clearTimeout(timeoutId);
  }

  if (!response.ok) {
    throw new ApiError(response.status, `Data API request failed: ${url}`);
  }

  return response.json();
}
