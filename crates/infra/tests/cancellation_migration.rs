use sqlx::PgPool;

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn migration_recovers_cancelled_stage_from_audit_and_preserves_unknown(pool: PgPool) {
    let actor: i64 = sqlx::query_scalar("INSERT INTO staff_accounts (username, password_hash, role) VALUES ('tech', 'unused', 'Technician') RETURNING id")
        .fetch_one(&pool).await.unwrap();
    let client: i64 =
        sqlx::query_scalar("INSERT INTO clients (name, phone) VALUES ('Bob', '123') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    let device: i64 = sqlx::query_scalar("INSERT INTO devices (client_id, device_type, serial_number, reported_symptoms) VALUES ($1, 'Laptop', 'SN1', 'No power') RETURNING id")
        .bind(client).fetch_one(&pool).await.unwrap();
    sqlx::query("ALTER TABLE repair_orders DROP CONSTRAINT repair_orders_cancellation_source_check, DROP COLUMN cancelled_from")
        .execute(&pool).await.unwrap();
    let recovered: i64 = sqlx::query_scalar("INSERT INTO repair_orders (ticket_number, client_id, device_id, assigned_technician_id, status, labor_discount_cents, settlement_cents) VALUES ('T1', $1, $2, $3, 'Cancelled', 0, 3000) RETURNING id")
        .bind(client).bind(device).bind(actor).fetch_one(&pool).await.unwrap();
    let unknown: i64 = sqlx::query_scalar("INSERT INTO repair_orders (ticket_number, client_id, device_id, assigned_technician_id, status, labor_discount_cents, settlement_cents) VALUES ('T2', $1, $2, $3, 'Cancelled', 0, 0) RETURNING id")
        .bind(client).bind(device).bind(actor).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO audit_log (actor_id, record_type, record_id, field, previous_value, new_value) VALUES ($1, 'RepairOrder', $2, 'status', 'Diagnosing', 'Cancelled')")
        .bind(actor).bind(recovered).execute(&pool).await.unwrap();

    sqlx::raw_sql(include_str!(
        "../migrations/20261111000001_cancellation_source.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();

    let recovered_source: Option<String> =
        sqlx::query_scalar("SELECT cancelled_from FROM repair_orders WHERE id = $1")
            .bind(recovered)
            .fetch_one(&pool)
            .await
            .unwrap();
    let unknown_source: Option<String> =
        sqlx::query_scalar("SELECT cancelled_from FROM repair_orders WHERE id = $1")
            .bind(unknown)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(recovered_source.as_deref(), Some("Diagnosing"));
    assert_eq!(unknown_source, None);
}
