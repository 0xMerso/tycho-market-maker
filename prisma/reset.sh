source config/.env.monitor.ex
export DATABASE_URL=$DATABASE_URL
echo $DATABASE_URL

npx prisma migrate reset --force # ! Very destructive, will drop all data

sh prisma/1.push.sh
sh prisma/2.sea-orm.sh
