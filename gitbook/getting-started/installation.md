# Installation

## 📦 Installation Methods

Choose your preferred installation method:

### Method 1: From Source (Recommended)

#### 1. Clone the Repository

```bash
git clone https://github.com/yourusername/tycho-market-maker.git
cd tycho-market-maker
```

#### 2. Build the Project

```bash
# Development build (faster compilation, with debug symbols)
cargo build

# Production build (optimized, smaller binary)
cargo build --release
```

#### 3. Verify Installation

```bash
# Check binaries were created
ls -la target/release/
# Should show: maker, monitor
```

### Method 2: Using Docker

#### 1. Build Docker Images

```bash
# Build all services
sh ops/dock.sh build

# Or manually
docker-compose build
```

#### 2. Verify Images

```bash
docker images | grep tycho
```

### Method 3: Direct Cargo Install

```bash
# Install from the repository
cargo install --git https://github.com/yourusername/tycho-market-maker.git

# Binaries will be in ~/.cargo/bin/
```

## 🗄️ Database Setup

### 1. PostgreSQL Configuration

Create the database and user:

```bash
# Connect to PostgreSQL
sudo -u postgres psql

# Create database and user
CREATE DATABASE tycho_mm;
CREATE USER tycho_user WITH PASSWORD 'secure_password';
GRANT ALL PRIVILEGES ON DATABASE tycho_mm TO tycho_user;
\q
```

### 2. Run Migrations

```bash
# Automated setup script
sh prisma/all-in-one.sh

# Or manually:
sh prisma/0.reset.sh      # Reset database (if exists)
sh prisma/1.db-push.sh     # Push schema
sh prisma/2.sea-orm.sh     # Generate entities
```

### 3. Verify Database

```bash
# Check tables were created
psql -U tycho_user -d tycho_mm -c "\dt"

# Should show tables:
# - instances
# - prices
# - trades
```

## 🔑 Environment Configuration

### 1. Create Secrets Directory

```bash
mkdir -p config/secrets
```

### 2. Create Environment Files

For each network you plan to use:

```bash
# Mainnet configuration
cat > config/secrets/.env.mainnet << EOF
# API Keys
TYCHO_API_KEY="your-tycho-api-key"
WALLET_PRIVATE_KEY="0x..."

# Database
DATABASE_URL="postgresql://tycho_user:password@localhost/tycho_mm"
REDIS_URL="redis://localhost:6379"

# Monitoring
HEARTBEAT="https://uptime.betterstack.com/api/v1/heartbeat/YOUR_TOKEN"

# Mode
TESTING=false
EOF

# Base configuration
cp config/secrets/.env.mainnet config/secrets/.env.base
# Edit as needed

# Unichain configuration
cp config/secrets/.env.mainnet config/secrets/.env.unichain
# Edit as needed
```

### 3. Secure Your Files

```bash
# Set restrictive permissions
chmod 600 config/secrets/.env.*

# Add to .gitignore
echo "config/secrets/" >> .gitignore
```

## 🏗️ Project Structure

After installation, your project structure should look like:

```
tycho-market-maker/
├── target/
│   └── release/
│       ├── maker          # Main market maker binary
│       └── monitor        # Monitoring service binary
├── config/
│   ├── secrets/          # Environment files (gitignored)
│   │   ├── .env.mainnet
│   │   ├── .env.base
│   │   └── .env.unichain
│   ├── mainnet.eth-usdc.toml
│   ├── base.eth-usdc.toml
│   └── unichain.eth-usdc.toml
├── prisma/
│   └── schema.prisma      # Database schema
├── src/
│   ├── maker.rs          # Market maker entry point
│   ├── monitor.rs        # Monitor entry point
│   └── shd/              # Shared library code
└── Cargo.toml
```

## 🔧 Configuration Files

### 1. Trading Pair Configurations

Pre-configured trading pairs are in `config/`:

```toml
# Example: config/mainnet.eth-usdc.toml
pair_tag = "⚪️"
base_token = "ETH"
base_token_address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
quote_token = "USDC"
quote_token_address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
```

### 2. Create Custom Configurations

Copy and modify existing configs:

```bash
# Create custom config
cp config/mainnet.eth-usdc.toml config/mainnet.eth-dai.toml

# Edit token addresses and parameters
vim config/mainnet.eth-dai.toml
```

## 🚀 Running the Services

### 1. Start Redis

```bash
# Linux
sudo systemctl start redis-server

# macOS
brew services start redis

# Docker
docker run -d -p 6379:6379 redis:alpine
```

### 2. Start PostgreSQL

```bash
# Linux
sudo systemctl start postgresql

# macOS
brew services start postgresql

# Docker
docker run -d -p 5432:5432 \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=tycho_mm \
  postgres:14
```

### 3. Run Market Maker

```bash
# Set configuration
export CONFIG_PATH="config/mainnet.eth-usdc.toml"
export SECRET_PATH="config/secrets/.env.mainnet"

# Run market maker
cargo run --release --bin maker

# Or with custom logging
RUST_LOG=info cargo run --release --bin maker
```

### 4. Run Monitor (Optional)

```bash
# In a separate terminal
export SECRET_PATH="config/secrets/.env.mainnet"
cargo run --release --bin monitor
```

## 🐳 Docker Installation

### 1. Docker Compose Setup

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:14
    environment:
      POSTGRES_DB: tycho_mm
      POSTGRES_USER: tycho_user
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  redis:
    image: redis:alpine
    ports:
      - "6379:6379"

  maker:
    build: .
    environment:
      CONFIG_PATH: /app/config/mainnet.eth-usdc.toml
      SECRET_PATH: /app/config/secrets/.env.mainnet
    volumes:
      - ./config:/app/config
    depends_on:
      - postgres
      - redis

  monitor:
    build: .
    command: ["monitor"]
    environment:
      SECRET_PATH: /app/config/secrets/.env.mainnet
    depends_on:
      - postgres
      - redis

volumes:
  postgres_data:
```

### 2. Start All Services

```bash
# Start everything
docker-compose up -d

# View logs
docker-compose logs -f maker

# Stop everything
docker-compose down
```

## ✅ Verification

### 1. Check Service Health

```bash
# Check maker is running
ps aux | grep maker

# Check database connection
psql -U tycho_user -d tycho_mm -c "SELECT NOW();"

# Check Redis
redis-cli ping

# Check logs
tail -f logs/maker.log
```

### 2. Test Configuration

```bash
# Dry run with testing mode
TESTING=true cargo run --bin maker

# Should see:
# "Testing mode, skipping broadcast"
```

## 🔨 Troubleshooting

### Common Issues

#### "Failed to connect to database"
```bash
# Check PostgreSQL is running
sudo systemctl status postgresql

# Check connection string
psql "postgresql://tycho_user:password@localhost/tycho_mm"
```

#### "Redis connection refused"
```bash
# Check Redis is running
redis-cli ping

# Check Redis is listening
netstat -an | grep 6379
```

#### "Compilation errors"
```bash
# Update dependencies
cargo update

# Clean build
cargo clean
cargo build --release
```

## 📚 Next Steps

Installation complete! Now:

1. [Configure your environment](../configuration/environment-variables.md)
2. [Set up your first trading pair](../configuration/market-maker-config.md)
3. [Run your first test](first-run.md)
4. [Deploy to production](../deployment/production.md)