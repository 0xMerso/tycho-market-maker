#!/bin/bash
bash -c "source config/.env.monitor.ex && echo \$DATABASE_URL"
export DATABASE_URL=$DATABASE_URL
psql "$DATABASE_URL" -c '\l'
