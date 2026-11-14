//! Local HTTP server lifecycle and route composition.
mod api;
mod assets;
mod imports;
mod jcard;
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
        .merge(jcard::routes())
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024 * 1024))
        .layer(middleware::from_fn(security::same_origin))
        .with_state(app);
    let configured = std::env::var("CASSETTE_PORT").ok();
    let port = configured.as_deref().unwrap_or("3210").parse::<u16>()?;
    let listener = match tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await
    {
        Ok(listener) => listener,
        Err(error)
            if configured.is_none()
                && matches!(
                    error.kind(),
                    std::io::ErrorKind::AddrInUse | std::io::ErrorKind::PermissionDenied
                ) =>
        {
            // A reserved or occupied default port must not prevent a native window from starting.
            tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?
        }
        Err(error) => return Err(error.into()),
    };
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
