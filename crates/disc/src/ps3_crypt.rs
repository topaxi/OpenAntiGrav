//! Reading a PS3 disc's encrypted regions in place.
//!
//! A retail PS3 image holds its content in AES-128-CBC sectors, keyed by a
//! per-disc 16-byte key. [`DecryptSource`] wraps any [`SectorSource`] and hands
//! back plaintext sector by sector, so nothing is ever written out and no
//! decrypted copy exists. The format is documented in `docs/formats/ps3-disc.md`.
//!
//! # A key is never trusted, it is checked
//!
//! A wrong key yields gigabytes of plausible noise, and a *decrypted* image
//! still declares encrypted regions in its sector 0 (the table is copied through
//! decryption verbatim), so neither "the table says encrypted" nor "a key is
//! available" decides whether to decrypt. [`unlock`] decides by the oracle: a
//! file that starts inside an encrypted region must read as its own format's
//! magic, or as the full first sector of its plain twin. The raw view is tried
//! first, so an already-decrypted image is never double-decrypted.
//!
//! # The key never leaves this module as text
//!
//! [`DiscKey`] has a redacted `Debug`, and no error in this crate formats key
//! bytes. A `DiscImage` derives `Debug` over its boxed source, so a derived
//! impl here would leak the key into any `{:?}` log.

// Sector and block sizes arrive as slices from a `dyn SectorSource`; `chunks_exact`
// keeps the loops one line each and the lengths are checked at the boundary.
#![allow(clippy::chunks_exact_to_as_chunks)]

use std::path::{Path, PathBuf};

use aes::Aes128;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};

use crate::error::Result;
use crate::iso9660::Entry;
use crate::source::{SECTOR_SIZE, SectorSource};

/// The fixed secret a disc's `data1` is encrypted with to derive the sector key,
/// and the IV it is encrypted under (AES-128-CBC, one block). Both are the
/// constants RPCS3 carries as `key_d1` and `iv_d1` in `rpcs3/Loader/ISO.cpp`
/// (`iso_file_decryption::set_key_from_d1`, GPL-2.0), read from a checkout of
/// that file on 2026-10-07; the secret is also printed in
/// `docs/formats/ps3-disc.md`. Fixed, not per disc: opens nothing by itself.
///
/// **Not exercised against a real `data1`**: the maintainer's key is a redump
/// `.dkey`, the derived form. The oracle still decides, so a wrong IV here
/// rejects a good `data1` and can never accept a bad key.
const DATA1_SECRET: [u8; 16] = [
    0x38, 0x0b, 0xcf, 0x0b, 0x53, 0x45, 0x5b, 0x3c, 0x78, 0x17, 0xab, 0x4f, 0xa3, 0xba, 0x90, 0xed,
];

const DATA1_IV: [u8; 16] = [
    0x69, 0x47, 0x47, 0x72, 0xaf, 0x6f, 0xda, 0xb3, 0x42, 0x74, 0x3a, 0xef, 0xaa, 0x18, 0x62, 0x87,
];

/// Most plain-region pairs sector 0 may declare; a real disc has three.
const MAX_PLAIN_REGIONS: usize = 64;

/// Extensions whose first four bytes are known, for files inside an encrypted
/// region. Upper case.
const MAGIC_BY_EXT: [(&str, &[u8]); 5] = [
    (".PSARC", b"PSAR"),
    (".PNG", b"\x89PNG"),
    (".SPRX", b"SCE\0"),
    (".BIN", b"SCE\0"),
    (".PUP", b"SCEUF"),
];

/// A PS3 per-disc sector key. `Debug` is redacted on purpose.
#[derive(Clone, PartialEq, Eq)]
pub struct DiscKey([u8; 16]);

impl std::fmt::Debug for DiscKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DiscKey(<redacted>)")
    }
}

impl DiscKey {
    /// Parses a key from what a player has: 32 hex characters (surrounding
    /// whitespace, a `0x` prefix and inner spaces or dashes are ignored) or the
    /// 16 raw bytes of a redump `.dkey`.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if let Ok(text) = std::str::from_utf8(bytes) {
            let text = text.trim();
            let text = text.strip_prefix("0x").unwrap_or(text);
            let digits: Vec<u8> = text
                .bytes()
                .filter(|b| !matches!(b, b' ' | b'-' | b':' | b'\t'))
                .collect();
            if digits.len() == 32 && digits.iter().all(u8::is_ascii_hexdigit) {
                let mut key = [0u8; 16];
                for (out, pair) in key.iter_mut().zip(digits.chunks_exact(2)) {
                    let hex = std::str::from_utf8(pair).ok()?;
                    *out = u8::from_str_radix(hex, 16).ok()?;
                }
                return Some(Self(key));
            }
        }
        <[u8; 16]>::try_from(bytes).ok().map(Self)
    }

    /// The key as 32 lower-case hex digits, for storing in the player's own
    /// keys directory. Never log this.
    #[must_use]
    pub fn to_hex(&self) -> String {
        use std::fmt::Write;
        self.0.iter().fold(String::with_capacity(32), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
    }

    /// The two things a stored 16-byte value can be: the sector key itself (a
    /// redump `.dkey`), or `data1` run through the public secret. The oracle
    /// decides which.
    fn candidates(&self) -> [Aes128; 2] {
        let direct = Aes128::new(GenericArray::from_slice(&self.0));
        // One CBC block: encrypt `data1 XOR iv`.
        let mut block = GenericArray::clone_from_slice(&self.0);
        for (byte, iv) in block.iter_mut().zip(DATA1_IV) {
            *byte ^= iv;
        }
        Aes128::new(GenericArray::from_slice(&DATA1_SECRET)).encrypt_block(&mut block);
        let derived = Aes128::new(&block);
        [direct, derived]
    }
}

/// The plain regions sector 0 declares; every gap between them is encrypted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Regions {
    plain: Vec<(u32, u32)>,
}

impl Regions {
    /// Parses sector 0's table (big-endian: a count, a zero word, then
    /// inclusive `{first, last}` pairs), refusing anything that does not tile
    /// the whole image the way a real one does.
    #[must_use]
    pub fn parse(sector0: &[u8], sector_count: u32) -> Option<Self> {
        let count = u32::from_be_bytes(sector0.get(..4)?.try_into().ok()?) as usize;
        if count == 0 || count > MAX_PLAIN_REGIONS {
            return None;
        }
        let words = sector0.get(8..8 + 8 * count)?;
        let bounds: Vec<u32> = words
            .chunks_exact(4)
            .map(|w| u32::from_be_bytes([w[0], w[1], w[2], w[3]]))
            .collect();
        let ascending = bounds.windows(2).all(|w| w[0] < w[1]);
        if !ascending || bounds[0] != 0 || *bounds.last()? != sector_count.checked_sub(1)? {
            return None;
        }
        let plain = bounds.chunks_exact(2).map(|p| (p[0], p[1])).collect();
        Some(Self { plain })
    }

    /// Whether the table declares any encrypted span at all.
    #[must_use]
    pub fn has_encrypted(&self) -> bool {
        self.plain.len() > 1
    }

    /// Whether `lba` lies in an encrypted span.
    #[must_use]
    pub fn is_encrypted(&self, lba: u32) -> bool {
        !self.plain.iter().any(|&(a, b)| (a..=b).contains(&lba))
    }
}

/// Decrypts one encrypted sector in place: AES-128-CBC, no padding, IV of twelve
/// zero bytes then the **absolute** LBA big-endian, chaining reset per sector.
fn decrypt_sector(cipher: &Aes128, lba: u32, sector: &mut [u8]) {
    debug_assert_eq!(sector.len(), SECTOR_SIZE);
    let mut chain = [0u8; SECTOR_SIZE];
    chain.copy_from_slice(sector);
    for block in sector.chunks_exact_mut(16) {
        cipher.decrypt_block(GenericArray::from_mut_slice(block));
    }
    let mut previous = [0u8; 16];
    previous[12..].copy_from_slice(&lba.to_be_bytes());
    for (block, cipher_block) in sector.chunks_exact_mut(16).zip(chain.chunks_exact(16)) {
        for (byte, prev) in block.iter_mut().zip(previous) {
            *byte ^= prev;
        }
        previous.copy_from_slice(cipher_block);
    }
}

/// A [`SectorSource`] that decrypts the encrypted regions of the one under it.
pub struct DecryptSource {
    inner: Box<dyn SectorSource>,
    regions: Regions,
    cipher: Aes128,
}

impl std::fmt::Debug for DecryptSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecryptSource")
            .field("inner", &self.inner)
            .field("regions", &self.regions)
            .field("cipher", &"<redacted>")
            .finish()
    }
}

impl SectorSource for DecryptSource {
    fn sector_count(&self) -> u32 {
        self.inner.sector_count()
    }

    fn read_sector(&mut self, lba: u32, buf: &mut [u8]) -> Result<()> {
        self.inner.read_sector(lba, buf)?;
        if self.regions.is_encrypted(lba) {
            decrypt_sector(&self.cipher, lba, buf);
        }
        Ok(())
    }

    fn read_sectors(&mut self, lba: u32, count: u32) -> Result<Vec<u8>> {
        // One bulk read from the file, then decrypt in place: the default
        // loop would seek per sector.
        let mut out = self.inner.read_sectors(lba, count)?;
        for (i, sector) in out.chunks_exact_mut(SECTOR_SIZE).enumerate() {
            let at = lba + u32::try_from(i).unwrap_or(u32::MAX);
            if self.regions.is_encrypted(at) {
                decrypt_sector(&self.cipher, at, sector);
            }
        }
        Ok(out)
    }
}

/// What [`unlock`] concluded about a PS3 image.
#[derive(Debug)]
pub enum Unlocked {
    /// Sector 0 declares no encrypted region: not an encrypted PS3 image.
    NotEncrypted(Box<dyn SectorSource>),
    /// It declares them, but they already read as plaintext (a decrypted dump).
    AlreadyPlain(Box<dyn SectorSource>),
    /// Decrypting in place with the key the oracle accepted.
    Decrypting(Box<dyn SectorSource>),
    /// Encrypted and no supplied key passed the oracle. The raw source is
    /// returned so the caller can still read the plain regions.
    Locked(Box<dyn SectorSource>),
}

impl Unlocked {
    /// The source to read, whichever way it came out.
    #[must_use]
    pub fn into_source(self) -> Box<dyn SectorSource> {
        match self {
            Self::NotEncrypted(s)
            | Self::AlreadyPlain(s)
            | Self::Decrypting(s)
            | Self::Locked(s) => s,
        }
    }
}

/// Whether every oracle target on `source` reads as its predicted plaintext.
/// `false` when there is nothing to check, so a key is never accepted blind.
fn oracle_passes(source: &mut dyn SectorSource, entries: &[Entry], regions: &Regions) -> bool {
    let mut plain_twin: std::collections::HashMap<(String, u64), u32> =
        std::collections::HashMap::new();
    for e in entries.iter().filter(|e| !e.is_directory) {
        if !regions.is_encrypted(e.lba) {
            plain_twin
                .entry((e.name().to_ascii_uppercase(), e.size))
                .or_insert(e.lba);
        }
    }
    let mut checked = 0usize;
    let mut sector = vec![0u8; SECTOR_SIZE];
    for e in entries.iter().filter(|e| !e.is_directory) {
        if !regions.is_encrypted(e.lba) || e.size == 0 {
            continue;
        }
        let name = e.name().to_ascii_uppercase();
        let twin = plain_twin.get(&(name.clone(), e.size)).copied();
        let magic = MAGIC_BY_EXT
            .iter()
            .find(|(ext, _)| name.ends_with(ext))
            .map(|(_, m)| *m);
        let mut expect_twin = None;
        if let Some(twin_lba) = twin {
            let mut crib = vec![0u8; SECTOR_SIZE];
            if source.read_sector(twin_lba, &mut crib).is_err() {
                return false;
            }
            expect_twin = Some(crib);
        }
        if expect_twin.is_none() && magic.is_none() {
            continue;
        }
        if source.read_sector(e.lba, &mut sector).is_err() {
            return false;
        }
        let good = match (&expect_twin, magic) {
            (Some(crib), _) => sector == *crib,
            (None, Some(m)) => sector.starts_with(m),
            (None, None) => unreachable!("filtered above"),
        };
        if !good {
            return false;
        }
        checked += 1;
    }
    checked > 0
}

/// Decides how to read `source` given the keys the player supplied.
///
/// `source` is consumed and handed back inside the result. `entries` is the
/// raw view's directory walk (directories sit in plain regions on every real
/// disc; a walk that fails makes the image `Locked`, never a panic).
pub fn unlock(mut source: Box<dyn SectorSource>, keys: &[DiscKey]) -> Result<Unlocked> {
    let mut sector0 = vec![0u8; SECTOR_SIZE];
    source.read_sector(0, &mut sector0)?;
    let Some(regions) = Regions::parse(&sector0, source.sector_count()) else {
        return Ok(Unlocked::NotEncrypted(source));
    };
    if !regions.has_encrypted() {
        return Ok(Unlocked::NotEncrypted(source));
    }
    let Ok(entries) = crate::iso9660::walk(source.as_mut()) else {
        return Ok(Unlocked::Locked(source));
    };
    if oracle_passes(source.as_mut(), &entries, &regions) {
        return Ok(Unlocked::AlreadyPlain(source));
    }
    for key in keys {
        for cipher in key.candidates() {
            let mut view = DecryptSource {
                inner: source,
                regions: regions.clone(),
                cipher,
            };
            if oracle_passes(&mut view, &entries, &regions) {
                return Ok(Unlocked::Decrypting(Box::new(view)));
            }
            source = view.inner;
        }
    }
    Ok(Unlocked::Locked(source))
}

/// File names a key is looked for under, beside an image: `<stem>.dkey` and
/// `<stem>.key` (the redump convention), then the same two appended to the full
/// file name.
fn sibling_key_paths(image: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for ext in ["dkey", "key"] {
        out.push(image.with_extension(ext));
    }
    if let Some(name) = image.file_name() {
        for ext in ["dkey", "key"] {
            let mut full = name.to_os_string();
            full.push(format!(".{ext}"));
            out.push(image.with_file_name(full));
        }
    }
    out
}

/// The application's own key directory: the player's config dir, `oag/keys`.
#[must_use]
pub fn keys_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("oag").join("keys"))
}

/// Every key the player has put where this build looks: beside the image (on
/// the web, a key file registered under the image's mount path), then
/// every `*.dkey` / `*.key` in [`keys_dir`]. An unreadable or malformed file is
/// skipped silently (its content may be a key; it is never echoed).
#[must_use]
pub fn find_keys(image: &Path) -> Vec<DiscKey> {
    let mut keys: Vec<DiscKey> = sibling_key_paths(image)
        .iter()
        .filter_map(|p| crate::mount::read_all(p).ok())
        .filter_map(|bytes| DiscKey::parse(&bytes))
        .collect();
    let mut paths = Vec::new();
    if let Some(dir) = keys_dir()
        && let Ok(read) = std::fs::read_dir(dir)
    {
        let mut stored: Vec<PathBuf> = read
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "dkey" || x == "key"))
            .collect();
        stored.sort();
        paths.extend(stored);
    }
    keys.extend(
        paths
            .iter()
            .filter_map(|p| std::fs::read(p).ok())
            .filter_map(|bytes| DiscKey::parse(&bytes)),
    );
    keys
}

/// Stores `key` in [`keys_dir`] as `<name>.dkey` (the name only picks the
/// file; any key there is tried against any image), returning the path.
pub fn store_key(key: &DiscKey, name: &str) -> std::io::Result<PathBuf> {
    let dir = keys_dir()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no config directory"))?;
    std::fs::create_dir_all(&dir)?;
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let path = dir.join(format!("{safe}.dkey"));
    std::fs::write(&path, format!("{}\n", key.to_hex()))?;
    Ok(path)
}

/// Whether `key` opens the image at `path` - the check the in-game prompt runs
/// before it stores anything.
pub fn key_opens(path: &Path, key: &DiscKey) -> Result<bool> {
    let source = Box::new(crate::raw_source::RawSource::open(path)?);
    Ok(matches!(
        unlock(source, std::slice::from_ref(key))?,
        Unlocked::Decrypting(_)
    ))
}

#[cfg(test)]
mod tests;
