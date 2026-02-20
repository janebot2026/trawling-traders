// API Configuration for Trawling Traders Mobile App

// Development: Use localhost when running backend locally
// Production: Use api.trawlingtraders.com

const DEV_API_URL = 'http://localhost:3000';
const PROD_API_URL = 'https://api.trawlingtraders.com';

// EXPO_PUBLIC_API_URL overrides the default dev/prod selection.
// Usage: `make mobile-liveapi` or `EXPO_PUBLIC_API_URL=https://api.trawlingtraders.com npx expo start`
const ENV_API_URL = process.env.EXPO_PUBLIC_API_URL;
export const API_URL = ENV_API_URL || (__DEV__ ? DEV_API_URL : PROD_API_URL);

// Cedros Login configuration
// SDK appends /auth/* paths, so base must include /v1 to reach /v1/auth/*
export const CEDROS_CONFIG = {
  serverUrl: `${API_URL}/v1`,
  timeout: 30000,
  retries: 3,
};

const CEDROS_PAY_SERVER_URL = API_URL;

export interface CedrosPayConfig {
  stripePublicKey: string;
  serverUrl: string;
  solanaCluster: 'mainnet-beta';
}

export const CEDROS_PAY_FALLBACK_CONFIG: CedrosPayConfig = {
  stripePublicKey: '', // Stripe buttons will be disabled until config is fetched
  serverUrl: CEDROS_PAY_SERVER_URL,
  solanaCluster: 'mainnet-beta',
};

interface CedrosPayShopConfigResponse {
  stripe?: {
    publishableKey?: string;
    enabled?: boolean;
  };
}

export async function fetchCedrosPayConfig(): Promise<CedrosPayConfig> {
  const endpoint = `${CEDROS_PAY_SERVER_URL}/paywall/v1/shop`;
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 10_000);
  let response: Response;
  try {
    response = await fetch(endpoint, { signal: controller.signal });
  } finally {
    clearTimeout(timeout);
  }
  if (!response.ok) {
    throw new Error(`HTTP ${response.status} at ${endpoint}`);
  }

  const payload = (await response.json()) as CedrosPayShopConfigResponse;
  const stripePublicKey = payload?.stripe?.publishableKey?.trim();
  const stripeEnabled = payload?.stripe?.enabled === true;

  if (!stripeEnabled || !stripePublicKey) {
    throw new Error(`Stripe publishable key unavailable from ${endpoint}`);
  }

  return {
    stripePublicKey,
    serverUrl: CEDROS_PAY_SERVER_URL,
    solanaCluster: 'mainnet-beta',
  };
}
