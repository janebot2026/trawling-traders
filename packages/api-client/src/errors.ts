// Generic API error
export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
    public data?: unknown
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

// Auth session expired error - caller should redirect to login
export class AuthExpiredError extends ApiError {
  constructor(message: string = 'Session expired. Please log in again.') {
    super(401, message);
    this.name = 'AuthExpiredError';
  }
}

// Timeout error for distinguishing from other errors
export class TimeoutError extends ApiError {
  constructor(message: string = 'Request timed out') {
    super(0, message);
    this.name = 'TimeoutError';
  }
}

// Network error for connection failures (offline, DNS, etc)
export class NetworkError extends ApiError {
  constructor(message: string = 'Network error. Please check your connection.') {
    super(0, message);
    this.name = 'NetworkError';
  }
}

// Rate limit error for 429 responses
export class RateLimitError extends ApiError {
  constructor(public retryAfter?: number) {
    super(429, 'Too many requests. Please try again later.');
    this.name = 'RateLimitError';
  }
}

// Server error for 5xx responses
export class ServerError extends ApiError {
  constructor(status: number, message: string = 'Server error. Please try again.') {
    super(status, message);
    this.name = 'ServerError';
  }
}

// Forbidden error for 403 responses
export class ForbiddenError extends ApiError {
  constructor(message: string = 'Access denied.') {
    super(403, message);
    this.name = 'ForbiddenError';
  }
}
