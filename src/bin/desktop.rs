#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, WindowBuilder},
};
use wry::{WebContext, WebViewBuilder};
#[cfg(target_os = "linux")]
use {tao::platform::unix::WindowExtUnix, wry::WebViewBuilderExtUnix};

fn main() {
    if let Err(error) = run() {
        rfd::MessageDialog::new()
            .set_title("Cassette Studio")
            .set_description(error.to_string())
            .set_level(rfd::MessageLevel::Error)
            .show();
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Runtime::new()?;
    let mut server = runtime.block_on(cassette_studio::start_server())?;
    let events = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Cassette Studio")
        .with_window_icon(Some(Icon::from_rgba(
            include_bytes!("../../assets/cassette-studio-app.rgba").to_vec(),
            128,
            128,
        )?))
        .with_inner_size(LogicalSize::new(1200.0, 640.0))
        .with_min_inner_size(LogicalSize::new(860.0, 560.0))
        .build(&events)?;
    std::fs::create_dir_all(server.root.join("webview"))?;
    let mut context = WebContext::new(Some(server.root.join("webview")));
    let origin = server.url.clone();
    let builder = WebViewBuilder::new_with_web_context(&mut context)
        .with_url(&server.url)
        .with_devtools(cfg!(debug_assertions))
        .with_autoplay(true)
        .with_initialization_script("window.CASSETTE_DESKTOP=true;")
        .with_navigation_handler(move |url| url.starts_with(&origin))
        .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
        .with_download_started_handler(|_url, destination| {
            let filename = destination
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Cassette.wav".into());
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(filename)
                .add_filter("WAV audio", &["wav"])
                .save_file()
            {
                *destination = path;
                true
            } else {
                false
            }
        });
    #[cfg(not(target_os = "linux"))]
    let view = builder.build(&window)?;
    #[cfg(target_os = "linux")]
    let view = builder.build_gtk(window.gtk_window())?;
    let mut runtime = Some(runtime);
    events.run(move |event, _, flow| {
        let _keep_alive = (&view, &window, &context);
        *flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if let Some(shutdown) = server.shutdown.take() {
                    let _ = shutdown.send(());
                }
                *flow = ControlFlow::Exit;
            }
            Event::LoopDestroyed => {
                if let Some(runtime) = runtime.take() {
                    runtime.shutdown_timeout(std::time::Duration::from_secs(2));
                }
            }
            _ => {}
        }
    });
}
