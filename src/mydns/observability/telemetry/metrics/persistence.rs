use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::observability::database::ObservabilityDatabase;
use super::aggregator::MetricsAggregator;
use super::repository;

pub fn spawn_persistence(
    metrics: Arc<MetricsAggregator>,
    database: Arc<ObservabilityDatabase>,
    cancel: CancellationToken,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(error) = persist_once(&metrics, &database).await {
                        tracing::error!(%error, "Failed to persist telemetry metrics");
                    }
                }
                _ = cancel.cancelled() => {
                    if let Err(error) = persist_once(&metrics, &database).await {
                        tracing::error!(%error, "Failed to persist telemetry metrics during shutdown");
                    }
                    break;
                }
            }
        }
    });
}

async fn persist_once(metrics: &MetricsAggregator, database: &ObservabilityDatabase) -> anyhow::Result<()> {
    let now = chrono::Utc::now();
    metrics.finalize_due_periods(now);
    metrics.finalize_completed_buckets(now);

    let periods = metrics.pending_periods();
    let buckets = metrics.pending_buckets();
    if periods.is_empty() && buckets.is_empty() { return Ok(()); }

    repository::persist(database, &periods, &buckets).await?;
    metrics.acknowledge_periods(&periods);
    metrics.acknowledge_buckets(&buckets);
    Ok(())
}
