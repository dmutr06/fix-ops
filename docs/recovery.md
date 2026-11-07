# Backup and restore

The scripts use the PostgreSQL 18 tools in the Compose `db` container. `backup.sh` writes a PostgreSQL custom-format archive (`pg_dump -Fc`) to `var/backups` by default, named with the UTC date. Set `COMPOSE_PROJECT_NAME` and `FIXOPS_DB_USER` / `FIXOPS_DB_NAME` when the defaults differ.

Run a backup from the repository directory:

```sh
scripts/backup.sh
```

An optional first argument selects the archive path. A backup is written to a private temporary file and published only after `pg_dump` succeeds. Existing paths are never replaced.

## Daily schedule example

Install a cron entry for the service account that can run Docker Compose, replacing `/opt/fixops` with the deployment checkout:

```cron
17 2 * * * cd /opt/fixops && COMPOSE_PROJECT_NAME=fix-ops scripts/backup.sh >> var/log/backup.log 2>&1
```

Keep `var/backups` on storage with enough space and include it in the site's normal off-host retention policy. Check the cron log and backup file each day. The example schedules one run daily; it is not evidence of 24-hour monitoring or retention.

## Restore exercise

Choose a backup and a new database name. The restore command refuses the configured source database, requires the target name explicitly, and fails if that target already exists.

```sh
scripts/restore.sh var/backups/fixops-YYYY-MM-DD.dump fixops_recovery_YYYYMMDD
```

The command creates the target database and restores into it. If restoration fails, it drops only the target it created. It does not overwrite an existing database. Afterward, query important application records and their `audit_log` rows in the target, for example:

```sh
docker compose exec -T db psql -U fixops -d fixops_recovery_YYYYMMDD
```

Do not point the application at the recovery database until the operator has checked the restored data. Record the exercise start time, first successful verification query time, archive used, and any errors in the operational record. REQ-NF-004's 24-hour limit is measured from starting recovery to being able to query restored records. A single successful exercise does not establish ongoing 24-hour recovery performance; repeated timed exercises and daily backup checks remain operational work.
