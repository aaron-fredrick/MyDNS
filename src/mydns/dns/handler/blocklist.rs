use std::net::SocketAddr;
use hickory_proto::rr::RecordType;
use super::{DnsHandler, ResolutionResult};

impl DnsHandler {
        pub(crate) async fn query_blocklist(
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

}
