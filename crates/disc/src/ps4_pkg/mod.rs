//! Reading a PS4 fake package (`\x7fCNT`) in place.
//!
//! A `.pkg` made for HEN or a jailbreak is keyed with public constants, so it
//! opens on a PC with no console. The layout, outermost first (confidence in
//! `docs/formats/ps4-package.md`):
//!
//! 1. the header and entry table, big-endian, with `PARAM.SFO` in the clear;
//! 2. the **EKPFS**, unwrapped through two RSA-2048 keys and one AES-CBC layer
//!    ([`header::Header::ekpfs`]);
//! 3. the **outer PFS** at `pfs_image_offset`, AES-XTS encrypted in 4 KiB
//!    sectors under keys HMAC-derived from the EKPFS and the superblock seed;
//! 4. in it, `pfs_image.dat`, a **PFSC** container holding the **inner PFS**;
//! 5. the inner image's files, each either stored or a **PFSC** container again.
//!
//! Nothing is extracted: each layer decrypts or inflates the sectors a read
//! touches, so a multi-gigabyte `data00.psarc` costs what a request for it reads.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::error::Result;
use crate::package::{PackageFile, PackageSource};

pub mod crypto;
pub mod header;
pub mod keys;
pub mod layers;
pub mod pfs;
pub mod set;

use header::Header;
use layers::{Extents, FileSource, PfscSource, Shared, Source, XTS_SECTOR, XtsSource, bad};
use pfs::Pfs;
pub use set::Ps4Set;

/// The magic a PS4 package starts with.
pub const MAGIC: &[u8; 4] = &header::MAGIC;

/// Inflated sectors kept per file and for each of the two PFSC layers.
const FILE_CACHE_SECTORS: usize = 16;
const IMAGE_CACHE_SECTORS: usize = 64;

struct State {
    path: PathBuf,
    files: Vec<PackageFile>,
    nodes: Vec<u32>,
    inner: Pfs,
    image: Shared,
    readers: Vec<Option<Box<dyn Source>>>,
}

/// An opened package, shared process-wide by path so the several readers a race
/// opens (archives, sound banks, movies) decrypt each sector once between them.
pub struct Ps4Pkg {
    state: Arc<Mutex<State>>,
    files: Vec<PackageFile>,
    /// The package's `content_id`, e.g. `EP9000-CUSA05670_00-WIPEOUTOMEGA00EU`.
    pub content_id: String,
}

impl std::fmt::Debug for Ps4Pkg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ps4Pkg")
            .field("content_id", &self.content_id)
            .field("files", &self.files.len())
            .finish()
    }
}

type Registry = Vec<(PathBuf, Arc<Mutex<State>>, String)>;
static OPENED: Mutex<Registry> = Mutex::new(Vec::new());

/// `path`'s `CATEGORY` (`gd` for a base game, `gp` for a patch), read from the
/// plaintext `PARAM.SFO` entry without touching the file system under it.
#[must_use]
pub fn category(path: &Path) -> Option<String> {
    crate::sfo::text_field(&header::param_sfo(path)?, "CATEGORY")
}

/// `path`'s `TITLE_ID`, the same way.
#[must_use]
pub fn title_id(path: &Path) -> Option<String> {
    crate::sfo::title_id(&header::param_sfo(path)?)
}

impl Ps4Pkg {
    pub fn open(path: &Path) -> Result<Self> {
        let key = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        {
            let opened = OPENED
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some((_, state, id)) = opened.iter().find(|(p, ..)| *p == key) {
                let files = state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .files
                    .clone();
                return Ok(Self {
                    state: state.clone(),
                    files,
                    content_id: id.clone(),
                });
            }
        }
        let (state, content_id) = build(path)?;
        let files = state.files.clone();
        let state = Arc::new(Mutex::new(state));
        OPENED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((key, state.clone(), content_id.clone()));
        Ok(Self {
            state,
            files,
            content_id,
        })
    }
}

fn build(path: &Path) -> Result<(State, String)> {
    let file = std::fs::File::open(path).map_err(|e| crate::Error::io(path, e))?;
    let len = file
        .metadata()
        .map_err(|e| crate::Error::io(path, e))?
        .len();
    let mut src = FileSource {
        path: path.to_path_buf(),
        file,
    };
    let head = Header::read(&mut src, len)?;
    let ekpfs = head.ekpfs(&mut src)?;

    let mut seed = [0u8; 16];
    src.read_at(head.pfs_image_offset + 0x370, &mut seed)?;
    let mut sb = [0u8; 0x30];
    src.read_at(head.pfs_image_offset, &mut sb)?;
    let block_size = u64::from(u32::from_le_bytes(sb[0x20..0x24].try_into().expect("4")));
    let key = if head.new_crypt() {
        crypto::hmac_sha256(&ekpfs, &seed).to_vec()
    } else {
        ekpfs.clone()
    };
    let mut material = 1u32.to_le_bytes().to_vec();
    material.extend_from_slice(&seed);
    let enc = crypto::hmac_sha256(&key, &material);
    let tweak: [u8; 16] = enc[..16].try_into().expect("16");
    let data: [u8; 16] = enc[16..].try_into().expect("16");
    let xts = crypto::Xts::new(&tweak, &data);

    let outer_src = XtsSource::new(
        src,
        head.pfs_image_offset,
        head.pfs_image_size,
        xts,
        (block_size / XTS_SECTOR as u64).max(1),
    );
    let outer_shared = Shared::new(outer_src);
    let mut outer_read = outer_shared.clone();
    let outer = Pfs::open(&mut outer_read, "outer PFS")?;
    if outer.sb.mode & pfs::MODE_ENCRYPTED == 0 {
        return Err(bad(path, "outer PFS is not marked encrypted"));
    }
    let image_node = outer
        .files
        .iter()
        .find(|f| f.path == "pfs_image.dat")
        .ok_or_else(|| bad(path, "outer PFS has no pfs_image.dat"))?;
    let image_ino = &outer.inodes[image_node.inode as usize];
    let extents = Extents {
        inner: outer_shared.clone(),
        block_size: outer.sb.block_size,
        start: image_ino.direct[0] as u64,
        blocks: outer.block_list(&mut outer_read, image_node.inode)?,
        len: image_ino.size,
    };
    let image = Shared::new(PfscSource::open(
        extents,
        image_ino.size,
        IMAGE_CACHE_SECTORS,
    )?);
    let mut image_read = image.clone();
    let inner = Pfs::open(&mut image_read, "inner PFS")?;
    if inner.sb.mode & pfs::MODE_ENCRYPTED != 0 {
        return Err(bad(path, "the inner PFS is encrypted; no reader for that"));
    }
    let files = inner
        .files
        .iter()
        .map(|f| PackageFile {
            path: f.path.clone(),
            size: inner.inodes[f.inode as usize].size_logical,
        })
        .collect::<Vec<_>>();
    let nodes = inner.files.iter().map(|f| f.inode).collect();
    let count = files.len();
    Ok((
        State {
            path: path.to_path_buf(),
            files,
            nodes,
            inner,
            image,
            readers: (0..count).map(|_| None).collect(),
        },
        head.content_id,
    ))
}

impl State {
    fn reader(&mut self, index: usize) -> Result<&mut Box<dyn Source>> {
        if self.readers[index].is_none() {
            let ino = self.nodes[index];
            let node = self.inner.inodes[ino as usize].clone();
            let mut image = self.image.clone();
            let extents = Extents {
                inner: self.image.clone(),
                block_size: self.inner.sb.block_size,
                start: node.direct[0] as u64,
                blocks: self.inner.block_list(&mut image, ino)?,
                len: node.size,
            };
            let reader: Box<dyn Source> = if node.flags & pfs::FLAG_COMPRESSED != 0 {
                let pfsc = PfscSource::open(extents, node.size, FILE_CACHE_SECTORS)
                    .map_err(|e| bad(&self.path, format!("{}: {e}", self.files[index].path)))?;
                if pfsc.data_len != node.size_logical {
                    return Err(bad(
                        &self.path,
                        format!(
                            "{}: PFSC holds {} bytes but the inode says {}",
                            self.files[index].path, pfsc.data_len, node.size_logical
                        ),
                    ));
                }
                Box::new(pfsc)
            } else {
                Box::new(extents)
            };
            self.readers[index] = Some(reader);
        }
        Ok(self.readers[index].as_mut().expect("just set"))
    }
}

impl PackageSource for Ps4Pkg {
    fn files(&self) -> &[PackageFile] {
        &self.files
    }

    fn read(&mut self, index: usize, offset: u64, len: u64) -> Result<Vec<u8>> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let size = state.files[index].size;
        let end = size.min(offset.saturating_add(len));
        if offset >= end {
            return Ok(Vec::new());
        }
        let mut out = vec![0u8; (end - offset) as usize];
        state.reader(index)?.read_at(offset, &mut out)?;
        Ok(out)
    }
}
