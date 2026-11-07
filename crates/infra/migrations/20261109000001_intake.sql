-- Clients, devices and repair orders (REQ-F-003, REQ-F-004, REQ-F-005, REQ-F-017).

-- Trigram indexes serve the case-insensitive partial searches (REQ-NF-001).
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE clients (
    id           bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name         text NOT NULL CHECK (btrim(name) <> ''),
    phone        text NOT NULL CHECK (btrim(phone) <> ''),
    email        text CHECK (email LIKE '_%@_%._%'),
    loyalty_tier text NOT NULL DEFAULT 'Standard'
        CHECK (loyalty_tier IN ('Standard', 'Silver', 'Gold'))
);

CREATE INDEX clients_name_trgm_idx ON clients USING gin (name gin_trgm_ops);
CREATE INDEX clients_phone_trgm_idx ON clients USING gin (phone gin_trgm_ops);

CREATE TABLE devices (
    id                bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    client_id         bigint NOT NULL
        CONSTRAINT devices_client_id_fkey REFERENCES clients (id),
    device_type       text   NOT NULL CHECK (device_type IN ('Desktop', 'Laptop', 'Server')),
    serial_number     text   NOT NULL CHECK (btrim(serial_number) <> ''),
    reported_symptoms text   NOT NULL CHECK (btrim(reported_symptoms) <> ''),
    -- Target of the orders' (device, client) reference below.
    CONSTRAINT devices_id_client_id_key UNIQUE (id, client_id)
);

CREATE INDEX devices_client_id_idx ON devices (client_id);
CREATE INDEX devices_serial_number_trgm_idx ON devices USING gin (serial_number gin_trgm_ops);

-- Every field of the contract's RepairOrder. Amounts are whole cents; pairs that are recorded
-- together are both set or both NULL.
CREATE TABLE repair_orders (
    id                     bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    ticket_number          text   NOT NULL
        CONSTRAINT repair_orders_ticket_number_key UNIQUE
        CHECK (btrim(ticket_number) <> ''),
    client_id              bigint NOT NULL REFERENCES clients (id),
    device_id              bigint NOT NULL,
    assigned_technician_id bigint NOT NULL REFERENCES staff_accounts (id),
    status                 text   NOT NULL DEFAULT 'Received'
        CHECK (status IN ('Received', 'Diagnosing', 'InProgress', 'Ready', 'Completed', 'Cancelled')),
    diagnostic_findings    text,
    estimate_parts_cents   bigint CHECK (estimate_parts_cents >= 0),
    estimate_labor_cents   bigint CHECK (estimate_labor_cents >= 0),
    client_decision        text   CHECK (client_decision IN ('Approved', 'Declined')),
    client_decided_at      timestamptz,
    used_parts_cents       bigint  NOT NULL DEFAULT 0 CHECK (used_parts_cents >= 0),
    completed_labor_cents  bigint  NOT NULL DEFAULT 0 CHECK (completed_labor_cents >= 0),
    repair_checked         boolean NOT NULL DEFAULT false,
    payment_cents          bigint CHECK (payment_cents >= 0),
    paid_at                timestamptz,
    picked_up_at           timestamptz,
    -- Fixed when the order reaches Completed or Cancelled, NULL while it is open.
    labor_discount_cents   bigint CHECK (labor_discount_cents >= 0),
    settlement_cents       bigint CHECK (settlement_cents >= 0),
    version                bigint  NOT NULL DEFAULT 1 CHECK (version >= 1),

    -- The device belongs to the order's client.
    CONSTRAINT repair_orders_device_of_client_fkey
        FOREIGN KEY (device_id, client_id) REFERENCES devices (id, client_id),
    CHECK ((estimate_parts_cents IS NULL) = (estimate_labor_cents IS NULL)),
    CHECK ((client_decision IS NULL) = (client_decided_at IS NULL)),
    CHECK ((payment_cents IS NULL) = (paid_at IS NULL)),
    CHECK ((labor_discount_cents IS NULL) = (settlement_cents IS NULL)),
    CHECK ((settlement_cents IS NOT NULL) = (status IN ('Completed', 'Cancelled')))
);

CREATE INDEX repair_orders_client_id_idx ON repair_orders (client_id);
CREATE INDEX repair_orders_device_id_idx ON repair_orders (device_id, client_id);
CREATE INDEX repair_orders_assigned_technician_id_idx ON repair_orders (assigned_technician_id);
CREATE INDEX repair_orders_status_idx ON repair_orders (status);
