import React from 'react';
import { render, waitFor, fireEvent } from '../../test-utils';
import { api } from '@trawling-traders/api-client';
import { buildBot, buildBotConfig, buildEvent, buildMetrics, buildChatMessage } from '../../test-utils/factories';
import { BotDetailScreen } from '../BotDetailScreen';

// Mock the styles
jest.mock('../BotDetailScreen.styles', () => ({
  styles: new Proxy({}, { get: (_, prop) => ({}) }),
}));

// Mock components that are hard to render in tests
jest.mock('../../components/OceanBackground', () => ({
  OceanBackground: ({ children }: { children: React.ReactNode }) => children,
}));
jest.mock('../../components/PnlHistoryChart', () => ({
  PnlHistoryChart: () => null,
}));

// Mock useRoute to provide botId param
jest.mock('@react-navigation/native', () => {
  const actual = jest.requireActual('@react-navigation/native');
  return {
    ...actual,
    useRoute: () => ({
      params: { botId: 'bot-123' },
    }),
  };
});

const mockGetBot = api.bot.getBot as jest.Mock;
const mockGetEvents = api.bot.getEvents as jest.Mock;
const mockGetMetrics = api.bot.getMetrics as jest.Mock;
const mockGetChatMessages = api.bot.getChatMessages as jest.Mock;

beforeEach(() => {
  jest.clearAllMocks();
});

describe('BotDetailScreen', () => {
  const setupSuccessfulFetch = () => {
    const bot = buildBot({ id: 'bot-123', name: 'My Trawler', status: 'online', totalPnl: 250.50 });
    const config = buildBotConfig({ algorithmMode: 'trend', assetFocus: 'majors', strictness: 'high' });
    const events = [
      buildEvent({ type: 'trade_opened', message: 'Opened BTC position' }),
      buildEvent({ type: 'trade_closed', message: 'Closed ETH position' }),
    ];

    mockGetBot.mockResolvedValue({ bot, config });
    mockGetEvents.mockResolvedValue({ events, nextCursor: undefined });
    mockGetMetrics.mockResolvedValue({ metrics: buildMetrics(7), range: '7d' });
    mockGetChatMessages.mockResolvedValue({ messages: [] });

    return { bot, config, events };
  };

  it('shows loading state initially', () => {
    mockGetBot.mockReturnValue(new Promise(() => {}));
    mockGetEvents.mockReturnValue(new Promise(() => {}));
    mockGetMetrics.mockReturnValue(new Promise(() => {}));
    mockGetChatMessages.mockReturnValue(new Promise(() => {}));

    const { queryByText } = render(<BotDetailScreen />);
    // Bot name should not be visible while loading
    expect(queryByText('My Trawler')).toBeNull();
  });

  it('renders bot details after fetch', async () => {
    const { bot } = setupSuccessfulFetch();

    const { getByText } = render(<BotDetailScreen />);

    await waitFor(() => {
      expect(getByText('My Trawler')).toBeTruthy();
    });
    expect(getByText('ONLINE')).toBeTruthy();
    expect(getByText('+$250.50')).toBeTruthy();
  });

  it('renders strategy section', async () => {
    setupSuccessfulFetch();

    const { getByText } = render(<BotDetailScreen />);

    await waitFor(() => {
      expect(getByText('Strategy')).toBeTruthy();
    });
    expect(getByText('Algorithm: trend')).toBeTruthy();
    expect(getByText('Asset Focus: majors')).toBeTruthy();
  });

  it('renders trade history', async () => {
    setupSuccessfulFetch();

    const { getByText } = render(<BotDetailScreen />);

    await waitFor(() => {
      expect(getByText('Trade History')).toBeTruthy();
    });
    expect(getByText('Opened BTC position')).toBeTruthy();
    expect(getByText('Closed ETH position')).toBeTruthy();
  });

  it('shows Pause button when bot is online', async () => {
    setupSuccessfulFetch();

    const { getByText } = render(<BotDetailScreen />);

    await waitFor(() => {
      expect(getByText('Pause')).toBeTruthy();
    });
  });

  it('shows empty chat prompt when no messages', async () => {
    setupSuccessfulFetch();

    const { getByText } = render(<BotDetailScreen />);

    await waitFor(() => {
      expect(getByText('No conversation yet. Ask this bot about its plan.')).toBeTruthy();
    });
  });
});
