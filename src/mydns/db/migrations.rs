
use sqlx::SqlitePool;

/// Runs all DDL migrations: creates tables, enforces constraints, and applies
/// one-time schema amendments. Safe to call multiple times — every statement
/// is idempotent (CREATE IF NOT EXISTS, ADD COLUMN probed first, etc.).
pub(super) async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
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
        sqlx::query("ALTER TABLE dns_records ADD COLUMN is_dev INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await?;
    }

    // Domain blocklist — locally blocked domains that are never forwarded upstream.
    // Stored as lowercase canonical domain names (no trailing dot).
    // `source` is an open enum-ready column: 'manual' | 'imported' | 'remote'.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS blocklist (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            domain     TEXT    NOT NULL UNIQUE,
            enabled    INTEGER NOT NULL DEFAULT 1,
            source     TEXT    NOT NULL DEFAULT 'manual',
            reason     TEXT,
            created_at TEXT    NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT    NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blocklist_domain  ON blocklist(domain)")
        .execute(pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blocklist_enabled ON blocklist(enabled)")
        .execute(pool)
        .await?;

    Ok(())
}
