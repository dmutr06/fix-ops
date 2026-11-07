-- Staff accounts and their sign-in sessions (REQ-F-001, REQ-F-019, REQ-NF-002).

CREATE TABLE staff_accounts (
    id            bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    username      text    NOT NULL CONSTRAINT staff_accounts_username_key UNIQUE,
    password_hash text    NOT NULL,
    role          text    NOT NULL
        CHECK (role IN ('Receptionist', 'Technician', 'Administrator')),
    enabled       boolean NOT NULL DEFAULT true
);

-- Only the SHA-256 hash of a bearer token is stored.
CREATE TABLE sessions (
    token_hash bytea       PRIMARY KEY CHECK (length(token_hash) = 32),
    staff_id   bigint      NOT NULL REFERENCES staff_accounts (id),
    expires_at timestamptz NOT NULL
);

CREATE INDEX sessions_staff_id_idx ON sessions (staff_id);
