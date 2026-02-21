/**
 * Named constants for polling / refresh intervals used across the mobile app.
 * All values are in milliseconds.
 *
 * Using named constants here prevents magic numbers from being scattered across
 * the codebase and makes it easy to tune timing in one place.
 */
export const REFRESH_INTERVALS = {
  /** Bot list polling on the home / overview screens */
  BOT_LIST: 30000,
  /** Single bot detail screen polling */
  BOT_DETAILS: 15000,
  /** Metrics chart data refresh */
  METRICS: 60000,
  /** Bot event log refresh */
  EVENTS: 30000,
  /** Portfolio / price data refresh */
  PORTFOLIO: 30000,
  /** Network connectivity check interval */
  NETWORK_CHECK: 30000,
} as const;
