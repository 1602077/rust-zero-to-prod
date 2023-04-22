#!/bin/bash

set -eoa pipefail

DB_USER=${POSTGRES_USER:=postres}
DB_PASSWORD=${POSTGRES_PASSWORD:=password}
DB_NAME=${POSTGRES_DB:=newsletter}
DB_PORT=${POSTGRES_PORT:=5432}
DB_HOST=${POSTGRES_HOST:=localhost}
DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
export DATABASE_URL

[ ! -x "$(command -v psql)" ] && >&2 echo "ERROR: psql is not installed"
[ ! -x "$(command -v sqlx)" ] && cargo install --version="~0.6" sqlx-cli --no-default-features --features rustls,postgres;

if [[ -z "${SKIP_DOCKER}" ]]; then
  docker run \
    -e POSTGRES_USER=${DB_USER} \
    -e POSTGRES_PASSWORD=${DB_PASSWORD} \
    -e POSTGRES_DB=${DB_NAME} \
    -p "${DB_PORT}:5432" \
    -d postgres \
    -d postgres -N 1000
fi

export PGPASSWORD="${DB_PASSWORD}"
until psql -h "${DB_HOST}" -U "${DB_USER}" -d "postgres" -c '\q'; do
  >&2 echo "postgres is unavailable - sleeping"
  sleep 1 
done

>&2 echo "postgres is running on port ${DB_PORT}"

sqlx database create
sqlx migrate run

>&2 echo "postgres has been migrated"
