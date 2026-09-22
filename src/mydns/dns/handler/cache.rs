use std::net::SocketAddr;
use std::time::Duration;
use hickory_proto::rr::{RData, Record, RecordType};
use crate::cache::CacheResult;
use super::{DnsHandler, ResolutionResult, build_record};

impl DnsHandler {
        pub(crate) async fn query_memory_cache(
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
                    self.log_resolution(src, name, rtype, records, "memory");
                    ResolutionResult::Positive(records.clone(), is_authoritative)
                }
                CacheResult::Negative => {
                    self.log_negative_cache_hit(src, name, rtype, "memory");
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
    pub(crate) async fn query_persistent_cache(
        &self,
        name: &str,
        rtype: RecordType,
    ) -> Option<ResolutionResult> {
        self.query_persistent_cache_recursive(name, rtype, 0).await
    }

    #[async_recursion::async_recursion]
    async fn query_persistent_cacheRecursive(
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
                self.handle_cached_negative_result(name, rtype, rows[0].expires_at)
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
                        .query_persistent_cacheRecursive(&target, rtype, depth + 1)
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

    pub(crate) async fn handle_missing_record(&self, name: &str, rtype: RecordType, src: SocketAddr) {
        tracing::debug!(client = %src, query = %name, r#type = %rtype, "NXDOMAIN");
        let _ = self.state.log_tx.send(format!(
            "[NXDOMAIN] client={} query={} type={}",
            src, name, rtype
        ));
        self.save_negative_cache(name, rtype, 60).await;
    }

    pub(crate) async fn save_to_memory_cache(
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

    pub(crate) async fn save_to_all_caches(
        &self,
        name: &str,
        rtype: RecordType,
        records: Vec<Record>,
        ttl: u32,
    ) {
        self.save_to_memory_cache(name, rtype, records.clone(), ttl, false)
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

    fn log_resolution(
        &self,
        src: SocketAddr,
        name: &str,
        rtype: RecordType,
        records: &[Record],
        source: &str,
    ) {
        let values = records.iter().map(|r| r.data.to_string()).collect::<Vec<_>>().join(", ");
        let _ = self.state.log_tx.send(format!(
            "[CACHE] client={} query={} type={} value=[{}] source={}",
            src, name, rtype, values, source
        ));
    }

    fn log_negative_cache_hit(&self, src: SocketAddr, name: &str, rtype: RecordType, source: &str) {
        let _ = self.state.log_tx.send(format!(
            "[NEGATIVE CACHE] client={} query={} type={} source={}",
            src, name, rtype, source
        ));
    }

    async fn handle_cached_negative_result(&self, name: &str, rtype: RecordType, expires_at: i64) {
        let _ = self.state.log_tx.send(format!(
            "[NEGATIVE CACHE] query={} type={} expires_at={}",
            name, rtype, expires_at
        ));
    }

    async fn save_negative_cache(&self, name: &str, rtype: RecordType, ttl: u32) {
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
