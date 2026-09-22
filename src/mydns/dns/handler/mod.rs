use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use hickory_proto::op::{Metadata, ResponseCode};
use hickory_proto::rr::{Record, RecordType};
use hickory_server::net::runtime::Time;
use hickory_server::server::{Request, RequestHandler, ResponseHandler, ResponseInfo};
use hickory_server::zone_handler::MessageResponseBuilder;

use crate::state::AppState;

mod blocklist;
mod cache;
mod local;
mod records;
pub mod upstream;

pub(crate) use records::{build_record, failed_response_info};

enum ResolutionResult {
    Positive(Vec<Record>, bool), // records, is_authoritative (DNS AA bit)
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
                let response = records::failed_response_info(request);
                return response;
            }
        };
        let query = request_info.query;
        let name_fqdn = query.name().to_string();
        let rtype = query.query_type();
        let name = name_fqdn.trim_end_matches('.').to_lowercase();

        let recursion_desired = request.metadata.recursion_desired;
        tracing::debug!(client = %src, query = %name_fqdn, rtype = %rtype, recursion_desired, "DNS query received");
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
                        records::failed_response_info(request)
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
                        records::failed_response_info(request)
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
                        records::failed_response_info(request)
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
                        records::failed_response_info(request)
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
        // ── 1. Local DNS zone ───────────────────────────────────────────────
        // If the query falls within a configured local zone, resolve it
        // entirely from local records. Upstream, cache, and blocklist are
        // all bypassed — local zones are private namespaces.
        let local_zone: Option<String> = {
            let trie = self.state.zone_trie.read().await;
            trie.find_zone(name).map(|s| s.to_string())
        };
        let is_local_zone = local_zone.is_some();

        if is_local_zone {
            // Local zone path: only consult the local record index.
            // Cache (memory and persistent) are skipped to prevent stale
            // upstream data from shadowing locally managed records.
            if let Some(result) = self
                .queryRecordIndex(name, rtype, src, local_zone.as_deref())
                .await
            {
                return result;
            }

            if let Some(records) = self.querySpecialRecords(name, rtype, src).await {
                return ResolutionResult::Positive(records, true);
            }

            // Name is inside a local zone but no record exists → local NXDOMAIN.
            tracing::debug!(client = %src, query = %name, r#type = %rtype, "Local zone record not found");
            let _ = self.state.log_tx.send(format!(
                "[LOCAL NXDOMAIN] client={} query={} type={}",
                src, name, rtype
            ));
            return ResolutionResult::NxDomain(true);
        }

        // ── 2. Blocklist ────────────────────────────────────────────────────
        // Check the blocklist before cache so that newly blocked domains
        // cannot be served from a stale cache entry.
        if let Some(result) = self.queryBlocklist(name, rtype, src).await {
            return result;
        }

        // ── 3. Memory cache ─────────────────────────────────────────────────
        if let Some(result) = self.queryMemoryCache(name, rtype, src).await {
            return result;
        }

        // ── 4. Local record index (non-local-zone dev records, etc.) ────────
        if let Some(result) = self
            .queryRecordIndex(name, rtype, src, local_zone.as_deref())
            .await
        {
            return result;
        }

        // ── 5. Persistent cache ─────────────────────────────────────────────
        if let Some(result) = self.queryPersistentCache(name, rtype).await {
            return result;
        }

        // ── 6. Special synthetic records ────────────────────────────────────
        if let Some(records) = self.querySpecialRecords(name, rtype, src).await {
            return ResolutionResult::Positive(records, true);
        }

        // ── 7. Upstream ─────────────────────────────────────────────────────
        if !recursion_desired {
            tracing::debug!(client = %src, query = %name, r#type = %rtype, "Recursion not desired and record not in local DB or cache");
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
                tracing::debug!(client = %src, query = %name, r#type = %rtype, "NODATA");
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

    /// Checks whether `name` is on the blocklist.
    ///
    /// Returns `Some(NxDomain(false))` when blocked (never upstream, never
    /// cached). The blocklist is authoritative over the cache so that
    /// toggling a domain on/off takes effect immediately without a cache flush.
    #[tracing::instrument(
        name = "query_blocklist",
        level = tracing::Level::DEBUG,
        fields(name = %name, rtype = ?rtype),
        skip(self)
    )]
