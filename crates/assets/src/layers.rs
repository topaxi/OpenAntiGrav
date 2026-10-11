//! The two layers around a source's own archives: the player's **override** in
//! front of the disc and this project's **fallback** behind its DLC.
//!
//! See [`docs/architecture/adr/0061-two-asset-layers-around-the-disc.md`] for
//! the decision. In short:
//!
//! - [`Kind::Override`] is searched before every archive, so a file the player
//!   supplies replaces the disc's entry of the same name. It is user content,
//!   empty by default and never shipped.
//! - [`Kind::Fallback`] is searched after every archive including the DLC
//!   packs, so it answers only what the disc and its packs cannot. It holds
//!   this project's own art, and a last-place layer cannot shadow anything by
//!   construction.
//!
//! Both are **loose entries**, not archives: a name maps to a file or to bytes.
//! A name is matched case-insensitively with either separator and without a
//! leading one, so a WAD's `Data\XML\Foo.xml` and a PSARC's `/data/xml/foo.xml`
//! reach the same entry.
//!
//! A loose entry has no directory position and no name hash, so the hash reads
//! and the neighbour reads skip these layers - the way a PSARC already refuses
//! them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// Which side of the disc a layer sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// In front of the disc: the player's own files replace its entries.
    Override,
    /// Behind the disc and its DLC: this project's own art, for what they lack.
    Fallback,
}

impl Kind {
    /// The word a load report uses.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Override => "override",
            Self::Fallback => "fallback",
        }
    }
}

#[derive(Debug, Clone)]
enum Entry {
    File(PathBuf),
    Bytes(&'static [u8]),
}

/// One layer of loose entries.
#[derive(Debug, Clone)]
pub struct Layer {
    kind: Kind,
    label: String,
    entries: BTreeMap<String, Entry>,
}

/// `name` as every layer compares it: lowercase, `/` for `\`, no leading `/`.
fn key(name: &str) -> String {
    name.replace('\\', "/")
        .trim_start_matches('/')
        .to_ascii_lowercase()
}

impl Layer {
    /// An empty layer.
    #[must_use]
    pub fn new(kind: Kind, label: impl Into<String>) -> Self {
        Self {
            kind,
            label: label.into(),
            entries: BTreeMap::new(),
        }
    }

    /// A layer of every file under `root`, named by its path relative to `root`.
    ///
    /// `None` when `root` is not a directory or holds no file, so a caller can
    /// tell "no such layer" from "an empty one" without a second look.
    #[must_use]
    pub fn from_dir(kind: Kind, root: &Path) -> Option<Self> {
        let mut layer = Self::new(kind, root.display().to_string());
        layer.walk(root, root);
        (!layer.entries.is_empty()).then_some(layer)
    }

    fn walk(&mut self, root: &Path, dir: &Path) {
        let Ok(read) = std::fs::read_dir(dir) else {
            return;
        };
        let mut paths: Vec<PathBuf> = read.filter_map(|e| e.ok().map(|e| e.path())).collect();
        // Directory order is the filesystem's; two files differing only in case
        // would otherwise race for one key. Sorted, the later one wins always.
        paths.sort();
        for path in paths {
            if path.is_dir() {
                self.walk(root, &path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                let name = relative.to_string_lossy().into_owned();
                self.entries.insert(key(&name), Entry::File(path));
            }
        }
    }

    /// Adds one entry held in memory, for a layer this build carries itself.
    pub fn insert_bytes(&mut self, name: &str, bytes: &'static [u8]) {
        self.entries.insert(key(name), Entry::Bytes(bytes));
    }

    /// Which side of the disc this layer is.
    #[must_use]
    pub const fn kind(&self) -> Kind {
        self.kind
    }

    /// Where the layer came from, for a load report.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// How many entries it holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether it holds none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether this layer holds `name`.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(&key(name))
    }

    /// Reads `name`.
    ///
    /// # Errors
    ///
    /// [`Error::LayerRead`] when the file will not read, and
    /// [`Error::NoSuchLayerEntry`] when the layer does not hold `name`.
    pub fn read(&self, name: &str) -> Result<Vec<u8>> {
        match self.entries.get(&key(name)) {
            Some(Entry::Bytes(bytes)) => Ok(bytes.to_vec()),
            Some(Entry::File(path)) => std::fs::read(path).map_err(|source| Error::LayerRead {
                path: path.display().to_string(),
                source,
            }),
            None => Err(Error::NoSuchLayerEntry {
                layer: self.label.clone(),
                name: name.to_string(),
            }),
        }
    }
}

/// What one answered lookup says about the layer that served it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// The name the caller asked for.
    pub name: String,
    /// Which layer answered.
    pub kind: Kind,
    /// Where that layer came from.
    pub layer: String,
}

impl Hit {
    /// One load-report line.
    #[must_use]
    pub fn describe(&self) -> String {
        match self.kind {
            Kind::Override => format!(
                "{} answered by the player's override ({}), searched ahead of the disc's own entries",
                self.name, self.layer
            ),
            Kind::Fallback => format!(
                "{} answered by this project's fallback ({}): no disc or DLC entry of that \
                 name, so this is a substitute, chosen, not measured",
                self.name, self.layer
            ),
        }
    }
}
