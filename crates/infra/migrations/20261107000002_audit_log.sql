-- One row per changed field of every successful change (REQ-NF-003, REQ-F-018).

CREATE TABLE audit_log (
    id             bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    actor_id       bigint      NOT NULL REFERENCES staff_accounts (id),
    recorded_at    timestamptz NOT NULL DEFAULT now(),
    record_type    text        NOT NULL
        CHECK (record_type IN ('StaffAccount', 'Client', 'Device', 'RepairOrder')),
    record_id      bigint      NOT NULL CHECK (record_id >= 1),
    field          text        NOT NULL CHECK (field <> ''),
    previous_value text,
    new_value      text
);

CREATE INDEX audit_log_record_idx ON audit_log (record_type, record_id);

-- Audit rows are append-only.
CREATE FUNCTION audit_log_append_only() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'audit_log is append-only';
END;
$$;

CREATE TRIGGER audit_log_append_only
    BEFORE UPDATE OR DELETE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION audit_log_append_only();
