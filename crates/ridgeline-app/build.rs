fn main() {
    // gpui already embeds its application manifest (DPI awareness etc.), so
    // this only adds the app icon; a second manifest fails to link.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let dir = std::path::Path::new("resources/windows");
        let icons = std::path::Path::new("resources");
        println!("cargo:rerun-if-changed=resources/windows/ridgeline.rc");
        println!("cargo:rerun-if-changed=resources/icon.ico");
        embed_resource::compile(
            dir.join("ridgeline.rc"),
            embed_resource::ParamsIncludeDirs([dir, icons]),
        )
        .manifest_optional()
        .unwrap();
    }
}
