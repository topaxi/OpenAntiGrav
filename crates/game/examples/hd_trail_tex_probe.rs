//! Scratch probe: the trail textures' channel means as the loader decodes them.
fn main() -> anyhow::Result<()> {
    let options = oag_game::race::Options {
        source: "data/images/hdfury-ps3-eu-dec.iso".into(),
        ..Default::default()
    };
    let loaded = oag_game::race::load(&options)?;
    for (name, tex) in [
        ("noise/red", loaded.noise.as_ref()),
        ("shape/blue", loaded.trail_shape.as_ref()),
    ] {
        let Some(t) = tex else {
            println!("{name}: none");
            continue;
        };
        let px = t.rgba.as_chunks::<4>().0;
        let mut sums = [0u64; 4];
        for p in px {
            for c in 0..4 {
                sums[c] += u64::from(p[c]);
            }
        }
        let n = px.len() as u64;
        println!(
            "{name}: {}px mean rgba = {} {} {} {}",
            n,
            sums[0] / n,
            sums[1] / n,
            sums[2] / n,
            sums[3] / n
        );
    }
    Ok(())
}
