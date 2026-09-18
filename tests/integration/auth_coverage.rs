//! Auth coverage tests.
//!
//! Verifies that every authenticated API route returns HTTP 401 when called
//! without a JWT bearer token, and that the intentionally unauthenticated
//! `/stats` endpoint returns 200 without one.

mod common;

use reqwest::Client;

type RequestBuilder = fn(&Client, String) -> reqwest::RequestBuilder;

/// Asserts that every authenticated endpoint returns 401 without a token,
/// and the unauthenticated /stats endpoint returns 200.
#[tokio::test]
async fn test_all_protected_routes_require_auth() {
    let server = common::TestServer::start().await;
    let c = Client::new();
    let api = format!("{}/api/v1", server.base_url);

    // -- Unauthenticated routes (must pass) --
    assert_eq!(
        c.get(format!("{api}/stats")).send().await.unwrap().status(),
        200,
        "/stats should not require auth"
    );

    // -- Authenticated routes (must return 401 without token) --
    let protected: &[(&str, RequestBuilder)] = &[
        ("GET /records", |c, u| c.get(u)),
        ("POST /records", |c, u| c.post(u)),
        ("PUT /records/1", |c, u| c.put(u)),
        ("DELETE /records/1", |c, u| c.delete(u)),
        ("GET /settings", |c, u| c.get(u)),
        ("PUT /settings", |c, u| c.put(u)),
        ("GET /cache", |c, u| c.get(u)),
        ("DELETE /cache", |c, u| c.delete(u)),
    ];

    for (label, build) in protected {
        let url = if label.contains("/records") {
            if label.contains("/records/1") {
                format!("{api}/records/1")
            } else {
                format!("{api}/records")
            }
        } else if label.contains("/settings") {
            format!("{api}/settings")
        } else {
            format!("{api}/cache")
        };

        let status = build(&c, url).send().await.unwrap().status();
        assert_eq!(status, 401, "{label} should return 401 without auth token");
    }
}
