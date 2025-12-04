# Tycho Market Maker

A Rust market making bot that monitors price differences between reference prices (Binance, Chainlink) and on-chain liquidity pools, executing profitable trades via the [Tycho protocol](https://docs.propellerheads.xyz/).

A UI showing 2 instances running can be found at [https://stabiliser.vercel.app/](https://stabiliser.vercel.app/)

## Documentation

**[https://tycho-openmaker.gitbook.io/docs/](https://tycho-openmaker.gitbook.io/docs/)**

The documentation covers:
- **Quickstart** - Get up and running
- **Architecture** - System design and components
- **Configuration** - TOML config and environment setup
- **Algorithm** - Trading logic and price feed integration
- **UI** - Monitoring interface setup

## Quick Start

### Prerequisites

- Rust 1.70+
- PostgreSQL 14+ (for monitoring)
- Redis 6+ (for event streaming)
- RPC endpoints for target networks
- [Tycho API key](https://tycho.propellerheads.xyz/)

### Run

```bash
# Clone and build
git clone https://github.com/propeller-heads/tycho-market-maker
cd tycho-market-maker
cargo build --release

# Start market maker (example: Unichain ETH/USDC)
sh ops/maker.sh unichain.eth-usdc

# Or run directly
RUST_LOG="off,maker=trace,shd=trace" \
CONFIG_PATH=config/unichain.eth-usdc.toml \
SECRET_PATH=config/secrets/.env.unichain.eth-usdc \
cargo run --bin maker
```

Available configs: `mainnet.eth-usdc`, `unichain.eth-usdc`, `unichain.quickstart`

## Features

- **Multi-chain**: Ethereum mainnet, Unichain
- **MEV protection**: Flashbots integration for mainnet
- **Price feeds**: Binance WebSocket, Chainlink oracles
- **Monitoring**: Redis pub/sub + PostgreSQL persistence
- **Auto-recovery**: Panic recovery with restart loop

## Development

```bash
cargo test -- --nocapture
cargo fmt
cargo clippy --fix --allow-dirty --allow-staged --workspace --all-targets --all-features
```

See [CLAUDE.md](CLAUDE.md) for architecture details and development guidance.

## License

MIT

## Disclaimer

Trading cryptocurrencies involves risk of financial loss. Use at your own risk.
