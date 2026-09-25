//! Domain-neutral time-bucket primitives used by metric aggregation.

use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetricBucket {
    pub start: DateTime<Utc>,
    pub resolution_seconds: u32,
}

impl MetricBucket {
    pub fn new(start: DateTime<Utc>, resolution_seconds: u32) -> Self {
        assert!(resolution_seconds > 0, "bucket resolution must be positive");
        Self {
            start,
            resolution_seconds,
        }
    }

    pub fn end(&self) -> DateTime<Utc> {
        self.start + Duration::seconds(self.resolution_seconds as i64)
    }

    pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.start && timestamp < self.end()
    }

    pub fn next(&self) -> Self {
        Self::new(self.end(), self.resolution_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_has_fixed_half_open_window() {
        let start = DateTime::parse_from_rfc3339("2026-09-24T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let bucket = MetricBucket::new(start, 60);

        assert!(bucket.contains(start));
        assert!(bucket.contains(start + Duration::seconds(59)));
        assert!(!bucket.contains(bucket.end()));
        assert_eq!(bucket.next().start, bucket.end());
    }
}
