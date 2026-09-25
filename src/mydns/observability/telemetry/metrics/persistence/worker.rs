use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::observability::telemetry::metrics::domains::dns::DnsAggregator;
use super::repository;
use crate::observability::database::ObservabilityDatabase;

const PERSIST_INTERVAL: Duration = Duration::from_secs(60);

pub fn spawn_persistence(
    metrics: Arc<DnsAggregator>,
    database: Arc<ObservabilityDatabase>,
    cancel: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(PERSIST_INTERVAL);

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
    })
}

async fn persist_once(
    metrics: &DnsAggregator,
    database: &ObservabilityDatabase,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now();

    metrics.finalize_due_periods(now);
    metrics.finalize_completed_buckets(now);

    let periods = metrics.pending_periods();
    let buckets = metrics.pending_buckets();

    if !periods.is_empty() || !buckets.is_empty() {
        repository::persist(database, &periods, &buckets).await?;
        metrics.acknowledge_periods(&periods);
        metrics.acknowledge_buckets(&buckets);
    }

    repository::roll_up(database, now).await?;
    Ok(())
}
