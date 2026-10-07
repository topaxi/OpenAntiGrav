//! The stack of views a package's files are read through.
//!
//! A [`Source`] answers byte-range reads. The package file is one; AES-XTS over
//! it is the next; a PFS file's blocks over that; and PFSC decompression over
//! that again. Each layer owns the one below it, and a layer many files read
//! through (the outer image, the inner image) is a [`Shared`] handle to it.
//! Nothing is decrypted or inflated ahead of a read: a layer reads the sectors a
//! request touches and keeps a few of the last ones.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::crypto::Xts;
use crate::error::{Error, Result};

/// Anything a range can be read out of.
pub trait Source: Send + std::fmt::Debug {
    /// Fills `buf` from `offset`; a read past the end is an error, never short.
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<()>;
}

pub fn bad(path: &std::path::Path, reason: impl Into<String>) -> Error {
    Error::Package {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

/// The package file itself.
#[derive(Debug)]
pub struct FileSource {
    pub path: PathBuf,
    pub file: File,
}

impl Source for FileSource {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        self.file
            .seek(SeekFrom::Start(offset))
            .and_then(|_| self.file.read_exact(buf))
            .map_err(|e| Error::io(&self.path, e))
    }
}

/// A layer many readers share.
#[derive(Clone, Debug)]
pub struct Shared(pub Arc<Mutex<Box<dyn Source>>>);

impl Shared {
    pub fn new(source: impl Source + 'static) -> Self {
        Self(Arc::new(Mutex::new(Box::new(source))))
    }
}

impl Source for Shared {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .read_at(offset, buf)
    }
}

/// The XTS sector size of a PFS image.
pub const XTS_SECTOR: usize = 0x1000;

/// AES-XTS over an image that starts at `base` in the layer below. Sectors before
/// `plain_sectors` (the superblock) are stored in the clear.
#[derive(Debug)]
pub struct XtsSource<S: Source> {
    pub inner: S,
    pub base: u64,
    pub len: u64,
    pub xts: Xts,
    pub plain_sectors: u64,
    cache: Vec<(u64, Vec<u8>)>,
}

impl<S: Source> XtsSource<S> {
    pub fn new(inner: S, base: u64, len: u64, xts: Xts, plain_sectors: u64) -> Self {
        Self {
            inner,
            base,
            len,
            xts,
            plain_sectors,
            cache: Vec::new(),
        }
    }

    fn sector(&mut self, number: u64) -> Result<&[u8]> {
        if let Some(i) = self.cache.iter().position(|(n, _)| *n == number) {
            let hit = self.cache.remove(i);
            self.cache.push(hit);
        } else {
            let mut data = vec![0u8; XTS_SECTOR];
            self.inner
                .read_at(self.base + number * XTS_SECTOR as u64, &mut data)?;
            if number >= self.plain_sectors {
                self.xts.decrypt_sector(&mut data, number);
            }
            if self.cache.len() >= 32 {
                self.cache.remove(0);
            }
            self.cache.push((number, data));
        }
        Ok(&self.cache.last().expect("just pushed").1)
    }
}

impl<S: Source> Source for XtsSource<S> {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        if offset + buf.len() as u64 > self.len {
            return Err(Error::Package {
                path: PathBuf::new(),
                reason: format!("read of {} at {offset} past the image end", buf.len()),
            });
        }
        let mut done = 0usize;
        while done < buf.len() {
            let at = offset + done as u64;
            let number = at / XTS_SECTOR as u64;
            let within = (at % XTS_SECTOR as u64) as usize;
            let take = (XTS_SECTOR - within).min(buf.len() - done);
            let sector = self.sector(number)?;
            buf[done..done + take].copy_from_slice(&sector[within..within + take]);
            done += take;
        }
        Ok(())
    }
}

/// A file's blocks as one range: contiguous from `start`, or a block list.
#[derive(Debug)]
pub struct Extents<S: Source> {
    pub inner: S,
    pub block_size: u64,
    pub start: u64,
    pub blocks: Option<Vec<u32>>,
    pub len: u64,
}

impl<S: Source> Source for Extents<S> {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        if offset + buf.len() as u64 > self.len {
            return Err(Error::Package {
                path: PathBuf::new(),
                reason: format!("read of {} at {offset} past the file end", buf.len()),
            });
        }
        let Some(blocks) = &self.blocks else {
            return self
                .inner
                .read_at(self.start * self.block_size + offset, buf);
        };
        let mut done = 0usize;
        while done < buf.len() {
            let at = offset + done as u64;
            let index = (at / self.block_size) as usize;
            let within = at % self.block_size;
            let take = ((self.block_size - within) as usize).min(buf.len() - done);
            let block = u64::from(*blocks.get(index).ok_or_else(|| Error::Package {
                path: PathBuf::new(),
                reason: format!("block {index} is outside the file's block list"),
            })?);
            self.inner.read_at(
                block * self.block_size + within,
                &mut buf[done..done + take],
            )?;
            done += take;
        }
        Ok(())
    }
}

/// The PFSC header's magic, `PFSC` read little-endian.
pub const PFSC_MAGIC: [u8; 4] = *b"PFSC";

/// A PFSC compressed file: 64 KiB sectors each stored, inflated, or (when the
/// map says it spans more than a sector) all zero.
#[derive(Debug)]
pub struct PfscSource<S: Source> {
    inner: S,
    block: u64,
    offsets: Vec<u64>,
    pub data_len: u64,
    cache: Vec<(u64, Vec<u8>)>,
    cache_blocks: usize,
}

impl<S: Source> PfscSource<S> {
    /// Reads the header and sector map. Anything the layout does not say is
    /// refused: an unknown block size, a length that is not a whole number of
    /// sectors (the reference silently drops that tail), a map that runs out of
    /// the file.
    pub fn open(mut inner: S, file_len: u64, cache_blocks: usize) -> Result<Self> {
        let mut head = [0u8; 0x30];
        inner.read_at(0, &mut head)?;
        let refuse = |why: String| Error::Package {
            path: PathBuf::new(),
            reason: format!("PFSC: {why}"),
        };
        if head[..4] != PFSC_MAGIC {
            return Err(refuse("no PFSC magic".into()));
        }
        let word = |at: usize| u32::from_le_bytes(head[at..at + 4].try_into().expect("4 bytes"));
        let quad = |at: usize| u64::from_le_bytes(head[at..at + 8].try_into().expect("8 bytes"));
        let block = u64::from(word(0x0C));
        if word(4) != 0 || block != quad(0x10) || !block.is_power_of_two() || block < 0x1000 {
            return Err(refuse(format!(
                "unexpected header (unk4 {}, block {block}, block2 {})",
                word(4),
                quad(0x10)
            )));
        }
        let (map_at, data_len) = (quad(0x18), quad(0x28));
        if data_len % block != 0 {
            return Err(refuse(format!(
                "data length {data_len} is not a multiple of the {block}-byte sector"
            )));
        }
        let count = (data_len / block) as usize;
        if map_at + (count as u64 + 1) * 8 > file_len {
            return Err(refuse("sector map runs past the file".into()));
        }
        let mut raw = vec![0u8; (count + 1) * 8];
        inner.read_at(map_at, &mut raw)?;
        let offsets: Vec<u64> = raw
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| u64::from_le_bytes(*c))
            .collect();
        if offsets.windows(2).any(|w| w[1] < w[0]) || offsets[count] > file_len {
            return Err(refuse(
                "sector map is not ascending or leaves the file".into(),
            ));
        }
        Ok(Self {
            inner,
            block,
            offsets,
            data_len,
            cache: Vec::new(),
            cache_blocks,
        })
    }

    fn sector(&mut self, index: u64) -> Result<&[u8]> {
        if let Some(i) = self.cache.iter().position(|(n, _)| *n == index) {
            let hit = self.cache.remove(i);
            self.cache.push(hit);
        } else {
            let (from, to) = (
                self.offsets[index as usize],
                self.offsets[index as usize + 1],
            );
            let span = to - from;
            let block = self.block as usize;
            let data = if span == self.block {
                let mut stored = vec![0u8; block];
                self.inner.read_at(from, &mut stored)?;
                stored
            } else if span > self.block {
                vec![0u8; block]
            } else {
                let mut packed = vec![0u8; span as usize];
                self.inner.read_at(from, &mut packed)?;
                inflate_sector(&packed, block, index)?
            };
            if self.cache.len() >= self.cache_blocks {
                self.cache.remove(0);
            }
            self.cache.push((index, data));
        }
        Ok(&self.cache.last().expect("just pushed").1)
    }
}

/// A zlib stream (two-byte header, deflate, checksum) inflated to exactly one sector.
fn inflate_sector(packed: &[u8], block: usize, index: u64) -> Result<Vec<u8>> {
    let refuse = |why: String| Error::Package {
        path: PathBuf::new(),
        reason: format!("PFSC sector {index}: {why}"),
    };
    let body = packed
        .get(2..)
        .ok_or_else(|| refuse("shorter than its header".into()))?;
    let out = miniz_oxide::inflate::decompress_to_vec_with_limit(body, block)
        .map_err(|e| refuse(format!("inflate failed: {:?}", e.status)))?;
    if out.len() != block {
        return Err(refuse(format!(
            "inflated to {} of {block} bytes",
            out.len()
        )));
    }
    Ok(out)
}

impl<S: Source> Source for PfscSource<S> {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<()> {
        if offset + buf.len() as u64 > self.data_len {
            return Err(Error::Package {
                path: PathBuf::new(),
                reason: format!("read of {} at {offset} past the PFSC end", buf.len()),
            });
        }
        let mut done = 0usize;
        while done < buf.len() {
            let at = offset + done as u64;
            let index = at / self.block;
            let within = (at % self.block) as usize;
            let take = (self.block as usize - within).min(buf.len() - done);
            let sector = self.sector(index)?;
            buf[done..done + take].copy_from_slice(&sector[within..within + take]);
            done += take;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
