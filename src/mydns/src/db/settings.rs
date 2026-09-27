use anyhow::Context;
use sqlx::SqlitePool;

/// Looks up a setting value. Returns `None` when the key is absent.
pub async fn get_setting(pool: &SqlitePool, key: &str) -> anyhow::Result<Option<String>> {
    let row = sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

/// Inserts or replaces a setting value.
pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await
        .context("Failed to persist setting")?;
    Ok(())
}
