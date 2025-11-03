# Tycho Market Maker

A market making bot for decentralized exchanges using the [Tycho protocol](https://tycho.propellerheads.xyz/). This system provides market making and price stabilization across multiple EVM-compatible networks including Ethereum, Base, and Unichain.

## Documentation

**📚 Full documentation is available at [https://tycho-openmaker.gitbook.io/docs/](https://tycho-openmaker.gitbook.io/docs/)**

The GitBook contains comprehensive guides including:
- Getting started and installation
- Configuration reference
- Architecture and design patterns
- Network-specific features
- Trading strategies
- Troubleshooting
- API reference

## Quick Start

### Prerequisites

- Rust 1.70+ ([rustup](https://rustup.rs/))
- PostgreSQL 14+ (for monitoring)
- Redis 6+ (for event streaming)
- RPC endpoints for target networks
- Tycho API key ([get one here](https://tycho.propellerheads.xyz/))

### Installation

```bash
# Clone the repository
git clone https://github.com/propeller-heads/tycho-market-maker
cd tycho-market-maker

# Build the project
cargo build --release
```

### Running

```bash
# Start market maker for Unichain ETH/USDC
sh ops/maker.sh unichain.eth-usdc

# Start monitoring service (in separate terminal)
sh ops/monitor.sh
```

For detailed setup instructions, configuration options, and usage examples, see the [full documentation](https://tycho-openmaker.gitbook.io/docs/).

## Features

- **Multi-Chain Support**: Ethereum mainnet, Base L2, and Unichain
- **MEV Protection**: Flashbots integration for Ethereum mainnet
- **Price Feeds**: Binance and Chainlink oracle support
- **Real-Time Monitoring**: Redis pub/sub for event streaming
- **Persistent Storage**: PostgreSQL for trade history and analytics
- **Automatic Recovery**: Panic recovery with configurable restart delays
- **Testing Mode**: Safe testing without real trades

## Development

See [CLAUDE.md](CLAUDE.md) for detailed development guidance and architecture documentation.

```bash
# Run tests
cargo test -- --nocapture

# Format code
cargo fmt

# Run linter
cargo clippy --fix --allow-dirty --allow-staged --workspace --all-targets --all-features
```

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Resources

- **Documentation**: [https://tycho-openmaker.gitbook.io/docs/](https://tycho-openmaker.gitbook.io/docs/)
- **Tycho Protocol**: [https://docs.propellerheads.xyz/](https://docs.propellerheads.xyz/)
- **Alloy Framework**: [https://github.com/alloy-rs/alloy](https://github.com/alloy-rs/alloy)
- **Flashbots**: [https://docs.flashbots.net/](https://docs.flashbots.net/)

## Disclaimer

This software is provided "as is" without warranty of any kind. Trading cryptocurrencies involves risk of financial loss. Use at your own risk.
