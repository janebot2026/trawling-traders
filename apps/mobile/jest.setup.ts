/**
 * Post-framework setup: testing library matchers and global test utilities.
 */
import '@testing-library/jest-native/extend-expect';

// Silence console.warn/error in tests unless debugging
if (!process.env.DEBUG_TESTS) {
  jest.spyOn(console, 'warn').mockImplementation(() => {});
  jest.spyOn(console, 'error').mockImplementation(() => {});
}

// Global fetch mock — individual tests override as needed
global.fetch = jest.fn(async () =>
  new Response(JSON.stringify({}), { status: 200 })
) as jest.Mock;

// Provide __DEV__ global
(global as Record<string, unknown>).__DEV__ = true;
