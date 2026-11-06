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
//!
//! **One submission.** [`Log::encode`] gives the same order without a submission per view: the
//! log's bytes go up as one write into an [`UploadBuffer`], and its writes become copies out of
//! that buffer in a command buffer of their own, which the caller places just ahead of the
//! commands that read them. It has to be a command buffer of its own: a view's encoder already
//! holds passes recorded while its writes were still being made, and every one of those passes
//! reads the view's last write, as it did from the queue.

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

/// Where every [`Log::encode`] of a frame puts its bytes: a buffer that lives as long as the
/// scene and grows to what a frame logs, so a frame allocates one staging buffer per log and
/// not one per write. Reusing it is ordered: the next frame's write into it lands after this
/// frame's copies out of it.
#[derive(Debug, Default)]
pub struct UploadBuffer {
    buffer: Option<wgpu::Buffer>,
    used: wgpu::BufferAddress,
}

impl UploadBuffer {
    /// Starts a frame's logs from the start of the buffer.
    pub fn rewind(&mut self) {
        self.used = 0;
    }

    /// `bytes` written behind what this frame has put here: the buffer, and where they start.
    /// A log that outgrows the buffer gets a larger one; the old one lives on in the commands
    /// that copy from it.
    fn place(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
    ) -> (wgpu::Buffer, wgpu::BufferAddress) {
        let size = bytes.len() as wgpu::BufferAddress;
        let start = self.used.next_multiple_of(UPLOAD_ALIGNMENT);
        let buffer = match &self.buffer {
            Some(buffer) if start + size <= buffer.size() => buffer.clone(),
            _ => {
                self.used = 0;
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("frame uploads"),
                    size: (2 * size).max(MIN_UPLOAD_SIZE),
                    usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.buffer = Some(buffer.clone());
                buffer
            }
        };
        let start = self.used.next_multiple_of(UPLOAD_ALIGNMENT);
        queue.write_buffer(&buffer, start, bytes);
        self.used = start + size;
        (buffer, start)
    }
}

/// Enough for a copy to a texture of any format: its offset must be a whole number of blocks,
/// and no format's block is larger.
const UPLOAD_ALIGNMENT: wgpu::BufferAddress = 16;

/// A frame of Pure's or HD's writes (`docs/rendering/split-screen.md`, "One submission"), so a
/// race does not grow the buffer on its first frames.
const MIN_UPLOAD_SIZE: wgpu::BufferAddress = 4 << 20;

impl Log {
    fn push(&mut self, target: Target, data: &[u8]) {
        let start = self.bytes.len();
        self.bytes.extend_from_slice(data);
        self.writes.push((target, start..self.bytes.len()));
    }

    /// Keeps a texture write with its rows at the spacing a copy out of a buffer asks for,
    /// which a queue write does not: every row aligned to
    /// [`wgpu::COPY_BYTES_PER_ROW_ALIGNMENT`], and the first to a whole block.
    fn push_texture(
        &mut self,
        texture: wgpu::TexelCopyTextureInfo<'_>,
        data: &[u8],
        layout: wgpu::TexelCopyBufferLayout,
        size: wgpu::Extent3d,
    ) {
        let format = texture.texture.format();
        let (block_width, block_height) = format.block_dimensions();
        let block = format
            .block_copy_size(Some(texture.aspect))
            .expect("a written aspect has one block size");
        let row = size.width.div_ceil(block_width) * block;
        let rows = size.height.div_ceil(block_height);
        let padded = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let stride = layout.bytes_per_row.unwrap_or(row) as usize;
        let rows_per_image = layout.rows_per_image.unwrap_or(rows) as usize;
        let aligned = self.bytes.len().next_multiple_of(UPLOAD_ALIGNMENT as usize);
        self.bytes.resize(aligned, 0);
        let start = self.bytes.len();
        for layer in 0..size.depth_or_array_layers as usize {
            for line in 0..rows as usize {
                let from = layout.offset as usize + (layer * rows_per_image + line) * stride;
                self.bytes
                    .extend_from_slice(&data[from..from + row as usize]);
                self.bytes
                    .resize(self.bytes.len() + (padded - row) as usize, 0);
            }
        }
        let end = self.bytes.len();
        self.bytes.resize(end.next_multiple_of(4), 0);
        self.writes.push((
            Target::Texture {
                texture: texture.texture.clone(),
                mip_level: texture.mip_level,
                origin: texture.origin,
                aspect: texture.aspect,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(rows),
                },
                size,
            },
            start..end,
        ));
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

    /// Every kept write as a copy out of `upload`, in the order they were made, in an encoder
    /// the caller places ahead of the commands that read them, and the log emptied. `None` for
    /// a log that holds nothing.
    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        upload: &mut UploadBuffer,
    ) -> Option<wgpu::CommandEncoder> {
        if self.bytes.is_empty() {
            self.writes.clear();
            return None;
        }
        let (source, base) = upload.place(device, queue, &self.bytes);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frame uploads"),
        });
        for (target, range) in self.writes.drain(..) {
            let start = base + range.start as wgpu::BufferAddress;
            match target {
                Target::Buffer { buffer, offset } if !range.is_empty() => encoder
                    .copy_buffer_to_buffer(
                        &source,
                        start,
                        &buffer,
                        offset,
                        range.len() as wgpu::BufferAddress,
                    ),
                Target::Buffer { .. } => {}
                Target::Texture {
                    texture,
                    mip_level,
                    origin,
                    aspect,
                    layout,
                    size,
                } => encoder.copy_buffer_to_texture(
                    wgpu::TexelCopyBufferInfo {
                        buffer: &source,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: start,
                            ..layout
                        },
                    },
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level,
                        origin,
                        aspect,
                    },
                    size,
                ),
            }
        }
        self.bytes.clear();
        Some(encoder)
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
        self.borrow_mut()
            .push_texture(texture, data, data_layout, size);
    }
}

#[cfg(test)]
mod tests;
