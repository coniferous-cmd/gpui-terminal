//! One file written whole: a temp file beside it, synced, then renamed over it.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::Context as _;

/// Writes `text` to `path`, so a reader sees the file before or after, each whole.
pub fn write(path: &Path, text: &str) -> anyhow::Result<()> {
    let folder = path
        .parent()
        .with_context(|| format!("atomic: {} has no folder", path.display()))?;
    fs::create_dir_all(folder)
        .with_context(|| format!("atomic: cannot create {}", folder.display()))?;
    let temp = temp(path);
    let written = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .and_then(|mut file| {
            file.write_all(text.as_bytes())?;
            file.sync_all()
        })
        .and_then(|()| fs::rename(&temp, path));
    if let Err(error) = written {
        if temp.exists()
            && let Err(left) = fs::remove_file(&temp)
        {
            log::error!("atomic: {} stays behind: {left}", temp.display());
        }
        return Err(error).with_context(|| {
            format!(
                "atomic: cannot write {} through {}",
                path.display(),
                temp.display()
            )
        });
    }
    Ok(())
}

/// A hidden name beside `path`, one this process has not used yet.
fn temp(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map_or_else(|| "file".into(), |name| name.to_string_lossy().into_owned());
    let count = WRITES.fetch_add(1, Ordering::Relaxed);
    path.with_file_name(format!(".{name}.{}-{count}.tmp", std::process::id()))
}

/// Writes made by this process, so each temp file has its own name.
static WRITES: AtomicU64 = AtomicU64::new(0);
