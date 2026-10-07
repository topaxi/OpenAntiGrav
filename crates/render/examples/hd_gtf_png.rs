//! Scratch probe: decode one `.gtf` entry to a PNG and print its alpha range.
//!
//! ```sh
//! cargo run --release -p oag-render --example hd_gtf_png -- <spec> <entry> <out.png>
//! ```

use oag_assets::Container;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args.next().ok_or_else(|| anyhow::anyhow!("no spec"))?;
    let entry = args.next().ok_or_else(|| anyhow::anyhow!("no entry"))?;
    let out = args.next().ok_or_else(|| anyhow::anyhow!("no output"))?;
    let blob = Container::open(&spec)?.read_entry(&entry)?;
    let gtf = oag_texture::gtf::Gtf::parse(&blob)?;
    let tex = gtf
        .only()
        .ok_or_else(|| anyhow::anyhow!("not one texture"))?;
    let (w, h) = tex.level_size(0);
    let rgba = tex.to_rgba(&blob)?;
    let a: Vec<u8> = rgba.iter().map(|p| p[3]).collect();
    println!(
        "{w}x{h} alpha min {} max {}",
        a.iter().min().unwrap(),
        a.iter().max().unwrap()
    );
    let flat: Vec<u8> = rgba
        .iter()
        .take((w * h) as usize)
        .flatten()
        .copied()
        .collect();
    std::fs::write(&out, oag_texture::png::encode_rgba(w, h, &flat))?;
    Ok(())
}
