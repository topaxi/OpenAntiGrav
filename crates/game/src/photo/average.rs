//! The average of the frames a shot is made of, and its way back into a target: the part of an
//! accumulated shot that photo mode's session and `--from-png` share.
//!
//! The frames are summed in 16 bits a channel (at most 130 frames of 255 fit) and rounded
//! once, so no frame loses its low bits to the ones before it.

/// The channels of the frames so far, summed.
#[derive(Debug, Default)]
pub(crate) struct Sum {
    channels: Vec<u16>,
    size: (u32, u32),
}

impl Sum {
    /// The size of the frames: that of the first taken in.
    pub(crate) fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Takes `pixels` (RGBA, `size`) in, unless the frames so far were another size. Whether it
    /// was taken.
    pub(crate) fn add(&mut self, pixels: &[u8], size: (u32, u32)) -> bool {
        if self.channels.is_empty() {
            self.channels = vec![0; pixels.len()];
            self.size = size;
        }
        if self.size != size || self.channels.len() != pixels.len() {
            return false;
        }
        for (sum, &channel) in self.channels.iter_mut().zip(pixels) {
            *sum += u16::from(channel);
        }
        true
    }

    /// The mean of `frames` frames, rounded to the nearest.
    pub(crate) fn average(&self, frames: u32) -> Vec<u8> {
        let half = frames / 2;
        self.channels
            .iter()
            .map(|&total| ((u32::from(total) + half) / frames) as u8)
            .collect()
    }
}

/// Rows of a buffer to texture copy start on this many bytes.
const ROW_ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

/// Records the copy of `pixels` (RGBA, `size`) into `texture`, which must be that size: the
/// average of a finished shot put back where the HUD is drawn over it.
pub(crate) fn encode(
    (device, queue): (&wgpu::Device, &wgpu::Queue),
    encoder: &mut wgpu::CommandEncoder,
    texture: &wgpu::Texture,
    pixels: &[u8],
    (width, height): (u32, u32),
) {
    let swap = matches!(
        texture.format(),
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    );
    let row = width * 4;
    let padded = row.div_ceil(ROW_ALIGN) * ROW_ALIGN;
    let mut padded_pixels = vec![0u8; padded as usize * height as usize];
    for (line, out) in pixels
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
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("photo shot average"),
        size: padded_pixels.len() as u64,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, &padded_pixels);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sum_rounds_once_and_refuses_another_size() {
        let mut sum = Sum::default();
        assert!(sum.add(&[1, 2, 3, 255], (1, 1)));
        assert!(sum.add(&[2, 2, 4, 255], (1, 1)));
        assert!(!sum.add(&[9; 8], (2, 1)));
        assert_eq!(sum.size(), (1, 1));
        assert_eq!(sum.average(2), vec![2, 2, 4, 255]);
    }
}
