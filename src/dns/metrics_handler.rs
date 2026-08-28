use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use hickory_proto::op::ResponseCode;
use hickory_server::net::runtime::Time;
use hickory_server::server::{Request, RequestHandler, ResponseHandler, ResponseInfo};

use crate::dns::handler::DnsHandler;
use crate::state::AppState;

/// RequestHandler decorator that records resolver-wide request latency and outcomes
/// without putting telemetry logic into the DNS resolution path itself.
pub struct MetricsHandler {
    inner: DnsHandler,
    state: Arc<AppState>,
}

impl MetricsHandler {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            inner: DnsHandler::new(Arc::clone(&state)),
            state,
        }
    }
}

#[async_trait]
impl RequestHandler for MetricsHandler {
    async fn handle_request<R: ResponseHandler, T: Time>(
        &self,
        request: &Request,
        response_handle: R,
    ) -> ResponseInfo {
        let started = Instant::now();

        if let Ok(info) = request.request_info() {
            self.state
                .metrics
                .record_query(&info.query.query_type().to_string());
        }

        let response = self.inner.handle_request(request, response_handle).await;
        let response_ms = started.elapsed().as_secs_f64() * 1000.0;
        let outcome = match response.response_code() {
            ResponseCode::NoError => "NOERROR",
            ResponseCode::NXDomain => "NXDOMAIN",
            ResponseCode::ServFail => "SERVFAIL",
            ResponseCode::Refused => "REFUSED",
            _ => "OTHER",
        };
        self.state.metrics.record_outcome(outcome);
        self.state.metrics.record_latency(response_ms, None);
        response
    }
}
