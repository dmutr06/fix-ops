#!/usr/bin/env bash
set -euo pipefail

temporary_dir=$(mktemp -d "${TMPDIR:-/tmp}/fixops-cleanup.XXXXXX")
trap 'rm -rf "$temporary_dir"' EXIT
cat > "$temporary_dir/docker" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
shift 6 # compose -p PROJECT exec -T db
database=${!#}
case "$1" in
    createdb)
        if [[ "$COLLISION" == source || "$database" == recovery_dst_* ]]; then
            printf 'database already exists: %s\n' "$database" >&2
            exit 1
        fi
        printf '%s\n' "$database" >> "$EXPECTED_DROPS"
        ;;
    dropdb) printf '%s\n' "$database" >> "$ACTUAL_DROPS" ;;
    psql) cat >/dev/null ;;
    pg_dump) printf 'custom archive fixture' ;;
    *) exit 2 ;;
esac
MOCK
chmod +x "$temporary_dir/docker"

export EXPECTED_DROPS="$temporary_dir/expected"
export ACTUAL_DROPS="$temporary_dir/actual"
failures=0
for collision in source destination; do
    : > "$EXPECTED_DROPS"
    : > "$ACTUAL_DROPS"
    if PATH="$temporary_dir:$PATH" COLLISION="$collision" bash tests/recovery.sh; then
        printf 'expected recovery to fail on %s collision\n' "$collision" >&2
        failures=1
    fi
    if ! cmp -s "$EXPECTED_DROPS" "$ACTUAL_DROPS"; then
        printf 'cleanup dropped unowned databases after %s collision:\n' "$collision" >&2
        cat "$ACTUAL_DROPS" >&2
        failures=1
    fi
done
if [[ $failures -ne 0 ]]; then
    exit 1
fi
printf 'cleanup preserved pre-existing source and destination databases\n'
