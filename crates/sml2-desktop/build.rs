fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var_os("CARGO_FEATURE_NATIVE").is_some()
    {
        winresource::WindowsResource::new()
            .set_icon("../../assets/icon.ico")
            .compile()
            .expect("failed to embed the Windows application icon");
    }
}
