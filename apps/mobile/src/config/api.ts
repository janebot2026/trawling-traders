// API Configuration for Trawling Traders Mobile App

// Development: Use localhost when running backend locally
// Production: Use api.trawlingtraders.com

const DEV_API_URL = 'http://localhost:3000';
const PROD_API_URL = 'https://api.trawlingtraders.com';

// Set to true for local development
const IS_DEV = __DEV__;

export const API_URL = IS_DEV ? DEV_API_URL : PROD_API_URL;

// Cedros Login configuration
// SDK appends /auth/* paths, so base must include /v1 to reach /v1/auth/*
export const CEDROS_CONFIG = {
  serverUrl: `${API_URL}/v1`,
  timeout: 30000,
  retries: 3,
};

// Cedros Pay configuration
// SDK appends /paywall/v1/* paths, so base must include /v1/pay to reach /v1/pay/paywall/v1/*
export const CEDROS_PAY_CONFIG = {
  apiUrl: `${API_URL}/v1/pay`,
};
