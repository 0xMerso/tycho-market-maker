# Docker Setup for Tycho Market Maker

## Prerequisites

1. **External Network**: Create the external Docker network:
   ```bash
   docker network create tmm
   ```

2. **Environment Files**: Create the required environment files:
   - `config/.env.maker.ex` - for the market maker service
   - `config/.env.monitor.ex` - for the monitor service

## Environment Variables

### Required for both services:
```bash
# Database
DATABASE_URL=postgresql://username:password@host:port/database
DATABASE_NAME=your_database_name

# Testing mode
TESTING=false

# APIs
HEARTBEAT=your_heartbeat_url
TYCHO_API_KEY=your_tycho_api_key

# Redis
REDIS_HOST=redis
REDIS_PORT=6379
```

### Additional for market_maker service:
```bash
# Wallet
WALLET_PUBLIC_KEY=your_wallet_public_key
WALLET_PRIVATE_KEY=your_wallet_private_key

# Config
CONFIG_PATH=config/mmc.base.eth-usdc.toml
```

### Additional for monitor service:
```bash
# Config paths
CONFIGS_PATHS=config/
```

## Usage

1. **Build and start all services**:
   ```bash
   docker-compose up --build
   ```

2. **Start in background**:
   ```bash
   docker-compose up -d --build
   ```

3. **View logs**:
   ```bash
   docker-compose logs -f
   ```

4. **Stop services**:
   ```bash
   docker-compose down
   ```

## Services

- **maker-base-alpha**: Market maker service (port 42042)
- **monitor**: Monitoring service
- **redis**: Redis server (port 6379)

## Health Checks

All services include health checks:
- Redis: Pings the server
- Market maker: Checks if market_maker process is running
- Monitor: Checks if monitor process is running

## Troubleshooting

1. **Network issues**: Ensure the `tmm` network exists
2. **Environment files**: Check that `.env.maker.ex` and `.env.monitor.ex` exist and are properly configured
3. **Database connection**: Verify DATABASE_URL is correct and accessible
4. **Redis connection**: Ensure Redis is running and accessible on port 6379 