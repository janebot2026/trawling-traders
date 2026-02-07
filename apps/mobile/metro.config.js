const { getDefaultConfig } = require('expo/metro-config');
const path = require('path');
const fs = require('fs');

const projectRoot = __dirname;
const workspaceRoot = path.resolve(projectRoot, '../..');
const rootModules = path.resolve(workspaceRoot, 'node_modules');

const config = getDefaultConfig(projectRoot);

// 1. Watch all files within the monorepo
config.watchFolders = [workspaceRoot];

// 2. Let Metro know where to resolve packages and in what order
config.resolver.nodeModulesPaths = [
  path.resolve(projectRoot, 'node_modules'),
  rootModules,
];

// 3. Force Metro to resolve local packages
config.resolver.disableHierarchicalLookup = false;

// 4. Explicit mappings for packages Metro can't resolve through hoisting
//    - Node stdlib polyfills (crypto, stream)
//    - @solana sub-packages that web3.js needs but Metro can't find
const extraNodeModules = {
  crypto: path.resolve(rootModules, 'crypto-browserify'),
  stream: path.resolve(rootModules, 'stream-browserify'),
  '@ledgerhq/devices/hid-framing': path.resolve(rootModules, '@ledgerhq/devices/lib/hid-framing.js'),
};

// Auto-map all @solana/* packages from root node_modules
const solanaDir = path.resolve(rootModules, '@solana');
if (fs.existsSync(solanaDir)) {
  for (const pkg of fs.readdirSync(solanaDir)) {
    extraNodeModules[`@solana/${pkg}`] = path.resolve(solanaDir, pkg);
  }
}

config.resolver.extraNodeModules = extraNodeModules;
config.resolver.unstable_enablePackageExports = true;

// Solana packages ship native ESM entrypoints (e.g. index.native.mjs).
// Expo SDK 49/Metro in this repo does not always resolve these by default.
config.resolver.sourceExts = Array.from(
  new Set([...(config.resolver.sourceExts || []), 'mjs', 'cjs'])
);

module.exports = config;
