//! The opt-in ASTC re-encode of block-compressed textures, for a device that
//! cannot sample BC.
//!
//! **What it replaces.** On a device without `TEXTURE_COMPRESSION_BC` (a phone's
//! Mali or Adreno) [`super::texture::upload`] decodes every BC texture the disc
//! ships back to RGBA8: four to eight times the memory of the blocks, about
//! 1.1 GB of an HD race on a Galaxy S21. With the setting on and a device that
//! samples ASTC LDR, the same decoded picture and box-filtered mip chain go up as
//! ASTC 4x4 blocks instead (`docs/rendering/texture-compression.md`).
//!
//! **Lossy, so opt-in.** ASTC is a re-encode of this project's own RGBA output,
//! not a format any disc authors; the default (`original`) never reaches this
//! module and the upload is the RGBA8 it always was.
//!
//! **Encoded once, off the render thread, into an on-disk cache.** A load that
//! finds a texture's entry uploads it; one that does not uploads RGBA8 as before
//! and queues the texture for the one background encoder thread, so the cache
//! fills while the player races and the next load of that circuit finds it. An
//! entry is keyed by the source blocks and every encoder setting
//! ([`Key::of`]), written whole or not at all (`oag_io::write_bytes`), and is
//! always safe to delete.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use sha2::Digest;

use crate::mesh::BlockFormat;

/// The ASTC block footprint every entry is encoded at.
///
/// 4x4 (8 bits a texel) by constraint, not taste: a compressed texture's base
/// must be a whole number of blocks, and 6x6 (3.56 bits) divides no
/// power-of-two side, which is every side the discs ship.
pub const BLOCK: u32 = 4;

/// What encodes a level: one level of RGBA8 texels (`width` by `height`) to ASTC 4x4
/// blocks, [`level_len`] bytes. Injected by the caller ([`configure`]), so this crate links no
/// encoder: `oag_game` brings Arm's astcenc, which is C++, and nothing that links this crate
/// without the game (the server) has to carry it.
#[derive(Debug, Clone, Copy)]
pub struct Encoder {
    /// The encoder build and preset, part of every key: another encode is another entry.
    pub name: &'static str,
    /// The preset on the encoder's own scale, also part of every key.
    pub quality: f32,
    /// One level, or `None` if it would not encode.
    pub encode: fn(u32, u32, &[u8]) -> Option<Vec<u8>>,
}

/// Bumped when an entry's layout or what it was encoded from changes, so an old
/// entry is never read as a new one.
const FORMAT_VERSION: u32 = 1;

/// What an entry starts with.
const MAGIC: &[u8; 8] = b"OAGASTC\0";

/// The most source bytes waiting for the encoder at once; a texture past it is
/// not queued this load and is offered again the next. Chosen, not measured: it
/// bounds the CPU memory a cold cache costs during a race.
const QUEUE_BYTES: u64 = 128 * 1024 * 1024;

/// The wgpu format an entry uploads as: linear, like the RGBA8 it replaces
/// (ADR-0020: every stage works on the stored bytes).
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Astc {
    block: wgpu::AstcBlock::B4x4,
    channel: wgpu::AstcChannel::Unorm,
};

/// Whether a device with `features` takes the re-encode: it cannot sample BC and
/// can sample ASTC LDR. A device with BC keeps the disc's own blocks, so the
/// setting is inert there.
#[must_use]
pub fn takes_astc(features: wgpu::Features) -> bool {
    !features.contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
        && features.contains(wgpu::Features::TEXTURE_COMPRESSION_ASTC)
}

/// What the last adapter a device was asked of said: 0 not yet asked, 1 no, 2 yes.
static OFFERED: AtomicU8 = AtomicU8::new(0);

/// Records whether an adapter offers the re-encode, from
/// [`super::device::device_descriptor`].
pub(super) fn note_adapter(features: wgpu::Features) {
    OFFERED.store(if takes_astc(features) { 2 } else { 1 }, Ordering::Relaxed);
}

/// Whether this process's device takes the re-encode: `None` before any device
/// was asked for. The menu keeps the row while this is unknown and drops it once
/// it reads `Some(false)`.
#[must_use]
pub fn offered() -> Option<bool> {
    match OFFERED.load(Ordering::Relaxed) {
        1 => Some(false),
        2 => Some(true),
        _ => None,
    }
}

/// The cache the setting is on with, `None` while it is `original`.
static ACTIVE: Mutex<Option<Arc<Cache>>> = Mutex::new(None);

/// Turns the re-encode on with its cache in `dir`, encoding with `encoder`, or off with `None`.
/// Read by the next texture upload, so a change lands on the next race load.
pub fn configure(on: Option<(PathBuf, Encoder)>) {
    let mut active = ACTIVE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match (on, active.as_ref()) {
        (Some((dir, encoder)), Some(cache))
            if cache.dir == dir && cache.encoder.name == encoder.name => {}
        (Some((dir, encoder)), _) => *active = Some(Arc::new(Cache::new(dir, encoder))),
        (None, _) => *active = None,
    }
}

/// The cache to upload through on `device`, when the setting is on and the
/// device takes the re-encode.
pub(super) fn active(device: &wgpu::Device) -> Option<Arc<Cache>> {
    if !takes_astc(device.features()) {
        return None;
    }
    ACTIVE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// What this process's uploads have done, for the loader report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    /// Textures uploaded as ASTC from the cache.
    pub hits: usize,
    /// Of those, the bytes they occupy on the GPU, mip chains included.
    pub hit_bytes: u64,
    /// Textures uploaded as RGBA8 because their entry was not there yet.
    pub misses: usize,
    /// Textures the encoder thread has written to the cache.
    pub encoded: usize,
}

static HITS: AtomicUsize = AtomicUsize::new(0);
static HIT_BYTES: AtomicU64 = AtomicU64::new(0);
static MISSES: AtomicUsize = AtomicUsize::new(0);
static ENCODED: AtomicUsize = AtomicUsize::new(0);

/// The running [`Tally`], and zeroes the hit and miss counts so the next load
/// reports its own. The encoder's count runs on.
pub fn take_tally() -> Tally {
    Tally {
        hits: HITS.swap(0, Ordering::Relaxed),
        hit_bytes: HIT_BYTES.swap(0, Ordering::Relaxed),
        misses: MISSES.swap(0, Ordering::Relaxed),
        encoded: ENCODED.load(Ordering::Relaxed),
    }
}

/// An entry's name: SHA-256 over the source and every setting that shapes the
/// encode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key([u8; 32]);

/// Every input [`Key::of`] hashes, so a test can move one at a time.
#[derive(Debug, Clone, Copy)]
pub struct KeyInputs<'a> {
    /// The disc's block format of `source`.
    pub format: BlockFormat,
    /// The source level's blocks, the ones decoded and encoded.
    pub source: &'a [u8],
    /// That level's size in texels.
    pub width: u32,
    pub height: u32,
    /// How many mip levels the entry holds.
    pub levels: usize,
    /// The ASTC footprint ([`BLOCK`]).
    pub block: u32,
    /// The encoder's [`Encoder::name`] and [`Encoder::quality`].
    pub encoder: &'a str,
    pub quality: f32,
}

impl Key {
    /// The key of an encode of `inputs`.
    #[must_use]
    pub fn of(inputs: &KeyInputs<'_>) -> Self {
        let mut hash = sha2::Sha256::new();
        hash.update(MAGIC);
        hash.update(FORMAT_VERSION.to_le_bytes());
        hash.update(inputs.encoder.as_bytes());
        hash.update([0]);
        hash.update(inputs.block.to_le_bytes());
        hash.update(inputs.quality.to_bits().to_le_bytes());
        hash.update([format_tag(inputs.format)]);
        hash.update(inputs.width.to_le_bytes());
        hash.update(inputs.height.to_le_bytes());
        hash.update((inputs.levels as u64).to_le_bytes());
        hash.update(inputs.source);
        Self(hash.finalize().into())
    }

    /// The entry's file name.
    #[must_use]
    pub fn file_name(&self) -> String {
        let mut name = String::with_capacity(70);
        for byte in self.0 {
            name.push_str(&format!("{byte:02x}"));
        }
        name.push_str(".astc");
        name
    }
}

fn format_tag(format: BlockFormat) -> u8 {
    match format {
        BlockFormat::Bc1 => 1,
        BlockFormat::Bc2 => 2,
        BlockFormat::Bc3 => 3,
        BlockFormat::Bc7 => 7,
        BlockFormat::R16Float => 16,
    }
}

/// The bytes of one ASTC level `width` by `height` texels: one 16-byte block per
/// started 4x4 tile.
#[must_use]
pub fn level_len(width: u32, height: u32) -> usize {
    (width.div_ceil(BLOCK) * height.div_ceil(BLOCK) * 16) as usize
}

/// The size of every level of a chain from `width` by `height`, `levels` long.
#[must_use]
pub fn level_dims(width: u32, height: u32, levels: usize) -> Vec<(u32, u32)> {
    (0..levels)
        .map(|level| {
            let shift = level.min(31) as u32;
            ((width >> shift).max(1), (height >> shift).max(1))
        })
        .collect()
}

/// A texture the encoder thread is to encode and write.
struct Job {
    key: Key,
    format: BlockFormat,
    source: Vec<u8>,
    width: u32,
    height: u32,
    levels: usize,
}

/// The on-disk entries and the thread that fills them.
#[derive(Debug)]
pub struct Cache {
    dir: PathBuf,
    encoder: Encoder,
    queue: Mutex<Queue>,
}

#[derive(Debug)]
struct Queue {
    /// Keys queued or being encoded, so a texture two models share, or a second
    /// load before the first finished, is encoded once.
    pending: HashSet<Key>,
    bytes: u64,
    sender: Option<Sender<Job>>,
}

impl Cache {
    fn new(dir: PathBuf, encoder: Encoder) -> Self {
        Self {
            dir,
            encoder,
            queue: Mutex::new(Queue {
                pending: HashSet::new(),
                bytes: 0,
                sender: None,
            }),
        }
    }

    /// The directory the entries live in.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The levels of `key`'s entry, each checked against the size `dims` says it
    /// has; `None` for no entry or one that does not match.
    #[must_use]
    pub fn load(&self, key: &Key, dims: &[(u32, u32)]) -> Option<Vec<Vec<u8>>> {
        let bytes = std::fs::read(self.dir.join(key.file_name())).ok()?;
        parse_entry(&bytes, dims)
    }

    /// Queues `source` for the encoder thread, unless it is already queued or the
    /// queue is full. The thread starts on the first request.
    fn request(self: &Arc<Self>, job: Job) {
        let mut queue = self
            .queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let size = job.source.len() as u64;
        if queue.pending.contains(&job.key) || queue.bytes + size > QUEUE_BYTES {
            return;
        }
        if queue.sender.is_none() {
            let (sender, receiver) = std::sync::mpsc::channel();
            let cache = Arc::clone(self);
            let started = std::thread::Builder::new()
                .name("astc-encode".into())
                .spawn(move || cache.run(&receiver));
            if let Err(why) = started {
                log::warn!("astc: could not start the encoder thread: {why}");
                return;
            }
            queue.sender = Some(sender);
        }
        let key = job.key;
        if queue
            .sender
            .as_ref()
            .is_some_and(|sender| sender.send(job).is_ok())
        {
            queue.pending.insert(key);
            queue.bytes += size;
        }
    }

    fn run(&self, jobs: &Receiver<Job>) {
        if let Err(why) = std::fs::create_dir_all(&self.dir) {
            log::warn!("astc: cache {} not writable: {why}", self.dir.display());
        }
        let mut done = 0usize;
        let mut started = std::time::Instant::now();
        for job in jobs {
            if done == 0 {
                started = std::time::Instant::now();
            }
            let key = job.key;
            let size = job.source.len() as u64;
            match encode_job(&job, self.encoder.encode) {
                Some(entry) => {
                    let path = self.dir.join(key.file_name());
                    if let Err(why) = oag_io::write_bytes(&path, &entry) {
                        log::warn!("astc: writing {}: {why}", path.display());
                    } else {
                        done += 1;
                        ENCODED.fetch_add(1, Ordering::Relaxed);
                    }
                }
                None => log::debug!(
                    "astc: a {}x{} texture did not encode",
                    job.width,
                    job.height
                ),
            }
            let mut queue = self
                .queue
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            queue.pending.remove(&key);
            queue.bytes = queue.bytes.saturating_sub(size);
            if queue.pending.is_empty() && done > 0 {
                log::debug!(
                    "astc: encoded {done} texture(s) into {} in {:.1} s",
                    self.dir.display(),
                    started.elapsed().as_secs_f64()
                );
                done = 0;
            }
        }
    }
}

/// The levels an entry holds, if its header and every level's length agree with
/// `dims`.
fn parse_entry(bytes: &[u8], dims: &[(u32, u32)]) -> Option<Vec<Vec<u8>>> {
    let rest = bytes.strip_prefix(MAGIC.as_slice())?;
    let (count, mut rest) = rest.split_first_chunk::<4>()?;
    if u32::from_le_bytes(*count) as usize != dims.len() {
        return None;
    }
    let mut levels = Vec::with_capacity(dims.len());
    for &(width, height) in dims {
        let (len, tail) = rest.split_first_chunk::<4>()?;
        let len = u32::from_le_bytes(*len) as usize;
        if len != level_len(width, height) || tail.len() < len {
            return None;
        }
        let (level, tail) = tail.split_at(len);
        levels.push(level.to_vec());
        rest = tail;
    }
    rest.is_empty().then_some(levels)
}

/// An entry's bytes for `levels`.
fn write_entry(levels: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + levels.iter().map(|l| l.len() + 4).sum::<usize>());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(levels.len() as u32).to_le_bytes());
    for level in levels {
        out.extend_from_slice(&(level.len() as u32).to_le_bytes());
        out.extend_from_slice(level);
    }
    out
}

/// Decodes the job's blocks, builds the same box-filtered chain the RGBA8 upload
/// builds, and encodes every level.
fn encode_job(job: &Job, encode: fn(u32, u32, &[u8]) -> Option<Vec<u8>>) -> Option<Vec<u8>> {
    let rgba: Vec<u8> = job
        .format
        .decode_level(&job.source, job.width, job.height)?
        .into_flattened();
    let mut chain = super::texture::mip_chain(job.width, job.height, &rgba);
    chain.truncate(job.levels);
    let levels = chain
        .iter()
        .map(|(width, height, texels)| {
            encode(*width, *height, texels)
                .filter(|blocks| blocks.len() == level_len(*width, *height))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(write_entry(&levels))
}

/// The view of `levels` (ASTC blocks, base first) as one texture.
fn upload_levels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    dims: &[(u32, u32)],
    levels: &[Vec<u8>],
) -> wgpu::TextureView {
    let (width, height) = dims[0];
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
        texture_binding_view_dimension: None,
    });
    for (mip, ((level_width, level_height), bytes)) in dims.iter().zip(levels).enumerate() {
        // The block-aligned extent, as for BC: a compressed copy is validated in
        // whole blocks against the level's physical size.
        let (blocks_x, blocks_y) = (level_width.div_ceil(BLOCK), level_height.div_ceil(BLOCK));
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: mip as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(blocks_x * 16),
                rows_per_image: Some(blocks_y),
            },
            wgpu::Extent3d {
                width: blocks_x * BLOCK,
                height: blocks_y * BLOCK,
                depth_or_array_layers: 1,
            },
        );
    }
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// The ASTC upload of a BC texture's level `source` (`width` by `height` texels,
/// `levels` mip levels from it), when `cache` holds it; otherwise queues it and
/// returns `None` for the caller's RGBA8 upload.
///
/// `None` without queueing for a base that is not a whole number of blocks:
/// that one stays RGBA8 for good.
#[allow(clippy::too_many_arguments)]
pub(super) fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cache: &Arc<Cache>,
    label: &str,
    format: BlockFormat,
    source: &[u8],
    (width, height): (u32, u32),
    levels: usize,
) -> Option<super::texture::Placed> {
    if width % BLOCK != 0 || height % BLOCK != 0 || levels == 0 {
        return None;
    }
    let key = Key::of(&KeyInputs {
        format,
        source,
        width,
        height,
        levels,
        block: BLOCK,
        encoder: cache.encoder.name,
        quality: cache.encoder.quality,
    });
    let dims = level_dims(width, height, levels);
    let Some(encoded) = cache.load(&key, &dims) else {
        MISSES.fetch_add(1, Ordering::Relaxed);
        cache.request(Job {
            key,
            format,
            source: source.to_vec(),
            width,
            height,
            levels,
        });
        return None;
    };
    let gpu_bytes = encoded.iter().map(|level| level.len() as u64).sum();
    HITS.fetch_add(1, Ordering::Relaxed);
    HIT_BYTES.fetch_add(gpu_bytes, Ordering::Relaxed);
    Some(super::texture::Placed {
        view: upload_levels(device, queue, label, &dims, &encoded),
        gpu_bytes,
        dropped_levels: 0,
    })
}

#[cfg(test)]
mod tests;
