use axum::{http::header, response::IntoResponse};

/// Serves the embedded single-page dashboard HTML.
///
/// The file is compiled into the binary via `include_str!` so no static file
/// server or separate deployment step is required.
#[allow(non_snake_case)]
pub async fn serveDashboard() -> impl IntoResponse {
    let html = include_str!("../assets/dashboard.html");
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
}
