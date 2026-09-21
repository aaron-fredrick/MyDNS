use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, SqlitePool};

/// A domain blocklist entry as stored in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistEntry {
    pub id: i64,
    pub domain: String,
    pub enabled: bool,
    /// Provenance tag: `"manual"` | `"imported"` | `"remote"`.
    pub source: String,
    pub reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl<'r> FromRow<'r, sqlx::sqlite::SqliteRow> for BlocklistEntry {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            domain: row.try_get("domain")?,
            enabled: row.try_get("enabled")?,
            source: row.try_get("source")?,
            reason: row.try_get("reason")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// Payload for creating a new blocklist entry.
#[derive(Debug, Deserialize)]
pub struct CreateBlocklistEntry {
    pub domain: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_source")]
    pub source: String,
    pub reason: Option<String>,
}

/// Payload for updating an existing blocklist entry.
#[derive(Debug, Deserialize)]
pub struct UpdateBlocklistEntry {
    pub enabled: Option<bool>,
    pub reason: Option<String>,
}

fn default_true() -> bool {
    true
}
fn default_source() -> String {
    "manual".to_string()
}

// ── Domain normalisation & validation ────────────────────────────────────────

/// Normalises and validates a domain name for blocklist storage.
///
/// - Converts to lowercase
/// - Strips a trailing dot
/// - Validates all labels (non-empty, ≤63 chars, alphanumeric/hyphens,
///   no leading/trailing hyphen)
/// - Rejects the root `"."` zone
pub fn normalize_domain(raw: &str) -> anyhow::Result<String> {
    if raw == "." {
        anyhow::bail!("The root zone '.' cannot be blocked");
    }
    let normalized = raw.trim_end_matches('.').to_lowercase();
    if normalized.is_empty() {
        anyhow::bail!("Domain name must not be empty");
    }
    if normalized.contains('/') || normalized.contains(':') || normalized.contains('@') {
        anyhow::bail!("Domain must be a plain DNS name (e.g. ads.example.com)");
    }
    for label in normalized.split('.') {
        if label.is_empty() || label.len() > 63 {
            anyhow::bail!("Domain contains an empty or oversized label");
        }
        if label.starts_with('-') || label.ends_with('-') {
            anyhow::bail!("DNS labels must not start or end with '-'");
        }
        if !label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            anyhow::bail!("DNS labels may only contain letters, digits, and hyphens");
        }
    }
    Ok(normalized)
}

// ── CRUD ─────────────────────────────────────────────────────────────────────

/// Returns all blocklist entries ordered by domain.
pub async fn list_entries(pool: &SqlitePool) -> anyhow::Result<Vec<BlocklistEntry>> {
    sqlx::query_as::<_, BlocklistEntry>(
        "SELECT id, domain, enabled, source, reason, created_at, updated_at \
         FROM blocklist ORDER BY domain",
    )
    .fetch_all(pool)
    .await
    .context("Failed to list blocklist entries")
}

/// Returns only the canonical domain strings of **enabled** blocklist entries.
///
/// Used exclusively on the DNS hot path and at startup to build
/// `BlocklistIndex`. Filters to `enabled = 1` so that disabled entries do
/// not affect resolution.
pub async fn list_enabled_domains(pool: &SqlitePool) -> anyhow::Result<Vec<String>> {
    sqlx::query_scalar::<_, String>(
        "SELECT domain FROM blocklist WHERE enabled = 1 ORDER BY domain",
    )
    .fetch_all(pool)
    .await
    .context("Failed to list enabled blocklist domains")
}

/// Returns a single entry by its primary key.
pub async fn get_entry(pool: &SqlitePool, id: i64) -> anyhow::Result<Option<BlocklistEntry>> {
    sqlx::query_as::<_, BlocklistEntry>(
        "SELECT id, domain, enabled, source, reason, created_at, updated_at \
         FROM blocklist WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch blocklist entry")
}

/// Inserts a new blocklist entry and returns the inserted row.
///
/// The domain is normalised via [`normalize_domain`] before insertion.
/// Returns an error if the domain already exists (UNIQUE constraint).
pub async fn create_entry(
    pool: &SqlitePool,
    req: &CreateBlocklistEntry,
) -> anyhow::Result<BlocklistEntry> {
    let canonical = normalize_domain(&req.domain)?;
    let source = req.source.trim();
    if !matches!(source, "manual" | "imported" | "remote") {
        anyhow::bail!(
            "Invalid source '{}'; must be one of: manual, imported, remote",
            source
        );
    }

    let id =
        sqlx::query("INSERT INTO blocklist (domain, enabled, source, reason) VALUES (?, ?, ?, ?)")
            .bind(&canonical)
            .bind(req.enabled as i64)
            .bind(source)
            .bind(req.reason.as_deref())
            .execute(pool)
            .await
            .context("Failed to insert blocklist entry")?
            .last_insert_rowid();

    get_entry(pool, id)
        .await?
        .context("Inserted blocklist entry not found after insert")
}

/// Updates `enabled` and/or `reason` on an existing entry.
///
/// Only non-`None` fields are changed. Returns `None` when the id does not
/// exist.
pub async fn update_entry(
    pool: &SqlitePool,
    id: i64,
    req: &UpdateBlocklistEntry,
) -> anyhow::Result<Option<BlocklistEntry>> {
    sqlx::query(
        "UPDATE blocklist SET \
            enabled    = COALESCE(?, enabled), \
            reason     = COALESCE(?, reason), \
            updated_at = datetime('now') \
         WHERE id = ?",
    )
    .bind(req.enabled.map(|e| e as i64))
    .bind(req.reason.as_deref())
    .bind(id)
    .execute(pool)
    .await
    .context("Failed to update blocklist entry")?;

    get_entry(pool, id).await
}

/// Deletes an entry by ID. Returns `true` if a row was removed.
pub async fn delete_entry(pool: &SqlitePool, id: i64) -> anyhow::Result<bool> {
    let rows = sqlx::query("DELETE FROM blocklist WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete blocklist entry")?
        .rows_affected();
    Ok(rows > 0)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_lowercase() {
        assert_eq!(
            normalize_domain("ADS.Example.COM").unwrap(),
            "ads.example.com"
        );
    }

    #[test]
    fn normalize_strips_trailing_dot() {
        assert_eq!(normalize_domain("example.com.").unwrap(), "example.com");
    }

    #[test]
    fn normalize_rejects_empty() {
        assert!(normalize_domain("").is_err());
        assert!(normalize_domain(".").is_err());
    }

    #[test]
    fn normalize_rejects_url_chars() {
        assert!(normalize_domain("http://ads.example.com").is_err());
    }

    #[test]
    fn normalize_rejects_leading_hyphen() {
        assert!(normalize_domain("-bad.example.com").is_err());
    }

    #[test]
    fn normalize_rejects_trailing_hyphen() {
        assert!(normalize_domain("bad-.example.com").is_err());
    }

    #[test]
    fn normalize_rejects_empty_label() {
        assert!(normalize_domain("bad..example.com").is_err());
    }

    #[test]
    fn normalize_accepts_valid() {
        assert!(normalize_domain("tracker.ads.example.com").is_ok());
        assert!(normalize_domain("xn--nxasmq6b.com").is_ok());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_accepts_maximum_label_length() {
        let domain = format!("{}.example.com", "a".repeat(63));
        assert_eq!(normalize_domain(&domain).unwrap(), domain);
    }

    #[test]
    fn normalize_rejects_oversized_label() {
        let domain = format!("{}.example.com", "a".repeat(64));
        assert!(normalize_domain(&domain).is_err());
    }

    #[test]
    fn normalize_rejects_colon_and_at_sign() {
        assert!(normalize_domain("dns:example.com").is_err());
        assert!(normalize_domain("user@example.com").is_err());
    }

    #[test]
    fn normalize_preserves_internal_labels_and_only_strips_trailing_dots() {
        assert_eq!(normalize_domain("  ADS.Example.COM.  ").unwrap(), "  ads.example.com.  ");
    }

    #[test]
    fn default_create_fields_are_stable() {
        assert!(default_true());
        assert_eq!(default_source(), "manual");
    }
}
