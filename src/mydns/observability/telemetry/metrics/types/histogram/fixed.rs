use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Histogram {
    pub bounds: Vec<f64>,
    pub counts: Vec<u64>,
    pub count: u64,
    pub sum: f64,
}

impl Histogram {
    pub fn new(bounds: &[f64]) -> Self {
        assert!(!bounds.is_empty(), "histogram bounds must not be empty");
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

        let index = self
            .bounds
            .iter()
            .position(|bound| value <= *bound)
            .unwrap_or(self.bounds.len());

        self.count = self.count.saturating_add(1);
        self.sum += value;
        self.counts[index] = self.counts[index].saturating_add(1);
    }

    pub fn mean(&self) -> Option<f64> {
        (self.count > 0).then_some(self.sum / self.count as f64)
    }

    pub fn merge(&mut self, other: &Self) {
        assert_eq!(self.bounds, other.bounds, "histogram bounds must match");

        self.count = self.count.saturating_add(other.count);
        self.sum += other.sum;

        for (left, right) in self.counts.iter_mut().zip(&other.counts) {
            *left = left.saturating_add(*right);
        }
    }

    pub fn quantile(&self, q: f64) -> Option<f64> {
        if self.count == 0 || !q.is_finite() || !(0.0..=1.0).contains(&q) {
            return None;
        }

        let target = q * self.count as f64;
        let mut cumulative = 0_u64;

        for (index, &bucket_count) in self.counts.iter().enumerate() {
            if bucket_count == 0 {
                continue;
            }

            cumulative = cumulative.saturating_add(bucket_count);
            if cumulative as f64 >= target {
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

                let previous = cumulative.saturating_sub(bucket_count);
                let position = if bucket_count == 0 {
                    0.0
                } else {
                    (target - previous as f64) / bucket_count as f64
                };

                return Some(lower + (upper - lower) * position.clamp(0.0, 1.0));
            }
        }

        self.bounds.last().copied()
    }
}
