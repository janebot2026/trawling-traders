module.exports = {
  root: true,
  parser: '@typescript-eslint/parser',
  parserOptions: {
    ecmaVersion: 'latest',
    sourceType: 'module',
    ecmaFeatures: {
      jsx: true,
    },
  },
  env: {
    es2022: true,
    node: true,
    jest: true,
  },
  plugins: ['react-hooks'],
  ignorePatterns: ['node_modules/', 'dist/', 'android/', 'ios/'],
  rules: {},
};
