import React from 'react';
import { Text } from 'react-native';
import { render, fireEvent } from '../../test-utils';
import { ErrorBoundary } from '../ErrorBoundary';

// Component that throws on render
function ThrowingChild({ shouldThrow }: { shouldThrow: boolean }) {
  if (shouldThrow) throw new Error('Test explosion');
  return <Text>All good</Text>;
}

describe('ErrorBoundary', () => {
  it('renders children when no error', () => {
    const { getByText } = render(
      <ErrorBoundary>
        <Text>Hello</Text>
      </ErrorBoundary>
    );

    expect(getByText('Hello')).toBeTruthy();
  });

  it('shows fallback UI when child throws', () => {
    const { getByText } = render(
      <ErrorBoundary>
        <ThrowingChild shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(getByText('Something went wrong')).toBeTruthy();
    expect(getByText('Try Again')).toBeTruthy();
  });

  it('uses custom fallback when provided', () => {
    const { getByText, queryByText } = render(
      <ErrorBoundary fallback={<Text>Custom Error</Text>}>
        <ThrowingChild shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(getByText('Custom Error')).toBeTruthy();
    expect(queryByText('Something went wrong')).toBeNull();
  });

  it('recovers when Try Again is pressed', () => {
    // Use a ref-like pattern to control when the child throws
    let shouldThrow = true;

    function ConditionalThrower() {
      if (shouldThrow) throw new Error('Boom');
      return <Text>Recovered</Text>;
    }

    const { getByText, queryByText } = render(
      <ErrorBoundary>
        <ConditionalThrower />
      </ErrorBoundary>
    );

    expect(getByText('Something went wrong')).toBeTruthy();

    // Stop throwing before pressing Try Again
    shouldThrow = false;
    fireEvent.press(getByText('Try Again'));

    expect(getByText('Recovered')).toBeTruthy();
    expect(queryByText('Something went wrong')).toBeNull();
  });
});
