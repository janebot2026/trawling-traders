import React, { useMemo, useState } from 'react';
import { StyleSheet, Text, useWindowDimensions, View } from 'react-native';
import { LineChart } from 'react-native-chart-kit';
import type { MetricPoint } from '@trawling-traders/types';
import { lightTheme } from '../theme';

interface PnlHistoryChartProps {
  metrics: MetricPoint[];
  height?: number;
}

export function PnlHistoryChart({ metrics, height = 170 }: PnlHistoryChartProps) {
  const { width: windowWidth } = useWindowDimensions();
  const chartWidth = windowWidth - 56;
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);

  const series = useMemo(() => {
    const sorted = [...metrics].sort(
      (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
    );
    const points = sorted.map((m) => m.value);
    const labels = sorted.map((m, idx) => {
      if (idx === 0 || idx === sorted.length - 1) {
        return new Date(m.timestamp).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
      }
      return '';
    });

    return {
      points,
      labels,
      sorted,
    };
  }, [metrics]);

  if (series.points.length < 2) {
    return (
      <View style={[styles.emptyState, { height }]}> 
        <Text style={styles.emptyTitle}>Not enough history yet</Text>
        <Text style={styles.emptyText}>P&L chart appears after more bot activity.</Text>
      </View>
    );
  }

  const selectedPoint = selectedIndex !== null ? series.sorted[selectedIndex] : null;

  return (
    <View>
      {selectedPoint && (
        <Text style={styles.selectedValue}>
          {selectedPoint.value >= 0 ? '+' : ''}${selectedPoint.value.toFixed(2)} ·{' '}
          {new Date(selectedPoint.timestamp).toLocaleString()}
        </Text>
      )}
      <LineChart
        data={{
          labels: series.labels,
          datasets: [{ data: series.points }],
        }}
        width={chartWidth}
        height={height}
        bezier
        withInnerLines={false}
        withOuterLines={false}
        withVerticalLines={false}
        withShadow={false}
        withHorizontalLabels={false}
        chartConfig={{
          backgroundGradientFrom: lightTheme.colors.surface,
          backgroundGradientTo: lightTheme.colors.surface,
          color: (opacity = 1) => `rgba(14, 165, 233, ${opacity})`,
          labelColor: () => lightTheme.colors.wave[500],
          decimalPlaces: 2,
          propsForDots: {
            r: '3',
            strokeWidth: '1',
            stroke: lightTheme.colors.primary[700],
          },
        }}
        style={styles.chart}
        onDataPointClick={(data) => setSelectedIndex(data.index)}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  chart: {
    marginTop: 8,
    borderRadius: 14,
  },
  emptyState: {
    borderWidth: 1,
    borderColor: lightTheme.colors.wave[200],
    borderRadius: 14,
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: lightTheme.colors.wave[50],
    padding: 20,
  },
  emptyTitle: {
    fontSize: 14,
    fontWeight: '600',
    color: lightTheme.colors.wave[700],
  },
  emptyText: {
    marginTop: 6,
    fontSize: 12,
    color: lightTheme.colors.wave[500],
    textAlign: 'center',
  },
  selectedValue: {
    fontSize: 12,
    color: lightTheme.colors.wave[600],
    marginBottom: 6,
  },
});
