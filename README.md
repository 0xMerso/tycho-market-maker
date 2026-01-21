# Tycho Stabiliser

A Rust-based market maker that monitors price discrepancies between any source of price (Binance, Chainlink) and on-chain liquidity pools, executing trades to align pool prices with reference prices via the [Tycho protocol](https://docs.propellerheads.xyz/).

Unlike basic arbitrage bots that exploit spreads when profitable, this stabiliser actively works to enforce price alignment, particularly suited for stablecoin pairs and liquid trading pairs with strong price discovery on any source of price.

## Documentation

**[https://tycho-stabiliser.gitbook.io/docs/](https://tycho-stabiliser.gitbook.io/docs/)**  

## Live Demo

A live UI showing running instances: **[https://stabiliser.vercel.app/](https://stabiliser.vercel.app/)**

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

- **Multi-chain**: Ethereum mainnet, Unichain, Base
- **MEV protection**: Flashbots integration (with Flashblocks support on Unichain and Base)
- **Price feeds**: Real-time Binance WebSocket, Chainlink oracles
- **Monitoring**: Redis pub/sub + PostgreSQL persistence for trade history
- **Auto-recovery**: Automatic error handling with restart capabilities

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

Trading crypto involves risk of financial loss. Use at your own risk.
