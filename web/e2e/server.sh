#!/bin/sh
# Builds the web app and starts the service on a fresh `fixops_e2e` database.
set -eu
cd "$(dirname "$0")/.."
pnpm build
cd ..
admin_url=${E2E_ADMIN_DATABASE_URL:-postgres://fixops:fixops@localhost:5432/fixops}
reset="DROP DATABASE IF EXISTS fixops_e2e WITH (FORCE); CREATE DATABASE fixops_e2e;"
if command -v psql >/dev/null; then
  printf '%s\n' "$reset" | psql -q "$admin_url"
else
  printf '%s\n' "$reset" | docker compose exec -T db psql -q -U fixops -d fixops
fi
export DATABASE_URL="${admin_url%/*}/fixops_e2e"
export FIXOPS_BOOTSTRAP_ADMIN_USERNAME=admin FIXOPS_BOOTSTRAP_ADMIN_PASSWORD=admin-pass-123
export FIXOPS_LOG_DIR=target/e2e-log
exec cargo run -q
