-- Optimistic concurrency for clients and devices, as for repair orders: every update checks
-- the version it read and increments it, so a concurrent save cannot silently overwrite
-- another one (REQ-F-017, REQ-NF-003).

ALTER TABLE clients ADD COLUMN version bigint NOT NULL DEFAULT 0 CHECK (version >= 0);
ALTER TABLE devices ADD COLUMN version bigint NOT NULL DEFAULT 0 CHECK (version >= 0);
