//! Scratch probe: decode a bare `.mip` file to PNG beside it. Not part of the
//! deliverable - used to eyeball a decoded texture during RE work.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in std::env::args().skip(1) {
        let blob = std::fs::read(&path)?;
        let texture = oag_texture::texture::Texture::parse(&blob)?;
        let rgba = texture.to_rgba();
        let png = oag_texture::png::encode_rgba(
            u32::from(texture.width),
            u32::from(texture.height),
            &rgba,
        );
        let out = format!("{path}.png");
        std::fs::write(&out, png)?;
        println!(
            "{out} ({}x{}, {}bpp, mip_levels={}, unknown={:02x?})",
            texture.width,
            texture.height,
            texture.bits_per_pixel,
            texture.mip_levels,
            texture.unknown
        );
    }
    Ok(())
}
