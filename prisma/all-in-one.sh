#!/bin/bash
bash -c "source config/.env.monitor.ex && echo \$DATABASE_URL"
export DATABASE_URL=$DATABASE_URL
sh prisma/0.reset.sh
sh prisma/1.db-push.sh
sh prisma/2.sea-orm.sh
