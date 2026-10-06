//! Scratch probe behind [`pads.md`](../../../docs/rendering/pads.md)'s "What
//! binds the `_ne` file": an **offline** picture of what the disc's own
//! measured emissive defaults would look like, composited outside the
//! renderer entirely - no change to `Pick`, `skin()` or `mesh.wesl`. Do not
//! wire this into the renderer; see `pads.md` for why (one `aux` texture
//! slot per material, and the lightmap already owns it).
//!
//! **This picture is chosen, not measured, and carries no confidence
//! score - and, per `pads.md`, it is not even the best-supported of the
//! candidates.** What is measured: `emissive`'s tint parameter
//! (`0xe8bcd7f5`) is absent from both pads' material records and is never
//! patched into either pad's fragment microcode either (`hd_pad_ne_tint_probe`),
//! so `oag_mesh::mesh::rcs::emissive` would fall back to its own default,
//! `[1.0, 1.0, 1.0]` white - the tint used below is that fallback, **not a
//! disc-authored value**. What is genuinely **not** measured: which of unit
//! 1's channels the accumulate instruction actually reads (sampled RGB,
//! RGB×alpha, or alpha alone), and **which alpha gates it** - `mesh.wesl`'s
//! own generic version of this shape gates by the *diffuse's* alpha
//! (`first.a`), which `_cs`'s own alpha (measured, `hd_pad_ne_colour_probe`)
//! covers over 93% of the texture, not the ~7% `_ne`'s own alpha covers.
//! This probe draws `_ne`'s alpha as the gate and white as the colour -
//! the shape closest to "only the light bars", matching the maintainer's
//! report structurally, but **neither channel choice is confirmed for a pad
//! material specifically**. Composited here for illustration only; do not
//! treat this PNG as a measured fact, and see `pads.md` for the full set of
//! open readings.

use oag_mesh::mesh;
use oag_texture::gtf;

fn decode(spec: &str, path: &str) -> anyhow::Result<(u32, u32, Vec<[u8; 4]>)> {
    let blob = mesh::read_blob(spec, path)?;
    let parsed = gtf::Gtf::parse(&blob)?;
    let texture = parsed
        .only()
        .ok_or_else(|| anyhow::anyhow!("not one texture"))?;
    let rgba = texture.to_rgba(&blob)?;
    let (w, h) = texture.level_size(0);
    Ok((w, h, rgba))
}

fn composite(cs_path: &str, ne_path: &str, spec: &str, out: &str) -> anyhow::Result<()> {
    let (cw, ch, cs) = decode(spec, cs_path)?;
    let (nw, nh, ne) = decode(spec, ne_path)?;
    anyhow::ensure!((cw, ch) == (nw, nh), "cs/ne size mismatch");

    // emissive's own measured default: tint [1.0, 1.0, 1.0], no scroll - see
    // pads.md. Add white, scaled by the _ne alpha coverage, clamped.
    let mut out_rgba = cs.clone();
    for (px, &[_, _, _, a]) in out_rgba.iter_mut().zip(ne.iter()) {
        let boost = a; // 0..=255, alpha as additive white intensity
        for channel in px.iter_mut().take(3) {
            *channel = channel.saturating_add(boost);
        }
    }
    let flat: Vec<u8> = out_rgba.into_iter().flatten().collect();
    let png = oag_texture::png::encode_rgba(cw, ch, &flat);
    std::fs::write(out, png)?;
    println!("wrote {out}");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let spec = format!("{image}:PS3_GAME/USRDIR/DATA02.PSARC");
    let dir = "/data/environments/12_sol_2/hd_textures/dds";

    composite(
        &format!("{dir}/ds_weaponup_cs.gtf"),
        &format!("{dir}/ds_weaponup_ne.gtf"),
        &spec,
        ".claude/scratch/ds_weaponup_composite_white.png",
    )?;
    composite(
        &format!("{dir}/ds_speedup_cs.gtf"),
        &format!("{dir}/ds_speedup_ne.gtf"),
        &spec,
        ".claude/scratch/ds_speedup_composite_white.png",
    )?;

    Ok(())
}
