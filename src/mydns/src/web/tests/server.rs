use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::mydns::web::server::run;
use crate::mydns::web::tests::fixtures::TestContext;

#[tokio::test]
async fn run_binds_and_shuts_down_gracefully() {
    let ctx = TestContext::new().await;

    {
        let mut config = ctx.state.config.write().await;
        config.http_host = "127.0.0.1".parse().unwrap();
        config.http_port = 0;
    }

    let cancel = CancellationToken::new();
    let task = tokio::spawn(run(Arc::clone(&ctx.state), cancel.clone()));

    tokio::time::sleep(Duration::from_millis(50)).await;
    cancel.cancel();

    let result = tokio::time::timeout(Duration::from_secs(1), task)
        .await
        .expect("HTTP server should shut down after cancellation")
        .expect("HTTP server task should not panic");

    assert!(result.is_ok(), "server should exit cleanly: {result:?}");
}

#[tokio::test]
async fn run_reports_listener_bind_failure() {
    let ctx = TestContext::new().await;

    let blocker = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("failed to reserve test port");
    let port = blocker.local_addr().unwrap().port();

    {
        let mut config = ctx.state.config.write().await;
        config.http_host = "127.0.0.1".parse().unwrap();
        config.http_port = port;
    }

    let cancel = CancellationToken::new();
    let result = run(Arc::clone(&ctx.state), cancel).await;

    let error = result.expect_err("server should fail when the port is already bound");
    assert!(
        error.to_string().contains("Failed to bind HTTP server"),
        "unexpected bind error: {error}"
    );
}
