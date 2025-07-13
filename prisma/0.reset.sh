#!/bin/bash

bash -c "source config/.env.monitor.ex && echo \$DATABASE_URL"
npx prisma migrate reset --force # ! Very destructive, will drop all data
# npx prisma migrate reset is a destructive command that completely resets your database. Here's what it does:
# Drops the entire database (deletes all data)
# Recreates the database from scratch
# Applies all migrations in order
# Runs seed scripts (if configured)
