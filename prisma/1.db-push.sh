#!/bin/bash
bash -c "source config/.env.monitor.ex && echo \$DATABASE_URL"
export DATABASE_URL=$DATABASE_URL
npx prisma db push --schema=prisma/schema.prisma --force-reset
npx prisma generate --schema=prisma/schema.prisma
# npx prisma db pull --schema=prisma/schema.prisma
