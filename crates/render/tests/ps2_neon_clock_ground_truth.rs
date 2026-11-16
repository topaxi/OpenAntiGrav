//! Orcus' neon strips on Pulse PS2 change colour with the world animation clock.
//!
//! **`#[ignore]`d and never run in CI**; it needs the PS2 disc image (`just test-data`).
//!
//! The strips along the left wall of Orcus (`15_Track`) draw `lob_gradient_glow.tga`, a
//! rainbow gradient, under an authored texture scroll with a 600 s loop. Which colour
//! a strip shows is therefore the clock it is drawn at, not a decode property: a frame
//! drawn at the race's tick count (28.05 s) reads magenta, and the original's clock for
//! that moment (`*(g_ingame + 0x40)`, `docs/ghidra/functions/ps2-pulse-eu/anim-transform.md`)
//! reads yellow. `docs/tools/compare-page.md`, "Pulse PS2: a race pair for every
//! circuit", records the pair; the pair script feeds ours the original's clock
//! (`--world-clock-offset`), and `scripts/check-compare-environment.py` checks that offset.

use oag_mesh::mesh::{self, AnimTrack, Texels};

/// Ours' own clock at the Orcus White pair's frame: `1683` ticks at 60 Hz.
const TICK_CLOCK_S: f32 = 28.05;

/// The original's world clock at that frame: `*(g_ingame + 0x40)` in the state (33.5335 s) run on
/// by the race clock's advance from the state (26.3787 s) to the grab's mean (27.7742 s).
const ORIGINAL_CLOCK_S: f32 = 34.93;

#[test]
#[ignore = "needs a disc image in data/images/"]
fn orcus_neon_strips_are_magenta_on_the_tick_clock_and_yellow_on_the_originals() {
    let Some(image) = oag_testdata::image("pulse-ps2-eu.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");
    let track = r"Data\Environments\15_Track\track.vex";
    let blob = archives.read_name(track).expect("reading track.vex");
    let set_blob = archives.read_preceding(track).expect("the texture set");
    let set = mesh::Ps2TextureSet::parse(&set_blob).expect("parsing the texture set");
    let model = mesh::build_with_textures(track, &blob, Some(&set)).expect("decoding track.vex");

    let mut strips = 0;
    for draw in model.draws.iter().chain(&model.alpha_tested_draws) {
        let Some(texture) = draw.texture.and_then(|t| model.textures[t].as_ref()) else {
            continue;
        };
        if texture.label != "lob_gradient_glow.tga" {
            continue;
        }
        let Texels::Rgba8(texels) = &texture.texels else {
            panic!("the gradient is not plain RGBA8");
        };
        let animated: Vec<_> = draw
            .range
            .clone()
            .map(|i| &model.vertices[model.indices[i as usize] as usize])
            .filter(|v| v.anim != 0)
            .collect();
        let Some(first) = animated.first() else {
            continue;
        };
        let AnimTrack::Psp(track) = &model.anim_tracks[first.anim as usize - 1] else {
            panic!("a PS2 track carries a PSP-shaped block");
        };
        assert!(
            (track.loop_seconds - 600.0).abs() < 1e-3,
            "the strips scroll on a 600 s loop, not a short flicker: {}",
            track.loop_seconds
        );
        let at = |seconds: f32| {
            let (scale, offset) = track.sample(seconds);
            let u = (first.texcoord[0] * scale[0] + offset[0]).rem_euclid(1.0);
            let v = (first.texcoord[1] * scale[1] + offset[1]).rem_euclid(1.0);
            let x = ((u * texture.width as f32) as u32).min(texture.width - 1);
            let y = ((v * texture.height as f32) as u32).min(texture.height - 1);
            let p = &texels[((y * texture.width + x) * 4) as usize..][..3];
            [p[0], p[1], p[2]]
        };
        let [r, g, b] = at(TICK_CLOCK_S);
        assert!(
            r > 200 && g < 60 && b > 100,
            "magenta on the tick clock, got {:?}",
            [r, g, b]
        );
        let [r, g, b] = at(ORIGINAL_CLOCK_S);
        assert!(
            r > 200 && g > 200 && b < 60,
            "yellow on the original's clock, got {:?}",
            [r, g, b]
        );
        strips += 1;
    }
    assert!(
        strips > 0,
        "no draw of the neon gradient was found on 15_Track"
    );
}
