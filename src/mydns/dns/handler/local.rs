use std::net::{IpAddr, SocketAddr};
use hickory_proto::rr::{Record, RecordType};
use crate::dns::record_index::IndexResolution;
use super::{DnsHandler, ResolutionResult, build_record};

#[allow(non_snake_case)]
impl DnsHandler {
        pub(crate) async fn queryRecordIndex(
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

    pub(crate) async fn querySpecialRecords(
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
            tracing::debug!(client = %src, query = %name, r#type = %rtype, value = %target_ip, "Special record hit");
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

}

fn isPrivateIp(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            let segs = v6.segments();
            (segs[0] & 0xfe00) == 0xfc00 || (segs[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Returns true for PTR query names that correspond to loopback addresses.
fn isLoopbackPtrName(name: &str) -> bool {
    if let Some(rest) = name.strip_suffix(".in-addr.arpa") {
        if rest.split('.').next_back() == Some("127") {
            return true;
        }
    }
    name == "1.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.ip6.arpa"
}
