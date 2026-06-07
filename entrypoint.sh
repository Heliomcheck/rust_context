#!/bin/bash
# exec /usr/local/bin/Krug_server
set -e

echo "==> Waiting for PostgreSQL to be ready..."

while ! nc -z postgres 5432; do
  sleep 0.5
done

echo "==> PostgreSQL is up, checking database connection..."

sleep 1

echo "==> Starting application..."

exec "$@"