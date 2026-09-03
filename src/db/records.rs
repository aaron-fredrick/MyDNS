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
    /// When true the record is ephemeral and will be deleted on the next restart.
    pub is_dev: bool,
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
    /// When true the record is treated as an ephemeral dev record and will be
    /// deleted on the next server restart. Dev records bypass authoritative-zone
    /// validation, allowing testing against arbitrary domains (e.g. google.com).
    #[serde(default)]
    pub is_dev: bool,
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

/// Returns all authoritative (non-dev) DNS records ordered by name.
/// Used to build the in-memory record index at startup.
pub async fn list_records(pool: &SqlitePool) -> anyhow::Result<Vec<DnsRecord>> {
    sqlx::query_as::<_, DnsRecord>(
        "SELECT id, name, record_type, value, ttl, priority, created_at, updated_at, is_dev \
         FROM dns_records WHERE is_dev = 0 ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .context("Failed to list DNS records")
}

/// Returns all DNS records including dev records, ordered by name.
/// Used by the management API so the UI can display dev records.
pub async fn list_all_records(pool: &SqlitePool) -> anyhow::Result<Vec<DnsRecord>> {
    sqlx::query_as::<_, DnsRecord>(
        "SELECT id, name, record_type, value, ttl, priority, created_at, updated_at, is_dev \
         FROM dns_records ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .context("Failed to list DNS records")
}

/// Returns records matching a specific name (case-insensitive domain normalisation).
pub async fn find_by_name(pool: &SqlitePool, name: &str) -> anyhow::Result<Vec<DnsRecord>> {
    sqlx::query_as::<_, DnsRecord>(
        "SELECT id, name, record_type, value, ttl, priority, created_at, updated_at, is_dev \
         FROM dns_records WHERE lower(name) = lower(?)",
    )
    .bind(name)
    .fetch_all(pool)
    .await
    .context("Failed to query DNS records by name")
}

/// Returns a single record by its primary key.
pub async fn get_record(pool: &SqlitePool, id: i64) -> anyhow::Result<Option<DnsRecord>> {
    sqlx::query_as::<_, DnsRecord>(
        "SELECT id, name, record_type, value, ttl, priority, created_at, updated_at, is_dev \
         FROM dns_records WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch DNS record")
}

/// Inserts a new DNS record and returns the inserted row.
pub async fn create_record(pool: &SqlitePool, req: &CreateRecord) -> anyhow::Result<DnsRecord> {
    let id = sqlx::query(
        "INSERT INTO dns_records (name, record_type, value, ttl, priority, is_dev) \
         VALUES (?, upper(?), ?, ?, ?, ?)",
    )
    .bind(&req.name)
    .bind(&req.record_type)
    .bind(&req.value)
    .bind(req.ttl as i64)
    .bind(req.priority.map(|p| p as i64))
    .bind(req.is_dev as i64)
    .execute(pool)
    .await
    .context("Failed to insert DNS record")?
    .last_insert_rowid();

    get_record(pool, id)
        .await?
        .context("Inserted record not found after insert")
}

/// Updates a record in-place. Only non-`None` fields are changed.
pub async fn update_record(
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

    get_record(pool, id).await
}

/// Deletes a record by ID. Returns `true` if a row was removed.
pub async fn delete_record(pool: &SqlitePool, id: i64) -> anyhow::Result<bool> {
    let rows = sqlx::query("DELETE FROM dns_records WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete DNS record")?
        .rows_affected();
    Ok(rows > 0)
}

/// Looks up a user's hashed password by username.
pub async fn find_user_hash(pool: &SqlitePool, username: &str) -> anyhow::Result<Option<String>> {
    sqlx::query_scalar::<_, String>("SELECT password_hash FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await
        .context("Failed to query user")
}

/// Inserts the admin user if not already present.
pub async fn seed_admin(
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

// ── Zone Management ──────────────────────────────────────────────────────────

/// A zone entry as stored in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Zone {
    pub id: i64,
    pub name: String,
    pub created_at: String,
}

/// Returns all configured authoritative zones ordered by name.
pub async fn list_zones(pool: &SqlitePool) -> anyhow::Result<Vec<Zone>> {
    sqlx::query_as::<_, Zone>("SELECT id, name, created_at FROM zones ORDER BY name")
        .fetch_all(pool)
        .await
        .context("Failed to list zones")
}

/// Returns just the zone name strings from the DB (used to rebuild the trie).
pub async fn list_zone_names(pool: &SqlitePool) -> anyhow::Result<Vec<String>> {
    sqlx::query_scalar::<_, String>("SELECT name FROM zones ORDER BY name")
        .fetch_all(pool)
        .await
        .context("Failed to list zone names")
}

/// Inserts zones from the config file that are not already present in the DB.
/// Called once at startup; subsequent zone management is done via the API.
pub async fn seed_zones(pool: &SqlitePool, zones: &[String]) -> anyhow::Result<()> {
    for zone in zones {
        let normalized = zone.trim_end_matches('.').to_lowercase();
        if normalized.is_empty() && zone != "." {
            continue;
        }
        let canonical = if zone == "." {
            ".".to_string()
        } else {
            normalized
        };
        sqlx::query("INSERT INTO zones (name) VALUES (?) ON CONFLICT(name) DO NOTHING")
            .bind(&canonical)
            .execute(pool)
            .await
            .context("Failed to seed zone")?;
    }
    Ok(())
}

/// Inserts a new zone. Returns the inserted row or an error on duplicate.
pub async fn add_zone(pool: &SqlitePool, name: &str) -> anyhow::Result<Zone> {
    let id = sqlx::query("INSERT INTO zones (name) VALUES (?)")
        .bind(name)
        .execute(pool)
        .await
        .context("Failed to add zone")?
        .last_insert_rowid();

    sqlx::query_as::<_, Zone>("SELECT id, name, created_at FROM zones WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .context("Zone not found after insert")
}

/// Deletes a zone by name. Returns `true` if a row was removed.
pub async fn remove_zone(pool: &SqlitePool, name: &str) -> anyhow::Result<bool> {
    let mut tx = pool.begin().await.context("Failed to begin transaction")?;

    let rows = sqlx::query("DELETE FROM zones WHERE name = ?")
        .bind(name)
        .execute(&mut *tx)
        .await
        .context("Failed to remove zone")?
        .rows_affected();

    let pattern = format!("%.{}", name);
    sqlx::query("DELETE FROM dns_records WHERE name = ? OR name LIKE ?")
        .bind(name)
        .bind(pattern)
        .execute(&mut *tx)
        .await
        .context("Failed to remove associated records")?;

    tx.commit().await.context("Failed to commit zone removal")?;

    Ok(rows > 0)
}

// ── Dev Records ───────────────────────────────────────────────────────────────

/// Deletes all records marked `is_dev = 1`. Called on startup before loading
/// the record index so that ephemeral dev records do not persist across restarts.
pub async fn delete_dev_records(pool: &SqlitePool) -> anyhow::Result<u64> {
    let rows = sqlx::query("DELETE FROM dns_records WHERE is_dev = 1")
        .execute(pool)
        .await
        .context("Failed to delete dev records")?
        .rows_affected();
    Ok(rows)
}

// ── Cache Persistence ───────────────────────────────────────────────────────

pub async fn get_cache(
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

pub async fn insert_cache(
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
