# Environment Variables

## Overview

Environment variables contain sensitive data and runtime configurations that should not be committed to version control. These are stored in `.env` files within the `config/secrets/` directory.

## Required Variables

### 🔑 Authentication & API Keys

#### `TYCHO_API_KEY`
- **Description**: API key for accessing Tycho Protocol data streams
- **Format**: String
- **Example**: `"sk_live_abc123def456..."`
- **Required**: Yes
- **How to obtain**: Request access at [tycho-protocol.com](https://tycho-protocol.com)

#### `WALLET_PRIVATE_KEY`
- **Description**: Private key of the wallet that will execute trades
- **Format**: Hex string with 0x prefix
- **Example**: `"0x1234567890abcdef..."`
- **Required**: Yes
- **Security**: 
  - Never commit to git
  - Use separate keys for testing/production
  - Consider hardware wallet integration for production

### 📊 Database Configuration

#### `DATABASE_URL`
- **Description**: PostgreSQL connection string
- **Format**: `postgresql://[user]:[password]@[host]:[port]/[database]`
- **Example**: `"postgresql://tycho_user:secure_pass@localhost:5432/tycho_mm"`
- **Required**: Yes
- **Components**:
  - `user`: Database username
  - `password`: Database password
  - `host`: Database server (localhost for local)
  - `port`: PostgreSQL port (default: 5432)
  - `database`: Database name

#### `REDIS_URL`
- **Description**: Redis connection string for event publishing
- **Format**: `redis://[host]:[port]`
- **Example**: `"redis://localhost:6379"`
- **Required**: Yes (if `publish_events=true`)
- **Default**: `"redis://127.0.0.1:42044"`

### 📡 Monitoring

#### `HEARTBEAT`
- **Description**: Heartbeat monitoring endpoint URL
- **Format**: HTTPS URL
- **Example**: `"https://uptime.betterstack.com/api/v1/heartbeat/abc123"`
- **Required**: No (but recommended for production)
- **Purpose**: External monitoring of service health

### 🧪 Runtime Mode

#### `TESTING`
- **Description**: Enables testing mode (simulates but doesn't broadcast transactions)
- **Format**: Boolean string
- **Values**: `"true"` or `"false"`
- **Default**: `"false"`
- **Effects when `true`:
  - Transactions are simulated but not sent
  - Shorter restart delays (6s vs 60s)
  - Additional debug logging
  - Safe for development/testing

## Optional Variables

### 🎯 Advanced Configuration

#### `RUST_LOG`
- **Description**: Logging verbosity level
- **Format**: Log level string
- **Values**: `error`, `warn`, `info`, `debug`, `trace`
- **Example**: `"info,shd=debug,maker=trace"`
- **Default**: `"info"`

#### `REDIS_HOST`
- **Description**: Override default Redis host
- **Format**: `host:port`
- **Example**: `"192.168.1.100:6379"`
- **Default**: `"127.0.0.1:42044"`

## Environment File Examples

### Development Environment (.env.dev)

```env
# Development configuration - DO NOT USE IN PRODUCTION
TYCHO_API_KEY="test_api_key_development"
WALLET_PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
DATABASE_URL="postgresql://dev_user:dev_pass@localhost:5432/tycho_dev"
REDIS_URL="redis://localhost:6379"
HEARTBEAT=""
TESTING=true
RUST_LOG=debug
```

### Testnet Environment (.env.testnet)

```env
# Testnet configuration
TYCHO_API_KEY="sk_test_abc123..."
WALLET_PRIVATE_KEY="0x..." # Testnet wallet only
DATABASE_URL="postgresql://test_user:test_pass@localhost:5432/tycho_test"
REDIS_URL="redis://localhost:6379"
HEARTBEAT="https://uptime.betterstack.com/api/v1/heartbeat/test123"
TESTING=true
RUST_LOG=info
```

### Production Environment (.env.mainnet)

```env
# Production configuration - SECURE THIS FILE!
TYCHO_API_KEY="sk_live_xyz789..."
WALLET_PRIVATE_KEY="0x..." # Production wallet - USE HARDWARE WALLET IF POSSIBLE
DATABASE_URL="postgresql://prod_user:strong_password@db.production.internal:5432/tycho_prod"
REDIS_URL="redis://redis.production.internal:6379"
HEARTBEAT="https://uptime.betterstack.com/api/v1/heartbeat/prod456"
TESTING=false
RUST_LOG=info,shd::maker=debug
```

## Network-Specific Configurations

### Ethereum Mainnet
```env
# .env.mainnet
TYCHO_API_KEY="your_key"
WALLET_PRIVATE_KEY="0x..."
# High security, real funds
TESTING=false
```

### Base L2
```env
# .env.base
TYCHO_API_KEY="your_key"
WALLET_PRIVATE_KEY="0x..."
# Lower gas costs, same security
TESTING=false
```

### Unichain
```env
# .env.unichain
TYCHO_API_KEY="your_key"
WALLET_PRIVATE_KEY="0x..."
# Beta network, test carefully
TESTING=true  # Recommended initially
```

## Security Best Practices

### 1. File Permissions

```bash
# Restrict access to owner only
chmod 600 config/secrets/.env.*

# Verify permissions
ls -la config/secrets/
```

### 2. Git Ignore

Ensure `.gitignore` includes:

```gitignore
# Secrets
config/secrets/
.env
.env.*
*.key
*.pem
```

### 3. Separate Keys

- **Development**: Use throwaway keys with minimal funds
- **Testing**: Use testnet-specific keys
- **Production**: Use dedicated production keys with proper security

### 4. Key Rotation

Regularly rotate sensitive credentials:

```bash
# Generate new wallet for production
cast wallet new

# Update environment file
vim config/secrets/.env.mainnet

# Restart services
systemctl restart tycho-maker
```

### 5. Secret Management Tools

For production, consider:

- **HashiCorp Vault**: Dynamic secrets management
- **AWS Secrets Manager**: Cloud-native solution
- **Docker Secrets**: For containerized deployments
- **Kubernetes Secrets**: For K8s deployments

## Loading Environment Variables

### Manual Loading

```bash
# Load for current session
export $(cat config/secrets/.env.mainnet | xargs)

# Or source directly
source config/secrets/.env.mainnet
```

### Programmatic Loading

The market maker automatically loads from `SECRET_PATH`:

```bash
export SECRET_PATH="config/secrets/.env.mainnet"
cargo run --bin maker
```

### Docker Loading

```yaml
# docker-compose.yml
services:
  maker:
    env_file:
      - config/secrets/.env.mainnet
```

## Validation

### Check Variables Are Set

```bash
# Verify critical variables
echo $TYCHO_API_KEY
echo $DATABASE_URL
echo $REDIS_URL

# Test database connection
psql $DATABASE_URL -c "SELECT 1"

# Test Redis connection
redis-cli -u $REDIS_URL ping
```

### Debug Loading Issues

```bash
# Enable debug logging
RUST_LOG=debug cargo run --bin maker

# Check which file is being loaded
echo "Loading from: $SECRET_PATH"
```

## Troubleshooting

### "Environment variable not found"

```bash
# Check file exists
ls -la config/secrets/

# Check variable is in file
grep TYCHO_API_KEY config/secrets/.env.mainnet

# Check it's exported
env | grep TYCHO
```

### "Invalid private key"

```bash
# Verify format (64 hex chars after 0x)
echo $WALLET_PRIVATE_KEY | wc -c
# Should be 66 (0x + 64 chars)
```

### "Database connection failed"

```bash
# Test connection directly
psql "postgresql://user:pass@localhost/tycho_mm"

# Check PostgreSQL is running
systemctl status postgresql
```

## Migration from Other Systems

### From JSON Config

```javascript
// old-config.json
{
  "apiKey": "abc123",
  "privateKey": "0x..."
}
```

Convert to:
```env
TYCHO_API_KEY="abc123"
WALLET_PRIVATE_KEY="0x..."
```

### From YAML Config

```yaml
# old-config.yaml
api:
  tycho_key: abc123
wallet:
  private_key: "0x..."
```

Convert to:
```env
TYCHO_API_KEY="abc123"
WALLET_PRIVATE_KEY="0x..."
```

## Next Steps

- Configure [Market Maker Parameters](market-maker-config.md)
- Set up [Network Settings](network-settings.md)
- Configure [Trading Parameters](trading-parameters.md)