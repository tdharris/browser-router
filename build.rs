fn main() {
    println!("cargo:rerun-if-changed=assets/app-icon.ico");

    embed_windows_resources();
}

#[cfg(windows)]
fn embed_windows_resources() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    winresource::WindowsResource::new()
        .set_icon("assets/app-icon.ico")
        .compile()
        .expect("failed to embed the application icon");
}

#[cfg(not(windows))]
fn embed_windows_resources() {}
