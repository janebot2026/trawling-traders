import React from 'react';
import { render, fireEvent, waitFor } from '../../test-utils';
import { useCedrosLogin, useEmailAuth, useOrgs } from '@cedros/login-react-native';
import { AuthScreen } from '../AuthScreen';

jest.mock('../auth/AuthScreen.styles', () => ({
  authScreenStyles: new Proxy({}, { get: (_, prop) => ({}) }),
}));

const mockLogin = jest.fn(async () => {});
const mockRegister = jest.fn(async () => {});
const mockClearError = jest.fn();

beforeEach(() => {
  jest.clearAllMocks();

  (useCedrosLogin as jest.Mock).mockReturnValue({
    isAuthenticated: false,
    isLoading: false,
    getAccessToken: jest.fn(() => null),
    logout: jest.fn(async () => {}),
  });

  (useEmailAuth as jest.Mock).mockReturnValue({
    login: mockLogin,
    register: mockRegister,
    isLoading: false,
    error: null,
    clearError: mockClearError,
  });

  (useOrgs as jest.Mock).mockReturnValue({
    activeOrg: null,
    orgs: [],
    orgsLoading: false,
  });
});

describe('AuthScreen', () => {
  it('renders sign in and sign up tabs', () => {
    const { getByText } = render(<AuthScreen />);
    expect(getByText('Sign in')).toBeTruthy();
    expect(getByText('Sign up')).toBeTruthy();
  });

  it('starts in login mode with email field', () => {
    const { getByPlaceholderText } = render(<AuthScreen />);
    expect(getByPlaceholderText('you@company.com')).toBeTruthy();
  });

  it('shows loading state when auth is loading', () => {
    (useCedrosLogin as jest.Mock).mockReturnValue({
      isAuthenticated: false,
      isLoading: true,
      getAccessToken: jest.fn(),
      logout: jest.fn(),
    });

    const { getByText } = render(<AuthScreen />);
    expect(getByText('Preparing secure trading console...')).toBeTruthy();
  });

  it('switches to register mode when Sign up pressed', () => {
    const { getByText, getByPlaceholderText } = render(<AuthScreen />);

    fireEvent.press(getByText('Sign up'));
    expect(getByPlaceholderText('Your name')).toBeTruthy();
    expect(getByText('Create account')).toBeTruthy();
  });

  it('shows inline error from auth hook', () => {
    (useEmailAuth as jest.Mock).mockReturnValue({
      login: mockLogin,
      register: mockRegister,
      isLoading: false,
      error: { message: 'Invalid credentials' },
      clearError: mockClearError,
    });

    const { getByText, getByPlaceholderText } = render(<AuthScreen />);
    // Advance to password step
    fireEvent.changeText(getByPlaceholderText('you@company.com'), 'test@example.com');
    fireEvent.press(getByText('Continue'));

    waitFor(() => {
      expect(getByText('Invalid credentials')).toBeTruthy();
    });
  });
});
