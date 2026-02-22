import React from 'react';
import { render, fireEvent } from '../../test-utils';
import { buildBot } from '../../test-utils/factories';
import { BotFleetCard } from '../home/BotFleetCard';

jest.mock('../../utils/animations', () => ({
  pressScale: jest.fn(),
}));

const mockNavigate = jest.fn();
jest.mock('@react-navigation/native', () => {
  const actual = jest.requireActual('@react-navigation/native');
  return {
    ...actual,
    useNavigation: () => ({
      navigate: mockNavigate,
    }),
  };
});

beforeEach(() => {
  jest.clearAllMocks();
});

describe('BotFleetCard', () => {
  it('renders bot name and persona', () => {
    const bot = buildBot({ name: 'Alpha Trawler', assistantStyle: 'beginner' });
    const { getByText } = render(
      <BotFleetCard bot={bot} index={0} onPauseResume={jest.fn()} />
    );

    expect(getByText('Alpha Trawler')).toBeTruthy();
    expect(getByText('beginner')).toBeTruthy();
  });

  it('renders status badge', () => {
    const bot = buildBot({ status: 'online' });
    const { getByText } = render(
      <BotFleetCard bot={bot} index={0} onPauseResume={jest.fn()} />
    );

    expect(getByText('ONLINE')).toBeTruthy();
  });

  it('renders today P&L formatted correctly', () => {
    const bot = buildBot({ todayPnl: -15.75 });
    const { getByText } = render(
      <BotFleetCard bot={bot} index={0} onPauseResume={jest.fn()} />
    );

    // Format: `{pnl >= 0 ? '+' : ''}${pnl.toFixed(2)}` → "$-15.75"
    expect(getByText('$-15.75')).toBeTruthy();
  });

  it('shows Pause button for online bots', () => {
    const bot = buildBot({ status: 'online' });
    const onPauseResume = jest.fn();
    const { getByText } = render(
      <BotFleetCard bot={bot} index={0} onPauseResume={onPauseResume} />
    );

    const pauseBtn = getByText('Pause');
    expect(pauseBtn).toBeTruthy();

    fireEvent.press(pauseBtn);
    expect(onPauseResume).toHaveBeenCalledWith(bot.id, 'pause');
  });

  it('shows Resume button for paused bots', () => {
    const bot = buildBot({ status: 'paused' });
    const onPauseResume = jest.fn();
    const { getByText } = render(
      <BotFleetCard bot={bot} index={0} onPauseResume={onPauseResume} />
    );

    const resumeBtn = getByText('Resume');
    fireEvent.press(resumeBtn);
    expect(onPauseResume).toHaveBeenCalledWith(bot.id, 'resume');
  });

  it('hides pause/resume for non-pausable statuses', () => {
    const bot = buildBot({ status: 'provisioning' });
    const { queryByText } = render(
      <BotFleetCard bot={bot} index={0} onPauseResume={jest.fn()} />
    );

    expect(queryByText('Pause')).toBeNull();
    expect(queryByText('Resume')).toBeNull();
  });
});
