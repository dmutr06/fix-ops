-- Retain the pricing stage for future corrections; approval and zero charges are ambiguous.
ALTER TABLE repair_orders ADD COLUMN cancelled_from text
    CHECK (cancelled_from IN ('Received', 'Diagnosing', 'InProgress'));
ALTER TABLE repair_orders ADD CONSTRAINT repair_orders_cancellation_source_check
    CHECK ((status = 'Cancelled') = (cancelled_from IS NOT NULL));
