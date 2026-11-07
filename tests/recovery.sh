#!/usr/bin/env bash
set -euo pipefail

export COMPOSE_PROJECT_NAME=${COMPOSE_PROJECT_NAME:-fix-ops}
db_user=${FIXOPS_DB_USER:-fixops}
source_db="recovery_src_$$"
restore_db="recovery_dst_$$"
failed_restore_db="recovery_bad_$$"
archive=$(mktemp "${TMPDIR:-/tmp}/fixops-recovery.XXXXXX")
invalid_archive="$archive.invalid"
preserved_backup="$archive.preserved"

cleanup() {
    docker compose -p "$COMPOSE_PROJECT_NAME" exec -T db dropdb --if-exists -U "$db_user" "$source_db" >/dev/null
    docker compose -p "$COMPOSE_PROJECT_NAME" exec -T db dropdb --if-exists -U "$db_user" "$restore_db" >/dev/null
    docker compose -p "$COMPOSE_PROJECT_NAME" exec -T db dropdb --if-exists -U "$db_user" "$failed_restore_db" >/dev/null
    rm -f "$archive"
    rm -f "$invalid_archive" "$preserved_backup"
}
trap cleanup EXIT

docker compose -p "$COMPOSE_PROJECT_NAME" exec -T db createdb -U "$db_user" "$source_db"
docker compose -p "$COMPOSE_PROJECT_NAME" exec -T db psql -U "$db_user" -d "$source_db" -v ON_ERROR_STOP=1 <<'SQL'
CREATE TABLE staff_accounts (id bigint PRIMARY KEY);
CREATE TABLE clients (id bigint PRIMARY KEY, name text NOT NULL);
CREATE TABLE audit_log (
    id bigint PRIMARY KEY,
    actor_id bigint NOT NULL REFERENCES staff_accounts (id),
    record_type text NOT NULL,
    record_id bigint NOT NULL,
    field text NOT NULL,
    new_value text
);
INSERT INTO staff_accounts VALUES (17);
INSERT INTO clients VALUES (31, 'Recovery fixture');
INSERT INTO audit_log VALUES (53, 17, 'Client', 31, 'name', 'Recovery fixture');
SQL

FIXOPS_DB_NAME="$source_db" scripts/backup.sh "$archive"
FIXOPS_DB_NAME="$source_db" scripts/restore.sh "$archive" "$restore_db"

actual=$(docker compose -p "$COMPOSE_PROJECT_NAME" exec -T db psql -U "$db_user" -d "$restore_db" -At -v ON_ERROR_STOP=1 \
    -c "SELECT c.name || '|' || a.actor_id || '|' || a.field || '|' || a.new_value FROM clients c JOIN audit_log a ON a.record_id = c.id")
[[ "$actual" == 'Recovery fixture|17|name|Recovery fixture' ]]

printf 'keep existing backup' > "$preserved_backup"
if FIXOPS_DB_NAME="missing_database_$$" scripts/backup.sh "$preserved_backup"; then
    printf 'backup unexpectedly succeeded for a missing database\n' >&2
    exit 1
fi
[[ "$(cat "$preserved_backup")" == 'keep existing backup' ]]

printf 'not a PostgreSQL archive' > "$invalid_archive"
if FIXOPS_DB_NAME="$source_db" scripts/restore.sh "$invalid_archive" "$failed_restore_db"; then
    printf 'restore unexpectedly succeeded for an invalid archive\n' >&2
    exit 1
fi
exists=$(docker compose -p "$COMPOSE_PROJECT_NAME" exec -T db psql -U "$db_user" -d postgres -At \
    -c "SELECT count(*) FROM pg_database WHERE datname = '$failed_restore_db'")
[[ "$exists" == 0 ]]
printf 'backup and restore recovered record and audit row\n'
