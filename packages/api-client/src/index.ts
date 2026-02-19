// Re-export error classes
export {
  ApiError,
  AuthExpiredError,
  TimeoutError,
  NetworkError,
  RateLimitError,
  ServerError,
  ForbiddenError,
} from './errors';

// Re-export auth provider types and setter
export type { AuthTokenProvider, TokenRefreshFn, ClearAuthFn } from './http';
export { setAuthProvider } from './http';

// Re-export domain APIs
export { botApi } from './bots';
export { userApi } from './auth';
export { docsApi, reportsApi } from './docs';
export { dataApi } from './data';

// Aggregated api object — preserves existing import patterns:
//   import { api } from '@trawling-traders/api-client'
//   api.bot.listBots()
import { botApi } from './bots';
import { userApi } from './auth';
import { docsApi, reportsApi } from './docs';
import { dataApi } from './data';

export const api = {
  bot: botApi,
  user: userApi,
  docs: docsApi,
  reports: reportsApi,
  data: dataApi,
};

export default api;
