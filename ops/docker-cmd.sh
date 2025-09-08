#!/bin/bash

# Docker Commands for Tycho Market Maker
# Raw commands for reference

# === Start Services ===
docker-compose up -d
docker-compose up -d maker-mainnet-eth-usdc
docker-compose up -d maker-unichain-eth-usdc
docker-compose up -d maker-unichain-btc-usdc

# === Stop Services ===
docker-compose down
docker-compose stop maker-mainnet-eth-usdc
docker-compose stop maker-unichain-eth-usdc
docker-compose stop maker-unichain-btc-usdc

# === Restart Services ===
docker-compose restart
docker-compose restart maker-mainnet-eth-usdc
docker-compose restart maker-unichain-eth-usdc
docker-compose restart maker-unichain-btc-usdc

# === Logs ===
docker-compose logs -f
docker-compose logs -f --tail=100
docker-compose logs -f maker-mainnet-eth-usdc
docker-compose logs -f maker-unichain-eth-usdc
docker-compose logs -f maker-unichain-btc-usdc
docker-compose logs -f monitor
docker-compose logs -f maker-mainnet-eth-usdc maker-unichain-eth-usdc maker-unichain-btc-usdc

# === Status ===
docker-compose ps
docker ps
docker stats

# === Using Profiles ===
docker-compose --profile mainnet up -d
docker-compose --profile unichain up -d
docker-compose --profile all up -d

# === Database ===
docker-compose exec postgres psql -U tycho_user -d tycho_mm
docker-compose exec -T postgres pg_dump -U tycho_user tycho_mm > backup.sql
docker-compose exec -T postgres psql -U tycho_user tycho_mm < backup.sql

# === Redis ===
docker-compose exec redis redis-cli -a redis_password
docker-compose exec redis redis-cli -a redis_password FLUSHALL

# === Build ===
docker-compose build
docker-compose build --no-cache
docker-compose pull

# === Clean ===
docker-compose down --remove-orphans
docker-compose down -v
docker system prune -f
docker system prune -af

# === Shell Access ===
docker-compose exec maker-mainnet-eth-usdc /bin/bash
docker-compose exec maker-unichain-eth-usdc /bin/bash
docker-compose exec maker-unichain-btc-usdc /bin/bash
docker-compose exec monitor /bin/bash

# === Health Checks ===
docker-compose exec postgres pg_isready -U tycho_user
docker-compose exec redis redis-cli -a redis_password ping

# === Resource Usage ===
docker stats --no-stream $(docker-compose ps -q)
docker-compose top