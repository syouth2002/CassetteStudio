fn main() {
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
