use std::net::SocketAddr;
use hickory_proto::rr::RecordType;
use super::{DnsHandler, ResolutionResult};

#[allow(non_snake_case)]
impl DnsHandler {
        pub(crate) async fn queryBlocklist(
        &self,
        name: &str,
        rtype: RecordType,
        src: SocketAddr,
    ) -> Option<ResolutionResult> {
        let blocked = {
            let idx = self.state.blocklist_index.read().await;
            idx.is_blocked(name)
        };
        if !blocked {
            return None;
        }
        self.state.metrics.record_blocked();
        tracing::info!(
            client = %src,
            domain = %name,
            r#type = %rtype,
            reason = "blocklist",
            "DNS query blocked"
        );
        let _ = self.state.log_tx.send(format!(
            "[BLOCKED] client={} query={} type={} reason=blocklist",
            src, name, rtype
        ));
        Some(ResolutionResult::NxDomain(false))
    }

    #[tracing::instrument(
        name = "query_memory_cache",
        level = tracing::Level::DEBUG,
        fields(name = %name, rtype = ?rtype),
        skip(self)
    )]
}
