//! The HUD of an accumulated photo mode shot: drawn once, over the average of the frames, at the
//! tick the shot is taken at.
//!
//! Every frame of the shot is drawn without the HUD, because a HUD drawn into each is averaged
//! over the shutter (ghosted digits, a thrust gauge that reads the recorded input). The average
//! is copied into the presentation target by the frame that follows the last, which then draws
//! the HUD over it as it draws it for a single shot, and that frame is read back.

use super::Session;

/// The average of a finished shot, until the HUD is over it.
#[derive(Debug)]
pub(crate) struct Averaged {
    pub(super) pixels: Vec<u8>,
    pub(super) size: (u32, u32),
    pub(super) path: std::path::PathBuf,
    pub(super) pending: Option<super::photo_meta::Pending>,
    pub(super) frames: u32,
    pub(super) started: web_time::Instant,
    /// The frame in flight carries the average under its HUD.
    pub(super) under_hud: bool,
}

/// Rows of a buffer to texture copy start on this many bytes.
const ROW_ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

impl Session {
    /// Whether a shot still has a frame to draw: a frame of it, the one that puts the HUD over
    /// its average, or the one that shows its bar before it begins.
    pub(crate) fn photo_wants_frames(&self) -> bool {
        self.photo_job_running()
            || self
                .photo
                .as_ref()
                .is_some_and(|open| open.averaged.is_some() || open.announced.is_some())
    }

    /// Whether the frame being drawn leaves the HUD out: a shot of several frames is under way.
    pub(super) fn photo_hides_hud(&self) -> bool {
        self.photo_job_running()
    }

    /// Records the copy of the finished average into the presentation target, after the scene is
    /// resolved into it and before the HUD goes on. Does nothing with no average waiting.
    pub(super) fn photo_encode_average(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let Some(averaged) = self.photo.as_mut().and_then(|open| open.averaged.as_mut()) else {
            return;
        };
        let (width, height) = averaged.size;
        if self.framebuffer.output_size() != averaged.size {
            return;
        }
        let texture = self.framebuffer.output_texture();
        let swap = matches!(
            texture.format(),
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        let row = width * 4;
        let padded = row.div_ceil(ROW_ALIGN) * ROW_ALIGN;
        let mut padded_pixels = vec![0u8; padded as usize * height as usize];
        for (line, out) in averaged
            .pixels
            .chunks_exact(row as usize)
            .zip(padded_pixels.chunks_exact_mut(padded as usize))
        {
            out[..row as usize].copy_from_slice(line);
            if swap {
                for pixel in out[..row as usize].as_chunks_mut::<4>().0 {
                    pixel.swap(0, 2);
                }
            }
        }
        let buffer = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("photo shot average"),
            size: padded_pixels.len() as u64,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu.queue.write_buffer(&buffer, 0, &padded_pixels);
        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            texture.as_image_copy(),
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        averaged.under_hud = true;
    }

    /// After the frame is submitted: writes the shot, from the frame the HUD went onto, or from
    /// the average itself when the frame could not carry it.
    pub(super) fn photo_finish_averaged(&mut self) {
        let Some(mut averaged) = self.photo.as_mut().and_then(|open| open.averaged.take()) else {
            return;
        };
        if averaged.under_hud {
            match self.read_photo_output() {
                Ok((pixels, ..)) => averaged.pixels = pixels,
                Err(why) => log::warn!("photo mode: cannot read the shot with its HUD: {why:#}"),
            }
        }
        self.photo_write_shot(averaged);
    }
}
