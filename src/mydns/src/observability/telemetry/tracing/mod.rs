//! MyDNS tracing vocabulary — span names and structured field constants.
//!
//! This module owns the tracing *vocabulary* for the MyDNS process:
//!
//! - canonical span names ([`span_names`]) — the documented span hierarchy
//! - canonical structured field names ([`fields`]) — the safe attribute set
//!
//! ## What this module does NOT own
//!
//! Subscriber composition, log formatting, and non-blocking writer lifetime
//! have moved to their correct owners:
//!
//! - [`super::logging`] — output layers, file/stdout formatting, writer guard
//! - [`super::pipeline`] — registry assembly and global subscriber installation
//!
//! # Usage
//!
//! ```rust,ignore
//! use mydns::observability::telemetry::tracing::{fields, span_names};
//!
//! // reference span/field names from application instrumentation code:
//! let _ = span_names::DNS_REQUEST;
//! let _ = fields::COMPONENT;
//! ```

pub mod fields;
pub mod span_names;
