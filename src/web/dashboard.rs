use axum::{http::header, response::IntoResponse};

/// Serves the main dashboard HTML.
#[allow(non_snake_case)]
pub async fn serveDashboard() -> impl IntoResponse {
    let html = include_str!("../assets/dashboard.html");
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html)
}

/// Serves the dashboard CSS.
#[allow(non_snake_case)]
pub async fn serveStyles() -> impl IntoResponse {
    let css = include_str!("../assets/style.css");
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], css)
}

/// Serves the dashboard JavaScript.
#[allow(non_snake_case)]
pub async fn serveScripts() -> impl IntoResponse {
    let js = include_str!("../assets/app.js");
    (
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        js,
    )
}
