source config/.env.moni.ex
export DATABASE_URL=$DATABASE_URL
echo $DATABASE_URL
npx prisma db push --schema=prisma/schema.prisma --force-reset
