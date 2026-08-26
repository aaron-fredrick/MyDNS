#![allow(non_snake_case)]

use anyhow::Context;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// A DNS record as stored in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DnsRecord {
    pub id: i64,
    pub name: String,
    pub record_type: String,
    pub value: String,
    pub ttl: i64,
    /// MX priority (only meaningful for MX records).
    pub priority: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CacheRow {
    pub id: i64,
    pub name: String,
    pub record_type: String,
    pub value: String,
    pub ttl: i64,
    pub expires_at: i64,
    pub priority: Option<i64>,
}

/// Payload for creating a new DNS record.
#[derive(Debug, Deserialize)]
pub struct CreateRecord {
    pub name: String,
    pub record_type: String,
    pub value: String,
    pub ttl: u32,
    pub priority: Option<u16>,
}

/// Payload for updating an existing DNS record.
#[derive(Debug, Deserialize)]
pub struct UpdateRecord {
    pub name: Option<String>,
    pub record_type: Option<String>,
    pub value: Option<String>,
    pub ttl: Option<u32>,
    pub priority: Option<u16>,
}

// ── CRUD ──────────────────────────────────────────────────────────────────────

/// Returns all DNS records ordered by name.
pub async fn listRecords(pool: &SqlitePool) -> anyhow::Result<Vec<DnsRecord>> {
    sqlx::query_as::<_, DnsRecord>(
        "SELECT id, name, record_type, value, ttl, priority, created_at, updated_at \
         FROM dns_records ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .context("Failed to list DNS records")
}

/// Returns records matching a specific name (case-insensitive domain normalisation).
pub async fn findByName(pool: &SqlitePool, name: &str) -> anyhow::Result<Vec<DnsRecord>> {
    sqlx::query_as::<_, DnsRecord>(
        "SELECT id, name, record_type, value, ttl, priority, created_at, updated_at \
         FROM dns_records WHERE lower(name) = lower(?)",
    )
    .bind(name)
    .fetch_all(pool)
    .await
    .context("Failed to query DNS records by name")
}

/// Returns a single record by its primary key.
pub async fn getRecord(pool: &SqlitePool, id: i64) -> anyhow::Result<Option<DnsRecord>> {
    sqlx::query_as::<_, DnsRecord>(
        "SELECT id, name, record_type, value, ttl, priority, created_at, updated_at \
         FROM dns_records WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch DNS record")
}

/// Inserts a new DNS record and returns the inserted row.
pub async fn createRecord(pool: &SqlitePool, req: &CreateRecord) -> anyhow::Result<DnsRecord> {
    let id = sqlx::query(
        "INSERT INTO dns_records (name, record_type, value, ttl, priority) \
         VALUES (?, upper(?), ?, ?, ?)",
    )
    .bind(&req.name)
    .bind(&req.record_type)
    .bind(&req.value)
    .bind(req.ttl as i64)
    .bind(req.priority.map(|p| p as i64))
    .execute(pool)
    .await
    .context("Failed to insert DNS record")?
    .last_insert_rowid();

    getRecord(pool, id)
        .await?
        .context("Inserted record not found after insert")
}

/// Updates a record in-place. Only non-`None` fields are changed.
pub async fn updateRecord(
    pool: &SqlitePool,
    id: i64,
    req: &UpdateRecord,
) -> anyhow::Result<Option<DnsRecord>> {
    sqlx::query(
        "UPDATE dns_records SET \
            name        = COALESCE(?, name), \
            record_type = COALESCE(upper(?), record_type), \
            value       = COALESCE(?, value), \
            ttl         = COALESCE(?, ttl), \
            priority    = COALESCE(?, priority), \
            updated_at  = datetime('now') \
         WHERE id = ?",
    )
    .bind(req.name.as_deref())
    .bind(req.record_type.as_deref())
    .bind(req.value.as_deref())
    .bind(req.ttl.map(|t| t as i64))
    .bind(req.priority.map(|p| p as i64))
    .bind(id)
    .execute(pool)
    .await
    .context("Failed to update DNS record")?;

    getRecord(pool, id).await
}

/// Deletes a record by ID. Returns `true` if a row was removed.
pub async fn deleteRecord(pool: &SqlitePool, id: i64) -> anyhow::Result<bool> {
    let rows = sqlx::query("DELETE FROM dns_records WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete DNS record")?
        .rows_affected();
    Ok(rows > 0)
}

/// Looks up a user's hashed password by username.
pub async fn findUserHash(pool: &SqlitePool, username: &str) -> anyhow::Result<Option<String>> {
    sqlx::query_scalar::<_, String>("SELECT password_hash FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await
        .context("Failed to query user")
}

/// Inserts the admin user if not already present.
pub async fn seedAdmin(
    pool: &SqlitePool,
    username: &str,
    password_hash: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO users (username, password_hash) VALUES (?, ?) \
         ON CONFLICT(username) DO NOTHING",
    )
    .bind(username)
    .bind(password_hash)
    .execute(pool)
    .await
    .context("Failed to seed admin user")?;
    Ok(())
}
// ── Cache Persistence ───────────────────────────────────────────────────────

pub async fn getCache(
    pool: &SqlitePool,
    name: &str,
    record_type: &str,
) -> anyhow::Result<Vec<CacheRow>> {
    let now = Utc::now().timestamp();
    sqlx::query_as::<_, CacheRow>(
        "SELECT id, name, record_type, value, ttl, expires_at, priority \
         FROM dns_cache WHERE lower(name) = lower(?) AND upper(record_type) = upper(?) \
         AND expires_at > ?",
    )
    .bind(name)
    .bind(record_type)
    .bind(now)
    .fetch_all(pool)
    .await
    .context("Failed to query DNS cache")
}

pub async fn insertCache(
    pool: &SqlitePool,
    name: &str,
    record_type: &str,
    value: &str,
    ttl: u32,
    priority: Option<i64>,
) -> anyhow::Result<()> {
    let expires_at = Utc::now().timestamp() + (ttl as i64);
    sqlx::query(
        "INSERT INTO dns_cache (name, record_type, value, ttl, expires_at, priority) \
         VALUES (lower(?), upper(?), ?, ?, ?, ?) \
         ON CONFLICT DO UPDATE SET \
            ttl = excluded.ttl, \
            expires_at = excluded.expires_at, \
            priority = excluded.priority",
    )
    .bind(name)
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

pub async fn listCacheEntries(pool: &SqlitePool) -> anyhow::Result<Vec<CacheRow>> {
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

pub async fn deleteCacheEntry(pool: &SqlitePool, name: &str, rtype: &str) -> anyhow::Result<()> {
    sqlx::query(
        "DELETE FROM dns_cache WHERE lower(name) = lower(?) AND upper(record_type) = upper(?)",
    )
    .bind(name)
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
pub async fn findCnameDependents(pool: &SqlitePool, name: &str) -> anyhow::Result<Vec<String>> {
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
pub async fn deleteCacheForName(pool: &SqlitePool, name: &str) -> anyhow::Result<()> {
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

pub async fn clearCache(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM dns_cache")
        .execute(pool)
        .await
        .context("Failed to clear cache")?;
    Ok(())
}

pub async fn pruneCache(pool: &SqlitePool) -> anyhow::Result<u64> {
    let now = Utc::now().timestamp();
    let rows = sqlx::query("DELETE FROM dns_cache WHERE expires_at <= ?")
        .bind(now)
        .execute(pool)
        .await
        .context("Failed to prune cache")?
        .rows_affected();
    Ok(rows)
}
