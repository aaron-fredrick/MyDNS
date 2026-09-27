use axum::http::StatusCode;

use crate::mydns::web::frontend::{serve_asset, serve_frontend, serve_frontend_root};

#[tokio::test]
async fn serve_asset_empty_path_serves_root_html() {
    let response = serve_asset("");
    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get("content-type")
        .expect("content-type header")
        .to_str()
        .unwrap();
    assert_eq!(content_type, "text/html; charset=utf-8");
}

#[tokio::test]
async fn serve_asset_index_html_serves_html() {
    let response = serve_asset("index.html");
    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get("content-type")
        .expect("content-type header")
        .to_str()
        .unwrap();
    assert_eq!(content_type, "text/html");

    // With leading slash
    let response_slash = serve_asset("/index.html");
    assert_eq!(response_slash.status(), StatusCode::OK);
}

#[tokio::test]
async fn serve_asset_serves_css_and_js() {
    let css = serve_asset("assets/index-BphobTOE.css");
    assert_eq!(css.status(), StatusCode::OK);
    let css_ct = css.headers().get("content-type").unwrap().to_str().unwrap();
    assert_eq!(css_ct, "text/css");

    let js = serve_asset("assets/index-h-EE90li.js");
    assert_eq!(js.status(), StatusCode::OK);
    let js_ct = js.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(js_ct.contains("javascript"));
}

#[tokio::test]
async fn serve_asset_falls_back_to_index_for_spa_routes() {
    let response = serve_asset("dashboard");
    assert_eq!(response.status(), StatusCode::OK);

    let response_nested = serve_asset("records/123");
    assert_eq!(response_nested.status(), StatusCode::OK);
}

#[tokio::test]
async fn serve_frontend_root_handler_returns_ok() {
    let response = serve_frontend_root().await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn serve_frontend_path_handler_returns_ok() {
    let response = serve_frontend(axum::extract::Path("index.html".to_string())).await;
    assert_eq!(response.status(), StatusCode::OK);
}
