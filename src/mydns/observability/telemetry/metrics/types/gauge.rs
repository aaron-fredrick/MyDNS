//! Gauge metric type.

use serde::{Deserialize, Serialize};

/// A point-in-time numeric value that may increase or decrease.
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
}
