#!/bin/bash

set -eoa pipefail

DB_USER=${POSTGRES_USER:=postgres}
DB_PASSWORD=${POSTGRES_PASSWORD:=password}
DB_NAME=${POSTGRES_DB:=newsletter}
DB_PORT=${POSTGRES_PORT:=5432}
DB_HOST=${POSTGRES_HOST:=localhost}
DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
export DATABASE_URL

[ ! -x "$(command -v psql)" ] && (
	echo >&2 "ERROR: psql is not installed"
	exit 1
)
[ ! -x "$(command -v sqlx)" ] && cargo install --version="~0.6" sqlx-cli --no-default-features --features rustls,postgres

if [[ -z "${SKIP_DOCKER}" ]]; then
	docker run \
		-e POSTGRES_USER=${DB_USER} \
		-e POSTGRES_PASSWORD=${DB_PASSWORD} \
		-e POSTGRES_DB=${DB_NAME} \
		-p "${DB_PORT}:5432" \
		-d \
		--name "postgres_$(date '+%s')" \
		-d postgres -N 1000
fi

export PGPASSWORD="${DB_PASSWORD}"
until psql -h "${DB_HOST}" -U "${DB_USER}" -d "postgres" -c '\q'; do
	echo >&2 "postgres is unavailable - sleeping"
	sleep 1
done

echo >&2 "postgres is running on port ${DB_PORT}"

sqlx database create
sqlx migrate run

echo >&2 "postgres has been migrated"
