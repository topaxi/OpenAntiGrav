//! The UnityFS container: a header, a compressed table of blocks and nodes, and
//! the blocks themselves. Decompressed, the blocks are one byte run that the
//! nodes slice into named files (SerializedFiles and `.resS` streams).

use crate::compress;
use crate::error::{Error, Result};
use crate::reader::{Endian, Reader};

/// What this reader will allocate or believe. Untrusted input is bounded by
/// these, never by a count read from the file alone.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// The most decompressed bytes of block data, summed over all blocks.
    pub max_decompressed: u64,
    /// The most nodes a bundle may list.
    pub max_nodes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_decompressed: 1 << 30,
            max_nodes: 1 << 16,
        }
    }
}

const SIGNATURE: &str = "UnityFS";
const FLAG_COMPRESSION_MASK: u32 = 0x3f;
const FLAG_BLOCKS_AND_DIRECTORY: u32 = 0x40;
const FLAG_INFO_AT_END: u32 = 0x80;
const FLAG_PADDING_AT_START: u32 = 0x200;
const KNOWN_FLAGS: u32 =
    FLAG_COMPRESSION_MASK | FLAG_BLOCKS_AND_DIRECTORY | FLAG_INFO_AT_END | FLAG_PADDING_AT_START;

/// One named file inside the decompressed block data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Offset into the decompressed data.
    pub offset: usize,
    /// Length.
    pub size: usize,
    /// Node flags (bit 2 marks a SerializedFile).
    pub flags: u32,
    /// Path, such as `CAB-<hash>` or `CAB-<hash>.resS`.
    pub path: String,
}

impl Node {
    /// Whether the node is a SerializedFile (flag bit 2) rather than a raw
    /// resource stream such as a `.resS`.
    #[must_use]
    pub fn is_serialized_file(&self) -> bool {
        self.flags & 4 != 0
    }
}

/// A parsed UnityFS bundle with its blocks already decompressed.
#[derive(Debug)]
pub struct Bundle {
    /// Archive format version (6 and 7 are read).
    pub archive_version: u32,
    /// The "player" version string (`5.x.x` in the content seen).
    pub player_version: String,
    /// The engine version that wrote it, such as `2018.3.8f1`.
    pub engine_version: String,
    /// Compression of the data blocks as the first block declares it.
    pub block_compression: u32,
    /// Number of data blocks.
    pub block_count: usize,
    data: Vec<u8>,
    nodes: Vec<Node>,
}

struct BlockInfo {
    usize: u32,
    csize: u32,
    flags: u16,
}

impl Bundle {
    /// Parses `bytes`, which must begin at the `UnityFS` signature.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        Self::parse_with(bytes, &Limits::default())
    }

    /// [`Self::parse`] with explicit limits.
    pub fn parse_with(bytes: &[u8], limits: &Limits) -> Result<Self> {
        let mut r = Reader::new(bytes, Endian::Big);
        if r.cstr().map_err(|_| Error::BadSignature)? != SIGNATURE {
            return Err(Error::BadSignature);
        }
        let archive_version = r.u32()?;
        if !(6..=7).contains(&archive_version) {
            return Err(Error::UnsupportedArchiveVersion(archive_version));
        }
        let player_version = r.cstr()?.to_string();
        let engine_version = r.cstr()?.to_string();
        let total_size = r.i64()?;
        let info_csize = r.u32()?;
        let info_usize = r.u32()?;
        let flags = r.u32()?;
        if flags & !KNOWN_FLAGS != 0 {
            return Err(Error::UnsupportedFlags(flags));
        }
        if total_size < 0 || total_size as u64 > bytes.len() as u64 {
            return Err(Error::Malformed(
                "declared bundle size exceeds the bytes given",
            ));
        }
        if archive_version >= 7 || flags & FLAG_PADDING_AT_START != 0 {
            r.align(16)?;
        }
        let info_comp = if flags & FLAG_INFO_AT_END != 0 {
            let start = bytes
                .len()
                .checked_sub(info_csize as usize)
                .ok_or(Error::Malformed("blocks info larger than the file"))?;
            if start < r.pos() {
                return Err(Error::Malformed("blocks info overlaps the header"));
            }
            &bytes[start..]
        } else {
            r.bytes(info_csize as usize)?
        };
        if u64::from(info_usize) > limits.max_decompressed {
            return Err(Error::LimitExceeded {
                what: "blocks info size",
                value: u64::from(info_usize),
                limit: limits.max_decompressed,
            });
        }
        let info = compress::decompress(
            flags & FLAG_COMPRESSION_MASK,
            info_comp,
            info_usize as usize,
        )?;

        let mut ir = Reader::new(&info, Endian::Big);
        ir.bytes(16)?; // the info's own hash, not verified
        let block_count = ir.i32()?;
        let block_count = usize::try_from(block_count)
            .ok()
            .filter(|&n| n <= ir.remaining() / 10)
            .ok_or(Error::LimitExceeded {
                what: "block count",
                value: block_count as u64,
                limit: (ir.remaining() / 10) as u64,
            })?;
        let mut blocks = Vec::with_capacity(block_count);
        let mut total = 0u64;
        for _ in 0..block_count {
            let b = BlockInfo {
                usize: ir.u32()?,
                csize: ir.u32()?,
                flags: ir.u16()?,
            };
            total += u64::from(b.usize);
            if total > limits.max_decompressed {
                return Err(Error::LimitExceeded {
                    what: "decompressed block data",
                    value: total,
                    limit: limits.max_decompressed,
                });
            }
            blocks.push(b);
        }
        let node_count = ir.i32()?;
        let node_limit = limits.max_nodes.min((ir.remaining() / 20) as u64);
        if node_count < 0 || node_count as u64 > node_limit {
            return Err(Error::LimitExceeded {
                what: "node count",
                value: node_count as u64,
                limit: node_limit,
            });
        }
        let mut nodes = Vec::with_capacity(node_count as usize);
        for _ in 0..node_count {
            let offset = ir.i64()?;
            let size = ir.i64()?;
            let flags = ir.u32()?;
            let path = ir.cstr()?.to_string();
            if offset < 0 || size < 0 || offset as u64 + size as u64 > total {
                return Err(Error::Malformed("node range outside the block data"));
            }
            nodes.push(Node {
                offset: offset as usize,
                size: size as usize,
                flags,
                path,
            });
        }

        let mut data = Vec::with_capacity(total as usize);
        for b in &blocks {
            let comp = r.bytes(b.csize as usize)?;
            let out = compress::decompress(
                u32::from(b.flags) & FLAG_COMPRESSION_MASK,
                comp,
                b.usize as usize,
            )?;
            data.extend_from_slice(&out);
        }
        let block_compression = blocks
            .first()
            .map_or(0, |b| u32::from(b.flags) & FLAG_COMPRESSION_MASK);
        Ok(Self {
            archive_version,
            player_version,
            engine_version,
            block_compression,
            block_count: blocks.len(),
            data,
            nodes,
        })
    }

    /// The node table, in file order.
    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// A node's bytes.
    #[must_use]
    pub fn node_bytes(&self, node: &Node) -> &[u8] {
        &self.data[node.offset..node.offset + node.size]
    }

    /// A node's bytes by exact path.
    #[must_use]
    pub fn node_by_path(&self, path: &str) -> Option<&[u8]> {
        self.nodes
            .iter()
            .find(|n| n.path == path)
            .map(|n| self.node_bytes(n))
    }
}
