//! A disc with no system picture (Pulse on PS2) has its card put together from
//! its title screen's own parts: the logo centred over one frame of the menu backdrop movie
//! (`docs/ui/launcher.md`, "The PS2 card").

use std::io::Write;
use std::path::Path;
use std::process::Stdio;

use oag_video::ipf;

use super::Picture;

/// The PSP's raster: the logo's texels are its pixels, and the backdrop, an
/// anamorphic encode of a wide picture, is resampled into it.
const WIDTH: u32 = 480;
const HEIGHT: u32 = 272;
/// The backdrop frame used, one with its rays in view. Every IPU frame is
/// intra-coded, so it decodes alone.
const FRAME: usize = 30;

/// The composed picture, `None` for a source that holds no title's card parts.
/// Without an `ffmpeg` the logo sits on black.
pub(super) fn compose(path: &Path) -> Option<Picture> {
    let source = path.to_str()?;
    let (title, parts, movie) = crate::title::ALL.iter().find_map(|&title| {
        let parts = title.launcher_card?;
        let (_, movie) = oag_assets::source::read_loose_file(source, &parts.backdrop).ok()??;
        Some((title, parts, movie))
    })?;
    let mut archives = oag_assets::Archives::open(source, title).ok()?;
    // The PS2 build keeps a `.mip` name's texture under `.pct`.
    let blob = archives.read_name(parts.logo).ok().or_else(|| {
        let ps2 = oag_formats::wad::ps2_texture_name(parts.logo)?;
        archives.read_name(&ps2).ok()
    })?;
    let logo = oag_texture::decode::decode_sniffed(&blob).ok()?;
    let mut rgba = backdrop(&movie).unwrap_or_else(|| {
        log::debug!("{source}: no backdrop frame decoded, the logo sits on black");
        [0, 0, 0, 255].repeat((WIDTH * HEIGHT) as usize)
    });
    over(&mut rgba, &logo.rgba, logo.width.into(), logo.height.into());
    Some(Picture {
        width: WIDTH,
        height: HEIGHT,
        rgba,
    })
}

/// [`FRAME`] of the movie as RGBA at the card's size, through `ffmpeg`.
fn backdrop(movie: &[u8]) -> Option<Vec<u8>> {
    let parsed = ipf::parse(movie).ok()?;
    let frame = parsed.frames.get(FRAME)?;
    let wrapped = ipf::ipum(frame, parsed.header.width, parsed.header.height, 1);
    let mut child = oag_ffmpeg::command(oag_ffmpeg::Tool::Ffmpeg)
        .args(["-v", "error", "-f", "ipu", "-i", "pipe:0", "-frames:v", "1"])
        .args(["-vf", &format!("scale={WIDTH}:{HEIGHT}")])
        .args(["-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    // One frame is smaller than a pipe's buffer, so writing it all before
    // reading cannot block. Dropping the handle ends the input.
    child.stdin.take()?.write_all(&wrapped).ok()?;
    let output = child.wait_with_output().ok()?;
    (output.status.success() && output.stdout.len() == (WIDTH * HEIGHT * 4) as usize)
        .then_some(output.stdout)
}

/// Blends `image`, `width` x `height` straight-alpha RGBA, over the middle of
/// the card's `canvas`, cut to it.
fn over(canvas: &mut [u8], image: &[u8], width: i64, height: i64) {
    let left = (i64::from(WIDTH) - width) / 2;
    let top = (i64::from(HEIGHT) - height) / 2;
    for (index, pixel) in image.as_chunks::<4>().0.iter().enumerate() {
        let x = left + index as i64 % width;
        let y = top + index as i64 / width;
        if !(0..i64::from(WIDTH)).contains(&x) || !(0..i64::from(HEIGHT)).contains(&y) {
            continue;
        }
        let at = (y * i64::from(WIDTH) + x) as usize * 4;
        let alpha = u32::from(pixel[3]);
        for channel in 0..3 {
            let under = u32::from(canvas[at + channel]);
            let blended = (u32::from(pixel[channel]) * alpha + under * (255 - alpha) + 127) / 255;
            canvas[at + channel] = blended as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_image_is_blended_over_the_middle_by_its_alpha() {
        let mut canvas = [10, 20, 30, 255].repeat((WIDTH * HEIGHT) as usize);
        let image = [[210, 220, 230, 255], [210, 220, 230, 0]].concat();
        over(&mut canvas, &image, 2, 1);
        let row = (HEIGHT - 1) / 2;
        let at = |x: u32| (row * WIDTH + x) as usize * 4;
        assert_eq!(canvas[at(WIDTH / 2 - 1)..][..4], [210, 220, 230, 255]);
        assert_eq!(canvas[at(WIDTH / 2)..][..4], [10, 20, 30, 255]);
    }

    #[test]
    fn an_image_larger_than_the_card_is_cut_to_it() {
        let mut canvas = [0, 0, 0, 255].repeat((WIDTH * HEIGHT) as usize);
        let (width, height) = (WIDTH + 2, HEIGHT + 2);
        let image = [255, 255, 255, 255].repeat((width * height) as usize);
        over(&mut canvas, &image, width.into(), height.into());
        assert!(canvas.iter().all(|&byte| byte == 255));
    }
}
