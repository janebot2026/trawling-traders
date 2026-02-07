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

export async function fetchCedrosPayConfig(): Promise<CedrosPayConfig> {
  const response = await fetch(`${CEDROS_PAY_SERVER_URL}/paywall/v1/shop`);
  if (!response.ok) {
    throw new Error(`Failed to fetch Cedros pay shop config (HTTP ${response.status})`);
  }

  const payload = await response.json();
  const stripePublicKey = extractStripePublishableKey(payload);
  if (!stripePublicKey) {
    throw new Error('Stripe publishable key not found in /v1/pay/paywall/v1/shop response');
  }

  return {
    stripePublicKey,
    serverUrl: CEDROS_PAY_SERVER_URL,
    solanaCluster: 'mainnet-beta',
  };
}
