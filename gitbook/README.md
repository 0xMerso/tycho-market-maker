# Tycho Market Maker

## 🚀 Overview

Tycho Market Maker is a high-performance automated market making bot built for decentralized exchanges (DEXs) on EVM-compatible blockchains. It leverages the Tycho Protocol to provide liquidity and capture arbitrage opportunities across multiple networks including Ethereum, Base, and Unichain.

## 🎯 Purpose

The market maker serves two primary functions:

1. **Price Stabilization**: Maintains price equilibrium between different liquidity pools and external reference prices
2. **Arbitrage Capture**: Identifies and executes profitable trading opportunities when price discrepancies arise

## ✨ Key Features

- **Multi-Network Support**: Deploy on Ethereum mainnet, Base L2, and Unichain
- **Real-Time Price Monitoring**: Streams pool states and reacts to price movements in milliseconds
- **Flexible Price Feeds**: Supports multiple price sources including Binance API and Chainlink oracles
- **Smart Execution**: Network-specific execution strategies with MEV protection on mainnet
- **Risk Management**: Built-in inventory controls, slippage protection, and position limits
- **Monitoring Suite**: Redis-based event system with PostgreSQL persistence for trade analytics

## 🏗️ Architecture

The system consists of two main components:

### Market Maker Binary (`maker`)
- Connects to Tycho Protocol for real-time pool data
- Monitors price discrepancies between pools and reference prices
- Executes trades when profitable opportunities arise
- Manages token approvals and transaction broadcasting

### Monitor Service (`monitor`)
- Subscribes to Redis events from the market maker
- Persists trade data to PostgreSQL for analysis
- Provides heartbeat monitoring for system health

## 🔄 How It Works

1. **Pool Discovery**: Connects to Tycho API and discovers relevant liquidity pools
2. **Price Monitoring**: Continuously compares pool prices with external reference prices
3. **Opportunity Detection**: Identifies when price spreads exceed configured thresholds
4. **Trade Optimization**: Calculates optimal trade amounts to maximize profit
5. **Execution**: Submits transactions through network-specific execution strategies
6. **Monitoring**: Publishes trade events for persistence and analysis

## 📈 Trading Strategy

The market maker implements a **statistical arbitrage** strategy:

- Monitors price deviations between DEX pools and centralized exchange prices
- When spreads exceed minimum thresholds (configurable via `min_executable_spread_bps`)
- Calculates optimal trade size considering:
  - Available inventory
  - Gas costs
  - Price impact
  - Slippage tolerance
- Executes trades to capture the spread while maintaining risk limits

## 🌐 Supported Networks

| Network | Chain ID | Features |
|---------|----------|----------|
| Ethereum | 1 | Flashbots MEV protection |
| Base | 8453 | L2 optimizations |
| Unichain | 130 | Beta support |

## 🛠️ Technology Stack

- **Language**: Rust 🦀
- **Blockchain Interaction**: Alloy framework
- **Protocol Integration**: Tycho simulation & execution libraries
- **Async Runtime**: Tokio
- **Database**: PostgreSQL (via SeaORM)
- **Event System**: Redis pub/sub
- **Price Feeds**: Binance API, Chainlink oracles

## 📊 Performance

- Sub-second reaction time to price movements
- Automatic recovery from crashes with configurable restart delays
- Parallel transaction simulation and broadcasting
- Optimized for high-frequency trading with minimal latency

## 🔒 Security Features

- Private key management via environment variables
- Approval management with Permit2 integration
- Simulation before execution to prevent failed transactions
- Maximum slippage protection
- Inventory ratio limits to prevent overexposure

## 📚 Next Steps

- [Getting Started](getting-started/quick-start.md) - Set up your first market maker
- [Configuration Guide](configuration/overview.md) - Customize trading parameters
- [Architecture Deep Dive](architecture/overview.md) - Understand the system internals
- [Deployment Guide](deployment/production.md) - Deploy to production

## 🤝 Contributing

This project is open source. Contributions are welcome! Please read our contributing guidelines before submitting PRs.

## 📄 License

MIT License - see LICENSE file for details

## ⚠️ Disclaimer

This software is provided as-is. Trading cryptocurrencies carries significant risk. Users are responsible for their own trading decisions and should thoroughly test configurations before deploying with real funds.