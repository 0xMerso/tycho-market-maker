# Prerequisites

## System Requirements

### Hardware

**Minimum Requirements:**
- CPU: 2 cores
- RAM: 4 GB
- Storage: 20 GB SSD
- Network: Stable internet connection (low latency preferred)

**Recommended for Production:**
- CPU: 4+ cores
- RAM: 8 GB
- Storage: 50 GB SSD
- Network: Dedicated server with <50ms latency to RPC

### Operating System

- Linux (Ubuntu 20.04+, Debian 11+)
- macOS (12.0+)
- Windows (WSL2 recommended)

## Software Dependencies

### 1. Rust Toolchain

Install Rust 1.70 or higher:

```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

### 2. PostgreSQL Database

Install PostgreSQL 14 or higher:

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install postgresql postgresql-contrib

# macOS
brew install postgresql
brew services start postgresql

# Verify
psql --version
```

Create a database for the market maker:

```sql
sudo -u postgres psql
CREATE DATABASE tycho_mm;
CREATE USER tycho_user WITH PASSWORD 'your_password';
GRANT ALL PRIVILEGES ON DATABASE tycho_mm TO tycho_user;
```

### 3. Redis

Install Redis for event publishing:

```bash
# Ubuntu/Debian
sudo apt install redis-server
sudo systemctl start redis-server

# macOS
brew install redis
brew services start redis

# Verify
redis-cli ping
# Should return: PONG
```

### 4. Git

```bash
# Ubuntu/Debian
sudo apt install git

# macOS
brew install git

# Verify
git --version
```

### 5. Development Tools

```bash
# Ubuntu/Debian
sudo apt install build-essential pkg-config libssl-dev

# macOS
xcode-select --install
```

## Blockchain Requirements

### 1. Wallet

You need an Ethereum wallet with:
- Private key access
- Sufficient native token balance for gas
- Token balances for trading

**Creating a New Wallet:**

```bash
# Using cast from Foundry
cast wallet new

# Save the private key securely!
```

### 2. RPC Endpoints

You need access to RPC endpoints for your target networks:

**Free Options:**
- [Alchemy](https://www.alchemy.com/) - Reliable, good for production
- [Infura](https://infura.io/) - Well-established provider
- [QuickNode](https://www.quicknode.com/) - Fast, good uptime

**Example Endpoints:**
```
Ethereum: https://eth-mainnet.g.alchemy.com/v2/YOUR-API-KEY
Base: https://base-mainnet.g.alchemy.com/v2/YOUR-API-KEY
Unichain: https://unichain-mainnet.g.alchemy.com/v2/YOUR-API-KEY
```

### 3. Tycho API Access

Request access to Tycho Protocol API:

1. Visit [Tycho Protocol](https://tycho-protocol.com)
2. Request API access
3. Save your API key securely

## External Services

### 1. Price Feeds

The market maker supports multiple price sources:

**Binance API:**
- No API key required for public data
- Rate limits apply
- Endpoint: `https://api.binance.com/api/v3`

**Chainlink Oracles:**
- On-chain price feeds
- Requires RPC access
- Gas costs for reading

### 2. Monitoring (Optional)

For production deployments:

**Better Stack (Recommended):**
- Sign up at [betterstack.com](https://betterstack.com)
- Create a heartbeat monitor
- Get your heartbeat URL

## Network-Specific Requirements

### Ethereum Mainnet

- Higher gas costs (100-200 gwei typical)
- MEV protection recommended (Flashbots)
- Minimum 0.1 ETH for operations

### Base L2

- Lower gas costs (<1 gwei)
- Faster block times (2 seconds)
- Minimum 0.01 ETH for operations

### Unichain

- Beta network - expect changes
- Very low gas costs
- Minimum 0.01 ETH for operations

## Token Requirements

### For Testing

- **Base Token**: 0.1 ETH or equivalent
- **Quote Token**: 100 USDC or equivalent
- **Gas Token**: 0.05 ETH for transaction fees

### For Production

- **Base Token**: Based on your strategy (typically 1-10 ETH)
- **Quote Token**: Equivalent value to base token
- **Gas Token**: 0.5+ ETH recommended buffer

## Environment Setup

### 1. Create Project Directory

```bash
mkdir ~/tycho-market-maker
cd ~/tycho-market-maker
```

### 2. Clone Repository

```bash
git clone https://github.com/yourusername/tycho-market-maker.git .
```

### 3. Install Cargo Extensions

```bash
# Useful development tools
cargo install cargo-watch  # Auto-rebuild on changes
cargo install cargo-edit   # Manage dependencies
cargo install cargo-audit  # Security audits
```

## Verification Checklist

Run these commands to verify everything is installed:

```bash
# Check Rust
rustc --version
cargo --version

# Check Database
psql --version
redis-cli ping

# Check Git
git --version

# Check Network
curl -X POST YOUR_RPC_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

## Security Considerations

### Private Key Management

**Never:**
- Commit private keys to git
- Share private keys
- Use production keys in testing

**Always:**
- Use environment variables
- Keep separate keys for testing/production
- Use hardware wallets when possible

### API Key Security

- Store in `.env` files
- Add `.env` to `.gitignore`
- Rotate keys regularly
- Use separate keys for development/production

## Next Steps

Once all prerequisites are installed:

1. Continue to [Installation](installation.md)
2. Configure your [Environment Variables](../configuration/environment-variables.md)
3. Start with a [Test Run](first-run.md)