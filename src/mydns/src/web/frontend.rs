use axum::http::header as http_header;
use axum::http::StatusCode;
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "out/web/"]
#[allow_missing = true]
pub(crate) struct FrontendAssets;

pub(crate) async fn serve_frontend(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> axum::response::Response {
    serve_asset(&path)
}

pub(crate) async fn serve_frontend_root() -> axum::response::Response {
    serve_asset("")
}

pub(crate) fn serve_asset(path: &str) -> axum::response::Response {
    use axum::body::Body;
    use axum::http::Response;
    use axum::response::IntoResponse;

    let normalized = path.trim_start_matches('/');
    let asset = FrontendAssets::get(normalized).or_else(|| FrontendAssets::get("index.html"));
    let Some(asset) = asset else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mime = if normalized.is_empty() {
        "text/html; charset=utf-8".to_string()
    } else {
        mime_guess::from_path(normalized)
            .first_or_octet_stream()
            .essence_str()
            .to_string()
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(http_header::CONTENT_TYPE, mime)
        .body(Body::from(asset.data.into_owned()))
        .expect("valid frontend response")
}
