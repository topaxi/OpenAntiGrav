//! The cockpit hit static's noise off every disc whose title binds one:
//! `Data\Tex\noisematt.mip`, 64 by 64 - a `.mip` on the PSP pressings of Pulse
//! and Pure, a `.pct` on Pulse's PS2 one - and nothing requested, so nothing
//! reported absent, on the title that binds none (HD).
//!
//! The draw divides its extents by 64, so a texture of any other size is
//! refused by the loader; this is where a disc that ships one would show.
//!
//! **`#[ignore]`d and never run in CI**: it needs disc images.

use oag_raceplay as race;

/// What a title's load made of the noise: its size when it loaded, and the
/// report lines that name it.
struct Noise {
    size: Option<(u32, u32)>,
    lines: Vec<String>,
}

fn noise_of(image: &str) -> Option<Noise> {
    let path = oag_testdata::image(image)?;
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let size = loaded
        .setup
        .cockpit_hit_noise
        .as_ref()
        .map(|noise| (noise.width, noise.height));
    let lines = loaded
        .report
        .into_iter()
        .filter(|line| line.contains("noisematt") || line.contains("cockpit view's hit static"))
        .collect();
    Some(Noise { size, lines })
}

#[test]
#[ignore = "needs disc images in data/images/"]
fn the_noise_loads_off_every_disc_that_ships_it() {
    for image in [
        "pulse-psp-usa.chd",
        "pulse-psp-eu.chd",
        "pulse-ps2-eu.chd",
        "pure-psp-usa.chd",
        "pure-psp-eu.chd",
    ] {
        let Some(noise) = noise_of(image) else {
            continue;
        };
        println!("{image}: {:?} {:?}", noise.size, noise.lines);
        assert_eq!(noise.size, Some((64, 64)), "{image}: {:?}", noise.lines);
    }
}

#[test]
#[ignore = "needs disc images in data/images/"]
fn hd_binds_none_and_reports_none_absent() {
    let Some(noise) = noise_of("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    assert_eq!(noise.size, None);
    assert!(
        noise.lines.is_empty(),
        "HD asked for a cockpit static: {:?}",
        noise.lines
    );
}
