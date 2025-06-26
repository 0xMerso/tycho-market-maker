source config/.env.moni.ex
export DATABASE_URL=$DATABASE_URL
echo $DATABASE_URL
npx prisma migrate reset --force # ! Very destructive, will drop all data
sh prisma/prisma.push.sh
sh prisma/sea.orm.sh

# DROP TABLE "public"."Trade" CASCADE
# DROP TABLE "public"."Price" CASCADE
# DROP TABLE "public"."Instance" CASCADE
# DROP TABLE "public"."Configuration" CASCADE
