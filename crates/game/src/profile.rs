//! Where the player's own text files (`settings.toml`, `records.toml`) are
//! read and written: the filesystem on native, the browser's `localStorage`
//! on the web.
//!
//! On native every function here *is* its [`std::fs`] namesake and
//! [`config_dir`] is [`dirs::config_dir`]: nothing changes there. On
//! `wasm32-unknown-unknown` there is no filesystem and `dirs` places no
//! directory, so a run kept nothing between reloads; here each file is one
//! `localStorage` item keyed by its path under [`config_dir`]'s `/config`.
//! `localStorage` is synchronous, which the callers' shape needs, and holds a
//! few MiB per origin, far above what these two files reach. Ghosts and pilot
//! files are binary or a directory and stay unpersisted on the web.
//! See docs/tools/web.md.
//!
//! **Every write is atomic** (ADR-0095): [`write`], [`write_bytes`] and
//! [`write_private`] stage the new content in `<name>.tmp` beside the file, flush it to the disk
//! and rename it over the old one, so an interrupted write leaves the previous file readable.
//! A file under the profile folder is never written by `std::fs::write`.

#[cfg(not(target_arch = "wasm32"))]
pub use dirs::config_dir;
#[cfg(not(target_arch = "wasm32"))]
pub use std::fs::{create_dir_all, read_to_string, rename};

#[cfg(not(target_arch = "wasm32"))]
pub use oag_io::{write, write_bytes, write_private};

#[cfg(target_arch = "wasm32")]
pub use web::{
    config_dir, create_dir_all, read_to_string, rename, write, write_bytes, write_private,
};

/// Makes the next [`write`] on this thread die after `bytes` bytes of the new content, leaving its
/// temp file as a crashed process would.
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) fn cut_the_next_write_after(bytes: usize) {
    oag_io::cut_the_next_write_after(bytes);
}

#[cfg(target_arch = "wasm32")]
mod web {
    use std::io;
    use std::path::{Path, PathBuf};

    /// The root every persisted path sits under on the web.
    #[allow(clippy::unnecessary_wraps)]
    #[must_use]
    pub fn config_dir() -> Option<PathBuf> {
        Some(PathBuf::from("/config"))
    }

    fn storage() -> io::Result<web_sys::Storage> {
        web_sys::window()
            .and_then(|window| window.local_storage().ok().flatten())
            .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "no localStorage here"))
    }

    fn key(path: &Path) -> String {
        format!("oag:{}", path.display())
    }

    fn failed(why: wasm_bindgen::JsValue) -> io::Error {
        io::Error::other(format!("localStorage: {why:?}"))
    }

    /// As [`std::fs::read_to_string`]: [`io::ErrorKind::NotFound`] when the
    /// item was never written.
    ///
    /// # Errors
    /// No such item, or no `localStorage` in this browser.
    pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
        storage()?
            .get_item(&key(path.as_ref()))
            .map_err(failed)?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "never saved"))
    }

    /// As [`std::fs::write`], for text. One `setItem` is all or nothing (the Storage standard
    /// leaves the old value when the quota throws), so it needs no temp file (ADR-0095).
    ///
    /// # Errors
    /// The quota is spent, or there is no `localStorage` in this browser.
    pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<()> {
        let text = std::str::from_utf8(contents.as_ref())
            .map_err(|why| io::Error::new(io::ErrorKind::InvalidData, why))?;
        storage()?
            .set_item(&key(path.as_ref()), text)
            .map_err(failed)
    }

    /// As [`std::fs::write`] for a file that is not text: the web keeps none.
    ///
    /// # Errors
    /// Always, [`io::ErrorKind::Unsupported`].
    pub fn write_bytes(_path: impl AsRef<Path>, _contents: impl AsRef<[u8]>) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "binary files are not kept on the web",
        ))
    }

    /// A secret is not put where any script of the origin can read it.
    ///
    /// # Errors
    /// Always, [`io::ErrorKind::Unsupported`].
    pub fn write_private(_path: impl AsRef<Path>, _contents: impl AsRef<[u8]>) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "a secret is not kept in localStorage",
        ))
    }

    /// As [`std::fs::rename`].
    ///
    /// # Errors
    /// As [`read_to_string`] and [`write`].
    pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
        let text = read_to_string(&from)?;
        write(to, text)?;
        storage()?.remove_item(&key(from.as_ref())).map_err(failed)
    }

    /// Nothing to create: an item's key is its whole path.
    ///
    /// # Errors
    /// Never; the signature matches [`std::fs::create_dir_all`].
    #[allow(clippy::unnecessary_wraps)]
    pub fn create_dir_all(_path: impl AsRef<Path>) -> io::Result<()> {
        Ok(())
    }
}
