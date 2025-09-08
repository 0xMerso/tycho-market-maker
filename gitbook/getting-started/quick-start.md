# Quick Start

Get your Tycho Market Maker running in under 10 minutes!

## 🎯 Overview

This guide will help you quickly set up and run the Tycho Market Maker on a testnet. For production deployments, see the [Production Setup](../deployment/production.md) guide.

## 📋 Prerequisites

Before starting, ensure you have:

- ✅ Rust installed (1.70+)
- ✅ Git
- ✅ PostgreSQL (14+)
- ✅ Redis
- ✅ A wallet with testnet ETH
- ✅ Access to an RPC endpoint

## 🚀 Quick Setup

### 1. Clone the Repository

```bash
git clone https://github.com/yourusername/tycho-market-maker.git
cd tycho-market-maker
```

### 2. Create Environment File

Create a `.env` file in `config/secrets/`:

```bash
mkdir -p config/secrets
cp config/secrets/.env.example config/secrets/.env.testnet
```

Edit the file with your settings:

```env
# Required Environment Variables
TYCHO_API_KEY="your-tycho-api-key"
WALLET_PRIVATE_KEY="0x..." # Your wallet private key
HEARTBEAT="https://uptime.betterstack.com/..."
TESTING=true # Set to true for testnet

# Database Configuration
DATABASE_URL="postgresql://user:pass@localhost/tycho_mm"
REDIS_URL="redis://localhost:6379"
```

### 3. Choose a Configuration

Select a pre-configured trading pair:

```bash
# For Ethereum mainnet ETH/USDC
export CONFIG_PATH="config/mainnet.eth-usdc.toml"
export SECRET_PATH="config/secrets/.env.mainnet"

# For Base ETH/USDC
export CONFIG_PATH="config/base.eth-usdc.toml"
export SECRET_PATH="config/secrets/.env.base"

# For Unichain ETH/USDC
export CONFIG_PATH="config/unichain.eth-usdc.toml"
export SECRET_PATH="config/secrets/.env.unichain"
```

### 4. Setup Database

```bash
# Run database migrations
sh prisma/all-in-one.sh
```

### 5. Build the Project

```bash
cargo build --release
```

### 6. Run the Market Maker

```bash
# Start with your chosen config
cargo run --bin maker
```

## 🎉 Success!

Your market maker should now be:
- 📡 Connecting to Tycho Protocol
- 👀 Monitoring pools for your configured pair
- 💹 Looking for arbitrage opportunities
- 📊 Publishing events to Redis (if configured)

## 📺 Monitoring

In a separate terminal, run the monitor to see trade events:

```bash
cargo run --bin monitor
```

## 🔍 Verify It's Working

You should see logs like:

```
INFO shd::maker: ✅ ProtocolStreamBuilder initialised successfully. Monitoring 3 targets
INFO shd::maker: 📊 Reference price at initialization: $3850.25
INFO shd::maker: 🟢 eth stream: b#19234567 with 45 states
```

## ⚙️ Configuration Tips

### Testing Mode

When `TESTING=true`:
- Transactions are simulated but not broadcast
- Restart delays are shortened
- Perfect for development

### Minimum Viable Config

For testing, you can start with conservative settings:

```toml
min_watch_spread_bps = 10.0      # Watch for 0.1% spreads
min_executable_spread_bps = 15.0 # Execute at 0.15% spreads
max_inventory_ratio = 0.1         # Use max 10% of inventory
```

## 🚨 Common Issues

### "Token not found"
- Ensure your token addresses are correct for the network
- Check that Tycho has indexed your tokens

### "Failed to get allowance"
- Your wallet needs to approve tokens to Permit2
- The bot will handle this automatically on first run

### "No opportunities found"
- Normal in stable markets
- Try adjusting `min_executable_spread_bps` lower
- Ensure your reference price feed is working

## 📚 Next Steps

- Read the [Configuration Guide](../configuration/overview.md) to customize parameters
- Learn about [Trading Parameters](../configuration/trading-parameters.md)
- Deploy to [Production](../deployment/production.md)
- Set up [Monitoring](../deployment/monitoring.md)

## 💡 Pro Tips

1. **Start Small**: Use small `max_inventory_ratio` initially
2. **Monitor Logs**: Watch for "potential_profit_delta_spread_bps" in logs
3. **Test First**: Always test with `TESTING=true` before real trading
4. **Gas Prices**: Monitor gas costs vs potential profits

## 🆘 Getting Help

- Check [Troubleshooting](../deployment/troubleshooting.md)
- Review [FAQ](../resources/faq.md)
- Join our Discord community
- Open an issue on GitHub