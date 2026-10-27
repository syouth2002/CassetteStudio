//! Embedded frontend resources; no source tree is needed at runtime.
use super::state::App;
use axum::{Router, response::Html, routing::get};
pub(super) fn routes() -> Router<App> {
    Router::new()
        .route(
            "/js/core.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/core.js"),
                )
            }),
        )
        .route(
            "/js/state.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/state.js"),
                )
            }),
        )
        .route(
            "/js/api.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/api.js"),
                )
            }),
        )
        .route(
            "/js/dialogs.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/dialogs.js"),
                )
            }),
        )
        .route(
            "/js/library.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/library.js"),
                )
            }),
        )
        .route(
            "/js/cassette.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/cassette.js"),
                )
            }),
        )
        .route(
            "/js/settings.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/settings.js"),
                )
            }),
        )
        .route(
            "/js/export.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/export.js"),
                )
            }),
        )
        .route(
            "/js/view.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/view.js"),
                )
            }),
        )
        .route(
            "/js/audio/loudness.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/audio/loudness.js"),
                )
            }),
        )
        .route(
            "/js/audio/player.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/audio/player.js"),
                )
            }),
        )
        .route(
            "/js/audio/reference-tone.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/audio/reference-tone.js"),
                )
            }),
        )
        .route(
            "/js/buttons/common.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/buttons/common.js"),
                )
            }),
        )
        .route(
            "/js/buttons/library.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/buttons/library.js"),
                )
            }),
        )
        .route(
            "/js/buttons/analysis.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/buttons/analysis.js"),
                )
            }),
        )
        .route(
            "/js/buttons/entries.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/buttons/entries.js"),
                )
            }),
        )
        .route(
            "/js/buttons/settings.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/buttons/settings.js"),
                )
            }),
        )
        .route(
            "/js/buttons/playback.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/buttons/playback.js"),
                )
            }),
        )
        .route(
            "/favicon.png",
            get(|| async {
                (
                    [("content-type", "image/png")],
                    include_bytes!("../../assets/cassette-studio-favicon.png").as_slice(),
                )
            }),
        )
        .route(
            "/",
            get(|| async { Html(include_str!("../../web/index.html")) }),
        )
        .route(
            "/reference-tone.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/js/audio/reference-tone.js"),
                )
            }),
        )
        .route(
            "/app.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/app.js"),
                )
            }),
        )
        .route(
            "/style.css",
            get(|| async {
                (
                    [("content-type", "text/css; charset=utf-8")],
                    include_str!("../../web/style.css"),
                )
            }),
        )
        .route(
            "/i18n.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../../web/i18n.js"),
                )
            }),
        )
        .route(
            "/en.json",
            get(|| async {
                (
                    [("content-type", "application/json; charset=utf-8")],
                    include_str!("../../web/en.json"),
                )
            }),
        )
}
