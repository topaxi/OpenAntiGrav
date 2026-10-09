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

#[cfg(not(target_arch = "wasm32"))]
pub use dirs::config_dir;
#[cfg(not(target_arch = "wasm32"))]
pub use std::fs::{create_dir_all, read_to_string, rename, write};

#[cfg(target_arch = "wasm32")]
pub use web::{config_dir, create_dir_all, read_to_string, rename, write};

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

    /// As [`std::fs::write`], for text.
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
