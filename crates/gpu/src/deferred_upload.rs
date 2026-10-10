//! Big uploads that wait for a frame, so the page's thread is not held for all
//! of them at once.
//!
//! **Why.** A browser's `queue.writeBuffer`/`writeTexture` copies at about
//! 88 MB/s (Chromium 153 under load, measured on the HD race build), and the
//! scene build of an HD circuit writes 140 MiB of vertices and indices and a
//! few hundred MiB of textures. On the web that build runs on the page's
//! thread (wgpu's web types are not `Send`), so those writes were one frozen
//! stretch of 0.7 to 1.5 s at the end of the load. With a [`Scope`] open,
//! [`write_buffer`] and [`defer_texture`] park a write instead and
//! [`drain`] performs it a chunk at a time, one frame's worth per call, while
//! the loading screen keeps drawing.
//!
//! **A thread-local scope, like `oag_mesh`'s `texture_sink` and
//! `pipeline_cache`.** The writes sit deep inside constructors that take a
//! device and a queue and nothing else. The scope is opened on the web only:
//! native builds on a thread of its own and keeps the immediate path, so a
//! write there is exactly the `queue.write_*` it always was.
//!
//! **Every write parks, small or large**: a browser's `writeBuffer` costs about
//! 0.13 ms a call whatever its size, and a pool of 128 weapon instances makes
//! three per instance, pool after pool, which was 500 ms of the stall. **What
//! waits holds its source, not a copy.** A parked write keeps an
//! `Arc` of whatever owns the bytes ([`Bytes`]): the model for its vertices,
//! the texture for its levels. The module's memory never shrinks, so a second
//! 140 MiB copy would be a permanent high-water mark.
//!
//! **The contract.** Nothing may read a parked write's target before it has
//! drained: [`pending_bytes`] is zero once every write has landed, and a caller
//! that draws or submits first draws zeros.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::Arc;

/// The most one [`drain`] step writes. A step is the unit the time budget is
/// checked at, so this is the longest a single step can hold the thread: about
/// 3 ms at 88 MB/s. Chosen, not measured.
pub const CHUNK: usize = 256 * 1024;

/// Whatever owns the bytes of a parked write.
pub trait Bytes {
    /// The whole payload.
    fn bytes(&self) -> &[u8];
}

impl<T: bytemuck::Pod> Bytes for Vec<T> {
    fn bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self)
    }
}

/// One level of a texture, as the rows of a copy: `rows` of `bytes_per_row`
/// bytes, each row `block_height` texels tall (1, or 4 for a BC format).
#[derive(Clone, Copy, Debug)]
pub struct Level {
    /// Mip level the rows belong to.
    pub mip: u32,
    /// Texels across, block-aligned for a compressed format.
    pub width: u32,
    /// Bytes of one row of blocks (or of texels).
    pub bytes_per_row: u32,
    /// Rows of blocks (or of texels) in the level.
    pub rows: u32,
    /// Texels one row spans vertically.
    pub block_height: u32,
}

enum Target {
    Buffer {
        buffer: wgpu::Buffer,
    },
    Texture {
        texture: wgpu::Texture,
        level: Level,
    },
}

struct Job {
    target: Target,
    source: Arc<dyn Bytes>,
    /// First byte of this job's payload within the source.
    base: usize,
    /// Payload length.
    len: usize,
    /// Bytes already written.
    done: usize,
}

struct State {
    queue: wgpu::Queue,
    jobs: VecDeque<Job>,
    pending: u64,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Opens the deferral for the current thread until dropped; dropping it
/// forgets every write that has not landed.
///
/// **Scopes do not nest**: the state is one thread-local slot.
#[derive(Debug)]
pub struct Scope {
    _private: (),
}

impl Scope {
    /// Large writes made on this thread from now on wait for [`drain`].
    #[must_use]
    pub fn open(queue: &wgpu::Queue) -> Self {
        STATE.with(|state| {
            *state.borrow_mut() = Some(State {
                queue: queue.clone(),
                jobs: VecDeque::new(),
                pending: 0,
            });
        });
        Self { _private: () }
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        STATE.with(|state| *state.borrow_mut() = None);
    }
}

/// Whether a scope is open on this thread.
#[must_use]
pub fn active() -> bool {
    STATE.with(|state| state.borrow().is_some())
}

/// Bytes still waiting on this thread.
#[must_use]
pub fn pending_bytes() -> u64 {
    STATE.with(|state| state.borrow().as_ref().map_or(0, |s| s.pending))
}

/// `queue.write_buffer(buffer, 0, contents)`, or - with a scope open - parked
/// for [`drain`], the `Vec` kept as it is.
pub fn write_buffer<T: bytemuck::Pod>(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    contents: Vec<T>,
) {
    let len = std::mem::size_of_val(&contents[..]);
    if active() {
        defer_buffer(buffer, 0, Arc::new(contents), 0..len);
    } else {
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(&contents));
    }
}

/// [`write_buffer`] for one small value. **Parked whatever its size, because
/// the cost of a browser write is the call**: a browser's
/// `writeBuffer` costs about 0.13 ms a call (Chromium 153) whether it carries
/// 64 bytes or 64 KiB,
pub fn write_value<T: bytemuck::Pod>(queue: &wgpu::Queue, buffer: &wgpu::Buffer, value: &T) {
    if active() {
        defer_buffer(
            buffer,
            0,
            Arc::new(vec![*value]),
            0..std::mem::size_of::<T>(),
        );
    } else {
        queue.write_buffer(buffer, 0, bytemuck::bytes_of(value));
    }
}

/// Parks `source`'s `range` as the contents of `buffer` at `offset`.
///
/// Call only with a scope open ([`active`]).
pub fn defer_buffer(
    buffer: &wgpu::Buffer,
    offset: u64,
    source: Arc<dyn Bytes>,
    range: std::ops::Range<usize>,
) {
    debug_assert!(offset == 0, "a parked buffer write starts at its origin");
    push(Job {
        target: Target::Buffer {
            buffer: buffer.clone(),
        },
        source,
        base: range.start,
        len: range.len(),
        done: 0,
    });
}

/// Parks `source`'s `range` as the rows of `level` of `texture`.
///
/// Call only with a scope open ([`active`]).
pub fn defer_texture(
    texture: &wgpu::Texture,
    level: Level,
    source: Arc<dyn Bytes>,
    range: std::ops::Range<usize>,
) {
    push(Job {
        target: Target::Texture {
            texture: texture.clone(),
            level,
        },
        source,
        base: range.start,
        len: range.len(),
        done: 0,
    });
}

fn push(job: Job) {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let state = state.as_mut().expect("a write is parked only in a scope");
        state.pending += job.len as u64;
        state.jobs.push_back(job);
    });
}

/// Writes parked bytes a chunk at a time for as long as `keep_going` says yes,
/// asked after every chunk, and at least one chunk if any is waiting. Returns
/// the bytes still waiting.
///
/// The order is the order the writes were parked, so a caller that asks for
/// the head of a queue gets it first.
pub fn drain(mut keep_going: impl FnMut() -> bool) -> u64 {
    loop {
        let more = STATE.with(|state| {
            let mut state = state.borrow_mut();
            let state = state.as_mut()?;
            let job = state.jobs.front_mut()?;
            let written = step(&state.queue, job);
            job.done += written;
            state.pending -= written as u64;
            if job.done >= job.len {
                state.jobs.pop_front();
            }
            Some(())
        });
        if more.is_none() || !keep_going() {
            return pending_bytes();
        }
    }
}

/// Writes the next chunk of `job` and returns its length.
fn step(queue: &wgpu::Queue, job: &Job) -> usize {
    let payload = &job.source.bytes()[job.base..job.base + job.len];
    match &job.target {
        Target::Buffer { buffer } => {
            let end = (job.done + CHUNK).min(job.len);
            queue.write_buffer(buffer, job.done as u64, &payload[job.done..end]);
            end - job.done
        }
        Target::Texture { texture, level } => {
            // Whole rows only, and whole blocks of rows: a band starts and ends
            // on a block boundary or a compressed copy is refused.
            let row = level.bytes_per_row as usize;
            let rows_per_chunk = (CHUNK / row.max(1)).max(1) as u32;
            let first = (job.done / row) as u32;
            let rows = rows_per_chunk.min(level.rows - first);
            let from = first as usize * row;
            let to = from + rows as usize * row;
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: level.mip,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: first * level.block_height,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &payload[from..to],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(level.bytes_per_row),
                    rows_per_image: Some(rows),
                },
                wgpu::Extent3d {
                    width: level.width,
                    height: rows * level.block_height,
                    depth_or_array_layers: 1,
                },
            );
            to - from
        }
    }
}

#[cfg(test)]
mod tests;
