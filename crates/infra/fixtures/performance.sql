-- Load only into a fresh, migrated performance database after bootstrap.
INSERT INTO staff_accounts (username, password_hash, role)
SELECT 'perf.technician', password_hash, 'Technician' FROM staff_accounts WHERE username = 'perf.admin';
INSERT INTO clients (id, name, phone) OVERRIDING SYSTEM VALUE
SELECT n, 'Performance Client ' || n, '+1555' || lpad(n::text, 7, '0')
FROM generate_series(1, 10000) AS n;
INSERT INTO devices (id, client_id, device_type, serial_number, reported_symptoms)
OVERRIDING SYSTEM VALUE
SELECT n, n, 'Laptop', 'PERF-SERIAL-' || n, 'Does not start'
FROM generate_series(1, 10000) AS n;
INSERT INTO repair_orders (id, ticket_number, client_id, device_id, assigned_technician_id)
OVERRIDING SYSTEM VALUE
SELECT n, 'PERF-' || n, n, n, (SELECT id FROM staff_accounts WHERE username = 'perf.technician')
FROM generate_series(1, 10000) AS n;
SELECT setval(pg_get_serial_sequence('clients', 'id'), 10000);
SELECT setval(pg_get_serial_sequence('devices', 'id'), 10000);
SELECT setval(pg_get_serial_sequence('repair_orders', 'id'), 10000);
ANALYZE clients;
ANALYZE devices;
ANALYZE repair_orders;
