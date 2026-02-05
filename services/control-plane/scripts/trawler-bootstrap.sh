#!/bin/bash
# Trawling Traders Bot Bootstrap Script
# Embedded in DigitalOcean droplet user_data
# Based on OpenClaw bootstrap with Node.js 20 LTS + pnpm support

set -e

# NOTE: Do not enable `set -x` (xtrace). This script handles secrets (bootstrap token)
# and xtrace would leak them into cloud-init logs.

export DEBIAN_FRONTEND=noninteractive

# Configuration from environment (passed by provisioning service)
BOT_ID="${BOT_ID}"
BOT_NAME="${BOT_NAME}"
CONTROL_PLANE_URL="${CONTROL_PLANE_URL:-https://api.trawling-traders.com}"
BOOTSTRAP_TOKEN="${BOOTSTRAP_TOKEN}"

# Toolchain configuration (defaults for Node.js 20 LTS)
TOOLCHAIN_NODE_MAJOR="${TOOLCHAIN_NODE_MAJOR:-20}"
TOOLCHAIN_INSTALL_PNPM="${TOOLCHAIN_INSTALL_PNPM:-true}"
TOOLCHAIN_PNPM_VERSION="${TOOLCHAIN_PNPM_VERSION:-}"
TOOLCHAIN_INSTALL_RUST="${TOOLCHAIN_INSTALL_RUST:-true}"
TOOLCHAIN_RUST_TOOLCHAIN="${TOOLCHAIN_RUST_TOOLCHAIN:-stable}"
TOOLCHAIN_EXTRA_APT_PACKAGES="${TOOLCHAIN_EXTRA_APT_PACKAGES:-}"

# Workspace configuration
WORKSPACE_DIR="${WORKSPACE_DIR:-/opt/trawling-traders}"
KEYPAIR_DIR="$WORKSPACE_DIR/.config/solana"
KEYPAIR_PATH="$KEYPAIR_DIR/id.json"
SECRETS_FILE="$WORKSPACE_DIR/.secrets"

# Downrigger configuration
DOWNRIGGER_REPO_URL="${DOWNRIGGER_REPO_URL:-https://github.com/janebot2026/downrigger.git}"
DOWNRIGGER_REF="${DOWNRIGGER_REF:-main}"

echo "=== Trawling Traders Bot Setup Starting ==="
echo "Bot ID: $BOT_ID"
echo "Bot Name: $BOT_NAME"
echo "Control Plane: $CONTROL_PLANE_URL"
echo "Node.js: v$TOOLCHAIN_NODE_MAJOR LTS"
echo "Date: $(date)"

# Update system
echo "=== [1/12] Updating System ==="
apt-get update
apt-get upgrade -y

# Install base dependencies
echo "=== [2/12] Installing Base Dependencies ==="
apt-get install -y \
    curl \
    wget \
    git \
    ca-certificates \
    gnupg \
    lsb-release \
    software-properties-common \
    apt-transport-https \
    jq \
    build-essential \
    pkg-config \
    libssl-dev

# Install extra apt packages if specified
if [ -n "$TOOLCHAIN_EXTRA_APT_PACKAGES" ]; then
    echo "Installing extra packages: $TOOLCHAIN_EXTRA_APT_PACKAGES"
    # shellcheck disable=SC2086
    apt-get install -y $TOOLCHAIN_EXTRA_APT_PACKAGES
fi

# Install Node.js (modern LTS version)
echo "=== [3/12] Installing Node.js $TOOLCHAIN_NODE_MAJOR LTS ==="
if command -v node >/dev/null 2>&1; then
    NODE_MAJOR=$(node -v 2>/dev/null | sed 's/^v\([0-9]*\).*/\1/')
else
    NODE_MAJOR=0
fi

if [ "${NODE_MAJOR:-0}" -lt "${TOOLCHAIN_NODE_MAJOR}" ]; then
    curl -fsSL "https://deb.nodesource.com/setup_${TOOLCHAIN_NODE_MAJOR}.x" | bash -
    apt-get install -y nodejs
else
    echo "Node already present: $(node -v)"
fi

echo "Node version: $(node -v)"
echo "NPM version: $(npm -v)"

# Install pnpm via corepack (modern package manager)
if [ "$TOOLCHAIN_INSTALL_PNPM" = "true" ]; then
    echo "=== [4/12] Installing pnpm via corepack ==="
    corepack enable
    if [ -n "$TOOLCHAIN_PNPM_VERSION" ]; then
        corepack prepare "pnpm@$TOOLCHAIN_PNPM_VERSION" --activate
    else
        corepack prepare pnpm@latest --activate
    fi
    echo "pnpm version: $(pnpm -v)"
else
    echo "=== [4/12] Skipping pnpm installation ==="
fi

# Install Rust
if [ "$TOOLCHAIN_INSTALL_RUST" = "true" ]; then
    echo "=== [5/12] Installing Rust ($TOOLCHAIN_RUST_TOOLCHAIN) ==="
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain "$TOOLCHAIN_RUST_TOOLCHAIN"
    source "$HOME/.cargo/env"
    echo "Rust version: $(rustc --version)"
    echo "Cargo version: $(cargo --version)"
else
    echo "=== [5/12] Skipping Rust installation ==="
fi

# Create workspace directory
echo "=== [6/12] Setting up workspace ==="
mkdir -p "$WORKSPACE_DIR"
mkdir -p "$KEYPAIR_DIR"
cd "$WORKSPACE_DIR"

# Install downrigger (trading-focused agent setup tool)
echo "=== [7/12] Installing downrigger ==="
DOWNRIGGER_DIR="$WORKSPACE_DIR/tools/downrigger"
mkdir -p "$(dirname "$DOWNRIGGER_DIR")"

if [ ! -d "$DOWNRIGGER_DIR/.git" ]; then
    git clone --no-checkout "$DOWNRIGGER_REPO_URL" "$DOWNRIGGER_DIR"
fi

(
    cd "$DOWNRIGGER_DIR" || exit 1
    git fetch --depth 1 origin "$DOWNRIGGER_REF" \
        || git fetch --depth 1 origin "refs/tags/$DOWNRIGGER_REF" \
        || git fetch origin "$DOWNRIGGER_REF"
    git checkout -f FETCH_HEAD || git checkout -f "$DOWNRIGGER_REF"

    if [ "$TOOLCHAIN_INSTALL_PNPM" = "true" ]; then
        pnpm install
    else
        npm ci
    fi
)

# Run downrigger init
echo "=== [8/12] Running downrigger init ==="
(
    cd "$DOWNRIGGER_DIR"
    node bin/downrigger.js init \
        --workspace-dir "$WORKSPACE_DIR/workspace" \
        --agent-name "$BOT_NAME" \
        --yes \
        --force \
        --skip-qmd \
        --skip-cron \
        --skip-git \
        --skip-heartbeat
) || echo "WARN: downrigger init failed, continuing..."

# Install claw-trader-cli
echo "=== [9/12] Installing claw-trader-cli ==="
git clone https://github.com/janebot2026/claw-trader-cli.git "$WORKSPACE_DIR/tools/claw-trader-cli"
(
    cd "$WORKSPACE_DIR/tools/claw-trader-cli"
    source "$HOME/.cargo/env"
    cargo build --release
    cp target/release/jup-cli /usr/local/bin/
    ln -sf /usr/local/bin/jup-cli /usr/local/bin/claw-trader
)

# Fetch secrets from control plane (one-time bootstrap)
echo "=== [10/12] Fetching secrets from control plane ==="
SECRETS_JSON=""
for i in {1..5}; do
    SECRETS_JSON=$(curl -sf -X POST "$CONTROL_PLANE_URL/v1/bot/$BOT_ID/secrets" \
        -H "Content-Type: application/json" \
        -d "{\"bootstrap_token\":\"$BOOTSTRAP_TOKEN\"}") && break
    echo "Secrets fetch attempt $i failed, retrying in 5s..."
    sleep 5
done

if [ -z "$SECRETS_JSON" ]; then
    echo "FATAL: Failed to fetch secrets from control plane"
    exit 1
fi

# Parse and store secrets securely
JUPITER_API_KEY=$(echo "$SECRETS_JSON" | jq -r '.jupiter_api_key // empty')
DATA_RETRIEVAL_URL=$(echo "$SECRETS_JSON" | jq -r '.data_retrieval_url // empty')
SOLANA_RPC_URL=$(echo "$SECRETS_JSON" | jq -r '.solana_rpc_url // empty')
LLM_PROVIDER=$(echo "$SECRETS_JSON" | jq -r '.llm_provider // empty')
LLM_MODEL=$(echo "$SECRETS_JSON" | jq -r '.llm_model // empty')
LLM_API_KEY=$(echo "$SECRETS_JSON" | jq -r '.llm_api_key // empty')
TELEGRAM_BOT_TOKEN=$(echo "$SECRETS_JSON" | jq -r '.telegram_bot_token // empty')

cat > "$SECRETS_FILE" << EOFSECRETS
JUPITER_API_KEY=$JUPITER_API_KEY
DATA_RETRIEVAL_URL=$DATA_RETRIEVAL_URL
SOLANA_RPC_URL=$SOLANA_RPC_URL
LLM_PROVIDER=$LLM_PROVIDER
LLM_MODEL=$LLM_MODEL
LLM_API_KEY=$LLM_API_KEY
TELEGRAM_BOT_TOKEN=$TELEGRAM_BOT_TOKEN
EOFSECRETS
chmod 600 "$SECRETS_FILE"

# Clear bootstrap token from environment (one-time use)
unset BOOTSTRAP_TOKEN

# Configure claw-trader-cli
echo "Configuring claw-trader-cli..."
mkdir -p "$WORKSPACE_DIR/.config/claw-trader"
cat > "$WORKSPACE_DIR/.config/claw-trader/config.toml" << EOF
[api]
ultra_base_url = "https://api.jup.ag/ultra/v1"
api_key = "$JUPITER_API_KEY"

[trading]
default_slippage_bps = 50
max_slippage_bps = 100
confirmation_commitment = "confirmed"
paper_trading_default = true

[agent]
enabled = true
auto_approve = false
EOF
chmod 600 "$WORKSPACE_DIR/.config/claw-trader/config.toml"

# Install bot-runner
echo "=== [11/12] Installing bot-runner ==="
git clone https://github.com/janebot2026/trawling-traders.git "$WORKSPACE_DIR/trawling-traders"
(
    cd "$WORKSPACE_DIR/trawling-traders/services/bot-runner"
    source "$HOME/.cargo/env"
    cargo build --release
    cp target/release/bot-runner /usr/local/bin/
)

# Create systemd service
echo "=== [12/12] Creating systemd service ==="
cat > /etc/systemd/system/bot-runner.service << EOFSERVICE
[Unit]
Description=Trawling Traders Bot Runner
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=$WORKSPACE_DIR
EnvironmentFile=$SECRETS_FILE
Environment="BOT_ID=$BOT_ID"
Environment="CONTROL_PLANE_URL=$CONTROL_PLANE_URL"
Environment="RUST_LOG=info"
Environment="CLAW_TRADER_PATH=/usr/local/bin/claw-trader"
Environment="CLAW_TRADER_CONFIG=$WORKSPACE_DIR/.config/claw-trader"
Environment="AGENT_WALLET_PATH=$KEYPAIR_PATH"
ExecStart=/usr/local/bin/bot-runner
Restart=always
RestartSec=10
StandardOutput=append:/var/log/bot-runner.log
StandardError=append:/var/log/bot-runner.log

[Install]
WantedBy=multi-user.target
EOFSERVICE

# Prepare log file
touch /var/log/bot-runner.log

# Register bot with control plane
echo "Registering bot with control plane..."
curl -X POST "$CONTROL_PLANE_URL/v1/bot/$BOT_ID/register" \
    -H "Content-Type: application/json" \
    -d '{}' || echo "Registration may have failed, bot-runner will retry"

# Enable and start services
systemctl daemon-reload
systemctl enable bot-runner
systemctl start bot-runner

echo "=== Trawling Traders Bot Setup Complete ==="
echo "Bot ID: $BOT_ID"
echo "Bot Name: $BOT_NAME"
echo "Node.js: $(node -v)"
echo "Status: online"
