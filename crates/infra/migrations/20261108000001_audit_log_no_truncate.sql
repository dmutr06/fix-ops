-- Row triggers do not fire on TRUNCATE; block it with a statement trigger so the audit log
-- stays append-only (REQ-NF-003).

CREATE TRIGGER audit_log_no_truncate
    BEFORE TRUNCATE ON audit_log
    FOR EACH STATEMENT EXECUTE FUNCTION audit_log_append_only();
