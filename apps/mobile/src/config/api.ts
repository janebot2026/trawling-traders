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

const CEDROS_PAY_SERVER_URL = `${API_URL}/v1/pay`;

export interface CedrosPayConfig {
  stripePublicKey: string;
  serverUrl: string;
  solanaCluster: 'mainnet-beta';
}

function extractStripePublishableKey(payload: unknown): string | null {
  if (!payload || typeof payload !== 'object') return null;
  const obj = payload as Record<string, unknown>;

  const directCandidates = [
    obj.publishableKey,
    obj.publishable_key,
    obj.stripePublicKey,
    obj.stripe_public_key,
    obj.stripePublishableKey,
    obj.publicKey,
  ];

  for (const candidate of directCandidates) {
    if (typeof candidate === 'string' && candidate.trim().length > 0) {
      return candidate.trim();
    }
  }

  const nestedCandidates = [obj.config, obj.stripe, obj.data];
  for (const nested of nestedCandidates) {
    const extracted = extractStripePublishableKey(nested);
    if (extracted) return extracted;
  }

  return null;
}

// Fallback config used when dynamic fetch fails (e.g. server unreachable)
export const CEDROS_PAY_FALLBACK_CONFIG: CedrosPayConfig = {
  stripePublicKey: '', // Stripe buttons will be disabled until config is fetched
  serverUrl: CEDROS_PAY_SERVER_URL,
  solanaCluster: 'mainnet-beta',
};

export async function fetchCedrosPayConfig(): Promise<CedrosPayConfig> {
  // cedros-pay /paywall/v1/shop returns shop config including Stripe publishable key
  const response = await fetch(`${CEDROS_PAY_SERVER_URL}/paywall/v1/shop`);
  if (!response.ok) {
    throw new Error(`Failed to fetch Cedros pay config (HTTP ${response.status})`);
  }

  const payload = await response.json();
  const stripePublicKey = extractStripePublishableKey(payload);
  if (!stripePublicKey) {
    console.warn('Stripe publishable key not found in /paywall/v1/shop response');
    return CEDROS_PAY_FALLBACK_CONFIG;
  }

  return {
    stripePublicKey,
    serverUrl: CEDROS_PAY_SERVER_URL,
    solanaCluster: 'mainnet-beta',
  };
}
