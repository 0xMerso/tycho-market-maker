#!/bin/bash
# bash -c "source config/secrets/.env.monitor.global"
source config/secrets/.env.monitor.global
export DATABASE_URL=$DATABASE_URL
echo "🔄 All in one script for database with at DATABASE_URL = $DATABASE_URL"
sh prisma/0.reset.sh
sh prisma/1.db-push.sh
sh prisma/2.sea-orm.sh
