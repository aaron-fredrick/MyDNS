use anyhow::Context;
use chrono::Utc;
use serde::Serialize;
use sqlx::{FromRow, Row, SqlitePool};

/// A persistent DNS cache row as stored in SQLite.
#[derive(Debug, Clone, Serialize)]
pub struct CacheRow {
    pub id: i64,
    pub name: String,
    pub record_type: String,
    pub value: String,
    pub ttl: i64,
    pub expires_at: i64,
    pub priority: Option<i64>,
}

impl<'r> FromRow<'r, sqlx::sqlite::SqliteRow> for CacheRow {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            record_type: row.try_get("record_type")?,
            value: row.try_get("value")?,
            ttl: row.try_get("ttl")?,
            expires_at: row.try_get("expires_at")?,
            priority: row.try_get("priority")?,
        })
    }
}

pub async fn get_cache(
    pool: &SqlitePool,
    name: &str,
    record_type: &str,
) -> anyhow::Result<Vec<CacheRow>> {
    let now = Utc::now().timestamp();
    sqlx::query_as::<_, CacheRow>(
        "SELECT id, name, record_type, value, ttl, expires_at, priority \
         FROM dns_cache WHERE lower(trim(name, '.')) = lower(trim(?, '.')) AND upper(record_type) = upper(?) \
         AND expires_at > ?",
    )
    .bind(name)
    .bind(record_type)
    .bind(now)
    .fetch_all(pool)
    .await
    .context("Failed to query DNS cache")
}

pub async fn insert_cache(
    pool: &SqlitePool,
    name: &str,
    record_type: &str,
    value: &str,
    ttl: u32,
    priority: Option<i64>,
) -> anyhow::Result<()> {
    let expires_at = Utc::now().timestamp() + (ttl as i64);
    let normalized_name = name.trim_end_matches('.').to_lowercase();
    sqlx::query(
        "INSERT INTO dns_cache (name, record_type, value, ttl, expires_at, priority) \
         VALUES (?, upper(?), ?, ?, ?, ?) \
         ON CONFLICT DO UPDATE SET \
            ttl = excluded.ttl, \
            expires_at = excluded.expires_at, \
            priority = excluded.priority",
    )
    .bind(&normalized_name)
    .bind(record_type)
    .bind(value)
    .bind(ttl as i64)
    .bind(expires_at)
    .bind(priority)
    .execute(pool)
    .await
    .context("Failed to insert into DNS cache")?;
    Ok(())
}

pub async fn list_cache_entries(pool: &SqlitePool) -> anyhow::Result<Vec<CacheRow>> {
    let now = Utc::now().timestamp();
    sqlx::query_as::<_, CacheRow>(
        "SELECT id, name, record_type, value, ttl, expires_at, priority \
         FROM dns_cache WHERE expires_at > ? ORDER BY name",
    )
    .bind(now)
    .fetch_all(pool)
    .await
    .context("Failed to list DNS cache")
}

pub async fn delete_cache_entry(pool: &SqlitePool, name: &str, rtype: &str) -> anyhow::Result<()> {
    let normalized_name = name.trim_end_matches('.').to_lowercase();
    sqlx::query(
        "DELETE FROM dns_cache WHERE lower(name) = lower(?) AND upper(record_type) = upper(?)",
    )
    .bind(&normalized_name)
    .bind(rtype)
    .execute(pool)
    .await
    .context("Failed to delete cache entry")?;
    Ok(())
}

/// Returns authoritative CNAME dependents of a name, recursively.
///
/// A dependent is a DNS name whose CNAME chain eventually points at `name`.
/// `UNION` (rather than `UNION ALL`) makes the traversal cycle-safe.
pub async fn find_cname_dependents(pool: &SqlitePool, name: &str) -> anyhow::Result<Vec<String>> {
    sqlx::query_scalar::<_, String>(
        r#"
        WITH RECURSIVE dependents(name) AS (
            SELECT lower(name)
            FROM dns_records
            WHERE record_type = 'CNAME'
              AND lower(trim(value, '.')) = lower(trim(?, '.'))
            UNION
            SELECT lower(r.name)
            FROM dns_records r
            JOIN dependents d
              ON r.record_type = 'CNAME'
             AND lower(trim(r.value, '.')) = d.name
        )
        SELECT name FROM dependents
        "#,
    )
    .bind(name)
    .fetch_all(pool)
    .await
    .context("Failed to find CNAME cache dependents")
}

/// Removes every persistent cache entry for a DNS name and its authoritative
/// CNAME dependents.
pub async fn delete_cache_for_name(pool: &SqlitePool, name: &str) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        WITH RECURSIVE dependents(name) AS (
            SELECT lower(name)
            FROM dns_records
            WHERE record_type = 'CNAME'
              AND lower(trim(value, '.')) = lower(trim(?, '.'))
            UNION
            SELECT lower(r.name)
            FROM dns_records r
            JOIN dependents d
              ON r.record_type = 'CNAME'
             AND lower(trim(r.value, '.')) = d.name
        )
        DELETE FROM dns_cache
        WHERE lower(name) = lower(trim(?, '.'))
           OR lower(name) IN (SELECT name FROM dependents)
        "#,
    )
    .bind(name)
    .bind(name)
    .execute(pool)
    .await
    .context("Failed to delete DNS cache entries for name")?;
    Ok(())
}

/// Removes every persistent cache entry for the zone apex and all subdomains.
///
/// Called when a new authoritative zone is added so that any upstream-resolved
/// data cached before the zone was registered cannot shadow authoritative records.
pub async fn delete_cache_for_zone(pool: &SqlitePool, zone: &str) -> anyhow::Result<()> {
    let zone_lower = zone.trim_end_matches('.').to_lowercase();
    let subdomain_pattern = format!("%.{}", zone_lower);
    sqlx::query("DELETE FROM dns_cache WHERE lower(name) = ? OR lower(name) LIKE ?")
        .bind(&zone_lower)
        .bind(&subdomain_pattern)
        .execute(pool)
        .await
        .context("Failed to delete DNS cache entries for zone")?;
    Ok(())
}

pub async fn clear_cache(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM dns_cache")
        .execute(pool)
        .await
        .context("Failed to clear cache")?;
    Ok(())
}

pub async fn prune_cache(pool: &SqlitePool) -> anyhow::Result<u64> {
    let now = Utc::now().timestamp();
    let rows = sqlx::query("DELETE FROM dns_cache WHERE expires_at <= ?")
        .bind(now)
        .execute(pool)
        .await
        .context("Failed to prune cache")?
        .rows_affected();
    Ok(rows)
}
