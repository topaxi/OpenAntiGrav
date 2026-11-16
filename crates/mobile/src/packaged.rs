//! The official mods an app package carries, unpacked to the folder `oag_mod`'s search
//! path already reads. See docs/tools/packaging.md, "Official mods".
//!
//! A desktop build ships them as a `mods` folder beside the program. A phone package is an
//! archive the mod loader cannot walk: `Mod::open`, the wasm host and the shader loaders all
//! take file paths. So the build writes the mods into the package's assets with an
//! [`INDEX`] (the NDK's asset directory listing returns files only, never subfolders), and
//! this module copies what the index names onto disk on the first launch of a build.

use std::io;
use std::path::{Component, Path, PathBuf};

/// The index asset: one `<size> <sha256> <path>` line per file, `path` relative to [`ROOT`].
pub const INDEX: &str = "official-mods.index";

/// The asset folder the files named by [`INDEX`] sit under.
pub const ROOT: &str = "official-mods";

/// The copy of the index kept inside the unpacked folder: equal bytes mean it is current.
const STAMP: &str = ".index";

/// What [`install`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Installed {
    /// The folder already held this build's mods.
    Current,
    /// The folder was written afresh.
    Extracted {
        /// Files written.
        files: usize,
        /// Bytes written.
        bytes: u64,
    },
}

/// One line of the index.
#[derive(Debug, PartialEq, Eq)]
struct Entry<'a> {
    size: u64,
    path: &'a str,
}

fn invalid(why: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why.into())
}

/// The files an index names. A path that is absolute or climbs out of the folder is refused:
/// the package is signed, but a reader of a list of paths checks them anyway.
fn parse(index: &str) -> io::Result<Vec<Entry<'_>>> {
    let mut entries = Vec::new();
    for line in index.lines().filter(|line| !line.trim().is_empty()) {
        let mut parts = line.splitn(3, ' ');
        let (Some(size), Some(_sha256), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            return Err(invalid(format!(
                "index line {line:?} is not `size sha256 path`"
            )));
        };
        let size = size
            .parse()
            .map_err(|_| invalid(format!("index line {line:?}: size is not a number")))?;
        if !Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        {
            return Err(invalid(format!("index path {path:?} leaves its folder")));
        }
        entries.push(Entry { size, path });
    }
    if entries.is_empty() {
        return Err(invalid("the index names no file"));
    }
    Ok(entries)
}

/// Makes `dest` hold the mods the package carries. `read` returns an asset's bytes by name
/// relative to the package's asset root ([`INDEX`], `official-mods/<path>`).
///
/// A folder whose [`STAMP`] equals the index is left as it is. Otherwise the files are
/// written beside it and swapped in whole, so a mod a newer build dropped does not linger
/// and load, and an interrupted copy leaves no half folder under `dest`'s name.
///
/// # Errors
///
/// The package has no index, an index line or a file does not match, or the disk refuses a
/// write. `dest` is then absent or the previous build's complete copy, never partial.
pub fn install(
    read: &mut dyn FnMut(&str) -> io::Result<Vec<u8>>,
    dest: &Path,
) -> io::Result<Installed> {
    let index = read(INDEX)?;
    if std::fs::read(dest.join(STAMP)).is_ok_and(|have| have == index) {
        return Ok(Installed::Current);
    }
    let text = std::str::from_utf8(&index).map_err(|_| invalid("the index is not UTF-8"))?;
    let entries = parse(text)?;

    let mut staging = dest.as_os_str().to_owned();
    staging.push(".new");
    let staging = PathBuf::from(staging);
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;
    let mut bytes = 0;
    for entry in &entries {
        let data = read(&format!("{ROOT}/{}", entry.path))?;
        if data.len() as u64 != entry.size {
            return Err(invalid(format!(
                "{}: {} bytes, the index says {}",
                entry.path,
                data.len(),
                entry.size
            )));
        }
        let target = staging.join(entry.path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, &data)?;
        bytes += entry.size;
    }
    std::fs::write(staging.join(STAMP), &index)?;

    if dest.exists() {
        std::fs::remove_dir_all(dest)?;
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(&staging, dest)?;
    Ok(Installed::Extracted {
        files: entries.len(),
        bytes,
    })
}

#[cfg(test)]
mod tests;
