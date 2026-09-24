use sqlx::{migrate::Migrator, SqlitePool};

static MIGRATOR: Migrator = sqlx::migrate!();

pub(super) async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    MIGRATOR.run(pool).await.map_err(|error| {
        anyhow::anyhow!("Failed to run observability database migrations: {error}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrations_create_metrics_schema() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let database =
            crate::observability::database::ObservabilityDatabase::init(
                file.path().to_str().unwrap(),
            )
            .await
            .unwrap();

        let table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'historical_buckets'",
        )
        .fetch_one(database.pool())
        .await
        .unwrap();

        assert_eq!(table_count, 1);
    }
}
