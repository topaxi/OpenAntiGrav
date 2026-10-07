//! A PFS image's superblock, inode table and directory tree. Little-endian.
//!
//! Superblock (block 0): version `0x00` (1), magic `0x08` (20130315), mode
//! `0x1C` (u16: 1 signed, 2 64-bit, 4 encrypted), block size `0x20`, inode count
//! `0x30`, inode-block count `0x40`, seed `0x370`. Inodes follow from block 1.

use super::layers::{Source, bad};
use crate::error::Result;
use std::path::PathBuf;

pub const MODE_SIGNED: u16 = 1;
pub const MODE_64BIT: u16 = 2;
pub const MODE_ENCRYPTED: u16 = 4;

/// Inode flag: the file's bytes are a PFSC container.
pub const FLAG_COMPRESSED: u32 = 1;

const DIRENT_FILE: i32 = 2;
const DIRENT_DIR: i32 = 3;

#[derive(Debug, Clone)]
pub struct Superblock {
    pub mode: u16,
    pub block_size: u64,
    pub inode_count: u64,
    pub inode_blocks: u64,
    pub seed: [u8; 16],
}

#[derive(Debug, Clone)]
pub struct Inode {
    pub mode: u16,
    pub flags: u32,
    /// The bytes the file occupies on disc.
    pub size: u64,
    /// The file's length once a PFSC layer is read through (equal to `size` when
    /// there is none). The reference names this field the other way round.
    pub size_logical: u64,
    pub blocks: u32,
    pub direct: [i32; 12],
    pub indirect: [i32; 5],
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub path: String,
    pub inode: u32,
}

/// A parsed image: its files, and how to reach each one's blocks.
#[derive(Debug)]
pub struct Pfs {
    pub sb: Superblock,
    pub inodes: Vec<Inode>,
    /// Files under `uroot`, paths relative to it, in directory order with each
    /// directory's files before its subdirectories.
    pub files: Vec<FileNode>,
    /// Files outside `uroot` (the outer image holds `pfs_image.dat` and a few
    /// flat files beside it).
    pub root_files: Vec<FileNode>,
    pub signed: bool,
}

fn le16(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(b[at..at + 2].try_into().expect("2 bytes"))
}
fn le32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}
fn le64(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().expect("8 bytes"))
}

impl Pfs {
    pub fn open(src: &mut dyn Source, what: &str) -> Result<Self> {
        let path = PathBuf::from(what);
        let mut head = vec![0u8; 0x400];
        src.read_at(0, &mut head)?;
        if le64(&head, 0) != 1 || le64(&head, 8) != 20_130_315 {
            return Err(bad(&path, "PFS superblock has the wrong version or magic"));
        }
        let mode = le16(&head, 0x1C);
        if mode & MODE_64BIT != 0 {
            return Err(bad(&path, "a 64-bit PFS has no reader here"));
        }
        let sb = Superblock {
            mode,
            block_size: u64::from(le32(&head, 0x20)),
            inode_count: le64(&head, 0x30),
            inode_blocks: le64(&head, 0x40),
            seed: head[0x370..0x380].try_into().expect("16 bytes"),
        };
        if !sb.block_size.is_power_of_two() || sb.block_size < 0x1000 || sb.inode_count > 1 << 22 {
            return Err(bad(&path, "PFS superblock sizes are implausible"));
        }
        let signed = mode & MODE_SIGNED != 0;
        let inode_size = if signed { 0x2C8 } else { 0xA8 };
        let per_block = sb.block_size as usize / inode_size;
        let mut inodes = Vec::with_capacity(sb.inode_count as usize);
        let mut block = vec![0u8; sb.block_size as usize];
        for i in 0..sb.inode_blocks {
            src.read_at(sb.block_size * (1 + i), &mut block)?;
            for j in 0..per_block {
                if inodes.len() as u64 >= sb.inode_count {
                    break;
                }
                inodes.push(parse_inode(&block[j * inode_size..], signed));
            }
        }
        if (inodes.len() as u64) < sb.inode_count {
            return Err(bad(&path, "inode table is shorter than the inode count"));
        }
        let mut pfs = Self {
            sb,
            inodes,
            files: Vec::new(),
            root_files: Vec::new(),
            signed,
        };
        pfs.walk(src, what)?;
        Ok(pfs)
    }

    fn dir_entries(
        &self,
        src: &mut dyn Source,
        ino: u32,
        what: &str,
    ) -> Result<Vec<(String, i32, u32)>> {
        let path = PathBuf::from(what);
        let node = self
            .inodes
            .get(ino as usize)
            .ok_or_else(|| bad(&path, format!("directory inode {ino} is outside the table")))?;
        let start = node.direct[0];
        if node.blocks < 1 || start < 1 || node.blocks > 1 << 20 {
            return Err(bad(&path, format!("inode {ino} is corrupt")));
        }
        let bs = self.sb.block_size as usize;
        let mut out = Vec::new();
        let mut block = vec![0u8; bs];
        for b in 0..node.blocks as u64 {
            src.read_at((start as u64 + b) * bs as u64, &mut block)?;
            let mut at = 0usize;
            while at + 16 <= bs {
                let ent_size = le32(&block, at + 12) as usize;
                if ent_size == 0 {
                    break;
                }
                let kind = le32(&block, at + 4) as i32;
                let name_len = le32(&block, at + 8) as usize;
                let name = block
                    .get(at + 16..at + 16 + name_len)
                    .ok_or_else(|| bad(&path, "directory entry name runs past its block"))?;
                out.push((
                    String::from_utf8_lossy(name).into_owned(),
                    kind,
                    le32(&block, at),
                ));
                at += ent_size;
            }
        }
        Ok(out)
    }

    fn walk(&mut self, src: &mut dyn Source, what: &str) -> Result<()> {
        let root = self.dir_entries(src, 0, what)?;
        let mut files = Vec::new();
        let mut root_files = Vec::new();
        for (name, kind, ino) in &root {
            if *kind == DIRENT_FILE {
                root_files.push(FileNode {
                    path: name.clone(),
                    inode: *ino,
                });
            }
        }
        for (name, kind, ino) in &root {
            if *kind == DIRENT_DIR && name == "uroot" {
                self.walk_dir(src, what, *ino, "", &mut files, 0)?;
            }
        }
        self.files = files;
        self.root_files = root_files;
        Ok(())
    }

    fn walk_dir(
        &self,
        src: &mut dyn Source,
        what: &str,
        ino: u32,
        prefix: &str,
        out: &mut Vec<FileNode>,
        depth: u32,
    ) -> Result<()> {
        if depth > 32 {
            return Err(bad(
                &PathBuf::from(what),
                "PFS directories nest past 32 levels",
            ));
        }
        let entries = self.dir_entries(src, ino, what)?;
        for (name, kind, child) in &entries {
            if *kind == DIRENT_FILE {
                out.push(FileNode {
                    path: format!("{prefix}{name}"),
                    inode: *child,
                });
            }
        }
        for (name, kind, child) in &entries {
            if *kind == DIRENT_DIR {
                self.walk_dir(
                    src,
                    what,
                    *child,
                    &format!("{prefix}{name}/"),
                    out,
                    depth + 1,
                )?;
            }
        }
        Ok(())
    }

    /// The block list of a file whose blocks are not one run, or `None` when
    /// they are contiguous from the first. A signed image stores a 32-byte
    /// signature before each block number in its indirect blocks.
    pub fn block_list(&self, src: &mut dyn Source, ino: u32) -> Result<Option<Vec<u32>>> {
        let node = &self.inodes[ino as usize];
        if node.blocks <= 1 || node.direct[1] == -1 {
            return Ok(None);
        }
        if !self.signed {
            return Err(bad(
                &PathBuf::new(),
                format!("inode {ino}: an unsigned PFS with scattered blocks"),
            ));
        }
        let bs = self.sb.block_size;
        let per = bs / 36;
        let total = u64::from(node.blocks);
        let mut list: Vec<u32> = node
            .direct
            .iter()
            .take(total.min(12) as usize)
            .map(|&b| b as u32)
            .collect();
        let read_sig_block = |src: &mut dyn Source, block: u64, i: u64| -> Result<u32> {
            let mut b = [0u8; 4];
            src.read_at(block * bs + i * 36 + 32, &mut b)?;
            Ok(u32::from_le_bytes(b))
        };
        let mut remaining = total.saturating_sub(12);
        for i in 0..remaining.min(per) {
            list.push(read_sig_block(src, node.indirect[0] as u64, i)?);
        }
        remaining = remaining.saturating_sub(per);
        let mut j = 0u64;
        while remaining > 0 {
            let table = u64::from(read_sig_block(src, node.indirect[1] as u64, j)?);
            for i in 0..remaining.min(per) {
                list.push(read_sig_block(src, table, i)?);
            }
            remaining = remaining.saturating_sub(per);
            j += 1;
        }
        let contiguous = list.windows(2).all(|w| w[0] + 1 == w[1]);
        Ok(if contiguous { None } else { Some(list) })
    }
}

fn parse_inode(b: &[u8], signed: bool) -> Inode {
    let mut direct = [0i32; 12];
    let mut indirect = [0i32; 5];
    let base = 0x64;
    for (i, d) in direct.iter_mut().enumerate() {
        *d = if signed {
            le32(b, base + i * 36 + 32)
        } else {
            le32(b, base + i * 4)
        } as i32;
    }
    for (i, d) in indirect.iter_mut().enumerate() {
        *d = if signed {
            le32(b, base + 12 * 36 + i * 36 + 32)
        } else {
            le32(b, base + 12 * 4 + i * 4)
        } as i32;
    }
    Inode {
        mode: le16(b, 0),
        flags: le32(b, 4),
        size: le64(b, 8),
        size_logical: le64(b, 16),
        blocks: le32(b, 0x60),
        direct,
        indirect,
    }
}
