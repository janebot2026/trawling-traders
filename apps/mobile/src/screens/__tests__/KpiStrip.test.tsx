import React from 'react';
import { render } from '../../test-utils';
import { buildBot } from '../../test-utils/factories';
import { KpiStrip } from '../home/KpiStrip';

describe('KpiStrip', () => {
  it('renders all KPI labels', () => {
    const bots = [buildBot({ todayPnl: 10, totalPnl: 100, status: 'online' })];
    const { getByText } = render(
      <KpiStrip bots={bots} openTrades={2} totalTrades={5} />
    );

    expect(getByText("Today's P&L")).toBeTruthy();
    expect(getByText('Active Bots')).toBeTruthy();
    expect(getByText('Open Trades')).toBeTruthy();
    expect(getByText('Net P&L')).toBeTruthy();
    expect(getByText('Win Rate (7D)')).toBeTruthy();
  });

  it('formats positive P&L with + prefix', () => {
    const bots = [buildBot({ todayPnl: 42.5, totalPnl: 150 })];
    const { getByText } = render(
      <KpiStrip bots={bots} openTrades={0} totalTrades={0} />
    );

    expect(getByText('+$42.50')).toBeTruthy();
    expect(getByText('+$150.00')).toBeTruthy();
  });

  it('formats negative P&L without + prefix', () => {
    const bots = [buildBot({ todayPnl: -20.3, totalPnl: -100 })];
    const { getByText } = render(
      <KpiStrip bots={bots} openTrades={0} totalTrades={0} />
    );

    // Format: `${pnl >= 0 ? '+' : ''}$${pnl.toFixed(2)}` → "$-20.30"
    expect(getByText('$-20.30')).toBeTruthy();
    expect(getByText('$-100.00')).toBeTruthy();
  });

  it('handles zero/null P&L values', () => {
    const bots = [buildBot({ todayPnl: 0, totalPnl: undefined })];
    const { getAllByText } = render(
      <KpiStrip bots={bots} openTrades={0} totalTrades={0} />
    );

    // todayPnl=0 → "+$0.00", totalPnl=undefined → treated as 0 → "+$0.00"
    // Both Today's P&L and Net P&L show +$0.00
    expect(getAllByText('+$0.00')).toHaveLength(2);
  });

  it('counts active bots correctly', () => {
    const bots = [
      buildBot({ status: 'online' }),
      buildBot({ status: 'paused' }),
      buildBot({ status: 'online' }),
    ];
    const { getByText } = render(
      <KpiStrip bots={bots} openTrades={0} totalTrades={0} />
    );

    expect(getByText('2')).toBeTruthy(); // 2 online bots
  });
});
