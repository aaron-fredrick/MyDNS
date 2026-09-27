use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, SqlitePool};

/// A zone entry as stored in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub id: i64,
    pub name: String,
    pub created_at: String,
}

impl<'r> FromRow<'r, sqlx::sqlite::SqliteRow> for Zone {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            created_at: row.try_get("created_at")?,
        })
    }
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
///
/// For each zone, mandatory apex SOA and NS records are also inserted so that
/// every configured zone has authoritative apex RRsets from first boot.
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
        create_apex_soa_and_ns(pool, &canonical).await?;
    }
    Ok(())
}

/// Inserts a new zone. Returns the inserted row or an error on duplicate.
///
/// Mandatory apex SOA and NS records are inserted atomically so the zone is
/// immediately queryable at its apex without manual record creation.
pub async fn add_zone(pool: &SqlitePool, name: &str) -> anyhow::Result<Zone> {
    let id = sqlx::query("INSERT INTO zones (name) VALUES (?)")
        .bind(name)
        .execute(pool)
        .await
        .context("Failed to add zone")?
        .last_insert_rowid();

    create_apex_soa_and_ns(pool, name).await?;

    sqlx::query_as::<_, Zone>("SELECT id, name, created_at FROM zones WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .context("Zone not found after insert")
}

/// Deletes a zone by name. Returns `true` if a row was removed.
///
/// All `dns_records` whose name matches the zone apex or any subdomain are
/// deleted in the same transaction, which automatically removes the apex SOA
/// and NS records inserted at zone creation time.
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

/// Inserts mandatory apex SOA and NS records for a zone if they do not already
/// exist. Safe to call multiple times (idempotent — checks for existence first).
///
/// SOA value format: "<mname>. <rname>. <serial> <refresh> <retry> <expire> <minimum>"
pub async fn create_apex_soa_and_ns(pool: &SqlitePool, zone: &str) -> anyhow::Result<()> {
    // Normalise: strip trailing dot for the zone name used in record names.
    let apex = zone.trim_end_matches('.');

    let soa_value = format!(
        "ns1.{apex}. hostmaster.{apex}. 1 3600 600 86400 300",
        apex = apex
    );
    let ns_value = format!("ns1.{apex}.", apex = apex);

    let has_soa: bool = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dns_records \
         WHERE lower(name) = lower(?) AND record_type = 'SOA' AND is_dev = 0",
    )
    .bind(apex)
    .fetch_one(pool)
    .await
    .context("Failed to check for existing apex SOA")?
        > 0;

    if !has_soa {
        sqlx::query(
            "INSERT INTO dns_records (name, record_type, value, ttl, priority, is_dev) \
             VALUES (?, 'SOA', ?, 3600, NULL, 0)",
        )
        .bind(apex)
        .bind(&soa_value)
        .execute(pool)
        .await
        .context("Failed to insert apex SOA record")?;
    }

    let has_ns: bool = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dns_records \
         WHERE lower(name) = lower(?) AND record_type = 'NS' AND is_dev = 0",
    )
    .bind(apex)
    .fetch_one(pool)
    .await
    .context("Failed to check for existing apex NS")?
        > 0;

    if !has_ns {
        sqlx::query(
            "INSERT INTO dns_records (name, record_type, value, ttl, priority, is_dev) \
             VALUES (?, 'NS', ?, 3600, NULL, 0)",
        )
        .bind(apex)
        .bind(&ns_value)
        .execute(pool)
        .await
        .context("Failed to insert apex NS record")?;
    }

    Ok(())
}
