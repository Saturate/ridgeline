use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::sync::Mutex;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::MakeWriterExt as _;

const MAX_BYTES: u64 = 5 * 1024 * 1024;

pub fn path() -> Option<PathBuf> {
    Some(std::env::home_dir()?.join("Library/Logs/Ridgeline/ridgeline.log"))
}

fn open() -> Option<File> {
    let path = path()?;
    std::fs::create_dir_all(path.parent()?).ok()?;
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_BYTES) {
        let _ = std::fs::rename(&path, path.with_extension("log.1"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        return OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(&path)
            .ok();
    }
    #[cfg(not(unix))]
    {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok()
    }
}

pub fn init() {
    let filter = EnvFilter::try_from_env("RIDGELINE_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info,naga=warn,wgpu=warn"));
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true);
    match open() {
        Some(file) => builder
            .with_ansi(false)
            .with_writer(Mutex::new(file).and(std::io::stderr))
            .init(),
        None => builder.with_writer(std::io::stderr).init(),
    }
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "Ridgeline starting");
}
