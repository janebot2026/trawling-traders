// Minimal crypto shim for React Native / Expo Go
// Provides randomBytes and getRandomValues via expo-crypto
// The global.crypto.getRandomValues polyfill is set in App.tsx before this loads

const randomBytes = (size) => {
  const bytes = new Uint8Array(size);
  if (typeof global.crypto !== 'undefined' && global.crypto.getRandomValues) {
    global.crypto.getRandomValues(bytes);
  } else {
    for (let i = 0; i < size; i++) {
      bytes[i] = Math.floor(Math.random() * 256);
    }
  }
  // Return Buffer-like object with slice support
  return {
    ...bytes,
    length: bytes.length,
    slice: (start, end) => bytes.slice(start, end),
    toString: (encoding) => {
      if (encoding === 'hex') {
        return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
      }
      return String.fromCharCode(...bytes);
    },
  };
};

module.exports = {
  randomBytes,
  getRandomValues: (arr) => {
    if (typeof global.crypto !== 'undefined' && global.crypto.getRandomValues) {
      return global.crypto.getRandomValues(arr);
    }
    for (let i = 0; i < arr.length; i++) {
      arr[i] = Math.floor(Math.random() * 256);
    }
    return arr;
  },
};
