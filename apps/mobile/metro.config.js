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

// 4. Enable package.json "exports" resolution with React Native conditions
//    This fixes warnings from @solana/*, rpc-websockets, @noble/hashes, etc.
config.resolver.unstable_enablePackageExports = true;
config.resolver.unstable_conditionNames = [
  'react-native',
  'browser',
  'import',
  'require',
  'default',
];

// 5. Support .mjs/.cjs extensions used by Solana v2 packages
config.resolver.sourceExts = Array.from(
  new Set([...(config.resolver.sourceExts || []), 'mjs', 'cjs'])
);

// 6. Explicit module mappings
const extraNodeModules = {
  buffer: path.resolve(rootModules, 'buffer'),
  // Lightweight crypto shim (secrets.js-grempe needs randomBytes)
  crypto: path.resolve(__dirname, 'stubs/crypto-shim.js'),
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

// Auto-map @solana-mobile/* packages
const solanaMobileDir = path.resolve(rootModules, '@solana-mobile');
if (fs.existsSync(solanaMobileDir)) {
  for (const pkg of fs.readdirSync(solanaMobileDir)) {
    extraNodeModules[`@solana-mobile/${pkg}`] = path.resolve(solanaMobileDir, pkg);
  }
}

config.resolver.extraNodeModules = extraNodeModules;

module.exports = config;
