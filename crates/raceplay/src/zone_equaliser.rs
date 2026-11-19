//! The texture a Zone race shows on billboard slots 1 to 7.
//!
//! HD's `Billboard_LoadModelAndBind` binds, in a Zone race, one shared texture to every
//! slot but 8: the Environment stage block's 512 x 512 target (`+0x32b4`), which the stage
//! pass draws into (`docs/ghidra/functions/ps3-hdfury-eu/billboards.md`, "what a Zone
//! race's shared `billboard` texture is"). Read off a running Zone race it is an
//! equaliser: a 32 x 32 grid of 16-pixel cells, each cell a ground colour and a 2-pixel
//! black line on its right and bottom edge, lit in bars by the music's spectrum.
//!
//! Only the grid with no bar lit is drawn. The bars' source is unread, and a Zone race's
//! first seconds (the start pair) show no bar.

#[cfg(feature = "view")]
use crate::drawable::Drawable;

/// The target's side in pixels.
pub const SIDE: u32 = 512;
/// One cell's side in pixels.
const CELL: u32 = 16;
/// The line on a cell's right and bottom edge, in pixels.
const LINE: u32 = 2;
/// A cell's ground, as read: `A8R8G8B8` `0x60010101`. Drawn opaque, the stage target's own
/// alpha not being a coverage on this surface.
const GROUND: [u8; 4] = [1, 1, 1, 255];
/// The line, as read: `0xff000000`.
const LINE_COLOUR: [u8; 4] = [0, 0, 0, 255];

/// What this texture is, for the load report.
pub const PROVENANCE: &str = "grid layout read off RPCS3, Zone race on Vineta K, confidence 80; \
     bars not drawn: the spectrum feeding them is unread";

/// The grid, `SIDE * SIDE` RGBA8 texels.
#[must_use]
pub fn texels() -> Vec<u8> {
    let mut out = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let line = x % CELL >= CELL - LINE || y % CELL >= CELL - LINE;
            out.extend_from_slice(if line { &LINE_COLOUR } else { &GROUND });
        }
    }
    out
}

#[cfg(feature = "view")]
/// Points every placeholder material of `track` whose slot is in `slots` at the grid.
pub(super) fn bind_into(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    track: &mut Drawable,
    placeholders: &[(usize, u32)],
    slots: &[u32],
) {
    if slots.is_empty() {
        return;
    }
    let size = wgpu::Extent3d {
        width: SIDE,
        height: SIDE,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("zone equaliser"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        &texels(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIDE * 4),
            rows_per_image: Some(SIDE),
        },
        size,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    for &(texture_slot, number) in placeholders {
        if slots.contains(&number) {
            track.set_albedo(device, queue, texture_slot, &view, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grid_has_the_cell_and_line_the_original_target_has() {
        let t = texels();
        let at = |x: u32, y: u32| &t[((y * SIDE + x) * 4) as usize..][..4];
        assert_eq!(at(0, 0), GROUND);
        assert_eq!(at(13, 13), GROUND);
        assert_eq!(at(14, 0), LINE_COLOUR);
        assert_eq!(at(0, 15), LINE_COLOUR);
        assert_eq!(at(16, 0), GROUND);
        assert_eq!(t.len(), (SIDE * SIDE * 4) as usize);
    }
}
