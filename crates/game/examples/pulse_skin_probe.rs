//! Dumps one team's race hull textures with and without a `PI_ModelSkin`, so
//! the repaint can be *looked at* rather than counted.
//!
//! A headless build has no frame to judge, and "the texture is bound" is
//! exactly the kind of check that passes while the ship on screen is wrong.
//! This writes the four texture slots either way as PNGs, side by side, which
//! is the closest a machine with no GPU gets to the question a player would
//! actually ask.
//!
//! ```sh
//! cargo run -p oag-game --example pulse_skin_probe -- \
//!     data/images/pulse-psp-usa.chd Assegai Alternative out/
//! ```
//!
//! Writes `<out>/<team>-<slot>-baseline.png` and `-<skin>.png`. Nothing is
//! committed: `data/` and any output directory under it are gitignored, and
//! the images are shipped content - see
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.

use oag_render::mesh;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args.next().unwrap_or_else(|| {
        eprintln!("usage: pulse_skin_probe <image> [team] [skin] [out dir]");
        std::process::exit(2);
    });
    let team = args.next().unwrap_or_else(|| "Assegai".to_string());
    let skin = args.next().unwrap_or_else(|| "Alternative".to_string());
    let out = args.next().unwrap_or_else(|| "data/skin-probe".to_string());
    std::fs::create_dir_all(&out)?;

    let load = |skin: Option<&str>| {
        oag_game::race::load(&oag_game::race::Options {
            source: image.clone(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            team: Some(team.clone()),
            skin: skin.map(str::to_string),
            ..oag_game::race::Options::default()
        })
    };
    let baseline = load(None)?;
    let painted = load(Some(&skin))?;
    for line in painted.report.iter().filter(|line| line.contains(".dat")) {
        println!("{line}");
    }

    let mut written = 0;
    for (which, loaded) in [("baseline", &baseline), (skin.as_str(), &painted)] {
        for texture in loaded.liveries[0].hull.textures.iter().flatten() {
            if mesh::ship_skin::slot_of(&texture.label).is_none() {
                continue;
            }
            let Some(rgba) = texture.rgba() else { continue };
            let stem = texture.label.replace(['\\', '/', '.'], "_");
            let path = format!("{out}/{team}-{stem}-{which}.png");
            std::fs::write(
                &path,
                oag_texture::png::encode_rgba(texture.width, texture.height, rgba),
            )?;
            println!("{path}: {}x{}", texture.width, texture.height);
            written += 1;
        }
    }
    println!("{written} image(s) written to {out}");
    Ok(())
}
