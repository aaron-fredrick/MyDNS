//! Scalar gauge metric type.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Gauge {
    value: f64,
}

impl Default for Gauge {
    fn default() -> Self {
        Self { value: 0.0 }
    }
}

impl Gauge {
    pub fn new(value: f64) -> Self {
        assert!(value.is_finite(), "gauge value must be finite");
        Self { value }
    }
    pub fn set(&mut self, value: f64) {
        if value.is_finite() {
            self.value = value;
        }
    }
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Merges another gauge snapshot by replacing this value with the other value.
    ///
    /// A gauge represents current state rather than an accumulative quantity, so
    /// it cannot be meaningfully summed like a counter or histogram. The caller
    /// is responsible for supplying the snapshot that should take precedence.
    pub fn merge(&mut self, other: &Self) {
        self.value = other.value;
    }
}
