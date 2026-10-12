//! A mod's files, read in place from a folder, a zip or the binary.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use zip::ZipArchive;

use crate::Error;

/// Which kind of place a mod is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// A directory.
    Folder,
    /// A `.zip`.
    Zip,
    /// A `.oagmod`.
    OagMod,
    /// Embedded in the binary.
    Builtin,
}

impl Origin {
    /// A word for the load report.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Folder => "folder",
            Self::Zip => "zip",
            Self::OagMod => "oagmod",
            Self::Builtin => "built in",
        }
    }
}

/// The files of one mod.
#[derive(Debug)]
pub struct Container {
    label: String,
    origin: Origin,
    kind: Kind,
}

#[derive(Debug)]
enum Kind {
    Folder(PathBuf),
    Zip {
        archive: Mutex<ZipArchive<BufReader<File>>>,
        /// Entry names as the zip spells them, files only.
        entries: Vec<String>,
        /// A single wrapping directory (`mymod/`) the zip's author zipped the
        /// folder with, stripped on every read; empty when there is none.
        prefix: String,
    },
    Memory(&'static [(&'static str, &'static [u8])]),
}

fn clean(origin: &str, name: &str) -> Result<String, Error> {
    let name = name.replace('\\', "/");
    let name = name.trim_start_matches("./");
    if name.starts_with('/')
        || name
            .split('/')
            .any(|part| part == ".." || part.contains(':'))
    {
        return Err(Error::BadName {
            origin: origin.to_string(),
            name: name.to_string(),
        });
    }
    Ok(name.to_string())
}

impl Container {
    /// A directory, `.zip` or `.oagmod`.
    ///
    /// # Errors
    ///
    /// The path will not open, or a zip's directory will not read.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let label = path.display().to_string();
        if path.is_dir() {
            return Ok(Self {
                label,
                origin: Origin::Folder,
                kind: Kind::Folder(path.to_path_buf()),
            });
        }
        let io = |source| Error::Io {
            origin: label.clone(),
            source,
        };
        let file = File::open(path).map_err(io)?;
        let archive = ZipArchive::new(BufReader::new(file)).map_err(|source| Error::Zip {
            origin: label.clone(),
            source,
        })?;
        let entries: Vec<String> = archive
            .file_names()
            .filter(|name| !name.ends_with('/'))
            .map(str::to_string)
            .collect();
        let prefix = wrapper(&entries);
        let origin = if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("oagmod"))
        {
            Origin::OagMod
        } else {
            Origin::Zip
        };
        Ok(Self {
            label,
            origin,
            kind: Kind::Zip {
                archive: Mutex::new(archive),
                entries,
                prefix,
            },
        })
    }

    /// Files compiled into the binary, as `(path, bytes)`.
    #[must_use]
    pub fn memory(label: &str, files: &'static [(&'static str, &'static [u8])]) -> Self {
        Self {
            label: label.to_string(),
            origin: Origin::Builtin,
            kind: Kind::Memory(files),
        }
    }

    /// The path, or the name of an embedded mod.
    #[must_use]
    pub fn label(&self) -> String {
        self.label.clone()
    }

    /// What kind of place it is.
    #[must_use]
    pub fn origin(&self) -> Origin {
        self.origin
    }

    /// Whether the mod holds this path.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.names().iter().any(|have| have == name)
    }

    /// Every file, by path inside the mod.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        let mut names = match &self.kind {
            Kind::Folder(root) => {
                let mut out = Vec::new();
                walk(root, root, &mut out);
                out
            }
            Kind::Zip {
                entries, prefix, ..
            } => entries
                .iter()
                .filter_map(|name| name.strip_prefix(prefix.as_str()))
                .map(str::to_string)
                .collect(),
            Kind::Memory(files) => files.iter().map(|(name, _)| (*name).to_string()).collect(),
        };
        names.sort();
        names
    }

    /// One file, or `None` when the mod has none by that path.
    ///
    /// # Errors
    ///
    /// The path escapes the mod, or the entry will not read (a method this
    /// build cannot decompress included).
    pub fn read(&self, name: &str) -> Result<Option<Vec<u8>>, Error> {
        let name = clean(&self.label, name)?;
        match &self.kind {
            Kind::Folder(root) => match std::fs::read(root.join(&name)) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(source) => Err(Error::Io {
                    origin: self.label.clone(),
                    source,
                }),
            },
            Kind::Zip {
                archive, prefix, ..
            } => {
                let full = format!("{prefix}{name}");
                let mut archive = archive.lock().unwrap_or_else(|p| p.into_inner());
                let mut entry = match archive.by_name(&full) {
                    Ok(entry) => entry,
                    Err(zip::result::ZipError::FileNotFound) => return Ok(None),
                    Err(source) => {
                        return Err(Error::Zip {
                            origin: self.label.clone(),
                            source,
                        });
                    }
                };
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).map_err(|source| Error::Io {
                    origin: format!("{}: {full}", self.label),
                    source,
                })?;
                Ok(Some(bytes))
            }
            Kind::Memory(files) => Ok(files
                .iter()
                .find(|(have, _)| *have == name)
                .map(|(_, bytes)| bytes.to_vec())),
        }
    }
}

/// The one directory every entry sits under, when the manifest is under it and
/// not at the root: what zipping a mod's folder (rather than its contents) makes.
fn wrapper(entries: &[String]) -> String {
    if entries.iter().any(|name| name == crate::MANIFEST) {
        return String::new();
    }
    let Some(first) = entries.first().and_then(|name| name.split_once('/')) else {
        return String::new();
    };
    let prefix = format!("{}/", first.0);
    if entries.iter().all(|name| name.starts_with(&prefix))
        && entries
            .iter()
            .any(|name| name == &format!("{prefix}{}", crate::MANIFEST))
    {
        prefix
    } else {
        String::new()
    }
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, out);
        } else if let Ok(relative) = path.strip_prefix(root) {
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}
