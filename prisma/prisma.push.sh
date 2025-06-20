source config/.env.moni.ex
export DATABASE_URL=$DATABASE_URL
echo $DATABASE_URL
npx prisma db push --schema=prisma/schema.prisma

# Environment variables loaded from .env
# Prisma schema loaded from prisma/schema.prisma
# Datasource "db": PostgreSQL database "neondb", schema "public" at "ep-quiet-voice-ab4cr3zu-pooler.eu-west-2.aws.neon.tech"
# 🚀  Your database is now in sync with your Prisma schema. Done in 866ms
# ✔ Generated Prisma Client (v6.10.1) to ./src/generated/prisma in 48ms

# ============================================== Prisma ==============================================

# npm install prisma --save-dev
# npm install @prisma/client
# npx prisma init
# 1. Run prisma dev to start a local Prisma Postgres server.
# 2. Define models in the schema.prisma file.
# 3. Run prisma migrate dev to migrate your local Prisma Postgres database.
# 4. Tip: Explore how you can extend the ORM with scalable connection pooling, global caching, and a managed serverless Postgres database. Read: https://pris.ly/cli/beyond-orm

# export DATABASE_URL="postgres://username:password@db.neon.tech/yourdb?sslmode=require"
# npx prisma db push
# npx prisma db pull
