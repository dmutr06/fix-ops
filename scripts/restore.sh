#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
    printf 'usage: %s ARCHIVE TARGET_DATABASE\n' "$0" >&2
    exit 2
fi

archive=$1
target_db=$2
db_user=${FIXOPS_DB_USER:-fixops}
source_db=${FIXOPS_DB_NAME:-fixops}
compose_project=${COMPOSE_PROJECT_NAME:-fix-ops}

case "$target_db" in
    ''|[!a-zA-Z_]*|*[!a-zA-Z0-9_]*)
        printf 'invalid target database name: %s\n' "$target_db" >&2
        exit 2
        ;;
esac
if [[ "$target_db" == "$source_db" ]]; then
    printf 'restore target must differ from source database: %s\n' "$source_db" >&2
    exit 2
fi
if [[ ! -s "$archive" ]]; then
    printf 'backup archive is missing or empty: %s\n' "$archive" >&2
    exit 1
fi

created=0
cleanup() {
    status=$?
    if [[ $created -eq 1 ]]; then
        docker compose -p "$compose_project" exec -T db dropdb --if-exists \
            -U "$db_user" "$target_db" >/dev/null 2>&1 || true
    fi
    return "$status"
}
trap cleanup EXIT

docker compose -p "$compose_project" exec -T db createdb -U "$db_user" "$target_db"
created=1
docker compose -p "$compose_project" exec -T db pg_restore \
    --exit-on-error -U "$db_user" -d "$target_db" < "$archive"
created=0
