pub mod blocklist;
pub mod handler;
pub mod metrics_handler;
pub mod record_index;
pub mod server;
pub mod zone_trie;

pub use handler::upstream;

#[cfg(test)]
mod tests;
