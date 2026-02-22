import React from 'react';
import { render, waitFor } from '../../test-utils';
import { api } from '@trawling-traders/api-client';
import { NetworkError, AuthExpiredError } from '@trawling-traders/api-client';
import { buildBot, buildEvent, buildMetrics } from '../../test-utils/factories';
import { HomeOverviewScreen } from '../HomeOverviewScreen';

// Mock useFocusEffect to execute callback immediately
jest.mock('@react-navigation/native', () => {
  const actual = jest.requireActual('@react-navigation/native');
  return {
    ...actual,
    useFocusEffect: (cb: () => void) => {
      const React = require('react');
      React.useEffect(() => { cb(); }, []);
    },
  };
});

jest.mock('../../hooks/useBots', () => ({
  useBotAction: () => ({
    performAction: jest.fn(async () => {}),
    loading: false,
    error: null,
  }),
}));

const mockListBots = api.bot.listBots as jest.Mock;
const mockGetMetrics = api.bot.getMetrics as jest.Mock;
const mockGetEvents = api.bot.getEvents as jest.Mock;

beforeEach(() => {
  jest.clearAllMocks();
});

describe('HomeOverviewScreen', () => {
  it('shows loading indicator initially', () => {
    mockListBots.mockReturnValue(new Promise(() => {})); // never resolves
    const { getByTestId, queryByText } = render(<HomeOverviewScreen />);
    // The screen renders an ActivityIndicator before bots load
    // Since we can't easily query ActivityIndicator, check that the fleet title is absent
    expect(queryByText('Your Fleet')).toBeNull();
  });

  it('shows onboarding when no bots exist', async () => {
    mockListBots.mockResolvedValue({ bots: [], total: 0 });

    const { queryByText } = render(<HomeOverviewScreen />);

    await waitFor(() => {
      // OnboardingSection rendered for empty bot list
      expect(queryByText('Your Fleet')).toBeNull();
    });
  });

  it('renders bot list when bots exist', async () => {
    const bots = [buildBot({ name: 'Alpha Trawler' }), buildBot({ name: 'Beta Skiff' })];
    mockListBots.mockResolvedValue({ bots, total: 2 });
    mockGetMetrics.mockResolvedValue({ metrics: buildMetrics(7), range: '7d' });
    mockGetEvents.mockResolvedValue({ events: [], nextCursor: undefined });

    const { getByText } = render(<HomeOverviewScreen />);

    await waitFor(() => {
      expect(getByText('Your Fleet')).toBeTruthy();
    });
    expect(getByText('Alpha Trawler')).toBeTruthy();
    expect(getByText('Beta Skiff')).toBeTruthy();
  });

  it('shows error on AuthExpiredError', async () => {
    mockListBots.mockRejectedValue(new AuthExpiredError());

    const { getByText } = render(<HomeOverviewScreen />);

    await waitFor(() => {
      expect(getByText('Session expired. Please log in again.')).toBeTruthy();
    });
  });

  it('shows offline error on NetworkError', async () => {
    mockListBots.mockRejectedValue(new NetworkError());

    const { getByText } = render(<HomeOverviewScreen />);

    await waitFor(() => {
      expect(getByText('You appear offline. Pull to refresh.')).toBeTruthy();
    });
  });
});
