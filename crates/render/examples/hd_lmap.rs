//! Scratch probe: decode a circuit's baked lighting atlas to a PNG, and say
//! how its texels are distributed - the question being whether a lightmap this
//! renderer samples as hard-edged white blocks is that way in the file.

use oag_mesh::mesh;
use oag_texture::gtf;

fn main() -> anyhow::Result<()> {
    let spec = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = std::env::args().nth(2).unwrap_or_else(|| {
        "/data/environments/talons_junction/lmaps/ile_mesh_combine_track01_01-lmap.gtf".into()
    });
    let out = std::env::args()
        .nth(3)
        .unwrap_or_else(|| "/tmp/lmap.png".into());
    let blob = mesh::read_blob(&spec, &name)?;
    let parsed = gtf::Gtf::parse(&blob).map_err(|e| anyhow::anyhow!("{e}"))?;
    let texture = parsed
        .only()
        .ok_or_else(|| anyhow::anyhow!("not one texture"))?;
    let rgba: Vec<u8> = texture
        .to_rgba(&blob)
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .into_iter()
        .flatten()
        .collect();
    let (width, height) = texture.level_size(0);
    let mut buckets = [0usize; 8];
    let mut alpha = [0usize; 8];
    for texel in rgba.as_chunks::<4>().0 {
        let luma = (0.3 * f32::from(texel[0])
            + 0.59 * f32::from(texel[1])
            + 0.11 * f32::from(texel[2])) as usize;
        buckets[(luma / 32).min(7)] += 1;
        alpha[(usize::from(texel[3]) / 32).min(7)] += 1;
    }
    let total = (rgba.len() / 4).max(1) as f32;
    println!("{name}: {width}x{height}");
    println!(
        "  luma octaves %: {:?}",
        buckets.map(|b| (100.0 * b as f32 / total).round())
    );
    println!(
        "  alpha octaves %: {:?}",
        alpha.map(|b| (100.0 * b as f32 / total).round())
    );
    std::fs::write(&out, oag_texture::png::encode_rgba(width, height, &rgba))?;
    println!("wrote {out}");
    Ok(())
}
