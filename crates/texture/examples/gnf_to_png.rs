//! Scratch probe: decode a bare `.gnf` file to PNG beside it.
//!
//! Only a **linear** `TileMode` decodes - see `oag_texture::gnf`'s own
//! module doc. Every real `.gnf` this project has sampled is genuinely
//! tiled, so this will fail with [`oag_texture::gnf::Error::Tiled`] on real
//! disc content today; it exists for the linear case and as the shape a
//! future untiler slots into.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in std::env::args().skip(1) {
        let blob = std::fs::read(&path)?;
        let texture = oag_texture::gnf::Texture::parse(&blob)?;
        let rgba: Vec<u8> = texture.decode(&blob)?.into_iter().flatten().collect();
        let png = oag_texture::png::encode_rgba(texture.width, texture.height, &rgba);
        let out = format!("{path}.png");
        std::fs::write(&out, png)?;
        println!(
            "{out} ({}x{}, {:?})",
            texture.width, texture.height, texture.surface_format
        );
    }
    Ok(())
}
