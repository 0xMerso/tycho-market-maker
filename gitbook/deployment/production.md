# Production Deployment

## 🚀 Production Checklist

Before deploying to production, ensure:

- [ ] Tested thoroughly on testnet
- [ ] Reviewed all configuration parameters
- [ ] Set up monitoring and alerting
- [ ] Secured private keys properly
- [ ] Configured database backups
- [ ] Set up log rotation
- [ ] Reviewed security best practices
- [ ] Prepared incident response plan

## 📋 Prerequisites

### Infrastructure Requirements

**Recommended Setup:**
- **Server**: Dedicated VPS or bare metal
- **CPU**: 4+ cores
- **RAM**: 8GB minimum
- **Storage**: 100GB SSD
- **Network**: 1Gbps connection
- **OS**: Ubuntu 22.04 LTS

### Network Access

Ensure firewall allows:
- **Outbound**: HTTPS (443) for APIs
- **Outbound**: PostgreSQL (5432) if remote
- **Outbound**: Redis (6379) if remote
- **Inbound**: SSH (22) for management
- **Inbound**: Monitoring ports if needed

## 🔧 System Setup

### 1. Create Dedicated User

```bash
# Create user for market maker
sudo useradd -m -s /bin/bash tycho-mm
sudo usermod -aG sudo tycho-mm

# Switch to new user
sudo su - tycho-mm
```

### 2. Install Dependencies

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install required packages
sudo apt install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    postgresql \
    redis-server \
    nginx \
    certbot \
    python3-certbot-nginx \
    git \
    curl \
    htop \
    tmux

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. Setup PostgreSQL

```bash
# Configure PostgreSQL
sudo -u postgres psql << EOF
CREATE DATABASE tycho_prod;
CREATE USER tycho_prod WITH ENCRYPTED PASSWORD 'strong_password_here';
GRANT ALL PRIVILEGES ON DATABASE tycho_prod TO tycho_prod;
ALTER DATABASE tycho_prod SET log_statement = 'all';
ALTER DATABASE tycho_prod SET log_duration = on;
EOF

# Enable remote connections if needed
sudo vim /etc/postgresql/14/main/postgresql.conf
# Set: listen_addresses = '*'

sudo vim /etc/postgresql/14/main/pg_hba.conf
# Add: host    all    all    0.0.0.0/0    md5

sudo systemctl restart postgresql
```

### 4. Setup Redis

```bash
# Configure Redis for production
sudo vim /etc/redis/redis.conf

# Key settings:
maxmemory 2gb
maxmemory-policy allkeys-lru
appendonly yes
appendfsync everysec

# Set password
requirepass your_redis_password

# Restart Redis
sudo systemctl restart redis
```

## 🏗️ Application Deployment

### 1. Clone and Build

```bash
# Clone repository
cd /home/tycho-mm
git clone https://github.com/yourusername/tycho-market-maker.git
cd tycho-market-maker

# Build release version
cargo build --release --bin maker --bin monitor

# Copy binaries
sudo cp target/release/maker /usr/local/bin/
sudo cp target/release/monitor /usr/local/bin/
```

### 2. Configuration Setup

```bash
# Create config directory
mkdir -p /etc/tycho-mm/secrets

# Copy configurations
cp config/*.toml /etc/tycho-mm/

# Create production environment file
cat > /etc/tycho-mm/secrets/.env.production << 'EOF'
# Production Configuration - KEEP SECURE!
TYCHO_API_KEY="sk_live_..."
WALLET_PRIVATE_KEY="0x..."
DATABASE_URL="postgresql://tycho_prod:password@localhost/tycho_prod"
REDIS_URL="redis://:password@localhost:6379"
HEARTBEAT="https://uptime.betterstack.com/api/v1/heartbeat/..."
TESTING=false
RUST_LOG=info,shd::maker=debug
EOF

# Secure the secrets
chmod 600 /etc/tycho-mm/secrets/.env.production
chown tycho-mm:tycho-mm /etc/tycho-mm/secrets/.env.production
```

### 3. Database Setup

```bash
# Run migrations
cd /home/tycho-mm/tycho-market-maker
export DATABASE_URL="postgresql://tycho_prod:password@localhost/tycho_prod"
sh prisma/all-in-one.sh
```

## 🎯 Systemd Services

### 1. Market Maker Service

Create `/etc/systemd/system/tycho-maker.service`:

```ini
[Unit]
Description=Tycho Market Maker
After=network.target postgresql.service redis.service
Requires=postgresql.service redis.service

[Service]
Type=simple
User=tycho-mm
Group=tycho-mm
WorkingDirectory=/home/tycho-mm/tycho-market-maker

# Environment
Environment="CONFIG_PATH=/etc/tycho-mm/mainnet.eth-usdc.toml"
Environment="SECRET_PATH=/etc/tycho-mm/secrets/.env.production"
Environment="RUST_LOG=info,shd::maker=debug"

# Execution
ExecStart=/usr/local/bin/maker
ExecStop=/bin/kill -TERM $MAINPID

# Restart policy
Restart=always
RestartSec=10
StartLimitInterval=60
StartLimitBurst=3

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/home/tycho-mm/tycho-market-maker/logs

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=tycho-maker

[Install]
WantedBy=multi-user.target
```

### 2. Monitor Service

Create `/etc/systemd/system/tycho-monitor.service`:

```ini
[Unit]
Description=Tycho Monitor Service
After=network.target postgresql.service redis.service tycho-maker.service
Requires=postgresql.service redis.service

[Service]
Type=simple
User=tycho-mm
Group=tycho-mm
WorkingDirectory=/home/tycho-mm/tycho-market-maker

Environment="SECRET_PATH=/etc/tycho-mm/secrets/.env.production"
Environment="RUST_LOG=info"

ExecStart=/usr/local/bin/monitor
Restart=always
RestartSec=10

StandardOutput=journal
StandardError=journal
SyslogIdentifier=tycho-monitor

[Install]
WantedBy=multi-user.target
```

### 3. Enable Services

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable services
sudo systemctl enable tycho-maker
sudo systemctl enable tycho-monitor

# Start services
sudo systemctl start tycho-maker
sudo systemctl start tycho-monitor

# Check status
sudo systemctl status tycho-maker
sudo systemctl status tycho-monitor
```

## 📊 Monitoring Setup

### 1. Prometheus Metrics

Create `/etc/prometheus/prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'tycho-mm'
    static_configs:
      - targets: ['localhost:9090']
```

### 2. Grafana Dashboard

Import dashboard for monitoring:
- Trade execution rate
- Profit/loss tracking
- Gas usage
- System resources

### 3. Log Aggregation

```bash
# Setup log rotation
cat > /etc/logrotate.d/tycho-mm << EOF
/home/tycho-mm/tycho-market-maker/logs/*.log {
    daily
    rotate 7
    compress
    missingok
    notifempty
    create 0644 tycho-mm tycho-mm
    sharedscripts
    postrotate
        systemctl reload tycho-maker
    endscript
}
EOF
```

### 4. Alerting Rules

Set up alerts for:
- Service downtime
- Failed trades
- Low wallet balance
- High error rate
- Database issues

## 🔒 Security Hardening

### 1. Firewall Configuration

```bash
# Setup UFW firewall
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow ssh
sudo ufw allow from trusted_ip to any port 5432  # PostgreSQL
sudo ufw allow from trusted_ip to any port 6379  # Redis
sudo ufw enable
```

### 2. SSH Hardening

```bash
# Configure SSH
sudo vim /etc/ssh/sshd_config

# Key settings:
PermitRootLogin no
PasswordAuthentication no
PubkeyAuthentication yes
AllowUsers tycho-mm

# Restart SSH
sudo systemctl restart sshd
```

### 3. Secrets Management

```bash
# Use HashiCorp Vault for production
vault kv put secret/tycho-mm \
    tycho_api_key="..." \
    wallet_private_key="..."

# Or use encrypted environment
sudo apt install age
age -p secrets.env > secrets.env.age
```

## 🔄 Backup Strategy

### 1. Database Backups

```bash
# Create backup script
cat > /home/tycho-mm/backup.sh << 'EOF'
#!/bin/bash
BACKUP_DIR="/home/tycho-mm/backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
mkdir -p $BACKUP_DIR
pg_dump tycho_prod | gzip > $BACKUP_DIR/tycho_prod_$TIMESTAMP.sql.gz
find $BACKUP_DIR -name "*.sql.gz" -mtime +7 -delete
EOF

chmod +x /home/tycho-mm/backup.sh

# Add to crontab
crontab -e
0 */6 * * * /home/tycho-mm/backup.sh
```

### 2. Configuration Backups

```bash
# Backup configurations
tar -czf configs_backup.tar.gz /etc/tycho-mm/
```

## 🚨 Incident Response

### Emergency Stop

```bash
# Stop trading immediately
sudo systemctl stop tycho-maker

# Check logs
journalctl -u tycho-maker -n 100

# Restart after fixing
sudo systemctl start tycho-maker
```

### Rollback Procedure

```bash
# Keep previous binary versions
cp /usr/local/bin/maker /usr/local/bin/maker.backup

# Rollback if needed
cp /usr/local/bin/maker.backup /usr/local/bin/maker
sudo systemctl restart tycho-maker
```

## 📈 Performance Tuning

### 1. System Optimization

```bash
# Increase file limits
echo "* soft nofile 65536" >> /etc/security/limits.conf
echo "* hard nofile 65536" >> /etc/security/limits.conf

# TCP tuning
sysctl -w net.core.rmem_max=134217728
sysctl -w net.core.wmem_max=134217728
```

### 2. PostgreSQL Tuning

```sql
-- Optimize for writes
ALTER SYSTEM SET shared_buffers = '2GB';
ALTER SYSTEM SET effective_cache_size = '6GB';
ALTER SYSTEM SET maintenance_work_mem = '512MB';
ALTER SYSTEM SET checkpoint_completion_target = 0.9;
ALTER SYSTEM SET wal_buffers = '16MB';
```

## 🔍 Health Checks

### Automated Health Monitoring

```bash
# Create health check script
cat > /home/tycho-mm/health_check.sh << 'EOF'
#!/bin/bash

# Check services
systemctl is-active tycho-maker || exit 1
systemctl is-active tycho-monitor || exit 1

# Check database
psql $DATABASE_URL -c "SELECT 1" || exit 1

# Check Redis
redis-cli ping || exit 1

echo "All systems operational"
EOF

chmod +x /home/tycho-mm/health_check.sh
```

## 📋 Maintenance

### Regular Tasks

**Daily:**
- Check service logs
- Monitor wallet balances
- Review trade performance

**Weekly:**
- Update price feed configurations
- Review and adjust trading parameters
- Check for software updates

**Monthly:**
- Security updates
- Performance review
- Database maintenance
- Backup verification

## 🎯 Launch Procedure

### Final Launch Steps

1. **Test in Production Environment**
   ```bash
   TESTING=true systemctl start tycho-maker
   # Verify everything works
   ```

2. **Go Live**
   ```bash
   # Update environment
   sed -i 's/TESTING=true/TESTING=false/' /etc/tycho-mm/secrets/.env.production
   
   # Restart services
   sudo systemctl restart tycho-maker
   sudo systemctl restart tycho-monitor
   ```

3. **Monitor Closely**
   ```bash
   # Watch logs
   journalctl -u tycho-maker -f
   
   # Monitor trades
   psql $DATABASE_URL -c "SELECT * FROM trades ORDER BY created_at DESC LIMIT 10;"
   ```

## 🆘 Support

For production issues:

1. Check logs: `journalctl -u tycho-maker -n 1000`
2. Review configuration: Ensure all parameters are correct
3. Verify connections: Database, Redis, RPC, APIs
4. Check wallet: Balance, nonce, approvals
5. Contact support: Open issue with logs and configuration (sanitized)