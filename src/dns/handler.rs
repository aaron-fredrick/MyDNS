use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use hickory_proto::op::{Header, HeaderCounts, Metadata, ResponseCode};
use hickory_proto::rr::{Name, RData, Record, RecordType};
use hickory_server::net::runtime::Time;
use hickory_server::server::{Request, RequestHandler, ResponseHandler, ResponseInfo};
use hickory_server::zone_handler::MessageResponseBuilder;

use crate::cache::CacheResult;
use crate::dns::record_index::IndexResolution;
use crate::dns::upstream::UpstreamResolution;
use crate::state::AppState;

#[derive(Debug)]
enum ResolutionResult {
    Positive(Vec<Record>, bool), // records, is_authoritative
    Nodata(bool),                // is_authoritative
    NxDomain(bool),              // is_authoritative
    ServFail,
}

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
    async fn handle_request<R: ResponseHandler, T: Time>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> ResponseInfo {
        let src = request.src();
        let request_info = match request.request_info() {
            Ok(info) => info,
            Err(e) => {
                tracing::warn!(client = %src, error = %e, "Invalid DNS request");
                let response = failed_response_info(request);
                return response;
            }
        };
        let query = request_info.query;
        let name_fqdn = query.name().to_string();
        let rtype = query.query_type();
        let name = name_fqdn.trim_end_matches('.').to_lowercase();

        let recursion_desired = request.metadata.recursion_desired;
        tracing::info!(client = %src, query = %name_fqdn, rtype = %rtype, recursion_desired, "DNS query received");
        let result = self
            .processResolution(&name, rtype, src, recursion_desired)
            .await;
        let builder = MessageResponseBuilder::from_message_request(request);
        let mut metadata = Metadata::response_from_request(&request.metadata);
        metadata.recursion_available = true;

        match result {
            ResolutionResult::Positive(records, is_authoritative) => {
                metadata.response_code = ResponseCode::NoError;
                metadata.authoritative = is_authoritative;
                let response = builder.build(metadata, records.iter(), &[], &[], &[]);
                response_handle
                    .send_response(response)
                    .await
                    .unwrap_or_else(|e| {
                        tracing::error!(error = %e, "Failed to send DNS response");
                        failed_response_info(request)
                    })
            }
            ResolutionResult::Nodata(is_authoritative) => {
                metadata.response_code = ResponseCode::NoError;
                metadata.authoritative = is_authoritative;
                let response = builder.build_no_records(metadata);
                response_handle
                    .send_response(response)
                    .await
                    .unwrap_or_else(|e| {
                        tracing::error!(error = %e, "Failed to send NODATA response");
                        failed_response_info(request)
                    })
            }
            ResolutionResult::NxDomain(is_authoritative) => {
                metadata.response_code = ResponseCode::NXDomain;
                metadata.authoritative = is_authoritative;
                let response = builder.build_no_records(metadata);
                response_handle
                    .send_response(response)
                    .await
                    .unwrap_or_else(|e| {
                        tracing::error!(error = %e, "Failed to send NXDOMAIN response");
                        failed_response_info(request)
                    })
            }
            ResolutionResult::ServFail => {
                metadata.response_code = ResponseCode::ServFail;
                let response = builder.build_no_records(metadata);
                response_handle
                    .send_response(response)
                    .await
                    .unwrap_or_else(|e| {
                        tracing::error!(error = %e, "Failed to send SERVFAIL response");
                        failed_response_info(request)
                    })
            }
        }
    }
}

#[allow(non_snake_case)]
impl DnsHandler {
    #[tracing::instrument(
        name = "process_resolution",
        level = tracing::Level::DEBUG,
        fields(name = %name, rtype = ?rtype, client = %src),
        skip(self)
    )]
    async fn processResolution(
        &self,
        name: &str,
        rtype: RecordType,
        src: SocketAddr,
        recursion_desired: bool,
    ) -> ResolutionResult {
        let authoritative_zone: Option<String> = {
            let trie = self.state.zone_trie.read().await;
            trie.find_zone(name).map(|s| s.to_string())
        };
        let is_authoritative_zone = authoritative_zone.is_some();

        //println!(">>>>>> Processing resolution <<<<<< src: {}, query: {}, rtype: {:?}, is_authoritative_zone: {}", src, name, rtype, is_authoritative_zone);
        //tracing::info!(">>>>>> Processing resolution <<<<<<", client = %src, query = %name, r#type = %rtype, is_authoritative_zone = is_authoritative_zone);

        // For authoritative zones: consult authoritative sources only.
        // Upstream caches (memory and persistent) are skipped entirely to
        // prevent stale upstream data from shadowing authoritative records.
        if !is_authoritative_zone {
            if let Some(result) = self.queryMemoryCache(name, rtype, src).await {
                return result;
            }
        }

        if let Some(result) = self
            .queryRecordIndex(name, rtype, src, authoritative_zone.as_deref())
            .await
        {
            return result;
        }

        if !is_authoritative_zone {
            if let Some(result) = self.queryPersistentCache(name, rtype).await {
                return result;
            }
        }

        if let Some(records) = self.querySpecialRecords(name, rtype, src).await {
            return ResolutionResult::Positive(records, true);
        }

        // If the query falls within an authoritative zone and was not found locally:
        // Return authoritative NXDOMAIN immediately and NEVER forward to upstream.
        if is_authoritative_zone {
            tracing::info!(client = %src, query = %name, r#type = %rtype, "Authoritative zone record not found");
            let _ = self.state.log_tx.send(format!(
                "[AUTHORITATIVE NXDOMAIN] client={} query={} type={}",
                src, name, rtype
            ));
            return ResolutionResult::NxDomain(true);
        }

        if !recursion_desired {
            tracing::info!(client = %src, query = %name, r#type = %rtype, "Recursion not desired and record not in local DB or cache");
            return ResolutionResult::NxDomain(false);
        }

        match self.queryUpstream(name, rtype, src).await {
            ResolutionResult::Positive(records, _) => {
                let ttl = records.iter().map(|r| r.ttl).min().unwrap_or(300);
                self.saveToAllCaches(name, rtype, records.clone(), ttl)
                    .await;
                ResolutionResult::Positive(records, false)
            }
            ResolutionResult::Nodata(_) => {
                tracing::info!(client = %src, query = %name, r#type = %rtype, "NODATA");
                let _ = self.state.log_tx.send(format!(
                    "[NODATA] client={} query={} type={}",
                    src, name, rtype
                ));
                ResolutionResult::Nodata(false)
            }
            ResolutionResult::NxDomain(_) => {
                self.handleMissingRecord(name, rtype, src).await;
                ResolutionResult::NxDomain(false)
            }
            ResolutionResult::ServFail => {
                tracing::warn!(client = %src, query = %name, r#type = %rtype, "SERVFAIL");
                let _ = self.state.log_tx.send(format!(
                    "[SERVFAIL] client={} query={} type={}",
                    src, name, rtype
                ));
                ResolutionResult::ServFail
            }
        }
    }

    #[tracing::instrument(
        name = "query_memory_cache",
        level = tracing::Level::DEBUG,
        fields(name = %name, rtype = ?rtype),
        skip(self)
    )]
    async fn queryMemoryCache(
        &self,
        name: &str,
        rtype: RecordType,
        src: SocketAddr,
    ) -> Option<ResolutionResult> {
        let cache = self.state.cache.read().await;
        if let Some((result, is_authoritative, records)) = cache.get(name, rtype) {
            self.state.cache_stats.record_hit();
            tracing::debug!(cache_type = "memory", result = ?result, "Cache hit");
            return Some(match result {
                CacheResult::Positive => {
                    self.logResolution(src, name, rtype, records, "memory");
                    ResolutionResult::Positive(records.clone(), is_authoritative)
                }
                CacheResult::Negative => {
                    self.logNegativeCacheHit(src, name, rtype, "memory");
                    ResolutionResult::NxDomain(false)
                }
            });
        }
        self.state.cache_stats.record_miss();
        None
    }

    #[tracing::instrument(
        name = "query_persistent_cache",
        level = tracing::Level::DEBUG,
        fields(name = %name, rtype = ?rtype),
        skip(self)
    )]
    async fn queryPersistentCache(
        &self,
        name: &str,
        rtype: RecordType,
    ) -> Option<ResolutionResult> {
        self.queryPersistentCacheRecursive(name, rtype, 0).await
    }

    #[async_recursion::async_recursion]
    async fn queryPersistentCacheRecursive(
        &self,
        name: &str,
        rtype: RecordType,
        depth: u8,
    ) -> Option<ResolutionResult> {
        if depth > 10 {
            tracing::warn!(name = %name, r#type = %rtype, depth = %depth, "CNAME recursion limit reached");
            return Some(ResolutionResult::ServFail);
        }

        let rows =
            match crate::db::records::get_cache(&self.state.db, name, &rtype.to_string()).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(error = %e, name = %name, "Failed to query persistent cache");
                    return Some(ResolutionResult::ServFail);
                }
            };

        if !rows.is_empty() {
            if rows.len() == 1 && rows[0].value == "NX" {
                self.handleCachedNegativeResult(name, rtype, rows[0].expires_at)
                    .await;
                return Some(ResolutionResult::NxDomain(false));
            }
            let mut records = Vec::new();
            for row in &rows {
                if let Some(record) =
                    build_record(name, rtype, &row.value, row.ttl as u32, row.priority)
                {
                    records.push(record);
                }
            }
            if !records.is_empty() {
                return Some(ResolutionResult::Positive(records, false));
            }
        }

        if rtype != RecordType::CNAME {
            if let Ok(cname_rows) =
                crate::db::records::get_cache(&self.state.db, name, "CNAME").await
            {
                if !cname_rows.is_empty() {
                    let target = cname_rows[0].value.trim_end_matches('.').to_string();
                    match self
                        .queryPersistentCacheRecursive(&target, rtype, depth + 1)
                        .await
                    {
                        Some(ResolutionResult::Positive(mut target_recs, _)) => {
                            if let Some(cname_rec) = build_record(
                                name,
                                RecordType::CNAME,
                                &cname_rows[0].value,
                                cname_rows[0].ttl as u32,
                                None,
                            ) {
                                target_recs.insert(0, cname_rec);
                            }
                            return Some(ResolutionResult::Positive(target_recs, false));
                        }
                        Some(other) => return Some(other),
                        None => {}
                    }
                }
            }
        }
        None
    }

    #[tracing::instrument(
        name = "query_record_index",
        level = tracing::Level::DEBUG,
        fields(name = %name, rtype = ?rtype),
        skip(self)
    )]
    async fn queryRecordIndex(
        &self,
        name: &str,
        rtype: RecordType,
        src: SocketAddr,
        zone_apex: Option<&str>,
    ) -> Option<ResolutionResult> {
        let rtype_str = rtype.to_string().to_uppercase();
        let resolution = {
            let index = self.state.record_index.read().await;
            index.resolve_authoritative(name, &rtype_str, zone_apex)
        };

        match resolution {
            IndexResolution::Found(db_records) => {
                let records: Vec<Record> = db_records
                    .iter()
                    .filter_map(|r| {
                        let parsed_type = r.record_type.parse::<RecordType>().ok()?;
                        build_record(&r.name, parsed_type, &r.value, r.ttl as u32, r.priority)
                    })
                    .collect();
                if records.is_empty() {
                    return None;
                }
                let ttl = records.iter().map(|r| r.ttl).min().unwrap_or(300);
                self.logResolution(src, name, rtype, &records, "INDEX");
                self.saveToMemoryCache(name, rtype, records.clone(), ttl, true)
                    .await;
                Some(ResolutionResult::Positive(records, true))
            }
            IndexResolution::Nodata => Some(ResolutionResult::Nodata(true)),
            IndexResolution::Miss => None,
            IndexResolution::ServFail => Some(ResolutionResult::ServFail),
        }
    }

    async fn querySpecialRecords(
        &self,
        name: &str,
        rtype: RecordType,
        src: SocketAddr,
    ) -> Option<Vec<Record>> {
        // Synthetic PTR responses for loopback addresses.
        // nslookup and similar tools reverse-resolve the server IP before sending queries.
        // Without a PTR answer the tool marks the server as unresponsive and drops all
        // subsequent queries. We synthesise localhost. here so no manual DB record is needed.
        if rtype == RecordType::PTR && isLoopbackPtrName(name) {
            if let Some(record) = build_record(name, RecordType::PTR, "localhost.", 3600, None) {
                tracing::debug!(query = %name, "Synthetic loopback PTR response");
                return Some(vec![record]);
            }
        }

        let dashboard_domain = {
            let cfg = self.state.config.read().await;
            cfg.dashboard_domain.clone()
        };

        if name != dashboard_domain || (rtype != RecordType::A && rtype != RecordType::AAAA) {
            return None;
        }
        let target_ip = self.getLocalInterfaceIpForClient(src.ip());
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

    #[tracing::instrument(
        name = "query_upstream",
        level = tracing::Level::DEBUG,
        fields(name = %name, rtype = ?rtype, client = %src),
        skip(self)
    )]
    async fn queryUpstream(
        &self,
        name: &str,
        rtype: RecordType,
        src: SocketAddr,
    ) -> ResolutionResult {
        let fqdn = format!("{}.", name);
        let parsed_name = match fqdn.parse::<Name>() {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!(query = %name, error = %e, "Invalid DNS name; returning SERVFAIL");
                return ResolutionResult::ServFail;
            }
        };
        let upstream = self.state.upstream.read().await;
        let addr = self.getUpstreamAddressString(&upstream).await;
        tracing::debug!(upstream_addr = %addr, "Querying upstream");

        let result = tokio::time::timeout(
            Duration::from_secs(5),
            upstream.resolve(&parsed_name, rtype),
        )
        .await;

        match result {
            Ok(UpstreamResolution::Positive(records, _ttl)) => {
                let values = self.getRecordValuesString(&records);
                tracing::debug!(client = %src, query = %name, r#type = %rtype, value = %values, upstream = %addr, "Upstream resolve hit");
                let _ = self.state.log_tx.send(format!(
                    "[UPSTREAM] client={} query={} type={} value=[{}] server={}",
                    src, name, rtype, values, addr
                ));
                ResolutionResult::Positive(records, false)
            }
            Ok(UpstreamResolution::Nodata) => ResolutionResult::Nodata(false),
            Ok(UpstreamResolution::NxDomain) => ResolutionResult::NxDomain(false),
            Ok(UpstreamResolution::ServFail) => ResolutionResult::ServFail,
            Err(_) => {
                tracing::warn!(client = %src, query = %name, r#type = %rtype, "Upstream resolve timed out; returning SERVFAIL");
                let _ = self.state.log_tx.send(format!(
                    "[UPSTREAM TIMEOUT] client={} query={} type={} server={}",
                    src, name, rtype, addr
                ));
                ResolutionResult::ServFail
            }
        }
    }

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
        is_authoritative: bool,
    ) {
        let mut cache = self.state.cache.write().await;
        cache.insert(
            name,
            rtype,
            records,
            Duration::from_secs(ttl as u64),
            is_authoritative,
        );
    }

    async fn saveToAllCaches(&self, name: &str, rtype: RecordType, records: Vec<Record>, ttl: u32) {
        self.saveToMemoryCache(name, rtype, records.clone(), ttl, false)
            .await;
        for r in &records {
            let owner = r.name.to_string().trim_end_matches('.').to_lowercase();
            let val = r.data.to_string();
            let prio = match &r.data {
                RData::MX(mx) => Some(mx.preference as i64),
                _ => None,
            };
            let _ = crate::db::records::insert_cache(
                &self.state.db,
                &owner,
                &r.record_type().to_string(),
                &val,
                r.ttl,
                prio,
            )
            .await;
        }
    }

    fn getLocalInterfaceIpForClient(&self, client_ip: IpAddr) -> String {
        if isPrivateIp(client_ip) {
            self.getLocalInterfaceIp().to_string()
        } else {
            "127.0.0.1".to_string()
        }
    }

    fn getLocalInterfaceIp(&self) -> IpAddr {
        self.state
            .config
            .try_read()
            .ok()
            .map(|cfg| cfg.bind_host)
            .unwrap_or(IpAddr::from([127, 0, 0, 1]))
    }

    async fn getUpstreamAddressString(
        &self,
        upstream: &crate::dns::upstream::UpstreamResolver,
    ) -> String {
        if upstream.mode == crate::config::ResolverMode::Recursive {
            "recursive (root hints)".to_string()
        } else if let Some(addr) = upstream.router_addr {
            format!("{} (router)", addr)
        } else {
            format!("{} (cloudflare)", upstream.cloudflare_addr)
        }
    }

    fn getRecordValuesString(&self, records: &[Record]) -> String {
        records
            .iter()
            .map(|r| r.data.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

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
            "[CACHE] client={} query={} type={} value=[{}] source={}",
            src, name, rtype, values, source
        ));
    }

    fn logNegativeCacheHit(&self, src: SocketAddr, name: &str, rtype: RecordType, source: &str) {
        let _ = self.state.log_tx.send(format!(
            "[NEGATIVE CACHE] client={} query={} type={} source={}",
            src, name, rtype, source
        ));
    }

    async fn handleCachedNegativeResult(&self, name: &str, rtype: RecordType, expires_at: i64) {
        let _ = self.state.log_tx.send(format!(
            "[NEGATIVE CACHE] query={} type={} expires_at={}",
            name, rtype, expires_at
        ));
    }

    async fn saveNegativeCache(&self, name: &str, rtype: RecordType, ttl: u32) {
        let _ = crate::db::records::insert_cache(
            &self.state.db,
            name,
            &rtype.to_string(),
            "NX",
            ttl,
            None,
        )
        .await;
    }
}

pub fn build_record(
    name: &str,
    rtype: RecordType,
    value: &str,
    ttl: u32,
    priority: Option<i64>,
) -> Option<Record> {
    use hickory_proto::rr::rdata::{A, AAAA, CNAME, MX, NS, PTR, TXT};
    let fqdn_str = if name.ends_with('.') {
        name.to_string()
    } else {
        format!("{}.", name)
    };
    let fqdn: Name = fqdn_str.parse().ok()?;
    let rdata = match rtype {
        RecordType::A => RData::A(A(value.parse().ok()?)),
        RecordType::AAAA => RData::AAAA(AAAA(value.parse().ok()?)),
        RecordType::CNAME => {
            let target_str = if value.ends_with('.') {
                value.to_string()
            } else {
                format!("{}.", value)
            };
            RData::CNAME(CNAME(target_str.parse().ok()?))
        }
        RecordType::MX => {
            let target_str = if value.ends_with('.') {
                value.to_string()
            } else {
                format!("{}.", value)
            };
            RData::MX(MX::new(
                priority.unwrap_or(10) as u16,
                target_str.parse().ok()?,
            ))
        }
        RecordType::NS => {
            let target_str = if value.ends_with('.') {
                value.to_string()
            } else {
                format!("{}.", value)
            };
            RData::NS(NS(target_str.parse().ok()?))
        }
        RecordType::PTR => {
            let target_str = if value.ends_with('.') {
                value.to_string()
            } else {
                format!("{}.", value)
            };
            RData::PTR(PTR(target_str.parse().ok()?))
        }
        RecordType::TXT => RData::TXT(TXT::new(vec![value.to_string()])),
        _ => return None,
    };
    Some(Record::from_rdata(fqdn, ttl, rdata))
}

fn failed_response_info(request: &Request) -> ResponseInfo {
    let mut metadata = Metadata::response_from_request(&request.metadata);
    metadata.response_code = ResponseCode::ServFail;
    ResponseInfo::from(Header {
        metadata,
        counts: HeaderCounts::default(),
    })
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

/// Returns true for PTR query names that correspond to loopback addresses:
/// - `1.0.0.127.in-addr.arpa` and any other `127.x.x.x.in-addr.arpa` range
/// - `1.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.ip6.arpa` (::1)
#[allow(non_snake_case)]
fn isLoopbackPtrName(name: &str) -> bool {
    // IPv4 loopback: 127.0.0.0/8 → ends with .127.in-addr.arpa
    if let Some(rest) = name.strip_suffix(".in-addr.arpa") {
        // The PTR name is the reversed octets, so 127.x.x.x becomes x.x.x.127
        if rest.split('.').next_back() == Some("127") {
            return true;
        }
    }
    // IPv6 loopback ::1 → 1.0.0...0.ip6.arpa (32 nibbles)
    if name == "1.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.ip6.arpa" {
        return true;
    }
    false
}
