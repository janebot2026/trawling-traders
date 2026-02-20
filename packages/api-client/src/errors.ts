// Generic API error
export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
    public data?: unknown
  ) {
    super(message);
    this.name = 'ApiError';
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

// Auth session expired error - caller should redirect to login
export class AuthExpiredError extends ApiError {
  constructor(message: string = 'Session expired. Please log in again.') {
    super(401, message);
    this.name = 'AuthExpiredError';
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

// Timeout error for distinguishing from other errors
export class TimeoutError extends ApiError {
  constructor(message: string = 'Request timed out') {
    super(0, message);
    this.name = 'TimeoutError';
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

// Network error for connection failures (offline, DNS, etc)
export class NetworkError extends ApiError {
  constructor(message: string = 'Network error. Please check your connection.') {
    super(0, message);
    this.name = 'NetworkError';
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

// Rate limit error for 429 responses
export class RateLimitError extends ApiError {
  constructor(public retryAfter?: number) {
    super(429, 'Too many requests. Please try again later.');
    this.name = 'RateLimitError';
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

// Server error for 5xx responses
export class ServerError extends ApiError {
  constructor(status: number, message: string = 'Server error. Please try again.') {
    super(status, message);
    this.name = 'ServerError';
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

// Forbidden error for 403 responses
export class ForbiddenError extends ApiError {
  constructor(message: string = 'Access denied.') {
    super(403, message);
    this.name = 'ForbiddenError';
    Object.setPrototypeOf(this, new.target.prototype);
  }
}
