.PHONY: all help setup dev db migrate check test clean stop status

# Default target - runs everything
all: setup db migrate dev-tmux

# Colors for output
BLUE := \033[36m
GREEN := \033[32m
YELLOW := \033[33m
RED := \033[31m
RESET := \033[0m

help: ## Show this help message
	@echo "$(BLUE)Trawling Traders - Development Commands$(RESET)"
	@echo ""
	@echo "$(GREEN)Quick Start: Just run 'make' and everything starts!$(RESET)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(YELLOW)Individual Steps:$(RESET)"
	@echo "  make setup       # Install dependencies"
	@echo "  make db          # Start PostgreSQL"
	@echo "  make migrate     # Run database migrations"
	@echo "  make dev         # Show how to start services"

setup: ## Check/install dependencies
	@echo "$(BLUE)🔧 Setting up Trawling Traders...$(RESET)"
	@echo ""
	@echo "$(YELLOW)Checking Rust toolchain...$(RESET)"
	@rustc --version || (echo "$(RED)❌ Rust not found. Install from https://rustup.rs/$(RESET)" && exit 1)
	@echo "$(GREEN)✓ Rust found$(RESET)"
	@echo ""
	@echo "$(YELLOW)Checking Node.js...$(RESET)"
	@node --version || (echo "$(RED)❌ Node.js not found. Install from https://nodejs.org/$(RESET)" && exit 1)
	@echo "$(GREEN)✓ Node.js found$(RESET)"
	@echo ""
	@echo "$(YELLOW)Checking PostgreSQL...$(RESET)"
	@which psql || (echo "$(RED)❌ PostgreSQL not found. Install: sudo pacman -S postgresql$(RESET)" && exit 1)
	@echo "$(GREEN)✓ PostgreSQL found$(RESET)"
	@echo ""
	@echo "$(YELLOW)Installing Rust dependencies...$(RESET)"
	cd services/control-plane && cargo fetch
	cd services/data-retrieval && cargo fetch
	@echo "$(GREEN)✓ Rust dependencies installed$(RESET)"
	@echo ""
	@echo "$(YELLOW)Installing Node dependencies...$(RESET)"
	cd apps/mobile && npm install
	@echo "$(GREEN)✓ Node dependencies installed$(RESET)"
	@echo ""
	@echo "$(GREEN)✅ Setup complete!$(RESET)"

db: ## Start PostgreSQL database
	@echo "$(BLUE)🐘 Starting PostgreSQL...$(RESET)"
	@if pg_isready -q 2>/dev/null; then \
		echo "$(GREEN)PostgreSQL already running$(RESET)"; \
	elif command -v brew >/dev/null 2>&1; then \
		brew services start postgresql@16 2>/dev/null || brew services start postgresql 2>/dev/null || echo "$(YELLOW)⚠️  Could not start PostgreSQL via brew$(RESET)"; \
	elif command -v pg_ctl >/dev/null 2>&1; then \
		pg_ctl start -D "$${PGDATA:-/var/lib/postgresql/data}" -l /tmp/postgres.log 2>/dev/null || echo "$(YELLOW)⚠️  Could not start PostgreSQL via pg_ctl$(RESET)"; \
	else \
		echo "$(YELLOW)⚠️  Please start PostgreSQL manually$(RESET)"; \
	fi
	@echo "$(GREEN)✓ PostgreSQL started$(RESET)"

migrate: ## Run database migrations
	@echo "$(BLUE)🔄 Running migrations...$(RESET)"
	@sleep 2  # Give postgres time to be ready
	cd services/control-plane && cargo sqlx migrate run --database-url postgres://postgres:postgres@localhost/trawling_traders || (echo "$(YELLOW)⚠️  Creating database first...$(RESET)" && createdb -U postgres trawling_traders && cargo sqlx migrate run --database-url postgres://postgres:postgres@localhost/trawling_traders)
	@echo "$(GREEN)✓ Migrations complete$(RESET)"

dev: ## Show service startup commands
	@echo "$(BLUE)🚀 Starting Trawling Traders development environment...$(RESET)"
	@echo ""
	@echo "$(YELLOW)Starting services:$(RESET)"
	@echo "  1. Data Retrieval (port 8080)"
	@echo "  2. Control Plane (port 3000)"
	@echo ""
	@echo "$(GREEN)Use 'make dev-tmux' to run in tmux panes$(RESET)"
	@echo "$(GREEN)Or run these in separate terminals:$(RESET)"
	@echo ""
	@echo "  Terminal 1: make data"
	@echo "  Terminal 2: make control"
	@echo "  Terminal 3: make mobile"

dev-tmux: ## Start all services in tmux panes (interactive)
	@echo "$(BLUE)🚀 Starting services in tmux...$(RESET)"
	@echo "$(YELLOW)Press Ctrl+B then D to detach, 'tmux attach -t tt' to reattach$(RESET)"
	@sleep 1
	tmux new-session -d -s tt 'make data'
	tmux split-window -h -t tt 'make control'
	tmux split-window -v -t tt 'make mobile'
	tmux select-layout -t tt tiled
	tmux attach -t tt

data: ## Start data retrieval service (port 8080)
	@echo "$(GREEN)📊 Starting Data Retrieval on port 8080...$(RESET)"
	cd services/data-retrieval && cargo run

control: ## Start control plane API (port 3000)
	@echo "$(GREEN)🎛️  Starting Control Plane on port 3000...$(RESET)"
	cd services/control-plane && DATABASE_URL=$${DATABASE_URL:-postgres://postgres:postgres@localhost/trawling_traders} cargo run

mobile: ## Start mobile app
	@echo "$(GREEN)📱 Starting Mobile App...$(RESET)"
	cd apps/mobile && npm run start

bot-runner: ## Run bot-runner locally for testing
	@echo "$(GREEN)🤖 Starting Bot Runner...$(RESET)"
	cd services/bot-runner && cargo run -- --bot-id test-bot --control-plane http://localhost:3000

check: ## Run cargo check on all Rust services
	@echo "$(BLUE)🔍 Checking Rust code...$(RESET)"
	cd services/control-plane && cargo check
	cd services/data-retrieval && cargo check
	cd services/bot-runner && cargo check

test: ## Run all tests
	@echo "$(BLUE)🧪 Running tests...$(RESET)"
	cd services/control-plane && cargo test
	cd services/data-retrieval && cargo test
	cd services/bot-runner && cargo test
	@echo "$(GREEN)✓ All tests passed$(RESET)"

clean: ## Clean build artifacts
	@echo "$(BLUE)🧹 Cleaning build artifacts...$(RESET)"
	cd services/control-plane && cargo clean
	cd services/data-retrieval && cargo clean
	cd services/bot-runner && cargo clean
	cd apps/mobile && rm -rf node_modules
	@echo "$(GREEN)✓ Clean complete$(RESET)"

stop: ## Stop all services and database
	@echo "$(BLUE)🛑 Stopping services...$(RESET)"
	-tmux kill-session -t tt 2>/dev/null || true
	@if command -v brew >/dev/null 2>&1; then \
		brew services stop postgresql@16 2>/dev/null || brew services stop postgresql 2>/dev/null || true; \
	elif command -v pg_ctl >/dev/null 2>&1; then \
		pg_ctl stop -D "$${PGDATA:-/var/lib/postgresql/data}" 2>/dev/null || true; \
	fi
	@echo "$(GREEN)✓ Services stopped$(RESET)"

status: ## Check service status
	@echo "$(BLUE)📊 Service Status:$(RESET)"
	@echo ""
	@printf "PostgreSQL:           "
	@pg_isready -q 2>/dev/null && echo "$(GREEN)up ✓$(RESET)" || echo "$(RED)down ✗$(RESET)"
	@printf "Data Retrieval:8080:  "
	@curl -s http://localhost:8080/healthz > /dev/null 2>&1 && echo "$(GREEN)up ✓$(RESET)" || echo "$(RED)down ✗$(RESET)"
	@printf "Control Plane:3000:   "
	@curl -s http://localhost:3000/healthz > /dev/null 2>&1 && echo "$(GREEN)up ✓$(RESET)" || echo "$(RED)down ✗$(RESET)"
	@echo ""
	@echo "$(YELLOW)tmux sessions:$(RESET)"
	@tmux ls 2>/dev/null | grep tt || echo "  (none)"
