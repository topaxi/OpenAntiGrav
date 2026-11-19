//! Downloading a [`Pin`], checking it against the hash in our source, and
//! unpacking the two executables and the license.
//!
//! Nothing lands in the install directory until everything has checked out: the
//! archive goes to a scratch directory beside it, is hashed as it arrives, is
//! unpacked there, and only then is the directory renamed into place. A hash
//! mismatch, a cancel, a full disk or a dead connection leaves nothing behind.

use crate::Tool;
use crate::pins::{Archive, Pin};
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

/// The license text kept beside the executables.
#[allow(unused)]
const LICENSE: &str = "LICENSE.txt";

/// Shared between the thread installing and the screen showing it.
#[derive(Debug, Default)]
pub struct Progress {
    /// Bytes of the archive received so far.
    pub done: AtomicU64,
    /// The archive's size: the server's `Content-Length`, else the pin's.
    pub total: AtomicU64,
    /// Set by the screen to stop; the install notices within one read.
    pub cancel: AtomicBool,
}

/// Stopped on request. Downcast an install error to this to tell it from a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the download was cancelled")
    }
}

impl std::error::Error for Cancelled {}

/// Installs `pin` under `root` and returns the directory holding `ffmpeg`,
/// `ffprobe` and `LICENSE.txt`. Already installed is a success without any
/// network use.
///
/// # Errors
/// Network, a hash that is not the pinned one, a malformed archive, a missing
/// member, the disk, or [`Cancelled`].
pub fn install(pin: &Pin, root: &Path, progress: &Progress) -> Result<PathBuf> {
    install_from(pin.url, pin, root, progress)
}

/// [`install`] from `url` instead of the pin's own, for a test's local server.
///
/// # Errors
/// As [`install`].
pub fn install_from(url: &str, pin: &Pin, root: &Path, progress: &Progress) -> Result<PathBuf> {
    let dest = root.join(pin.dir_name());
    if dest.join(Tool::Ffmpeg.file_name()).is_file() {
        return Ok(dest);
    }
    std::fs::create_dir_all(root).with_context(|| format!("creating {}", root.display()))?;
    let scratch = Scratch::new(root.join(format!(".partial-{}", pin.dir_name())))?;
    let archive = scratch.path.join("archive");
    download(url, pin, &archive, progress)?;
    let unpacked = scratch.path.join("unpacked");
    std::fs::create_dir(&unpacked)?;
    match pin.archive {
        Archive::TarXz => unpack_tar_xz(&archive, &unpacked, progress)?,
        Archive::Zip => unpack_zip(&archive, &unpacked, progress)?,
    }
    for name in [Tool::Ffmpeg.file_name(), Tool::Ffprobe.file_name(), LICENSE] {
        if !unpacked.join(name).is_file() {
            bail!("the archive has no {name}");
        }
    }
    if dest.exists() {
        std::fs::remove_dir_all(&dest)?;
    }
    std::fs::rename(&unpacked, &dest)
        .with_context(|| format!("moving the build to {}", dest.display()))?;
    Ok(dest)
}

/// A scratch directory removed on every way out.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(path: PathBuf) -> Result<Self> {
        if path.exists() {
            std::fs::remove_dir_all(&path)?;
        }
        std::fs::create_dir(&path).with_context(|| format!("creating {}", path.display()))?;
        Ok(Self { path })
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn check_cancel(progress: &Progress) -> Result<()> {
    if progress.cancel.load(Ordering::Relaxed) {
        return Err(Cancelled.into());
    }
    Ok(())
}

fn download(url: &str, pin: &Pin, to: &Path, progress: &Progress) -> Result<()> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(20)))
        .timeout_resolve(Some(Duration::from_secs(20)))
        .build()
        .into();
    let response = agent
        .get(url)
        .call()
        .with_context(|| format!("fetching {url}"))?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(pin.size);
    progress.total.store(total, Ordering::Relaxed);
    progress.done.store(0, Ordering::Relaxed);
    let mut body = response.into_body();
    let mut reader = body.with_config().limit(1 << 31).reader();
    let mut file = File::create(to)?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        check_cancel(progress)?;
        let n = reader.read(&mut buffer).context("receiving the archive")?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
        file.write_all(&buffer[..n])?;
        progress.done.fetch_add(n as u64, Ordering::Relaxed);
    }
    file.flush()?;
    let got = hex(&hash.finalize());
    if got != pin.sha256 {
        bail!(
            "the download's SHA-256 is {got}, not the pinned {}; nothing was installed",
            pin.sha256
        );
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// The file name to write `member` as, when it is one of the three we keep:
/// the two executables directly under a `bin` directory, and the license.
fn wanted(member: &Path) -> Option<&'static str> {
    let name = member.file_name()?.to_str()?;
    let in_bin = member
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|dir| dir == "bin");
    [Tool::Ffmpeg.file_name(), Tool::Ffprobe.file_name()]
        .into_iter()
        .find(|tool| in_bin && *tool == name)
        .or_else(|| (name == LICENSE).then_some(LICENSE))
}

fn unpack_tar_xz(archive: &Path, into: &Path, progress: &Progress) -> Result<()> {
    let file = BufReader::new(File::open(archive)?);
    let xz = lzma_rust2::XzReader::new(file, false);
    let mut tar = tar::Archive::new(xz);
    for entry in tar.entries().context("reading the tar")? {
        check_cancel(progress)?;
        let mut entry = entry.context("reading a tar entry")?;
        let path = entry.path()?.into_owned();
        let Some(name) = wanted(&path) else {
            continue;
        };
        write_member(&mut entry, &into.join(name), progress)?;
    }
    Ok(())
}

fn unpack_zip(archive: &Path, into: &Path, progress: &Progress) -> Result<()> {
    let mut zip = zip::ZipArchive::new(File::open(archive)?).context("reading the zip")?;
    for index in 0..zip.len() {
        check_cancel(progress)?;
        let mut entry = zip.by_index(index)?;
        let Some(path) = entry.enclosed_name() else {
            continue;
        };
        let Some(name) = wanted(&path) else {
            continue;
        };
        write_member(&mut entry, &into.join(name), progress)?;
    }
    Ok(())
}

fn write_member(from: &mut impl Read, to: &Path, progress: &Progress) -> Result<()> {
    let mut out = File::create(to)?;
    let mut buffer = vec![0_u8; 256 * 1024];
    loop {
        check_cancel(progress)?;
        let n = from.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        out.write_all(&buffer[..n])?;
    }
    out.flush()?;
    drop(out);
    #[cfg(unix)]
    if to.file_name().is_some_and(|n| n != LICENSE) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(to, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
