#!/bin/bash
bash -c "source config/.env.monitor.ex && echo \$DATABASE_URL"
export DATABASE_URL=$DATABASE_URL

sea-orm-cli generate entity \
    -u "$DATABASE_URL" \
    -o src/shd/entity \
    --with-serde=both

# Here
# • -u / --database-url points at your Neon URL (or omit if you’ve set DATABASE_URL in .env)
# • -s public picks the PostgreSQL schema (default is public)
# • -o src/entity is your output directory
# • --with-serde=both derives Serialize and Deserialize for every model  ￼
# • --lib makes it emit a lib.rs instead of mod.rs, which may suit a standalone crate

# cargo install sea-orm-cli
