# Performance verification

`REQ-NF-001` requires 10 concurrent staff, 10,000 orders, and at least 95% of each routine action within two seconds from request to visible result. This check measures HTTP response latency only. Browser rendering and visible-result timing still require separate validation.

Use a disposable, fresh PostgreSQL database and a release build. The fixture directly inserts 10,000 clients, devices and Received orders and a technician account; it is test data, not an audited production import. Ten separate administrator accounts sign in over HTTP. Ten workers each make 20 sequential cycles concurrently: client search, device search, order open, order creation (save), and Received-to-Diagnosing transition with findings. There are 200 samples per category. The driver checks successful status and relevant response content, reports nearest-rank p95, and fails if any request fails or a category exceeds two seconds. Database seeding, account creation, and sign-in are outside timing. Searches use matching names and serial numbers; they are not a comprehensive search workload. Order creation adds 200 orders during the run. The driver expects an empty fresh database on each run.

Example with the running Compose service and its existing `db` container, from the repository root:

```sh
docker compose -p fix-ops up -d --wait
docker compose -p fix-ops exec -T db createdb -U fixops fixops_performance
cargo build --release
DATABASE_URL=postgres://fixops:fixops@127.0.0.1:5432/fixops_performance \
  FIXOPS_LISTEN_ADDR=127.0.0.1:18080 \
  FIXOPS_BOOTSTRAP_ADMIN_USERNAME=perf.admin \
  FIXOPS_BOOTSTRAP_ADMIN_PASSWORD=performance-test-password \
  FIXOPS_LOG_DIR=/tmp/fixops-performance-logs target/release/fixops
```

In another terminal, after the server reports it is listening:

```sh
docker compose -p fix-ops exec -T db psql -v ON_ERROR_STOP=1 -U fixops -d fixops_performance < crates/infra/fixtures/performance.sql
python3 scripts/check-performance.py
```

The diagnosis route must be implemented before the transition can pass. The driver reports errors as failures even if their response times are fast. Stop the server when done; use a new database for another run. Preserve the output, environment, and service log analysis with the verification record. This is a local API check, not a browser or production-capacity test.
