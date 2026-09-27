use super::fixtures::TestDb;
use crate::mydns::db;
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn init_creates_all_tables_and_indexes() {
    let db = TestDb::new();
    let pool = db.init_pool().await;

    let tables: Vec<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .fetch_all(&pool)
            .await
            .unwrap();

    for table in [
        "blocklist",
        "dns_cache",
        "dns_records",
        "settings",
        "users",
        "zones",
    ] {
        assert!(
            tables.iter().any(|name| name == table),
            "missing table {table}"
        );
    }

    let indexes: Vec<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'index' ORDER BY name")
            .fetch_all(&pool)
            .await
            .unwrap();

    for index in [
        "idx_cache_identity",
        "idx_cache_name_type",
        "idx_blocklist_domain",
        "idx_blocklist_enabled",
    ] {
        assert!(
            indexes.iter().any(|name| name == index),
            "missing index {index}"
        );
    }

    let is_dev_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('dns_records') WHERE name = 'is_dev'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(is_dev_columns, 1);
}

#[tokio::test]
async fn init_upgrades_legacy_schema_and_removes_cache_duplicates() {
    let db = TestDb::new();
    let path = db.path_str();
    let legacy = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite://{}?mode=rwc", path))
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE dns_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            record_type TEXT NOT NULL,
            value TEXT NOT NULL,
            ttl INTEGER NOT NULL DEFAULT 300,
            priority INTEGER,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(&legacy)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE dns_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            record_type TEXT NOT NULL,
            value TEXT NOT NULL,
            ttl INTEGER NOT NULL,
            expires_at INTEGER NOT NULL,
            priority INTEGER
        )",
    )
    .execute(&legacy)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO dns_cache (name, record_type, value, ttl, expires_at, priority)
         VALUES ('dup.example', 'a', '192.0.2.1', 300, 9999999999, NULL)",
    )
    .execute(&legacy)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO dns_cache (name, record_type, value, ttl, expires_at, priority)
         VALUES ('dup.example', 'A', '192.0.2.1', 600, 9999999999, NULL)",
    )
    .execute(&legacy)
    .await
    .unwrap();
    legacy.close().await;

    let pool = db::init(&path).await.unwrap();

    let is_dev_default: i64 = sqlx::query_scalar("SELECT is_dev FROM dns_records WHERE id = 1")
        .fetch_optional(&pool)
        .await
        .unwrap()
        .unwrap_or(0);
    assert_eq!(is_dev_default, 0);

    let duplicate_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM dns_cache WHERE name = 'dup.example'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(duplicate_count, 1);

    let retained_ttl: i64 =
        sqlx::query_scalar("SELECT ttl FROM dns_cache WHERE name = 'dup.example'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(retained_ttl, 600);

    db::init(&path).await.unwrap();
}
