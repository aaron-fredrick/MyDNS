//! Domain-neutral time-period primitives used by metric implementations.

use chrono::{DateTime, Duration, Utc};

/// Identifies a metric aggregation period.
///
/// Unlike a bucket, a period is allowed to have an arbitrary duration. This is
/// useful for operational windows such as a local calendar day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetricPeriod {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl MetricPeriod {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        assert!(end > start, "metric period must have a positive duration");
        Self { start, end }
    }

    pub fn duration(&self) -> Duration {
        self.end - self.start
    }

    pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.start && timestamp < self.end
    }

    pub fn elapsed_seconds(&self) -> i64 {
        self.duration().num_seconds()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_is_half_open() {
        let start = DateTime::parse_from_rfc3339("2026-09-24T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let end = start + Duration::hours(24);
        let period = MetricPeriod::new(start, end);

        assert!(period.contains(start));
        assert!(!period.contains(end));
        assert_eq!(period.elapsed_seconds(), 86_400);
    }
}
