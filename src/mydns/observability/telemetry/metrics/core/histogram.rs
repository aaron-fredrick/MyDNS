//! Generic mergeable latency/distribution support.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeableHistogram {
    pub bounds_ms: Vec<f64>,
    pub counts: Vec<u64>,
    pub count: u64,
    pub sum_ms: f64,
}

impl MergeableHistogram {
    pub fn new(bounds_ms: &[f64]) -> Self {
        assert!(
            bounds_ms.windows(2).all(|window| window[0] < window[1]),
            "histogram bounds must be strictly increasing"
        );

        Self {
            bounds_ms: bounds_ms.to_vec(),
            counts: vec![0; bounds_ms.len() + 1],
            count: 0,
            sum_ms: 0.0,
        }
    }

    pub fn latency() -> Self {
        Self::new(&[
            1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 3000.0, 5000.0,
        ])
    }

    pub fn response() -> Self {
        // 500ms+ is a candidate warning threshold and 3000ms+ a candidate critical threshold
        // for DNS response latency. 5000ms is intentionally the final finite bound: above ~5s,
        // finer tail resolution is not useful for MyDNS operational decisions.
        // * IMPORTANT: these thresholds are candidates for alert rules, not alerting itself.
        // TODO: Wire suitable warning/critical latency thresholds into the alerting capability
        // after instrumentation and real workload data establish appropriate policy.
        Self::latency()
    }

    pub fn upstream() -> Self {
        // Upstream resolution can legitimately have a longer tail than local response work,
        // but 5000ms remains the practical upper bound for the same operational reason.
        Self::new(&[
            1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2000.0, 3000.0,
            5000.0,
        ])
    }

    pub fn record(&mut self, value_ms: f64) {
        if !value_ms.is_finite() || value_ms < 0.0 {
            return;
        }

        self.count += 1;
        self.sum_ms += value_ms;

        let index = self
            .bounds_ms
            .iter()
            .position(|bound| value_ms <= *bound)
            .unwrap_or(self.bounds_ms.len());
        self.counts[index] += 1;
    }

    pub fn merge(&mut self, other: &Self) {
        if self.bounds_ms != other.bounds_ms {
            return;
        }

        self.count += other.count;
        self.sum_ms += other.sum_ms;

        for (left, right) in self.counts.iter_mut().zip(&other.counts) {
            *left += right;
        }
    }

    pub fn mean_ms(&self) -> Option<f64> {
        (self.count > 0).then_some(self.sum_ms / self.count as f64)
    }

    /// Returns an approximate quantile using the histogram buckets.
    // TODO: Investigate a mergeable percentile/distribution algorithm or pipeline that can
    // retain enough information for more accurate p50/p95/p99 calculations. Determine whether
    // MyDNS actually needs exact percentile accuracy or whether bounded histograms are sufficient
    // for diagnostics, alerting, and long-term rollups before changing this representation.
    ///
    /// The estimate is intentionally bounded by the bucket containing the
    /// quantile. The overflow bucket has no finite upper bound, so quantiles
    /// landing there return the final configured bound.
    pub fn quantile(&self, quantile: f64) -> Option<f64> {
        if self.count == 0 || !quantile.is_finite() || !(0.0..=1.0).contains(&quantile) {
            return None;
        }

        let rank = quantile * self.count as f64;
        let mut cumulative = 0_u64;

        for (index, bucket_count) in self.counts.iter().copied().enumerate() {
            if bucket_count == 0 {
                continue;
            }

            let next = cumulative + bucket_count;
            if rank <= next as f64 {
                let lower = if index == 0 {
                    0.0
                } else {
                    self.bounds_ms[index - 1]
                };
                let upper = self
                    .bounds_ms
                    .get(index)
                    .copied()
                    .unwrap_or_else(|| *self.bounds_ms.last().unwrap());

                let position = if bucket_count == 1 {
                    0.5
                } else {
                    ((rank - cumulative as f64) / bucket_count as f64).clamp(0.0, 1.0)
                };

                return Some(lower + (upper - lower) * position);
            }

            cumulative = next;
        }

        self.bounds_ms.last().copied()
    }
}

impl Default for MergeableHistogram {
    fn default() -> Self {
        Self::response()
    }
}
