//! Scratch probe: are `LapBar*`/`PosBar*` baked yellow in the atlas itself,
//! rather than tinted at runtime? Samples the decoded `HUD_Components.gtf`
//! at each widget's own authored `U`/`V`/`TxtrWidth`/`TxtrHeight` rectangle
//! and reports the alpha-weighted average RGB, so a baked colour shows up
//! directly without guessing at a runtime override.
//!
//! **They are: every segment comes back yellow.** Two corrections over this
//! probe's first version, which reported pure white. A `.gtf`'s rows run
//! bottom-up and a layout's `V` counts from the top, so `V` has to be read
//! against the flipped image the way `crate::sprite`'s `decode_gtf` flips it
//! for the sheet; and a fully transparent texel's RGB is white in this
//! texture, so an unweighted mean of an empty region reads as white art.
//!
//! ```sh
//! cargo run -q -p oag-game --example hd_hud_bar_pixels -- data/images/hdfury-ps3-eu-dec.iso
//! ```

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".to_string());
    let mut archives = oag_hd::open(&path)?;
    let blob = archives.read_name(r"data/hud/textures/hud_components.gtf")?;
    let parsed = oag_texture::gtf::Gtf::parse(&blob)?;
    let tex = parsed.only().expect("one texture");
    let rgba = tex.to_rgba(&blob)?;
    let (w, h) = tex.level_size(0);
    let (w, h) = (w as i64, h as i64);

    // name, U, V, width, height (as HUD_positions.xml / HUD_lap_counters.xml author them)
    let rects: &[(&str, i64, i64, i64, i64)] = &[
        ("LapBar0", 339, 0, 37, 16),
        ("LapBar1", 339, 16, 37, 32),
        ("LapBar2", 339, 48, 37, 28),
        ("LapBar3", 339, 76, 37, 29),
        ("LapBar4", 339, 105, 37, 30),
        ("LapBar5", 339, 135, 37, 14),
        ("LapBar6", 339, 149, 38, 13),
        ("PosBar0", 376, 0, 39, 8),
        ("PosBar1", 376, 7, 39, 7),
        ("PosBar2", 376, 14, 39, 30),
        ("PosBar3", 376, 44, 39, 28),
        ("PosBar4", 376, 72, 39, 32),
        ("PosBar5", 376, 103, 39, 30),
        ("PosBar6", 376, 134, 39, 9),
        ("PosBar7", 376, 143, 39, 9),
    ];

    for (name, u, v, rw, rh) in rects {
        let mut sum = [0u64; 3];
        let mut n = 0u64;
        let mut px_count = 0u64;
        for dy in 0..*rh {
            for dx in 0..*rw {
                let x = u + dx;
                // `V` counts from the top; the decoded rows run bottom-up.
                let y = h - 1 - (v + dy);
                if x < 0 || y < 0 || x >= w || y >= h {
                    continue;
                }
                let idx = (y * w + x) as usize;
                if let Some(px) = rgba.get(idx) {
                    let a = u64::from(px[3]);
                    sum[0] += u64::from(px[0]) * a;
                    sum[1] += u64::from(px[1]) * a;
                    sum[2] += u64::from(px[2]) * a;
                    n += a;
                    px_count += 1;
                }
            }
        }
        if n == 0 {
            println!("{name}: fully transparent over {px_count} px");
            continue;
        }
        println!(
            "{name}: alpha-weighted rgb ({}, {}, {}) over {px_count} px",
            sum[0] / n,
            sum[1] / n,
            sum[2] / n
        );
    }

    Ok(())
}
