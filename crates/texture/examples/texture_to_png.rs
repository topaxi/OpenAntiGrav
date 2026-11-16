//! Decodes one extracted texture file (`.gtf`, `.gxt`, `.gnf`, `.mip`, `.pct`)
//! to two PNGs: its colour with alpha forced opaque, and its alpha as grey.
//!
//!     cargo run -p oag-texture --example texture_to_png -- in.gtf out
//!
//! writes `out.rgb.png` and `out.alpha.png`. A material's mask often lives in
//! the alpha of a picture whose colour reads complete without it, so the two
//! are written apart instead of as one RGBA file a viewer composites.

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(input), Some(stem)) = (args.next(), args.next()) else {
        return Err("usage: texture_to_png <texture file> <output stem>".into());
    };
    let blob = std::fs::read(&input)?;
    let picture = oag_texture::decode::decode_named(&input, &blob)?;
    let (width, height) = (picture.width, picture.height);
    let row = width as usize * 4;
    let mut rows: Vec<&[u8]> = picture.rgba.chunks_exact(row).collect();
    if picture.bottom_up {
        rows.reverse();
    }
    let top_down: Vec<u8> = rows.concat();
    let texels = top_down.as_chunks::<4>().0;
    let colour: Vec<u8> = texels
        .iter()
        .flat_map(|p| [p[0], p[1], p[2], 0xff])
        .collect();
    let alpha: Vec<u8> = texels
        .iter()
        .flat_map(|p| [p[3], p[3], p[3], 0xff])
        .collect();
    for (suffix, pixels) in [("rgb", &colour), ("alpha", &alpha)] {
        let path = PathBuf::from(format!("{stem}.{suffix}.png"));
        std::fs::write(&path, oag_texture::png::encode_rgba(width, height, pixels))?;
        println!("{} {width}x{height}", path.display());
    }
    Ok(())
}
