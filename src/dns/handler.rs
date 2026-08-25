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

#[allow(non_snake_case)]
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

        let records = self.processResolution(&name, rtype, src).await;
        let builder = MessageResponseBuilder::from_message_request(request);
        let mut header = Header::response_from_request(request.header());

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

#[allow(non_snake_case)]
impl DnsHandler {
    /// Orchestrates the multi-stage resolution pipeline.
    /// This is a Command that coordinate Queries.
    async fn processResolution(
        &self,
        name: &str,
        rtype: RecordType,
        src: SocketAddr,
    ) -> Vec<Record> {
        // Stage 1: Memory Cache lookup (Query)
        if let Some(records) = self.queryMemoryCache(name, rtype, src).await {
            return records;
        }

        // Stage 2: Persistent Cache lookup (Query)
        if let Some(records) = self.queryPersistentCache(name, rtype).await {
            if !records.is_empty() {
                self.logResolution(src, name, rtype, &records, "persistent");

                // Command: Update memory cache for faster subsequent access
                let ttl = records.iter().map(|r| r.ttl()).min().unwrap_or(300);
                self.saveToMemoryCache(name, rtype, records.clone(), ttl)
                    .await;
                return records;
            }
        }

        // Stage 3: Manual DNS Records lookup (Query)
        if let Some(records) = self.queryDatabase(name, rtype).await {
            self.logResolution(src, name, rtype, &records, "DB");

            // Command: Update memory cache
            let ttl = records.iter().map(|r| r.ttl()).min().unwrap_or(300);
            self.saveToMemoryCache(name, rtype, records.clone(), ttl)
                .await;
            return records;
        }

        // Stage 4: Special Records (e.g., mydns.local) (Query)
        if let Some(records) = self.querySpecialRecords(name, rtype, src).await {
            return records;
        }

        // Stage 5: Upstream resolution (Query)
        if let Some((records, ttl)) = self.queryUpstream(name, rtype, src).await {
            // Command: Save the new records to all caches
            self.saveToAllCaches(name, rtype, records.clone(), ttl)
                .await;
            return records;
        }

        // Stage 6: NXDOMAIN handling
        self.handleMissingRecord(name, rtype, src).await;
        vec![]
    }

    /// Query functionality for memory cache.
    async fn queryMemoryCache(
        &self,
        name: &str,
        rtype: RecordType,
        src: SocketAddr,
    ) -> Option<Vec<Record>> {
        let cache = self.state.cache.read().await;
        if let Some(records) = cache.get(name, rtype) {
            self.state.cache_stats.recordHit();
            self.logResolution(src, name, rtype, records, "memory");
            return Some(records.clone());
        }
        self.state.cache_stats.recordMiss();
        None
    }

    /// Query functionality for persistent DB cache.
    async fn queryPersistentCache(&self, name: &str, rtype: RecordType) -> Option<Vec<Record>> {
        self.queryPersistentCacheRecursive(name, rtype, 0).await
    }

    #[async_recursion::async_recursion]
    async fn queryPersistentCacheRecursive(
        &self,
        name: &str,
        rtype: RecordType,
        depth: u8,
    ) -> Option<Vec<Record>> {
        if depth > 10 {
            tracing::warn!(name = %name, r#type = %rtype, depth = %depth, "CNAME recursion limit reached");
            return None;
        }

        let rows =
            match crate::db::records::getCache(&self.state.db, name, &rtype.to_string()).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(error = %e, name = %name, "Failed to query persistent cache");
                    return None;
                }
            };

        if !rows.is_empty() {
            // Check for negative cache placeholder
            if rows.len() == 1 && rows[0].value == "NX" {
                return self
                    .handleCachedNegativeResult(name, rtype, rows[0].expires_at)
                    .await;
            }

            let mut records = Vec::new();
            for row in &rows {
                if let Some(record) =
                    buildRecord(name, rtype, &row.value, row.ttl as u32, row.priority)
                {
                    records.push(record);
                }
            }

            if !records.is_empty() {
                return Some(records);
            }
        }

        // CNAME chasing in database
        if rtype != RecordType::CNAME {
            if let Ok(cname_rows) =
                crate::db::records::getCache(&self.state.db, name, "CNAME").await
            {
                if !cname_rows.is_empty() {
                    let target = cname_rows[0].value.trim_end_matches('.').to_string();
                    if let Some(mut target_recs) = self
                        .queryPersistentCacheRecursive(&target, rtype, depth + 1)
                        .await
                    {
                        if let Some(cname_rec) = buildRecord(
                            name,
                            RecordType::CNAME,
                            &cname_rows[0].value,
                            cname_rows[0].ttl as u32,
                            None,
                        ) {
                            target_recs.insert(0, cname_rec);
                        }
                        return Some(target_recs);
                    }
                }
            }
        }

        None
    }

    /// Query functionality for manual DB records.
    async fn queryDatabase(&self, name: &str, rtype: RecordType) -> Option<Vec<Record>> {
        let rows = crate::db::records::findByName(&self.state.db, name)
            .await
            .ok()?;
        let rtype_str = rtype.to_string().to_uppercase();

        let mut records = Vec::new();
        for row in rows.iter().filter(|r| r.record_type == rtype_str) {
            if let Some(record) = buildRecord(name, rtype, &row.value, row.ttl as u32, row.priority)
            {
                records.push(record);
            }
        }

        if records.is_empty() {
            None
        } else {
            Some(records)
        }
    }

    /// Query functionality for special records.
    async fn querySpecialRecords(
        &self,
        name: &str,
        rtype: RecordType,
        src: SocketAddr,
    ) -> Option<Vec<Record>> {
        if name != "mydns.local" || (rtype != RecordType::A && rtype != RecordType::AAAA) {
            return None;
        }

        let target_ip = self.getLocalInterfaceIpForClient(src.ip());
        if let Some(record) = buildRecord(name, rtype, &target_ip, 60, None) {
            let _ = self.state.log_tx.send(format!(
                "[SPECIAL] client={} query={} type={} value=[{}]",
                src, name, rtype, target_ip
            ));
            tracing::info!(client = %src, query = %name, r#type = %rtype, value = %target_ip, "Special record hit");
            return Some(vec![record]);
        }
        None
    }

    /// Query functionality for upstream servers.
    async fn queryUpstream(
        &self,
        name: &str,
        rtype: RecordType,
        src: SocketAddr,
    ) -> Option<(Vec<Record>, u32)> {
        let fqdn = format!("{}.", name);
        let parsed_name = match fqdn.parse::<Name>() {
            Ok(n) => n,
            Err(_) => return None,
        };

        let upstream = self.state.upstream.read().await;
        let result = upstream.resolve(&parsed_name, rtype).await;
        let addr = self.getUpstreamAddressString(&upstream).await;

        match result {
            Some((records, ttl)) => {
                let values = self.getRecordValuesString(&records);
                tracing::info!(client = %src, query = %name, r#type = %rtype, value = %values, upstream = %addr, "Upstream resolve hit");
                let _ = self.state.log_tx.send(format!(
                    "[UPSTREAM] client={} query={} type={} value=[{}] server={}",
                    src, name, rtype, values, addr
                ));
                Some((records, ttl))
            }
            None => None,
        }
    }

    // ── Commands ──────────────────────────────────────────────────────────────

    async fn handleMissingRecord(&self, name: &str, rtype: RecordType, src: SocketAddr) {
        tracing::info!(client = %src, query = %name, r#type = %rtype, "NXDOMAIN");
        let _ = self.state.log_tx.send(format!(
            "[NXDOMAIN] client={} query={} type={}",
            src, name, rtype
        ));
        self.saveNegativeCache(name, rtype, 60).await;
    }

    async fn saveToMemoryCache(
        &self,
        name: &str,
        rtype: RecordType,
        records: Vec<Record>,
        ttl: u32,
    ) {
        let mut cache = self.state.cache.write().await;
        cache.insert(name, rtype, records, Duration::from_secs(ttl as u64));
    }

    async fn saveToAllCaches(&self, name: &str, rtype: RecordType, records: Vec<Record>, ttl: u32) {
        self.saveToMemoryCache(name, rtype, records.clone(), ttl)
            .await;
        for r in &records {
            let owner = r.name().to_string().trim_end_matches('.').to_lowercase();
            if let Some(val) = r.data().map(|d| d.to_string()) {
                let prio = match r.data() {
                    Some(RData::MX(mx)) => Some(mx.preference() as i64),
                    _ => None,
                };
                let _ = crate::db::records::insertCache(
                    &self.state.db,
                    &owner,
                    &r.record_type().to_string(),
                    &val,
                    ttl,
                    prio,
                )
                .await;
            }
        }
    }

    async fn saveNegativeCache(&self, name: &str, rtype: RecordType, ttl: u32) {
        let mut cache = self.state.cache.write().await;
        cache.insert(name, rtype, vec![], Duration::from_secs(ttl as u64));
        let _ = crate::db::records::insertCache(
            &self.state.db,
            name,
            &rtype.to_string(),
            "NX",
            ttl,
            None,
        )
        .await;
    }

    async fn handleCachedNegativeResult(
        &self,
        name: &str,
        rtype: RecordType,
        expires_at: i64,
    ) -> Option<Vec<Record>> {
        let ttl = (expires_at - chrono::Utc::now().timestamp()).max(0) as u32;
        if ttl > 0 {
            self.saveToMemoryCache(name, rtype, vec![], ttl).await;
        }
        Some(vec![])
    }

    // ── Utilities ─────────────────────────────────────────────────────────────

    fn logResolution(
        &self,
        src: SocketAddr,
        name: &str,
        rtype: RecordType,
        records: &[Record],
        source: &str,
    ) {
        let values = self.getRecordValuesString(records);
        let _ = self.state.log_tx.send(format!(
            "[CACHE HIT] client={} query={} type={} value=[{}] ({})",
            src, name, rtype, values, source
        ));
        tracing::info!(client = %src, query = %name, r#type = %rtype, value = %values, "Cache hit ({})", source);
    }

    fn getRecordValuesString(&self, records: &[Record]) -> String {
        records
            .iter()
            .filter_map(|r| r.data().map(|d| d.to_string()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    async fn getUpstreamAddressString(
        &self,
        upstream: &crate::dns::upstream::UpstreamResolver,
    ) -> String {
        let cfg = self.state.config.read().await;
        match upstream.priority {
            crate::config::ResolverPriority::CloudflareFirst => cfg.cloudflare_dns.to_string(),
            crate::config::ResolverPriority::RouterFirst => upstream
                .router_addr
                .map(|a| a.to_string())
                .unwrap_or_else(|| cfg.cloudflare_dns.to_string()),
        }
    }

    fn getLocalInterfaceIpForClient(&self, client_ip: IpAddr) -> String {
        if client_ip.is_loopback() {
            return "127.0.0.1".to_string();
        }
        if isPrivateIp(client_ip) {
            return local_ip_address::local_ip()
                .map(|ip| ip.to_string())
                .unwrap_or_else(|_| "127.0.0.1".to_string());
        }
        local_ip_address::local_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|_| "127.0.0.1".to_string())
    }
}

#[allow(non_snake_case)]
pub fn buildRecord(
    name: &str,
    rtype: RecordType,
    value: &str,
    ttl: u32,
    priority: Option<i64>,
) -> Option<Record> {
    use hickory_proto::rr::rdata::{A, AAAA, CNAME, MX, PTR};

    let fqdn: Name = name.parse().ok()?;
    let rdata = match rtype {
        RecordType::A => RData::A(A(value.parse().ok()?)),
        RecordType::AAAA => RData::AAAA(AAAA(value.parse().ok()?)),
        RecordType::CNAME => RData::CNAME(CNAME(value.parse().ok()?)),
        RecordType::MX => RData::MX(MX::new(priority.unwrap_or(10) as u16, value.parse().ok()?)),
        RecordType::PTR => RData::PTR(PTR(value.parse().ok()?)),
        _ => return None,
    };

    let mut record = Record::new();
    record.set_name(fqdn);
    record.set_ttl(ttl);
    record.set_record_type(rtype);
    record.set_data(Some(rdata));
    Some(record)
}

#[allow(non_snake_case)]
fn isPrivateIp(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            let segs = v6.segments();
            (segs[0] & 0xfe00) == 0xfc00 || (segs[0] & 0xffc0) == 0xfe80
        }
    }
}
