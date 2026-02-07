// Polyfills for React Native / Hermes
// This file MUST be imported before any other module (from index.js)

// 0. Ensure crypto.getRandomValues exists as early as possible
import 'react-native-get-random-values';

// 1. TextEncoder/TextDecoder (Hermes doesn't provide these)
import 'text-encoding-polyfill';

// 2. Buffer global (needed by Solana, crypto packages)
import { Buffer } from 'buffer';
(global as any).Buffer = Buffer;

// 3. Event class (Hermes doesn't provide the Web API Event constructor)
if (typeof global.Event === 'undefined') {
  (global as any).Event = class Event {
    type: string;
    constructor(type: string) {
      this.type = type;
    }
  };
}

// 4. crypto.getRandomValues (Expo Go can't use native modules)
import { getRandomValues } from 'expo-crypto';
if (typeof global.crypto === 'undefined') {
  (global as any).crypto = {};
}
if (!global.crypto.getRandomValues) {
  (global.crypto as any).getRandomValues = getRandomValues;
}
