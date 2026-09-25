//! Generic mergeable histogram support.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeableHistogram {
    pub bounds: Vec<f64>,
    pub counts: Vec<u64>,
    pub count: u64,
    pub sum: f64,
}

impl MergeableHistogram {
    pub fn new(bounds: &[f64]) -> Self {
        assert!(!bounds.is_empty(), "histogram requires at least one bound");
        assert!(
            bounds.windows(2).all(|window| window[0] < window[1]),
            "histogram bounds must be strictly increasing"
        );

        Self {
            bounds: bounds.to_vec(),
            counts: vec![0; bounds.len() + 1],
            count: 0,
            sum: 0.0,
        }
    }

    pub fn record(&mut self, value: f64) {
        if !value.is_finite() || value < 0.0 {
            return;
        }

        self.count += 1;
        self.sum += value;

        let index = self
            .bounds
            .iter()
            .position(|bound| value <= *bound)
            .unwrap_or(self.bounds.len());
        self.counts[index] += 1;
    }

    pub fn merge(&mut self, other: &Self) {
        if self.bounds != other.bounds {
            return;
        }

        self.count += other.count;
        self.sum += other.sum;

        for (left, right) in self.counts.iter_mut().zip(&other.counts) {
            *left += right;
        }
    }

    pub fn mean(&self) -> Option<f64> {
        (self.count > 0).then_some(self.sum / self.count as f64)
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
                    self.bounds[index - 1]
                };
                let upper = self
                    .bounds
                    .get(index)
                    .copied()
                    .unwrap_or_else(|| *self.bounds.last().unwrap());

                let position = if bucket_count == 1 {
                    0.5
                } else {
                    ((rank - cumulative as f64) / bucket_count as f64).clamp(0.0, 1.0)
                };

                return Some(lower + (upper - lower) * position);
            }

            cumulative = next;
        }

        self.bounds.last().copied()
    }
}
