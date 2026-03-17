use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use hickory_proto::op::{Header, ResponseCode};
use hickory_proto::rr::{Name, RData, Record, RecordType};
use hickory_server::authority::MessageResponseBuilder;
use hickory_server::server::{Request, RequestHandler, ResponseHandler, ResponseInfo};

use crate::state::AppState;

/// Implements the hickory-server [`RequestHandler`] trait.
///
/// Resolution pipeline for every incoming query:
/// 1. Local in-memory cache (TTL-aware).
/// 2. SQLite DNS records table.
/// 3. Upstream DNS resolver chain (Cloudflare → router, or reversed per config).
/// 4. NXDOMAIN if all sources miss.
pub struct DnsHandler {
    state: Arc<AppState>,
}

impl DnsHandler {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl RequestHandler for DnsHandler {
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> ResponseInfo {
        let src = request.src();
        let query = request.query();
        let name_fqdn = query.name().to_string();
        let rtype = query.query_type();

        // Normalize name: strip trailing dot and lowercase for internal lookup
        let name = name_fqdn.trim_end_matches('.').to_lowercase();

        tracing::info!(
            client = %src,
            query  = %name_fqdn,
            rtype  = %rtype,
            "DNS query received"
        );

        let records = self.resolve(&name, rtype, src).await;
        let builder = MessageResponseBuilder::from_message_request(request);
        let mut header = Header::response_from_request(request.header());

        // Send directly from each branch to avoid the type-mismatch between
        // `build` and `build_no_records` (different concrete return types).
        if records.is_empty() {
            header.set_response_code(ResponseCode::NXDomain);
            let response = builder.build_no_records(header);
            return response_handle
                .send_response(response)
                .await
                .unwrap_or_else(|e| {
                    tracing::error!(error = %e, "Failed to send NXDOMAIN response");
                    ResponseInfo::from(Header::new())
                });
        }

        header.set_response_code(ResponseCode::NoError);
        header.set_authoritative(false);
        let response = builder.build(header, records.iter(), &[], &[], &[]);
        response_handle
            .send_response(response)
            .await
            .unwrap_or_else(|e| {
                tracing::error!(error = %e, "Failed to send DNS response");
                ResponseInfo::from(Header::new())
            })
    }
}

impl DnsHandler {
    /// Orchestrates the four-stage resolution pipeline.
    async fn resolve(&self, name: &str, rtype: RecordType, src: SocketAddr) -> Vec<Record> {
        // Stage 1: Memory Cache
        {
            let cache = self.state.cache.read().await;
            if let Some(records) = cache.get(name, rtype) {
                self.state.cache_stats.record_hit();
                
                let values = records.iter()
                    .filter_map(|r| r.data().map(|d| d.to_string()))
                    .collect::<Vec<_>>()
                    .join(", ");

                let _ = self.state.log_tx.send(format!(
                    "[CACHE HIT] client={} query={} type={} value=[{}]",
                    src, name, rtype, values
                ));
                tracing::info!(client = %src, query = %name, r#type = %rtype, value = %values, "Cache hit");
                return records.clone();
            }
        }
        self.state.cache_stats.record_miss();

        // Stage 2: Persistent DB Cache
        if let Some(records) = self.resolve_from_persistent_cache(name, rtype).await {
            let values = records.iter()
                .filter_map(|r| r.data().map(|d| d.to_string()))
                .collect::<Vec<_>>()
                .join(", ");

            let _ = self.state.log_tx.send(format!(
                "[CACHE HIT] client={} query={} type={} value=[{}] (persistent)",
                src, name, rtype, values
            ));
            tracing::info!(client = %src, query = %name, r#type = %rtype, value = %values, "Persistent cache hit");
            
            // Query hit DB cache -> Command: Update memory cache
            let ttl = records.iter().map(|r| r.ttl()).min().unwrap_or(300);
            self.save_to_memory_cache(name, rtype, records.clone(), ttl).await;
            
            return records;
        }

        // Stage 3: SQLite DNS Records (Manual)
        if let Some(records) = self.resolve_from_db(name, rtype).await {
            let values = records.iter()
                .filter_map(|r| r.data().map(|d| d.to_string()))
                .collect::<Vec<_>>()
                .join(", ");

            let _ = self.state.log_tx.send(format!(
                "[DB HIT] client={} query={} type={} value=[{}]",
                src, name, rtype, values
            ));
            tracing::info!(client = %src, query = %name, r#type = %rtype, value = %values, "Resolved from DB");
            
            // Query hit DB -> Command: Update memory cache
            let ttl = records.iter().map(|r| r.ttl()).min().unwrap_or(300);
            self.save_to_memory_cache(name, rtype, records.clone(), ttl).await;
            
            return records;
        }

        // Stage 3: Special Records (mydns.local)
        if let Some(records) = self.resolve_special_records(name, rtype, src).await {
            return records;
        }

        // Stage 4: persistent DB cache
        if let Some(records) = self.resolve_from_persistent_cache(name, rtype).await {
            if !records.is_empty() {
                let values = records.iter()
                    .filter_map(|r| r.data().map(|d| d.to_string()))
                    .collect::<Vec<_>>()
                    .join(", ");
                tracing::info!(client = %src, query = %name, r#type = %rtype, value = %values, "Persistent cache hit");
                let _ = self.state.log_tx.send(format!(
                    "[CACHE HIT] client={} query={} type={} value=[{}] (persistent)",
                    src, name, rtype, values
                ));
            }
            return records;
        }

        // Stage 5: upstream chain
        if let Some((records, ttl)) = self.resolve_upstream(name, rtype, src).await {
            // Query hit upstream -> Command: Save to both caches
            self.save_to_all_caches(name, rtype, records.clone(), ttl).await;
            return records;
        }

        // Stage 6: NXDOMAIN (Negative Caching)
        tracing::info!(client = %src, query = %name, r#type = %rtype, "NXDOMAIN");
        let _ = self.state.log_tx.send(format!(
            "[NXDOMAIN] client={} query={} type={}",
            src, name, rtype
        ));
        
        // Command: Save negative result to cache (short TTL)
        self.save_negative_cache(name, rtype, 60).await;
        
        vec![]
    }

    async fn resolve_from_db(&self, name: &str, rtype: RecordType) -> Option<Vec<Record>> {
        let rows = crate::db::records::find_by_name(&self.state.db, name)
            .await
            .ok()?;

        let rtype_str = rtype.to_string().to_uppercase();
        let matching: Vec<_> = rows
            .into_iter()
            .filter(|r| r.record_type == rtype_str)
            .collect();

        if matching.is_empty() {
            return None;
        }

        let mut records = Vec::new();
        for row in &matching {
            if let Some(record) = build_record(name, rtype, &row.value, row.ttl as u32, row.priority) {
                records.push(record);
            }
        }

        if records.is_empty() {
            return None;
        }

        Some(records)
    }

    async fn resolve_from_persistent_cache(&self, name: &str, rtype: RecordType) -> Option<Vec<Record>> {
        self.resolve_from_persistent_cache_recursive(name, rtype, 0).await
    }

    #[async_recursion::async_recursion]
    async fn resolve_from_persistent_cache_recursive(
        &self,
        name: &str,
        rtype: RecordType,
        depth: u8,
    ) -> Option<Vec<Record>> {
        if depth > 10 {
            tracing::warn!(name = %name, r#type = %rtype, depth = %depth, "CNAME recursion limit reached");
            return None;
        }

        let rows = match crate::db::records::get_cache(&self.state.db, name, &rtype.to_string()).await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(error = %e, name = %name, "Failed to query persistent cache");
                return None;
            }
        };

        // If we found the requested records directly
        if !rows.is_empty() {
            // Check for negative cache placeholder
            if rows.len() == 1 && rows[0].value == "NX" {
                let ttl_secs = (rows[0].expires_at - chrono::Utc::now().timestamp()).max(0) as u64;
                if ttl_secs > 0 {
                    self.save_to_memory_cache(name, rtype, vec![], ttl_secs as u32).await;
                }
                return Some(vec![]);
            }

            let mut records = Vec::new();
            for row in &rows {
                if let Some(record) = build_record(name, rtype, &row.value, row.ttl as u32, row.priority) {
                    records.push(record);
                }
            }

            if !records.is_empty() {
                // Populate memory cache for next time
                let ttl_secs = (rows[0].expires_at - chrono::Utc::now().timestamp()).max(0) as u64;
                if ttl_secs > 0 {
                    self.save_to_memory_cache(name, rtype, records.clone(), ttl_secs as u32).await;
                }
                return Some(records);
            }
        }

        // CNAME chasing in database: if we didn't find the requested type, check for a CNAME
        if rtype != RecordType::CNAME {
            let cname_rows = match crate::db::records::get_cache(&self.state.db, name, "CNAME").await {
                Ok(r) => r,
                _ => return None,
            };

            if !cname_rows.is_empty() {
                let cname_target = cname_rows[0].value.trim_end_matches('.').to_string();
                
                // Recursively look for the original rtype for the CNAME target
                if let Some(mut target_records) = self.resolve_from_persistent_cache_recursive(&cname_target, rtype, depth + 1).await {
                    // Prepend the CNAME record itself
                    if let Some(cname_rec) = build_record(name, RecordType::CNAME, &cname_rows[0].value, cname_rows[0].ttl as u32, None) {
                        target_records.insert(0, cname_rec);
                    }
                    return Some(target_records);
                }
            }
        }

        None
    }

    async fn resolve_special_records(
        &self,
        name: &str,
        rtype: RecordType,
        src: SocketAddr,
    ) -> Option<Vec<Record>> {
        if name != "mydns.local" {
            return None;
        }

        // Only handle A and AAAA for our special record
        if rtype != RecordType::A && rtype != RecordType::AAAA {
            return None;
        }

        let client_ip = src.ip();
        let target_ip = if client_ip.is_loopback() {
            "127.0.0.1".to_string()
        } else if is_private_ip(client_ip) {
            // Get local interface IP or fallback
            local_ip_address::local_ip()
                .map(|ip: IpAddr| ip.to_string())
                .unwrap_or_else(|_| "127.0.0.1".to_string())
        } else {
            // Public or unknown: use primary IP (we'll just use the local IP for now
            // as detecting public IP reliably is complex without external STUN/API)
            local_ip_address::local_ip()
                .map(|ip: IpAddr| ip.to_string())
                .unwrap_or_else(|_| "127.0.0.1".to_string())
        };

        if let Some(record) = build_record(name, rtype, &target_ip, 60, None) {
             let _ = self.state.log_tx.send(format!(
                "[SPECIAL] client={} query={} type={} value=[{}]",
                src, name, rtype, target_ip
            ));
            tracing::info!(client = %src, query = %name, r#type = %rtype, value = %target_ip, "Special record hit");
            return Some(vec![record]);
        }

        None
    }

    async fn resolve_upstream(
        &self,
        name: &str,
        rtype: RecordType,
        src: SocketAddr,
    ) -> Option<(Vec<Record>, u32)> {
        let fqdn = format!("{}.", name);
        let parsed_name = match fqdn.parse::<Name>() {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!(query = %name, error = %e, "Failed to parse name for upstream");
                return None;
            }
        };

        let upstream = self.state.upstream.read().await;
        let result = upstream.resolve(&parsed_name, rtype).await;

        let upstream_addr = {
            let cfg = self.state.config.read().await;
            match upstream.priority {
                crate::config::ResolverPriority::CloudflareFirst => cfg.cloudflare_dns.to_string(),
                crate::config::ResolverPriority::RouterFirst => upstream
                    .router_addr
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| cfg.cloudflare_dns.to_string()),
            }
        };

        match result {
            Some((records, ttl)) => {
                let values = records
                    .iter()
                    .filter_map(|r| r.data().map(|d| d.to_string()))
                    .collect::<Vec<_>>()
                    .join(", ");

                tracing::info!(
                    client = %src, 
                    query = %name, 
                    r#type = %rtype, 
                    value = %values, 
                    upstream = %upstream_addr,
                    "Upstream resolve hit"
                );

                let _ = self.state.log_tx.send(format!(
                    "[UPSTREAM] client={} query={} type={} value=[{}] server={}",
                    src, name, rtype, values, upstream_addr
                ));

                Some((records, ttl))
            }
            None => None,
        }
    }

    // ── Commands ──────────────────────────────────────────────────────────────

    async fn save_to_memory_cache(&self, name: &str, rtype: RecordType, records: Vec<Record>, ttl: u32) {
        let mut cache = self.state.cache.write().await;
        cache.insert(name, rtype, records, Duration::from_secs(ttl as u64));
    }

    async fn save_to_all_caches(&self, query_name: &str, query_type: RecordType, records: Vec<Record>, ttl: u32) {
        // 1. Memory Cache
        // We cache the whole set for the query name to support fast Stage 1 hits
        self.save_to_memory_cache(query_name, query_type, records.clone(), ttl).await;

        // 2. Persistent DB Cache (Per-record caching for multi-owner/proper caching)
        for r in &records {
            let record_owner = r.name().to_string().trim_end_matches('.').to_lowercase();
            let record_type = r.record_type();
            
            if let Some(val) = r.data().map(|d| d.to_string()) {
                let prio = match r.data() {
                    Some(hickory_proto::rr::RData::MX(mx)) => Some(mx.preference() as i64),
                    _ => None,
                };
                
                // Log caching for visibility
                tracing::debug!(name = %record_owner, r#type = %record_type, "Caching record to DB");

                let _ = crate::db::records::insert_cache(
                    &self.state.db,
                    &record_owner,
                    &record_type.to_string(),
                    &val,
                    ttl,
                    prio,
                )
                .await;
            }
        }
    }

    async fn save_negative_cache(&self, name: &str, rtype: RecordType, ttl: u32) {
        // Memory Cache
        {
            let mut cache = self.state.cache.write().await;
            cache.insert(name, rtype, vec![], Duration::from_secs(ttl as u64));
        }

        // Persistent DB Cache (using 'NX' type or empty value)
        let _ = crate::db::records::insert_cache(
            &self.state.db,
            name,
            &rtype.to_string(),
            "NX", // Special placeholder value
            ttl,
            None
        ).await;
    }
}

/// Converts a DB-stored string value into a typed hickory [`Record`].
pub fn build_record(
    name: &str,
    rtype: RecordType,
    value: &str,
    ttl: u32,
    priority: Option<i64>,
) -> Option<Record> {
    use hickory_proto::rr::rdata::{A, AAAA, CNAME, MX, PTR};
    use std::net::{Ipv4Addr, Ipv6Addr};

    let fqdn: Name = name.parse().ok()?;
    let rdata = match rtype {
        RecordType::A => {
            let ip: Ipv4Addr = value.parse().ok()?;
            RData::A(A(ip))
        }
        RecordType::AAAA => {
            let ip: Ipv6Addr = value.parse().ok()?;
            RData::AAAA(AAAA(ip))
        }
        RecordType::CNAME => {
            let target: Name = value.parse().ok()?;
            RData::CNAME(CNAME(target))
        }
        RecordType::MX => {
            let exchange: Name = value.parse().ok()?;
            let pref = priority.unwrap_or(10) as u16;
            RData::MX(MX::new(pref, exchange))
        }
        RecordType::PTR => {
            let target: Name = value.parse().ok()?;
            RData::PTR(PTR(target))
        }
        _ => return None,
    };

    let mut record = Record::new();
    record.set_name(fqdn);
    record.set_ttl(ttl);
    record.set_record_type(rtype);
    record.set_data(Some(rdata));
    Some(record)
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            // Check for ULAs (fc00::/7) or link-local (fe80::/10)
            let segments = v6.segments();
            (segments[0] & 0xfe00) == 0xfc00 || (segments[0] & 0xffc0) == 0xfe80
        }
    }
}

#[cfg(test)]
mod tests {
    use hickory_proto::rr::RecordType;
    use crate::dns::handler::build_record;

    #[test]
    fn build_a_record_from_valid_ip() {
        let record = build_record("example.com.", RecordType::A, "1.2.3.4", 300, None);
        assert!(record.is_some(), "Should build A record from valid IPv4");
        let r = record.unwrap();
        assert_eq!(r.ttl(), 300);
        assert_eq!(r.record_type(), RecordType::A);
    }

    #[test]
    fn build_a_record_from_invalid_ip_returns_none() {
        let record = build_record("example.com.", RecordType::A, "not-an-ip", 300, None);
        assert!(record.is_none(), "Invalid IP should yield None");
    }

    #[test]
    fn build_aaaa_record() {
        let record = build_record("example.com.", RecordType::AAAA, "::1", 60, None);
        assert!(record.is_some());
        assert_eq!(record.unwrap().record_type(), RecordType::AAAA);
    }

    #[test]
    fn build_cname_record() {
        let record = build_record(
            "alias.example.com.",
            RecordType::CNAME,
            "target.example.com.",
            300,
            None,
        );
        assert!(record.is_some());
        assert_eq!(record.unwrap().record_type(), RecordType::CNAME);
    }

    #[test]
    fn build_mx_record_with_priority() {
        let record =
            build_record("example.com.", RecordType::MX, "mail.example.com.", 300, Some(20));
        assert!(record.is_some());
        let r = record.unwrap();
        assert_eq!(r.record_type(), RecordType::MX);
    }
}
