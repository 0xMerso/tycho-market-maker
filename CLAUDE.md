# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Building
```bash
# Build specific binary
cargo build --bin maker
cargo build --bin monitor

# Build release versions
cargo build --release --bin maker
cargo build --release --bin monitor
```

### Testing
```bash
# Run all tests with output
cargo test -- --nocapture

# Run specific test file
cargo test --test config_parsing

# Run specific test function
cargo test test_basic_endpoints -- --nocapture

# Run with debug logging
RUST_LOG=debug cargo test -- --nocapture
RUST_LOG=shd=debug,maker=info cargo test -- --nocapture
```

### Code Quality
```bash
# Format code
cargo fmt

# Run clippy with fixes
cargo clippy --fix --allow-dirty --allow-staged --workspace --all-targets --all-features

# Clean artifacts and format
sh misc/clean.sh
```

### Running the System
```bash
# Start market maker with specific config (from project root)
sh ops/maker.sh base.eth-usdc
sh ops/maker.sh mainnet.eth-usdc
sh ops/maker.sh unichain.eth-usdc

# Start monitoring service
sh ops/monitor.sh

# Docker operations
sh ops/dock.sh build  # Build containers
sh ops/dock.sh up     # Start services
sh ops/dock.sh logs   # View logs
sh ops/dock.sh down   # Stop services
```

### Database Operations
```bash
# Full database setup
sh prisma/all-in-one.sh

# Individual steps
sh prisma/0.reset.sh      # Reset database
sh prisma/1.db-push.sh     # Push schema
sh prisma/2.sea-orm.sh     # Generate entities
```

## Architecture Overview

### Binary Entry Points
- **src/maker.rs**: Market maker binary with automatic restart loop and panic recovery
- **src/monitor.rs**: Monitoring service that subscribes to Redis events and persists to PostgreSQL
- **src/shd/lib.rs**: Core library with all business logic

### Core Module Structure (src/shd/)

#### maker/
Market making implementation with modular components:
- **impl.rs**: Main MarketMaker implementation with trading logic
- **feed.rs**: Price feed implementations (Binance, Chainlink) using factory pattern
- **tycho.rs**: Tycho protocol integration for order execution
- **exec/chain/**: Chain-specific execution strategies
  - mainnet.rs: Ethereum mainnet with Flashbots support
  - base.rs: Base L2 optimizations
  - unichain.rs: Unichain network support

#### data/
Data persistence and event handling:
- **neon.rs**: PostgreSQL operations via SeaORM
- **pub.rs/sub.rs**: Redis pub/sub for real-time events
- **helpers.rs**: Utility functions for data operations

#### types/
Type definitions and configurations:
- **config.rs**: MarketMakerConfig and EnvConfig structures
- **maker.rs**: Core market maker types and interfaces
- **tycho.rs**: Tycho protocol type definitions
- **builder.rs**: MarketMakerBuilder implementation

#### utils/
Utilities and helpers:
- **evm.rs**: EVM blockchain interactions (provider creation, gas estimation, balances)
- **constants.rs**: Network-specific constants and addresses
- **static.rs**: Static data management

### Configuration System

Dual configuration approach:
1. **TOML files** (config/*.toml): Market maker parameters
   - Network settings (RPC, chain ID)
   - Token addresses and symbols
   - Trading parameters (spreads, slippage)
   - Price feed configuration

2. **Environment files** (config/secrets/.env.*): Sensitive data
   - Private keys
   - API keys
   - Database URLs

### Key Architectural Patterns

1. **Factory Pattern**: Dynamic instantiation of price feeds and execution strategies based on configuration
2. **Builder Pattern**: MarketMakerBuilder assembles components with validation
3. **Event-Driven**: Redis pub/sub for monitoring and UI updates
4. **Error Recovery**: Automatic restart with configurable delays on panic
5. **Multi-Chain Support**: Abstract execution strategies with chain-specific implementations

### Critical Dependencies

- **Tycho Protocol**: tycho-simulation, tycho-common, tycho-client, tycho-execution
- **Blockchain**: Alloy framework for Ethereum interactions
- **Database**: SeaORM with PostgreSQL (Neon), Prisma for schema
- **Async Runtime**: Tokio with full feature set
- **Event System**: Redis for pub/sub messaging

### Trading Flow

1. **Initialization**: Load config, create provider, setup price feed and execution strategy
2. **Market Monitoring**: Poll Tycho API for market state at configured intervals
3. **Price Discovery**: Fetch prices from configured feed (Binance/Chainlink)
4. **Opportunity Detection**: Compare market price with pool prices using spread thresholds
5. **Execution**: Build and submit transactions via chain-specific strategy
6. **Event Publishing**: Emit trade events to Redis for monitoring
7. **Persistence**: Store trade records in PostgreSQL

### Network-Specific Considerations

- **Mainnet**: Uses Flashbots for MEV protection, higher gas limits
- **Base**: Optimized for L2 with lower gas costs
- **Unichain**: Custom configurations for Unichain network

### Testing Strategy

- Integration tests in `tests/` directory test real network connectivity
- Use `--nocapture` flag to see println! output during tests
- Test specific configurations with actual RPC endpoints
- Verify configuration parsing and endpoint availability