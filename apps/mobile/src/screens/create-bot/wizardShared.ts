import type { AssetFocus, LlmModel, LlmProvider } from '@trawling-traders/types';

export type StrategyType = 'macro' | 'event-driven' | 'smart-money' | 'range';

export type Option<T extends string> = {
  value: T;
  label: string;
  description?: string;
  recommended?: boolean;
};

export type AssetSelectionMode = 'all' | 'custom';

export const SUBSCRIPT_DIGITS: Record<string, string> = {
  '0': '₀',
  '1': '₁',
  '2': '₂',
  '3': '₃',
  '4': '₄',
  '5': '₅',
  '6': '₆',
  '7': '₇',
  '8': '₈',
  '9': '₉',
};

export function toSubscript(value: number): string {
  return String(value)
    .split('')
    .map((char) => SUBSCRIPT_DIGITS[char] || char)
    .join('');
}

export const CAPTAIN_IMAGES = {
  trader: require('../../../../../assets/branding/tt-trader-captain.png'),
  sea: require('../../../../../assets/branding/tt-sea-captain.png'),
  rocky: require('../../../../../assets/branding/tt-rocky-captain.png'),
} as const;

export const BOAT_IMAGES = {
  live: require('../../../../../assets/branding/tt-boat-side.png'),
  paper: require('../../../../../assets/branding/tt-toy-side.png'),
} as const;

export const CATEGORY_IMAGES: Record<AssetFocus, number> = {
  tokenized_equities: require('../../../../../assets/branding/tt-stocks.png'),
  tokenized_metals: require('../../../../../assets/branding/tt-commodities.png'),
  majors: require('../../../../../assets/branding/tt-crypto-majors.png'),
  finance_2: require('../../../../../assets/branding/tt-finance-2.png'),
  memes: require('../../../../../assets/branding/tt-memecoins.png'),
  custom: require('../../../../../assets/branding/tt-finance-2.png'),
};

export const CATEGORY_COPY: Record<AssetFocus, string> = {
  tokenized_equities: 'Global stock exposure for broad directional and rotational setups.',
  tokenized_metals: 'Hard-asset markets for inflation and macro cycle positioning.',
  majors: 'Large-cap crypto pairs with deeper liquidity and tighter structure.',
  finance_2: 'On-chain finance leaders where narratives can shift quickly.',
  memes: 'High-volatility memecoin markets for aggressive momentum trawling.',
  custom: 'Custom portfolio scope for a manually curated trading universe.',
};

export function imageForCaptainKey(imageKey: string) {
  return CAPTAIN_IMAGES[imageKey as keyof typeof CAPTAIN_IMAGES] ?? CAPTAIN_IMAGES.trader;
}

export function imageForCategory(value: AssetFocus) {
  return CATEGORY_IMAGES[value] ?? CATEGORY_IMAGES['tokenized_equities'];
}

export function displayBoatName(value: string): string {
  return value
    .split('-')
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

export function hasSameTokens(left: string[], right: string[]): boolean {
  if (left.length !== right.length) {
    return false;
  }
  const rightSet = new Set(right);
  return left.every((token) => rightSet.has(token));
}

export type ModelsForProvider = { value: LlmModel; label: string }[];
export type LlmModelsMap = Record<LlmProvider, ModelsForProvider>;
