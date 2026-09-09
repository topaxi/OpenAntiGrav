//! Scratch probe: decode a bare `.gtf` file to PNG beside it.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in std::env::args().skip(1) {
        let blob = std::fs::read(&path)?;
        let parsed = oag_texture::gtf::Gtf::parse(&blob)?;
        let texture = parsed.only().ok_or("not one texture")?;
        let rgba: Vec<u8> = texture.to_rgba(&blob)?.into_iter().flatten().collect();
        let (width, height) = texture.level_size(0);
        let png = oag_texture::png::encode_rgba(width, height, &rgba);
        let out = format!("{path}.png");
        std::fs::write(&out, png)?;
        println!("{out} ({width}x{height}, {:?})", texture.format);
    }
    Ok(())
}
