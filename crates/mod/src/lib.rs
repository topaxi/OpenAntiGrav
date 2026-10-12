//! Mods: content a player adds to the game without touching the program.
//!
//! A mod is a folder, a `.zip` or a `.oagmod` (a zip container under this
//! project's own extension) holding an `oag-mod.toml` manifest and, by
//! convention, a `menu.toml` and `strings/<language>.toml`. It is **read in
//! place**: a folder is read file by file, a zip entry by entry, and nothing is
//! unpacked to disk. The format is [`docs/tools/mods.md`]'s subject; the model
//! is ADR-0066.
//!
//! This crate knows nothing of menus or titles. It finds mods ([`discover`]),
//! checks a manifest ([`Manifest`]), and hands a loaded [`Mod`]'s files to
//! whoever reads that kind of content. The game's own extras ride in the same
//! shape as [`extras`], embedded in the binary.
//!
//! [`docs/tools/mods.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/tools/mods.md

mod container;
mod discover;
pub mod extras;
mod manifest;
#[cfg(not(target_arch = "wasm32"))]
pub mod pack;

pub use container::{Container, Origin};
pub use discover::{Failed, Found, Loaded, SEARCH_ENV, load, mods_search_path, search};
pub use manifest::{
    Error as ManifestError, License, MANIFEST, Manifest, SCHEMA_KIND, SCHEMA_MAJOR,
};

/// Why a mod could not be opened or read.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The file system refused.
    #[error("{origin}: {source}")]
    Io {
        /// The mod's path or name.
        origin: String,
        /// What the OS said.
        source: std::io::Error,
    },
    /// The container is not a readable zip, or an entry in it is not.
    #[error("{origin}: {source}")]
    Zip {
        /// The mod's path.
        origin: String,
        /// What the zip reader said.
        source: zip::result::ZipError,
    },
    /// No `oag-mod.toml` at the root.
    #[error("{0}: no oag-mod.toml at the root")]
    NoManifest(String),
    /// The manifest is there and is wrong.
    #[error("{origin}: {source}")]
    Manifest {
        /// The mod's path.
        origin: String,
        /// What is wrong with it.
        source: ManifestError,
    },
    /// A file name that escapes the mod (`..`) or is absolute.
    #[error("{origin}: {name:?} is not a path inside the mod")]
    BadName {
        /// The mod's path.
        origin: String,
        /// The name asked for.
        name: String,
    },
}

/// A mod, opened: its manifest and a reader for its files.
#[derive(Debug)]
pub struct Mod {
    /// The checked manifest.
    pub manifest: Manifest,
    container: Container,
}

impl Mod {
    /// Opens a folder, a `.zip` or a `.oagmod` and checks its manifest.
    ///
    /// # Errors
    ///
    /// The container will not open, has no manifest, or the manifest is wrong.
    pub fn open(path: &std::path::Path) -> Result<Self, Error> {
        Self::from_container(Container::open(path)?)
    }

    /// A mod over an already opened container.
    ///
    /// # Errors
    ///
    /// No manifest, or a wrong one.
    pub fn from_container(container: Container) -> Result<Self, Error> {
        let origin = container.label();
        let bytes = container
            .read(MANIFEST)?
            .ok_or_else(|| Error::NoManifest(origin.clone()))?;
        let text = String::from_utf8_lossy(&bytes);
        let manifest = Manifest::parse(&text, &|name| container.contains(name))
            .map_err(|source| Error::Manifest { origin, source })?;
        Ok(Self {
            manifest,
            container,
        })
    }

    /// Where this mod came from, for a load report.
    #[must_use]
    pub fn label(&self) -> String {
        self.container.label()
    }

    /// The container's origin kind.
    #[must_use]
    pub fn origin(&self) -> Origin {
        self.container.origin()
    }

    /// One file by its path inside the mod, or `None` when the mod has none.
    ///
    /// # Errors
    ///
    /// The path escapes the mod, or the entry will not read.
    pub fn read(&self, name: &str) -> Result<Option<Vec<u8>>, Error> {
        self.container.read(name)
    }

    /// One file as UTF-8 text.
    ///
    /// # Errors
    ///
    /// As [`Self::read`]; a file that is not UTF-8 reads lossily.
    pub fn read_text(&self, name: &str) -> Result<Option<String>, Error> {
        Ok(self
            .read(name)?
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
    }

    /// Every file in the mod, by path.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.container.names()
    }

    /// The mod's menu file, `menu.toml`, when it has one.
    ///
    /// # Errors
    ///
    /// As [`Self::read_text`].
    pub fn menu(&self) -> Result<Option<String>, Error> {
        self.read_text("menu.toml")
    }

    /// The mod's strings for `language` (`strings/<language>.toml`), when it has them.
    ///
    /// # Errors
    ///
    /// As [`Self::read_text`].
    pub fn strings(&self, language: &str) -> Result<Option<String>, Error> {
        self.read_text(&format!("strings/{}.toml", language.to_ascii_lowercase()))
    }

    /// One line for the load report: id, version, license, where from.
    #[must_use]
    pub fn describe(&self) -> String {
        let m = &self.manifest;
        format!(
            "{} {} ({}) license: {}",
            m.id,
            m.version,
            self.label(),
            m.license.report()
        )
    }
}

#[cfg(test)]
mod tests;
