//! Uploading a texture the moment it is decoded, so the CPU never holds a
//! circuit's worth of them at once.
//!
//! **The peak this removes.** The race load decodes every texture of every
//! model before the scene builds any `Drawable`, so a model kept its decoded
//! texels until its upload - Tech De Ra's peak was all 2.3 GiB of track BC7
//! blocks, 0.5 GiB of craft liveries and 0.13 GiB of sky on the CPU together,
//! with `Model::release_texels` freeing them only afterwards. With a [`Scope`]
//! open, a texture decoded by an `.rcsmodel` build goes to the GPU the moment
//! it exists and is replaced by [`Texels::Uploaded`], which holds the view and
//! no texels.
//!
//! **A thread-local scope, like [`super::pipeline_cache::Scope`].** The decode
//! sits behind about fifteen `Textures<'_>` callbacks, none of which takes a
//! device, and the load runs on the `race-load` thread while the scene is built
//! on `race-build`, so [`super::pipeline_cache`]'s own cache - opened inside
//! `Scene::new` - cannot carry a view from one to the other. The view travels in
//! the texture instead.
//!
//! **Open it around a load whose scene will be built from the same device**,
//! and nowhere else: a texture decoded under a scope is bound to that device and
//! has no texels to fall back on.

use std::cell::RefCell;

use crate::mesh::{ModelTexture, Texels};

use super::texture;

/// Bytes uploaded between flushes of the queue's staging.
///
/// `Queue::write_texture` copies into a staging buffer that lives until the
/// next submit, so without a flush the bytes this exists to keep off the CPU
/// would pile up in the driver's staging instead. Chosen, not measured.
const FLUSH_EVERY: u64 = 64 * 1024 * 1024;

struct Sink {
    device: wgpu::Device,
    queue: wgpu::Queue,
    blocks: bool,
    since_flush: u64,
    stats: Stats,
}

/// What a scope has uploaded so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// Textures uploaded.
    pub textures: usize,
    /// Of those, kept as the disc's own blocks.
    pub block_compressed: usize,
    /// Bytes the uploads occupy on the GPU, mip chains included.
    pub gpu_bytes: u64,
    /// Bytes of decoded texels that were never held past their upload.
    pub cpu_bytes_streamed: u64,
}

thread_local! {
    static SINK: RefCell<Option<Sink>> = const { RefCell::new(None) };
}

/// Opens the sink for the current thread until dropped.
///
/// **Scopes do not nest**: the sink is one thread-local slot, so a second scope
/// replaces the first and dropping either empties it.
#[derive(Debug)]
pub struct Scope {
    _private: (),
}

impl Scope {
    /// Textures decoded on this thread from now on are uploaded through
    /// `device` and `queue`.
    #[must_use]
    pub fn open(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let blocks = device
            .features()
            .contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
        SINK.with(|cell| {
            *cell.borrow_mut() = Some(Sink {
                device: device.clone(),
                queue: queue.clone(),
                blocks,
                since_flush: 0,
                stats: Stats::default(),
            });
        });
        Self { _private: () }
    }

    /// What has gone up under this scope so far.
    #[must_use]
    pub fn stats(&self) -> Stats {
        SINK.with(|cell| {
            cell.borrow()
                .as_ref()
                .map(|sink| sink.stats)
                .unwrap_or_default()
        })
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        SINK.with(|cell| *cell.borrow_mut() = None);
    }
}

/// Uploads `texture` if a [`Scope`] is open on this thread, and returns it with
/// its texels replaced by the view; otherwise returns it unchanged.
///
/// A texture already [`Texels::Uploaded`] passes through.
pub(crate) fn offer(texture: ModelTexture) -> ModelTexture {
    SINK.with(|cell| {
        let mut slot = cell.borrow_mut();
        let Some(sink) = slot.as_mut() else {
            return texture;
        };
        if matches!(texture.texels, Texels::Uploaded { .. }) {
            return texture;
        }
        let cpu_bytes = texture.cpu_bytes();
        let block_compressed = matches!(texture.texels, Texels::Blocks { .. }) && sink.blocks;
        // A texture no level of which fits the device is handed back as it came,
        // texels and all: the build's own upload refuses it again and the loader
        // line names it. Counted in none of the stats.
        let Some(texture::Placed {
            view,
            gpu_bytes,
            dropped_levels,
        }) = texture::upload(&sink.device, &sink.queue, &texture, sink.blocks)
        else {
            return texture;
        };
        sink.stats.textures += 1;
        sink.stats.block_compressed += usize::from(block_compressed);
        sink.stats.gpu_bytes += gpu_bytes;
        sink.stats.cpu_bytes_streamed += cpu_bytes;
        sink.since_flush += gpu_bytes;
        if sink.since_flush >= FLUSH_EVERY {
            sink.since_flush = 0;
            sink.queue.submit(std::iter::empty());
            // A poll that does not wait: it lets the driver retire the
            // staging the submit just released.
            let _ = sink.device.poll(wgpu::PollType::Poll);
        }
        ModelTexture {
            texels: Texels::Uploaded {
                view,
                gpu_bytes,
                block_compressed,
                dropped_levels,
            },
            ..texture
        }
    })
}

#[cfg(test)]
mod tests;
