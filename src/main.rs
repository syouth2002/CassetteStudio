#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = cassette_studio::start_server().await?;
    tokio::signal::ctrl_c().await?;
    if let Some(shutdown) = server.shutdown.take() {
        let _ = shutdown.send(());
    }
    let _ = server.task.await;
    Ok(())
}
