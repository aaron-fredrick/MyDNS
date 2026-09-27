//! Procedural macros used by the MyDNS workspace.
//!
//! This crate is intentionally kept separate from the runtime mydns crate.
//! Macro implementations provide compile-time structural contracts for the
//! runtime observability infrastructure.

use proc_macro::TokenStream;

mod metric_category_aggregator;

/// Enforces the structural contract for a metric category aggregator.
#[proc_macro_attribute]
pub fn metric_category_aggregator(attr: TokenStream, item: TokenStream) -> TokenStream {
    metric_category_aggregator::expand(attr, item)
}
