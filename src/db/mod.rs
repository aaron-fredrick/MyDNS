use anyhow::Context;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool,
};
use std::{str::FromStr, time::Duration};

pub mod records;

/// Initialises the SQLite connection pool and runs all DDL migrations.
pub async fn init(db_path: &str) -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}?mode=rwc", db_path))
        .with_context(|| format!("Failed to parse SQLite database URL for '{}'", db_path))?
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .with_context(|| format!("Failed to open SQLite database at '{}'", db_path))?;

    runMigrations(&pool).await?;
    Ok(pool)
}

#[allow(non_snake_case)]
async fn runMigrations(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS dns_records (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT    NOT NULL,
            record_type TEXT    NOT NULL,
            value       TEXT    NOT NULL,
            ttl         INTEGER NOT NULL DEFAULT 300,
            priority    INTEGER,
            created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT    NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            username      TEXT    NOT NULL UNIQUE,
            password_hash TEXT    NOT NULL,
            created_at    TEXT    NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS dns_cache (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT    NOT NULL,
            record_type TEXT    NOT NULL,
            value       TEXT    NOT NULL,
            ttl         INTEGER NOT NULL,
            expires_at  INTEGER NOT NULL,
            priority    INTEGER
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Cache rows are one row per returned record, so identity includes the
    // value and MX priority rather than only name/type. Remove legacy
    // duplicates before enforcing that identity for future writes.
    sqlx::query(
        r#"
        DELETE FROM dns_cache
        WHERE id NOT IN (
            SELECT MAX(id)
            FROM dns_cache
            GROUP BY lower(name), upper(record_type), value, COALESCE(priority, -1)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_cache_identity ON dns_cache(lower(name), upper(record_type), value, COALESCE(priority, -1))",
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_cache_name_type ON dns_cache(name, record_type)")
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Authoritative zones — source of truth at runtime; seeded from config on
    // first boot and then managed exclusively via the Zones API / UI.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS zones (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            name       TEXT    NOT NULL UNIQUE,
            created_at TEXT    NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Ephemeral dev records are purged on every startup before the record index
    // is loaded. SQLite lacks ADD COLUMN IF NOT EXISTS, so we probe first.
    let has_is_dev: bool = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM pragma_table_info('dns_records') WHERE name = 'is_dev'",
    )
    .fetch_one(pool)
    .await
    .map(|n| n > 0)
    .unwrap_or(false);

    if !has_is_dev {
        sqlx::query(
            "ALTER TABLE dns_records ADD COLUMN is_dev INTEGER NOT NULL DEFAULT 0",
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

#[allow(non_snake_case)]
/// Looks up a setting value. Returns `None` when the key is absent.
pub async fn getSetting(pool: &SqlitePool, key: &str) -> anyhow::Result<Option<String>> {
    let row = sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

#[allow(non_snake_case)]
/// Inserts or replaces a setting value.
pub async fn setSetting(pool: &SqlitePool, key: &str, value: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
    Ok(())
}
