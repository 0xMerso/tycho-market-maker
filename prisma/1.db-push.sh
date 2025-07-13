#!/bin/bash
source config/.env.monitor.ex
export DATABASE_URL=$DATABASE_URL
echo $DATABASE_URL
npx prisma db push --schema=prisma/schema.prisma --force-reset
npx prisma generate --schema=prisma/schema.prisma
# npx prisma db pull --schema=prisma/schema.prisma
