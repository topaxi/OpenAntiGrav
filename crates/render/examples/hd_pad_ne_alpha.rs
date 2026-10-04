//! Alpha statistics of a pad `_ne` file: how much of the texture the light-bar
//! mask covers, per circuit. `cargo run -p oag-render --example hd_pad_ne_alpha -- <archive> <path>`

use oag_render::mesh;
use oag_texture::gtf;

fn main() -> anyhow::Result<()> {
    let spec = std::env::args().nth(1).expect("archive spec");
    let path = std::env::args().nth(2).expect("gtf path");
    let blob = mesh::read_blob(&spec, &path)?;
    let parsed = gtf::Gtf::parse(&blob)?;
    let texture = parsed
        .only()
        .ok_or_else(|| anyhow::anyhow!("not one texture"))?;
    let rgba = texture.to_rgba(&blob)?;
    let (w, h) = texture.level_size(0);
    let n = (w * h) as usize;
    let zero = rgba[..n].iter().filter(|p| p[3] == 0).count();
    let full = rgba[..n].iter().filter(|p| p[3] == 255).count();
    println!(
        "{path}: {w}x{h} {:?}: alpha 0 on {:.1}%, 255 on {:.1}%",
        texture.format,
        100.0 * zero as f32 / n as f32,
        100.0 * full as f32 / n as f32
    );
    Ok(())
}
