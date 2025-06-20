source config/.env.moni.ex
export DATABASE_URL=$DATABASE_URL
echo $DATABASE_URL
npx prisma db push --schema=prisma/schema.prisma

# Environment variables loaded from .env
# Prisma schema loaded from prisma/schema.prisma
# Datasource "db": PostgreSQL database "neondb", schema "public" at "ep-quiet-voice-ab4cr3zu-pooler.eu-west-2.aws.neon.tech"
# 🚀  Your database is now in sync with your Prisma schema. Done in 866ms
# ✔ Generated Prisma Client (v6.10.1) to ./src/generated/prisma in 48ms
