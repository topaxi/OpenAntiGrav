//! A title's PS4 packages as one file tree: the base `.pkg` and the patch(es)
//! beside it, each file's path prefixed `<package stem>/uroot/` so it reads as
//! the folder `LibOrbisPkg` extracts (`<out>/uroot/data09.psarc`) and an
//! archive resolves by its tail whichever package holds it.

use std::path::{Path, PathBuf};

use super::Ps4Pkg;
use crate::error::Result;
use crate::package::{PackageFile, PackageSource};

/// One or more packages of a title, read in place.
#[derive(Debug)]
pub struct Ps4Set {
    members: Vec<Ps4Pkg>,
    files: Vec<PackageFile>,
    at: Vec<(usize, usize)>,
    title_id: Option<String>,
}

/// Every `.pkg` in `path`'s folder that is a PS4 package of the same
/// `TITLE_ID` as `path` (which is first), base packages before patches, each
/// group sorted by name. Reads only each package's plaintext `PARAM.SFO`.
#[must_use]
pub fn siblings(path: &Path) -> Vec<PathBuf> {
    let Some(title) = super::title_id(path) else {
        return vec![path.to_path_buf()];
    };
    let mut found: Vec<(bool, PathBuf)> = vec![(
        super::category(path).as_deref() != Some("gd"),
        path.to_path_buf(),
    )];
    if let Some(dir) = path.parent() {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = entry.path();
            let is_pkg = p.extension().is_some_and(|x| x.eq_ignore_ascii_case("pkg"));
            if p == path || !p.is_file() || !is_pkg || !has_magic(&p) {
                continue;
            }
            if super::title_id(&p).as_deref() == Some(title.as_str()) {
                found.push((super::category(&p).as_deref() != Some("gd"), p));
            }
        }
    }
    found.sort();
    found.into_iter().map(|(_, p)| p).collect()
}

fn has_magic(path: &Path) -> bool {
    use std::io::Read;
    let mut magic = [0u8; 4];
    std::fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut magic))
        .is_ok()
        && magic == *super::MAGIC
}

impl Ps4Set {
    /// Opens `path` and its siblings, see [`siblings`].
    pub fn open(path: &Path) -> Result<Self> {
        let mut members = Vec::new();
        let mut files = Vec::new();
        let mut at = Vec::new();
        for (m, p) in siblings(path).iter().enumerate() {
            let pkg = Ps4Pkg::open(p)?;
            let stem = p
                .file_stem()
                .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
            for (e, file) in pkg.files().iter().enumerate() {
                files.push(PackageFile {
                    path: format!("{stem}/uroot/{}", file.path),
                    size: file.size,
                });
                at.push((m, e));
            }
            members.push(pkg);
        }
        Ok(Self {
            members,
            files,
            at,
            title_id: super::title_id(path),
        })
    }
}

impl PackageSource for Ps4Set {
    fn files(&self) -> &[PackageFile] {
        &self.files
    }

    fn read(&mut self, index: usize, offset: u64, len: u64) -> Result<Vec<u8>> {
        let (m, e) = self.at[index];
        self.members[m].read(e, offset, len)
    }

    fn platform(&self) -> crate::Platform {
        crate::Platform::Ps4
    }

    fn title_id(&mut self) -> Option<String> {
        self.title_id.clone()
    }
}
