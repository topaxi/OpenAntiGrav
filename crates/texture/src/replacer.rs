//! A PPSSPP texture pack, indexed and lazily read: what a texture's key finds.
//!
//! [`Replacements`] holds a pack's `textures.ini` as a key table and a [`Files`] reader
//! over the pack's PNGs. A texture is looked up by [`Stored::replacement`]'s key in
//! PPSSPP's own order (`(K, H)`, then `(K, 0)`, then `(0, H)`); a hit is decoded the
//! first time it is asked for and kept under a byte cap. Nothing here reads a pack at
//! boot and nothing reaches simulation state: it is what a decoder asks while it builds
//! a picture (`docs/formats/ppsspp-texture-pack.md`, "Ready to wire").
//!
//! One pack at a time is **active** ([`install`]), a process-wide switch the race load
//! sets for the title it opened and every decoder on the load path reads through
//! [`find`]; a decoder with no pack active pays one lock read.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crate::replacement::{Replacement, Stored};
use crate::replacement_pack::Pack;

/// A pack's files by path, as the ini spells them (forward slashes, relative to the pack).
pub trait Files: Send + Sync {
    /// The file's bytes, `None` when the pack does not ship it.
    fn read(&self, name: &str) -> Option<Vec<u8>>;
}

/// A decoded replacement picture, RGBA8888, top row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes.
    pub rgba: Vec<u8>,
}

/// Why a pack is not used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// `[options] hash` is not `xxh64`.
    Hash(String),
    /// `ignoreAddress` is not true: the keys carry guest addresses nothing here can know.
    Address,
    /// A section that changes what is hashed (`[hashranges]`, `[reducehashranges]`).
    Section(String),
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hash(kind) => write!(f, "[options] hash = {kind}, not xxh64"),
            Self::Address => f.write_str("[options] ignoreAddress is not true"),
            Self::Section(name) => write!(f, "a [{name}] section, which changes what is hashed"),
        }
    }
}

impl std::error::Error for Refused {}

/// The most decoded pixels kept at once, bytes. **Chosen**: a Pulse race touches a few
/// hundred replacements, and the 2026 pack's images total 553 MB, which a handheld
/// cannot hold.
pub const CACHE_CAP: usize = 192 << 20;

/// A pack: its key table over [`Files`].
pub struct Replacements {
    name: String,
    files: Box<dyn Files>,
    serials: Vec<String>,
    /// `(dim ^ clut, data)` to the PNG's path; an empty path is a blank line, which
    /// leaves the texture alone. A key with no data hash is stored with `data` 0.
    table: HashMap<(u32, u32), String>,
    cache: Mutex<Cache>,
    replaced: AtomicUsize,
}

#[derive(Default)]
struct Cache {
    images: HashMap<String, Option<Arc<Image>>>,
    bytes: usize,
}

impl std::fmt::Debug for Replacements {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Replacements")
            .field("name", &self.name)
            .field("keys", &self.table.len())
            .finish_non_exhaustive()
    }
}

impl Replacements {
    /// Indexes `ini`, the text of a pack's `textures.ini`, over `files`.
    ///
    /// # Errors
    ///
    /// The pack hashes differently from what [`Stored::replacement`] computes.
    pub fn new(name: &str, ini: &str, files: Box<dyn Files>) -> Result<Self, Refused> {
        let pack = Pack::parse(ini);
        if let Some(kind) = pack.hash_kind().filter(|kind| kind != "xxh64") {
            return Err(Refused::Hash(kind));
        }
        if !pack.flag("ignoreaddress") {
            return Err(Refused::Address);
        }
        if let Some(section) = pack.other_sections.keys().next() {
            return Err(Refused::Section(section.clone()));
        }
        let mut table = HashMap::new();
        for line in pack.hashes.iter().filter(|line| line.level == 0) {
            // The first of two lines naming one key wins, as PPSSPP's map keeps it.
            table
                .entry((line.cache_key as u32, line.data_hash.unwrap_or(0)))
                .or_insert_with(|| line.target.replace('\\', "/"));
        }
        Ok(Self {
            name: name.to_string(),
            files,
            serials: pack.games.into_iter().map(|(serial, _)| serial).collect(),
            table,
            cache: Mutex::new(Cache::default()),
            replaced: AtomicUsize::new(0),
        })
    }

    /// The pack's name, for the load report.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Keys the ini names.
    #[must_use]
    pub fn keys(&self) -> usize {
        self.table.len()
    }

    /// How many textures [`Self::find`] has handed out since this pack was made.
    #[must_use]
    pub fn replaced(&self) -> usize {
        self.replaced.load(Ordering::Relaxed)
    }

    /// Whether the pack's `[games]` names the disc `serial`, in either spelling
    /// (`UCES00465`, `UCES-00465`). A pack naming none serves every game.
    #[must_use]
    pub fn serves(&self, serial: &str) -> bool {
        let bare = |s: &str| s.replace('-', "").to_ascii_uppercase();
        self.serials.is_empty() || self.serials.iter().any(|s| bare(s) == bare(serial))
    }

    /// The picture that replaces `texture`, `None` when the pack has none for it, names
    /// none (a blank line), or ships a PNG that does not decode.
    ///
    /// A texture whose full key cannot be known from its file (the palette hash reads
    /// the heap, [`Replacement::key`] `None`) is found by data hash alone: only a line
    /// with no cache key can name it.
    #[must_use]
    pub fn find(&self, texture: &Stored) -> Option<Arc<Image>> {
        let image = self.lookup(&texture.replacement()?)?;
        self.replaced.fetch_add(1, Ordering::Relaxed);
        Some(image)
    }

    fn lookup(&self, wanted: &Replacement) -> Option<Arc<Image>> {
        let mut tries = Vec::with_capacity(3);
        if let Some(key) = wanted.key {
            tries.push((key.dim_clut, wanted.data));
            tries.push((key.dim_clut, 0));
        }
        tries.push((0, wanted.data));
        // PPSSPP stops at the first line that matches; a blank one is "leave it alone".
        let path = tries.iter().find_map(|key| self.table.get(key))?;
        if path.is_empty() {
            return None;
        }
        self.image(path)
    }

    fn image(&self, path: &str) -> Option<Arc<Image>> {
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(hit) = cache.images.get(path) {
            return hit.clone();
        }
        let image = self
            .files
            .read(path)
            .and_then(|bytes| decode_png(&bytes))
            .map(Arc::new);
        if let Some(image) = &image {
            if cache.bytes + image.rgba.len() > CACHE_CAP {
                cache.images.clear();
                cache.bytes = 0;
            }
            cache.bytes += image.rgba.len();
        }
        cache.images.insert(path.to_string(), image.clone());
        image
    }
}

/// A PNG as RGBA8888. `None` for anything the decoder refuses.
fn decode_png(bytes: &[u8]) -> Option<Image> {
    let mut decoder = ::png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(::png::Transformations::EXPAND | ::png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()?];
    let frame = reader.next_frame(&mut buffer).ok()?;
    let data = &buffer[..frame.buffer_size()];
    let rgba = match frame.color_type {
        ::png::ColorType::Rgba => data.to_vec(),
        ::png::ColorType::Rgb => data
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        ::png::ColorType::GrayscaleAlpha => data
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        ::png::ColorType::Grayscale => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        ::png::ColorType::Indexed => return None,
    };
    (frame.width > 0 && frame.height > 0).then_some(Image {
        width: frame.width,
        height: frame.height,
        rgba,
    })
}

static ACTIVE: RwLock<Option<Arc<Replacements>>> = RwLock::new(None);

/// Makes `pack` the one every decoder on the load path asks, or none.
pub fn install(pack: Option<Arc<Replacements>>) {
    *ACTIVE
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = pack;
}

/// The active pack, if any.
#[must_use]
pub fn active() -> Option<Arc<Replacements>> {
    ACTIVE
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// The active pack's picture for a standalone `.mip` blob, `None` when no pack is on, the
/// blob is not a `.mip`, or the pack has nothing for it. A picture drawn whole (UVs are
/// fractions of it), never an atlas cut by pixel offsets: those stay the disc's own.
#[must_use]
pub fn mip(blob: &[u8]) -> Option<Arc<Image>> {
    let pack = active()?;
    pack.find(&Stored::from_mip(blob).ok()?)
}

/// [`Replacements::find`] on the active pack.
#[must_use]
pub fn find(texture: &Stored) -> Option<Arc<Image>> {
    active()?.find(texture)
}

#[cfg(test)]
mod tests;
