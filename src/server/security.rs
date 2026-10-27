//! Reject mutations initiated from another web origin.
use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
pub(super) async fn same_origin(req: Request<axum::body::Body>, next: Next) -> Response {
    if req.method() != axum::http::Method::GET && req.method() != axum::http::Method::HEAD {
        let headers = req.headers();
        let bad_site =
            headers.get("sec-fetch-site").and_then(|s| s.to_str().ok()) == Some("cross-site");
        let bad_origin = headers
            .get("origin")
            .and_then(|s| s.to_str().ok())
            .is_some_and(|origin| {
                let host = headers
                    .get("host")
                    .and_then(|s| s.to_str().ok())
                    .unwrap_or("");
                origin != format!("http://{host}") && origin != format!("https://{host}")
            });
        if bad_site || bad_origin {
            return (
                StatusCode::FORBIDDEN,
                "別のサイトからの操作は許可されていません",
            )
                .into_response();
        }
    }
    next.run(req).await
}
