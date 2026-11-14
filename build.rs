fn main() {
    println!("cargo:rustc-check-cfg=cfg(cassette_rust_web)");
    println!("cargo:rerun-if-changed=ui/web/pkg/cassette_ui.js");
    println!("cargo:rerun-if-changed=ui/web/pkg/cassette_ui_bg.wasm");
    if std::path::Path::new("ui/web/pkg/cassette_ui_bg.wasm").is_file() {
        println!("cargo:rustc-cfg=cassette_rust_web");
    }
    println!("cargo:rerun-if-changed=assets/cassette-studio-app.ico");
    #[cfg(windows)]
    if std::env::var_os("CARGO_FEATURE_DESKTOP").is_some() {
        winresource::WindowsResource::new()
            .set_icon("assets/cassette-studio-app.ico")
            .set("ProductName", "Cassette Studio")
            .set("FileDescription", "Cassette Studio")
            .compile()
            .expect("Could not embed the Windows application icon");
    }
}
