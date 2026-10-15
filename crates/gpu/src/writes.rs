//! Where a frame's buffer and texture writes go: [`Writes`], which `wgpu::Queue` is, and
//! [`Log`], which keeps them for later.
//!
//! **Why a log.** A `queue.write_buffer` lands before the commands of the *next submission*,
//! wherever in the frame it was made. A split screen draws several views into one target and
//! relies on that: each view writes its camera into the same effect buffers and submits before
//! the next view writes. To record the views' commands concurrently, every view's writes are
//! made first, so a later view's must wait: they go into a [`Log`] and are replayed, in the
//! order they were made, just before that view's own submission. What lands before each
//! submission is then exactly what landed when the views were drawn one after another.
//!
//! **Not [`crate::deferred_upload`].** That one spreads a scene build's large writes over
//! frames on the web: a thread-local scope opened at load, holding the source of each write
//! and draining in chunks against a time budget. A frame's log is a value of one view, made
//! and replayed within the frame, and holds a copy of each write's bytes.

use std::cell::RefCell;

/// Something a frame's writes can go to: the queue itself, or a [`Log`] of them. The methods
/// are named and shaped as `wgpu::Queue`'s own, so a body written against the queue compiles
/// unchanged against `&dyn Writes`.
pub trait Writes {
    /// `wgpu::Queue::write_buffer`.
    fn write_buffer(&self, buffer: &wgpu::Buffer, offset: wgpu::BufferAddress, data: &[u8]);

    /// `wgpu::Queue::write_texture`.
    fn write_texture(
        &self,
        texture: wgpu::TexelCopyTextureInfo<'_>,
        data: &[u8],
        data_layout: wgpu::TexelCopyBufferLayout,
        size: wgpu::Extent3d,
    );
}

impl Writes for wgpu::Queue {
    fn write_buffer(&self, buffer: &wgpu::Buffer, offset: wgpu::BufferAddress, data: &[u8]) {
        wgpu::Queue::write_buffer(self, buffer, offset, data);
    }

    fn write_texture(
        &self,
        texture: wgpu::TexelCopyTextureInfo<'_>,
        data: &[u8],
        data_layout: wgpu::TexelCopyBufferLayout,
        size: wgpu::Extent3d,
    ) {
        wgpu::Queue::write_texture(self, texture, data, data_layout, size);
    }
}

enum Target {
    Buffer {
        buffer: wgpu::Buffer,
        offset: wgpu::BufferAddress,
    },
    Texture {
        texture: wgpu::Texture,
        mip_level: u32,
        origin: wgpu::Origin3d,
        aspect: wgpu::TextureAspect,
        layout: wgpu::TexelCopyBufferLayout,
        size: wgpu::Extent3d,
    },
}

/// Writes kept in the order they were made, for [`Log::replay`]. Its memory is kept between
/// frames: [`Log::replay`] empties it without giving the space back.
#[derive(Default)]
pub struct Log {
    writes: Vec<(Target, std::ops::Range<usize>)>,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for Log {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Log")
            .field("writes", &self.writes.len())
            .field("bytes", &self.bytes.len())
            .finish()
    }
}

impl Log {
    fn push(&mut self, target: Target, data: &[u8]) {
        let start = self.bytes.len();
        self.bytes.extend_from_slice(data);
        self.writes.push((target, start..self.bytes.len()));
    }

    /// How many writes wait.
    #[must_use]
    pub fn len(&self) -> usize {
        self.writes.len()
    }

    /// Whether nothing waits.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.writes.is_empty()
    }

    /// Makes every kept write on `queue`, in the order they were made, and empties the log.
    pub fn replay(&mut self, queue: &wgpu::Queue) {
        for (target, range) in self.writes.drain(..) {
            let data = &self.bytes[range];
            match target {
                Target::Buffer { buffer, offset } => queue.write_buffer(&buffer, offset, data),
                Target::Texture {
                    texture,
                    mip_level,
                    origin,
                    aspect,
                    layout,
                    size,
                } => queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level,
                        origin,
                        aspect,
                    },
                    data,
                    layout,
                    size,
                ),
            }
        }
        self.bytes.clear();
    }
}

impl Writes for RefCell<Log> {
    fn write_buffer(&self, buffer: &wgpu::Buffer, offset: wgpu::BufferAddress, data: &[u8]) {
        self.borrow_mut().push(
            Target::Buffer {
                buffer: buffer.clone(),
                offset,
            },
            data,
        );
    }

    fn write_texture(
        &self,
        texture: wgpu::TexelCopyTextureInfo<'_>,
        data: &[u8],
        data_layout: wgpu::TexelCopyBufferLayout,
        size: wgpu::Extent3d,
    ) {
        self.borrow_mut().push(
            Target::Texture {
                texture: texture.texture.clone(),
                mip_level: texture.mip_level,
                origin: texture.origin,
                aspect: texture.aspect,
                layout: data_layout,
                size,
            },
            data,
        );
    }
}

#[cfg(test)]
mod tests;
