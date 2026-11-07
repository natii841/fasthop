use crate::state::AppState;
use axum::{
    extract::Path,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use include_dir::{include_dir, Dir};
use mime_guess::from_path;
use std::sync::Arc;

static DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../dashboard/dist");

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(index))
        .route("/{*path}", get(asset))
}

async fn index() -> Response {
    file_response("index.html")
}

async fn asset(Path(path): Path<String>) -> Response {
    file_response(&path)
}

fn file_response(path: &str) -> Response {
    let Some(file) = DIST.get_file(path).or_else(|| DIST.get_file("index.html")) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let content_type = from_path(path).first_or_octet_stream();
    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(content_type.as_ref())
                .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
        )
        .body(axum::body::Body::from(file.contents().to_vec()))
        .unwrap()
}
