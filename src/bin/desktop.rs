#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = run() {
        cassette_ui::show_startup_error(error.to_string());
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Runtime::new()?;
    let mut server = runtime.block_on(cassette_studio::start_server())?;
    let result = cassette_ui::run_native(server.url.clone(), server.root.clone());
    if let Some(shutdown) = server.shutdown.take() {
        let _ = shutdown.send(());
    }
    runtime.block_on(async {
        let _ = server.task.await;
    });
    result.map_err(|e| e.to_string().into())
}
