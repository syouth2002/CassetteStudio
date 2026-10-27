//! Local HTTP server lifecycle and route composition.
mod api;
mod assets;
mod security;
mod state;

use axum::{Router, extract::DefaultBodyLimit, middleware};
use state::App;
pub use state::default_data_directory;
use std::path::PathBuf;

pub async fn start_server() -> Result<Server, Box<dyn std::error::Error>> {
    let app = App::open()?;
    let root = app.root.clone();
    let routes = Router::new()
        .merge(assets::routes())
        .merge(api::routes(&root))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024 * 1024))
        .layer(middleware::from_fn(security::same_origin))
        .with_state(app);
    let port = std::env::var("CASSETTE_PORT")
        .unwrap_or_else(|_| "3210".into())
        .parse::<u16>()?;
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    println!(
        "Cassette Studio → http://127.0.0.1:{}",
        listener.local_addr()?.port()
    );
    println!("Data: {}\nCtrl+C to stop", root.display());
    let address = listener.local_addr()?;
    let (shutdown, stopped) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, routes)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await;
    });
    Ok(Server {
        url: format!("http://{address}/"),
        root,
        shutdown: Some(shutdown),
        task,
    })
}
pub struct Server {
    pub url: String,
    pub root: PathBuf,
    pub shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    pub task: tokio::task::JoinHandle<()>,
}
