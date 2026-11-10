fn main() {
    // gpui embeds this manifest from its own build script, but the link flag
    // it emits only reaches binaries of the gpui package itself, so without
    // this ridgeline.exe ships without DPI awareness and with a generic icon.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let dir = std::path::Path::new("resources/windows");
        let icons = std::path::Path::new("resources");
        println!("cargo:rerun-if-changed=resources/windows/ridgeline.rc");
        println!("cargo:rerun-if-changed=resources/windows/ridgeline.manifest.xml");
        println!("cargo:rerun-if-changed=resources/icon.ico");
        embed_resource::compile(
            dir.join("ridgeline.rc"),
            embed_resource::ParamsIncludeDirs([dir, icons]),
        )
        .manifest_required()
        .unwrap();
    }
}
