use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::net::Ipv4Addr;

use hickory_proto::rr::{Name, Record, RecordType};
use hickory_resolver::config::{ConnectionConfig, NameServerConfig, ResolverConfig, ResolverOpts};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::TokioResolver;

use crate::config::ResolverPriority;
use crate::observability::Metrics;

// ... existing resolver implementation unchanged ...
