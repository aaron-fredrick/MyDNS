use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, SqlitePool};

/// A DNS record as stored in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl<'r> FromRow<'r, sqlx::sqlite::SqliteRow> for DnsRecord {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            record_type: row.try_get("record_type")?,
            value: row.try_get("value")?,
            ttl: row.try_get("ttl")?,
            priority: row.try_get("priority")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            is_dev: row.try_get("is_dev")?,
        })
    }
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

/// Returns the total number of DNS records.
pub async fn count_records(pool: &SqlitePool) -> anyhow::Result<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM dns_records")
        .fetch_one(pool)
        .await
        .context("Failed to count DNS records")
}

/// Returns records matching a specific name (case-insensitive domain normalisation).
pub async fn find_by_name(pool: &SqlitePool, name: &str) -> anyhow::Result<Vec<DnsRecord>> {
    sqlx::query_as::<_, DnsRecord>(
        "SELECT id, name, record_type, value, ttl, priority, created_at, updated_at, is_dev \
         FROM dns_records WHERE lower(trim(name, '.')) = lower(trim(?, '.'))",
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
