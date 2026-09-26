//! Top-level metric aggregation structure.

#[derive(Debug, Default)]
pub struct MetricsAggregator {
    pub performance: PerformanceMetrics,
    pub operational: OperationalMetrics,
}

#[derive(Debug, Default)]
pub struct PerformanceMetrics {
    pub dns: DnsPerformanceMetrics,
    pub api: ApiPerformanceMetrics,
    pub database: DatabasePerformanceMetrics,
}

#[derive(Debug, Default)]
pub struct OperationalMetrics {
    pub dns: DnsOperationalMetrics,
    pub api: ApiOperationalMetrics,
    pub database: DatabaseOperationalMetrics,
}

#[derive(Debug, Default)]
pub struct DnsPerformanceMetrics;

#[derive(Debug, Default)]
pub struct ApiPerformanceMetrics;

#[derive(Debug, Default)]
pub struct DatabasePerformanceMetrics;

#[derive(Debug, Default)]
pub struct DnsOperationalMetrics;

#[derive(Debug, Default)]
pub struct ApiOperationalMetrics;

#[derive(Debug, Default)]
pub struct DatabaseOperationalMetrics;
