#!/usr/bin/env bash
set -euo pipefail

db_user=${FIXOPS_DB_USER:-fixops}
db_name=${FIXOPS_DB_NAME:-fixops}
compose_project=${COMPOSE_PROJECT_NAME:-fix-ops}
archive=${1:-var/backups/fixops-$(date -u +%F).dump}
archive_dir=$(dirname "$archive")
archive_name=$(basename "$archive")

mkdir -p "$archive_dir"
if [[ -e "$archive" ]]; then
    printf 'backup already exists: %s\n' "$archive" >&2
    exit 1
fi

umask 077
temporary=$(mktemp "$archive_dir/.${archive_name}.XXXXXX")
trap 'rm -f "$temporary"' EXIT

docker compose -p "$compose_project" exec -T db pg_dump \
    -U "$db_user" -d "$db_name" -Fc > "$temporary"
[[ -s "$temporary" ]]
ln "$temporary" "$archive"
