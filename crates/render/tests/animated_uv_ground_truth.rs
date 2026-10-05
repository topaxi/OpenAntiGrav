//! The evidence behind `oag_pulse::textures::ANIMATED_TEXTURES`, re-measured from
//! real circuits.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this crate:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all
//! ```
//!
//! # What this is for
//!
//! No animated surface on a track is authored as such. The file holds no scroll
//! rate (material `+0x0c..0x14` is zero on every material of every circuit) and
//! no flag that separates animated from static (`flicker1nonalpha_GLOW` and the
//! plainly static `hub_banner_GLOW` both carry material flags `0x91`). So the
//! renderer keys the animation off an enumerated list of texture names, and that
//! list is only as good as the evidence that put each entry on it.
//!
//! The evidence is geometric. A texture whose rows are an animation is sampled
//! by quads authored at a **narrow V band**: the quad picks a phase by its V and
//! the global scroll walks it through the rows. A quad spanning the texture's
//! full height instead shows the whole strip at once, and scrolling V slides the
//! artwork rather than cycling it. So:
//!
//! - every entry in the table must have at least one narrow-band draw call, and
//! - the static art a naive `_GLOW`/`_ADD` suffix rule would have swept up must
//!   have none.
//!
//! Measured **per draw call**. Aggregating across draws destroys the signal:
//! separate quads deliberately sit at different V, that being the phase offset,
//! so a per-texture min/max reads a narrow-band texture as a wide one.
//!
//! # What this test does not establish
//!
//! That the original animates these surfaces. Narrow-band geometry plus banded
//! texture content is a strong structural argument and nothing more; the track
//! entries are recorded at confidence 65 for exactly that reason. Confirming
//! them needs a frame-accurate capture of the running game at each surface.

use std::path::PathBuf;

use oag_mesh::mesh::{self, Model};

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// The narrowest U span, in the texture's own columns, of any draw call
/// painting `label`, or `None` if the model never paints it.
///
/// The same measurement as [`narrowest_v_span`] on the other axis, and the
/// reason `GpuVertex` carries a scalar V rate rather than a `[f32; 2]`: nothing
/// on the disc is authored to scroll in U, and this is what says so rather than
/// assuming it.
fn narrowest_u_span(model: &Model, label: &str) -> Option<f32> {
    narrowest_span(model, label, 0)
}

/// The narrowest V span, in the texture's own rows, of any draw call painting
/// `label`, or `None` if the model never paints it.
fn narrowest_v_span(model: &Model, label: &str) -> Option<f32> {
    narrowest_span(model, label, 1)
}

/// `axis` is 0 for U against the texture's width, 1 for V against its height.
fn narrowest_span(model: &Model, label: &str, axis: usize) -> Option<f32> {
    let lists = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ];
    let mut best: Option<f32> = None;
    for draws in lists {
        for draw in draws.iter() {
            let Some(slot) = draw.texture else { continue };
            let Some(Some(texture)) = model.textures.get(slot) else {
                continue;
            };
            if !texture.label.eq_ignore_ascii_case(label) {
                continue;
            }
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for i in draw.range.clone() {
                let v = model.vertices[model.indices[i as usize] as usize];
                lo = lo.min(v.texcoord[axis]);
                hi = hi.max(v.texcoord[axis]);
            }
            if lo > hi {
                continue;
            }
            let extent = if axis == 0 {
                texture.width
            } else {
                texture.height
            };
            let span = (hi - lo) * extent as f32;
            best = Some(best.map_or(span, |b: f32| b.min(span)));
        }
    }
    best
}

fn track(archives: &mut oag_assets::Archives, circuit: &str) -> Model {
    let name = format!(r"Data\Environments\{circuit}_Track\track.vex");
    let blob = archives.read_name(&name).expect("reading track.vex");
    mesh::build(&name, &blob).expect("decoding track.vex")
}

/// A draw whose vertices span at most this many of its texture's rows is
/// treated as authored at a phase band rather than showing the whole strip.
///
/// Generous at 2.0: the point is to separate "a sliver of the texture" from "the
/// entire texture", and the real measurements fall either under 1.5 rows or
/// above 8, nowhere near this line.
const NARROW_ROWS: f32 = 2.0;

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_animated_track_texture_has_a_narrow_band_draw() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    // The circuit each table entry was measured on. `col_display7_GLOW` is on
    // all twelve; the rest are per-track art.
    let cases: &[(&str, &str)] = &[
        ("01", "col_display7_GLOW.tga"),
        ("07", "col_display7_GLOW.tga"),
        ("10", "col_display7_GLOW.tga"),
        ("13", "col_display7_GLOW.tga"),
        ("16", "col_display7_GLOW.tga"),
        ("07", "07_Pulse_light_BLEND_GLOW.TGA"),
        ("13", "rf_cyclegrad_GLOW.tga"),
        ("13", "rf_cyclegrad2_GLOW.tga"),
        ("13", "rf_cyclegrad3_GLOW.tga"),
        ("10", "SL_BlueStrip_GLOW.tga"),
        ("10", "SL_Purplestrip_GLOW.tga"),
        ("16", "col_display7_BLEND_GLOW.tga"),
    ];

    let mut wide = Vec::new();
    for &(circuit, label) in cases {
        let model = track(&mut archives, circuit);
        let Some(span) = narrowest_v_span(&model, label) else {
            panic!("{circuit}_Track no longer paints {label} at all");
        };
        println!("{circuit}_Track {label}: narrowest draw spans {span:.2} rows");
        assert!(
            oag_pulse::textures::animated_v_cycles(label).is_some(),
            "{label} is measured here but is not in ANIMATED_TEXTURES"
        );
        if span > NARROW_ROWS {
            wide.push(format!("{circuit}_Track {label} ({span:.2} rows)"));
        }
    }

    assert!(
        wide.is_empty(),
        "table entries with no narrow-band draw left - the V-scroll reading \
         does not hold for them and they should be removed: {wide:?}"
    );
}

/// The other half of the claim, and the one that keeps the table honest: the
/// static sponsor art that shares the `_GLOW`/`_ADD` naming convention is
/// painted by quads that span their whole texture, so it is excluded on
/// evidence rather than by taste.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn excluded_static_art_spans_its_whole_texture() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    let cases: &[(&str, &str)] = &[
        ("16", "hub_banner_GLOW.tga"),
        ("16", "col_banners2_ADD.tga"),
        ("13", "banner2.tga"),
        ("13", "billboard10.tga"),
        ("05", "tunnelanim_sb.tga"),
        ("16", "flicker1nonalpha_GLOW.tga"),
        ("16", "Plasma_scroll_ADD_GLOW.tga"),
    ];

    let mut narrow = Vec::new();
    for &(circuit, label) in cases {
        let model = track(&mut archives, circuit);
        let Some(span) = narrowest_v_span(&model, label) else {
            panic!("{circuit}_Track no longer paints {label} at all");
        };
        println!("{circuit}_Track {label}: narrowest draw spans {span:.2} rows");
        assert_eq!(
            oag_pulse::textures::animated_v_cycles(label),
            None,
            "{label} is excluded here but the table animates it"
        );
        if span <= NARROW_ROWS {
            narrow.push(format!("{circuit}_Track {label} ({span:.2} rows)"));
        }
    }

    assert!(
        narrow.is_empty(),
        "excluded texture(s) turn out to have narrow-band draws after all, so \
         the exclusion needs re-examining: {narrow:?}"
    );
}

/// **Trackside advertising does not animate on either axis.**
///
/// The V measurements above only rule out a filmstrip that steps through
/// *rows*. A horizontal filmstrip - frames laid out along U - would be invisible
/// to them, and one asset in particular looks exactly like that:
/// `FEISAR2anim.tga` is 256x32 splitting into 8 distinct 32x32 blocks, on
/// sponsor art, with `anim` in the artists' own name. Its shape alone cannot
/// settle it, because `piranha_banner_ADD_GLOW.tga` is the same 256x32 with the
/// same 8 blocks and is unambiguously a static banner.
///
/// The geometry settles both. Every hoarding, banner and sponsor logo is
/// painted by quads spanning essentially its whole width, so scrolling U would
/// slide the artwork rather than cut between frames. And `FEISAR2anim.tga` is
/// not painted at all: every circuit that embeds it references it from **zero**
/// materials, which is the same shape as `flicker1/2nonalpha_GLOW` and the
/// `Data\Pads\` geometry.
///
/// This is also the evidence for `GpuVertex::v_cycles` being a scalar rather
/// than a `[f32; 2]`: no surface on the disc is authored to scroll in U.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn no_advertising_surface_is_a_horizontal_filmstrip() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    // Painted, and spanning the full width: a static hoarding.
    let painted: &[(&str, &str)] = &[
        ("06", "piranha_banner_ADD_GLOW.tga"),
        ("06", "Moa_logo_GLOW.tga"),
        ("16", "hub_banner_GLOW.tga"),
        ("16", "col_banners2_ADD.tga"),
        ("13", "banner2.tga"),
        ("01", "WES_FEISAR_BANNER_A.tga"),
        ("01", "Lazerfence_ADD_GLOW.tga"),
        ("16", "Plasma_scroll_ADD_GLOW.tga"),
    ];
    let mut narrow = Vec::new();
    for &(circuit, label) in painted {
        let model = track(&mut archives, circuit);
        let Some(span) = narrowest_u_span(&model, label) else {
            panic!("{circuit}_Track no longer paints {label} at all");
        };
        println!("{circuit}_Track {label}: narrowest draw spans {span:.2} columns");
        if span <= NARROW_ROWS {
            narrow.push(format!("{circuit}_Track {label} ({span:.2} columns)"));
        }
    }
    assert!(
        narrow.is_empty(),
        "advertising art with a narrow U band - a horizontal filmstrip after \
         all, and the table needs a U component: {narrow:?}"
    );

    // Embedded on four circuits and drawn on none of them.
    for circuit in ["03", "04", "13", "14"] {
        let model = track(&mut archives, circuit);
        assert_eq!(
            narrowest_u_span(&model, "FEISAR2anim.tga"),
            None,
            "{circuit}_Track paints FEISAR2anim.tga now - it was excluded on \
             the grounds that no material references it"
        );
    }
}
