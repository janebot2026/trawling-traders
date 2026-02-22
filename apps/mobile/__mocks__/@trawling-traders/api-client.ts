/**
 * Module mock for @trawling-traders/api-client.
 * All API functions return jest.fn(); error classes are real for instanceof checks.
 */

// Re-export real error classes so instanceof works in test assertions
export {
  ApiError,
  AuthExpiredError,
  TimeoutError,
  NetworkError,
  RateLimitError,
  ServerError,
  ForbiddenError,
} from '../../../../packages/api-client/src/errors';

export const setAuthProvider = jest.fn();

export const botApi = {
  listBots: jest.fn(),
  checkNameAvailability: jest.fn(),
  listTradeableAssets: jest.fn(),
  listAssistantOptions: jest.fn(),
  createBot: jest.fn(),
  getBot: jest.fn(),
  updateBotConfig: jest.fn(),
  botAction: jest.fn(),
  getMetrics: jest.fn(),
  getEvents: jest.fn(),
  getChatMessages: jest.fn(),
  postChatMessage: jest.fn(),
};

export const userApi = {
  getCurrentUser: jest.fn(),
  checkDisplayNameAvailability: jest.fn(),
  getSettings: jest.fn(),
  updateSettings: jest.fn(),
  getBillingSummary: jest.fn(),
};

export const docsApi = {
  getDocs: jest.fn(),
  trackDocsEvent: jest.fn(),
};

export const reportsApi = {
  emailCsvReport: jest.fn(),
};

export const dataApi = {
  getPrice: jest.fn(),
  getPricesBatch: jest.fn(),
  getSupportedSymbols: jest.fn(),
  healthCheck: jest.fn(),
};

export const api = {
  bot: botApi,
  user: userApi,
  docs: docsApi,
  reports: reportsApi,
  data: dataApi,
};

export default api;
