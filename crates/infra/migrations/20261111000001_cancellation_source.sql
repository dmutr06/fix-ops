-- Recover legacy stages only when append-only status history proves the source.
ALTER TABLE repair_orders ADD COLUMN cancelled_from text;

WITH cancellation_history AS (
    SELECT DISTINCT ON (record_id) record_id, previous_value
    FROM audit_log
    WHERE record_type = 'RepairOrder'
      AND field = 'status'
      AND new_value = 'Cancelled'
      AND previous_value IN ('Received', 'Diagnosing', 'InProgress')
    ORDER BY record_id, recorded_at DESC, id DESC
)
UPDATE repair_orders AS orders
SET cancelled_from = cancellation_history.previous_value
FROM cancellation_history
WHERE orders.id = cancellation_history.record_id
  AND orders.status = 'Cancelled';

-- NULL is retained for legacy rows with no trustworthy transition history.
ALTER TABLE repair_orders ADD CONSTRAINT repair_orders_cancellation_source_check
    CHECK (
        cancelled_from IS NULL
        OR (
            status = 'Cancelled'
            AND cancelled_from IN ('Received', 'Diagnosing', 'InProgress')
        )
    );
