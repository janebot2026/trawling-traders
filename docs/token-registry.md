# Token Registry

The token registry provides a unified source of truth for token metadata used across the Trawling Traders platform.

## Overview

The registry maps trading symbols to:
- **Pyth price feed IDs** - Used by the data-retrieval service for price data
- **Solana token mint addresses** - Used by the bot-runner for trade execution
- **Token metadata** - Decimals, categories, activation status

## File Locations

- **Registry**: `services/bot-runner/src/tokens/registry.json`
- **Rust Module**: `services/bot-runner/src/tokens/mod.rs`
- **Verification Script**: `scripts/verify-token-mints.sh`

## Understanding Feed IDs vs Mint Addresses

### Pyth Price Feed IDs
- Used for fetching price data from Pyth Network oracle
- Example: `49f6b65cb1de6b10eaf75e7c03ca029c306d0357e91b5311b175084a5ad55688` (AAPL)
- These are 64-character hex strings
- All xStocks and metals have these defined

### Solana Token Mint Addresses
- Used for executing actual trades on Solana
- Example: `So11111111111111111111111111111111111111112` (SOL)
- These are 32-44 character base58-encoded addresses
- xStocks and metals need these verified before trading

## Token Categories

| Category | Description | Examples |
|----------|-------------|----------|
| `crypto` | Cryptocurrency tokens | SOL, BTC, ETH, BONK |
| `stablecoin` | USD-pegged stablecoins | USDC |
| `xstock` | Tokenized stocks | AAPL, TSLA, GOOGL |
| `metal` | Precious metals | XAU (gold), XAG (silver) |

## Token Status

### Active Tokens (✅ Tradable)
Active tokens have:
- `active: true` in registry
- Verified `solana_mint` address
- Both price feeds and trade execution available

Current active tokens:
- SOL, USDC, BTC, ETH, BONK, WIF

### Inactive Tokens (⏳ Pending Verification)
Inactive tokens have:
- `active: false` in registry
- `solana_mint: null` (not verified)
- Price feeds available but trading disabled

Tokens awaiting verification:
- xStocks: AAPL, TSLA, GOOGL, AMZN, MSFT, NVDA, META, NFLX, SPY, QQQ
- Metals: XAU, XAG

## Verifying Token Mints

### Step 1: Identify Token Issuer

**For xStocks, check:**
- [Backed Finance](https://backed.fi/) - bAAPL, bTSLA, etc.
- [Dinari](https://dinari.com/) - dAAPL, dTSLA, etc.
- [Kraken](https://kraken.com/) - xAAPL, xTSLA, etc.

**For metals, check:**
- [PAXG on Solana](https://solscan.io/token/3B3cG1b0cK9z3n4v3g9v5v3m5n3p7m8o2n9q3r8t2u7v) - Paxos Gold
- [ORO token](https://solscan.io/) - Search for gold-backed tokens

### Step 2: Verify on Solana Explorer

1. Go to [Solscan](https://solscan.io/) or [Solana Explorer](https://explorer.solana.com/)
2. Search for the token symbol
3. Verify:
   - Token mint address is legitimate
   - Token has liquidity/pools
   - Token matches the intended asset

### Step 3: Update Registry

```bash
# Edit the registry
nano services/bot-runner/src/tokens/registry.json
```

Update the token entry:
```json
{
  "symbol": "AAPL",
  "name": "Apple Inc.",
  "category": "xstock",
  "pyth_feed_id": "49f6b65cb1de6b10eaf75e7c03ca029c306d0357e91b5311b175084a5ad55688",
  "solana_mint": "VERIFIED_MINT_ADDRESS_HERE",
  "decimals": 6,
  "active": true
}
```

### Step 4: Run Verification Script

```bash
./scripts/verify-token-mints.sh
```

### Step 5: Test

```bash
cd services/bot-runner
cargo test tokens::
```

## Using the Token Registry in Code

### Basic Lookup

```rust
use bot_runner::tokens::TokenRegistry;

let registry = TokenRegistry::new();

// Get token metadata
if let Some(token) = registry.get_by_symbol("SOL") {
    println!("Name: {}", token.name);
    println!("Decimals: {}", token.decimals);
    println!("Pyth Feed: {}", token.pyth_feed_id);
}

// Check if tradable
if registry.is_tradable("SOL") {
    // Can execute trades
}
```

### Legacy Compatibility

The `executor.rs` module exports backward-compatible functions:

```rust
use bot_runner::executor::{symbol_to_mint, get_token_decimals};

// These now delegate to the registry
let mint = symbol_to_mint("SOL");  // Option<&str>
let decimals = get_token_decimals("So1111...");  // u8
```

## Adding New Tokens

1. Add entry to `registry.json`:
```json
"NEWTOKEN": {
  "symbol": "NEWTOKEN",
  "name": "New Token Name",
  "category": "crypto",
  "pyth_feed_id": "PYTH_FEED_ID_HERE",
  "solana_mint": "MINT_ADDRESS_HERE",
  "decimals": 6,
  "active": true
}
```

2. Verify Pyth feed ID at [pyth.network/price-feeds](https://pyth.network/price-feeds)

3. Test with:
```bash
cargo test tokens::
```

## Registry Schema

```json
{
  "version": "1.0.0",
  "last_updated": "2026-02-12",
  "description": "...",
  "tokens": {
    "SYMBOL": {
      "symbol": "SYMBOL",
      "name": "Full Name",
      "category": "crypto|xstock|metal|stablecoin",
      "pyth_feed_id": "64-char-hex",
      "solana_mint": "base58-address-or-null",
      "decimals": 6,
      "active": true|false,
      "notes": "Optional notes"
    }
  }
}
```

## Related Documentation

- [xStocks Data Sources](./xstocks-data-sources.md) - Pyth integration details
- [Executor Module](../services/bot-runner/src/executor.rs) - Trade execution
- [Pyth Client](../services/data-retrieval/src/sources/pyth.rs) - Price feed client
