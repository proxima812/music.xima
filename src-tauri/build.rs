fn main() {
    // Android 15+ runs on 16 KB memory pages, but the NDK still links shared
    // libraries at 4 KB alignment by default — the platform then flags the app
    // as incompatible. https://developer.android.com/16kb-page-size
    //
    // Set via build script rather than .cargo/config.toml: the Tauri CLI exports
    // CARGO_TARGET_<triple>_RUSTFLAGS, and that env var would override any
    // rustflags declared in config.toml.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-link-arg=-Wl,-z,max-page-size=16384");
    }

    tauri_build::build()
}
