//! The `\x7fCNT` header, its entry table and the key chain that ends in the
//! image's EKPFS. Everything here is big-endian.

use std::path::Path;

use super::crypto::{aes_cbc_decrypt, rsa_decrypt, sha256};
use super::keys;
use super::layers::{FileSource, Source, bad};
use crate::error::Result;

/// The package magic, `\x7fCNT`.
pub const MAGIC: [u8; 4] = *b"\x7fCNT";

pub const ENTRY_KEYS: u32 = 0x10;
pub const IMAGE_KEY: u32 = 0x20;
pub const PARAM_SFO: u32 = 0x1000;

/// `pfs_flags` bit that selects the second key derivation.
const PFS_NEW_CRYPT: u64 = 0x2000_0000_0000_0000;

/// One row of the entry table, kept as the 32 bytes it was read from because the
/// image key's IV is a hash of that record.
#[derive(Debug, Clone)]
pub struct Entry {
    pub id: u32,
    pub data_offset: u32,
    pub data_size: u32,
    pub raw: [u8; 32],
}

/// What the header and entry table say.
#[derive(Debug, Clone)]
pub struct Header {
    pub content_id: String,
    pub entries: Vec<Entry>,
    pub pfs_flags: u64,
    pub pfs_image_offset: u64,
    pub pfs_image_size: u64,
}

fn be32(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}
fn be64(b: &[u8], at: usize) -> u64 {
    u64::from_be_bytes(b[at..at + 8].try_into().expect("8 bytes"))
}

impl Header {
    pub fn read(src: &mut FileSource, file_len: u64) -> Result<Self> {
        let path = src.path.clone();
        if file_len < 0x1000 {
            return Err(bad(&path, "too short for a PS4 package header"));
        }
        let mut head = vec![0u8; 0x1000];
        src.read_at(0, &mut head)?;
        if head[..4] != MAGIC {
            return Err(bad(&path, "not a PS4 package (no \\x7fCNT magic)"));
        }
        let count = be32(&head, 0x10) as usize;
        let table = u64::from(be32(&head, 0x18));
        if count > 4096 || table + count as u64 * 32 > file_len {
            return Err(bad(&path, "entry table leaves the file"));
        }
        let mut raw = vec![0u8; count * 32];
        src.read_at(table, &mut raw)?;
        let entries = raw
            .as_chunks::<32>()
            .0
            .iter()
            .map(|r| Entry {
                id: be32(r, 0),
                data_offset: be32(r, 16),
                data_size: be32(r, 20),
                raw: *r,
            })
            .collect();
        let content_id = String::from_utf8_lossy(&head[0x40..0x64])
            .trim_end_matches('\0')
            .to_string();
        let pfs_image_offset = be64(&head, 0x410);
        let pfs_image_size = be64(&head, 0x418);
        if pfs_image_offset == 0 || pfs_image_offset + pfs_image_size > file_len {
            return Err(bad(
                &path,
                format!(
                    "PFS image {pfs_image_offset:#x}+{pfs_image_size:#x} leaves the \
                     {file_len}-byte file (an incomplete download?)"
                ),
            ));
        }
        Ok(Self {
            content_id,
            entries,
            pfs_flags: be64(&head, 0x408),
            pfs_image_offset,
            pfs_image_size,
        })
    }

    pub fn entry(&self, id: u32) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// An entry's stored bytes.
    pub fn read_entry(&self, src: &mut FileSource, id: u32) -> Result<Option<Vec<u8>>> {
        let Some(e) = self.entry(id) else {
            return Ok(None);
        };
        let mut data = vec![0u8; e.data_size as usize];
        src.read_at(u64::from(e.data_offset), &mut data)?;
        Ok(Some(data))
    }

    /// The EKPFS: entry key 3 unwrapped with the derived key-3 private key, which
    /// keys an AES-CBC layer over the image key entry, which unwraps with the fake
    /// package key.
    pub fn ekpfs(&self, src: &mut FileSource) -> Result<Vec<u8>> {
        let path = src.path.clone();
        let keys_entry = self
            .read_entry(src, ENTRY_KEYS)?
            .ok_or_else(|| bad(&path, "no entry-keys entry"))?;
        let mut image_key = self
            .read_entry(src, IMAGE_KEY)?
            .ok_or_else(|| bad(&path, "no image-key entry"))?;
        let image_meta = self.entry(IMAGE_KEY).expect("read above");
        // Seed digest 0x20, seven digests 0x20 each, then seven 0x100-byte keys.
        let key3 = keys_entry
            .get(32 + 7 * 32 + 3 * 256..32 + 7 * 32 + 4 * 256)
            .ok_or_else(|| bad(&path, "entry-keys entry is shorter than its seven keys"))?;
        let dk3 = rsa_decrypt(key3, &keys::ENTRY_KEY3_D, &keys::ENTRY_KEY3_N).ok_or_else(|| {
            bad(
                &path,
                "entry key 3 does not unwrap with the fake-package key: not a fake package \
                 (a retail package is keyed to a console)",
            )
        })?;
        let mut seed = image_meta.raw.to_vec();
        seed.extend_from_slice(&dk3);
        let iv_key = sha256(&seed);
        if image_key.len() % 16 != 0 {
            return Err(bad(
                &path,
                "image-key entry is not a whole number of AES blocks",
            ));
        }
        aes_cbc_decrypt(
            &mut image_key,
            iv_key[16..].try_into().expect("16 bytes"),
            iv_key[..16].try_into().expect("16 bytes"),
        );
        rsa_decrypt(&image_key, &keys::IMAGE_KEY_D, &keys::IMAGE_KEY_N)
            .ok_or_else(|| bad(&path, "image key does not unwrap to an EKPFS"))
    }

    /// Whether the image uses the second XTS-key derivation.
    pub fn new_crypt(&self) -> bool {
        self.pfs_flags & PFS_NEW_CRYPT != 0
    }
}

/// `path`'s PARAM.SFO, without opening the file system under it.
pub fn param_sfo(path: &Path) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let mut src = FileSource {
        path: path.to_path_buf(),
        file,
    };
    let header = Header::read(&mut src, len).ok()?;
    header.read_entry(&mut src, PARAM_SFO).ok()?
}
