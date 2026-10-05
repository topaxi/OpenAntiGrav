//! Which of Pulse's textures animate, and how fast.
//!
//! A table, in the sense of [ADR-0022]: it names shipped assets and the rate
//! each one scrolls at. Which surfaces move is this title's business, because
//! the names are `Data\Tex\` entries off Pulse's own disc and no other title
//! ships them.
//!
//! The evidence for every entry was measured by
//! `crates/render/tests/animated_uv_ground_truth.rs`, which still owns the
//! measurement; this file owns the conclusion.
//!
//! # Nothing draws through this any more
//!
//! It was the renderer's animation source while nothing on the disc was known
//! to say which surfaces animate or how fast. Something does: each animated
//! material carries its own keyframed `TEXSCALE`/`TEXOFFSET` block, and
//! `oag_mesh::mesh_render::TexAnims` replays those instead - see
//! `docs/ghidra/functions/psp-pulse-usa/texture-animation.md`.
//!
//! **Where the two disagree, this table is the one that is wrong.** Its rates
//! were chosen rather than recovered, and its axis was inferred: it scrolls
//! `col_display7_GLOW` in V, where all twelve circuits author that surface as
//! a **U** scroll. What it is still good for is the question it was built to
//! answer - which surfaces on a circuit are *meant* to move - which makes it a
//! useful cross-check on the authored reading and the only record of the
//! narrow-V-band survey.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// How many whole V sweeps of the blink-light palette pass per
/// `oag_raceplay`'s `ANIM_PERIOD_TICKS`.
///
/// Two, because the measured single-cycle period is ~30 ticks and the texture
/// repeats its 8-frame curve twice down its 16 rows, so a full sweep is ~60
/// ticks. Every other entry in [`ANIMATED_TEXTURES`] reuses this rather than
/// inventing its own: it is the one rate with a measurement behind it, and a
/// shared wrong rate is easier to correct than seven separate guesses.
pub const BLINK_V_CYCLES: f32 = 2.0;

/// Textures whose surfaces animate by scrolling their V (row) coordinate, and
/// how fast, in whole sweeps per `oag_raceplay`'s `ANIM_PERIOD_TICKS`.
///
/// Matched case-insensitively against the filename of the texture's **runtime
/// asset path** (`Texture` node payload `+0x38`) - the only name a track
/// texture has, since its node header carries none.
///
/// # Why a list and not a name rule
///
/// `_GLOW` and `_ADD` are the artists' blend-mode notes, not animation markers:
/// `_ADD` is known to mean additive compositing, and the suffix sits on
/// `hub_banner_GLOW.tga`, `col_banners2_ADD.tga` and `FEISAR3_GLOW.tga`, which
/// are plainly static sponsor art. A suffix rule would make a whole circuit
/// flicker. The material `flags` word at `+0x00` is no better: the static
/// `hub_banner_GLOW` and the banded `flicker1nonalpha_GLOW` both carry `0x91`.
/// And the file itself holds no rate - material `+0x0c..0x14` is zero on every
/// material of every circuit. So the only honest key is an enumerated list, and
/// each entry has to earn its place.
///
/// # What earned a place
///
/// Every entry has at least one draw call whose vertices sit in a **narrow V
/// band** - the authored signature of a quad that picks a phase by V and lets
/// the global scroll step it through the texture's rows. Measured per draw call
/// (not aggregated: separate quads sit at different V deliberately, and merging
/// them destroys the signal) by
/// `crates/render/tests/animated_uv_ground_truth.rs`.
///
/// The band is evidence for *which* textures, not a runtime rule. The engine
/// scrolls a shared texture globally and any geometry sampling it inherits the
/// animation, wide V span included - Assegai's and Piranha's blink quads span
/// 8.1 and 9.6 of 16 rows and still animate. Gating at runtime on a narrow band
/// would switch those two ships off.
///
/// **Confidence 85 for `colours_flashing_GLOW.tga`** (surveyed across all eight
/// ships and confirmed against a frame-accurate capture); **65 for every track
/// entry** - the narrow-band geometry and the banded texture content are real
/// measurements, but no capture of the original has confirmed that these
/// particular surfaces move, and the rates are chosen rather than recovered.
/// Below 70 deliberately: see `docs/reverse-engineering/confidence-rubric.md`.
///
/// # Deliberately excluded
///
/// - `tunnelanim_sb.tga`, despite the name: all 14 of its draws span its full
///   32 rows, so a V scroll would slide the artwork rather than cycle it.
/// - `flicker1/2nonalpha_GLOW.tga`: banded like the blink palette, but its
///   draws tile ~4x vertically, and the pad geometry that mainly uses it is not
///   loaded at all - every circuit embeds the texture while referencing it from
///   zero materials.
/// - `Plasma_scroll_ADD_GLOW.tga`: exactly one tile in both axes (63.75 of 64
///   columns, 64.00 of 64 rows), with all rows distinct and flat luminance. That
///   is a continuous scroll, a different mechanism, and no rate for it has been
///   recovered.
/// - Every `billboard*`, `banner*`, `WES_*_BANNER*` and `*_shinemap`: full-tile
///   or environment-mapped on every draw.
/// - `FEISAR2anim.tga`, which is the one asset that looks like a *horizontal*
///   filmstrip - 256x32 in 8 distinct 32x32 blocks, on sponsor art, with `anim`
///   in the artists' name. It is referenced by **zero materials** on all four
///   circuits that embed it, so it is not drawn at all.
///
/// The U axis was measured too, not assumed: every hoarding and banner spans
/// essentially its full width (`piranha_banner_ADD_GLOW` 254.00 of 256,
/// `hub_banner_GLOW` 128.00 of 128, `WES_FEISAR_BANNER_A` 63.50 of 64), so
/// trackside advertising is static on **both** axes. That is also why this is a
/// scalar V rate rather than a `[f32; 2]`.
pub const ANIMATED_TEXTURES: &[(&str, f32)] = &[
    // Ships. Every one of the 8 playable teams carries a mesh whose material
    // resolves to this one shared texture.
    ("colours_flashing_glow", BLINK_V_CYCLES),
    // On all 12 circuits, and the cleanest signature on the disc: its draws sit
    // at a single exact V line (span 0.00 of 8 rows on 01, 07, 10, 13 and 16).
    ("col_display7_glow", BLINK_V_CYCLES),
    // Same family, 16_Track, narrowest draw 0.19 of 8 rows.
    ("col_display7_blend_glow", BLINK_V_CYCLES),
    // 07_Track, narrowest of 21 draws 0.00 of 32 rows.
    ("07_pulse_light_blend_glow", BLINK_V_CYCLES),
    // 13_Track. "cycle gradient" in the artists' own words, and the row means
    // form a clean symmetric hump. Narrowest draws 0.25, 1.50 and 0.50 rows.
    ("rf_cyclegrad_glow", BLINK_V_CYCLES),
    ("rf_cyclegrad2_glow", BLINK_V_CYCLES),
    ("rf_cyclegrad3_glow", BLINK_V_CYCLES),
    // 10_Track, narrowest draws 0.45 of 4 rows and 0.00 of 64.
    ("sl_bluestrip_glow", BLINK_V_CYCLES),
    ("sl_purplestrip_glow", BLINK_V_CYCLES),
];

/// The V scroll rate for a decoded texture, or `None` if it does not animate.
#[must_use]
pub fn animated_v_cycles(label: &str) -> Option<f32> {
    let key = label.to_ascii_lowercase();
    // Longest match first, so `col_display7_BLEND_GLOW` is not claimed by the
    // `col_display7_GLOW` entry through a shared prefix.
    ANIMATED_TEXTURES
        .iter()
        .filter(|(name, _)| key.contains(name))
        .max_by_key(|(name, _)| name.len())
        .map(|&(_, cycles)| cycles)
}

/// Whether a decoded texture's name identifies its surface as the shared
/// blink-light palette, animated by scrolling its V (row) coordinate.
///
/// **Confidence: 85.** Every one of the 8 playable PSP ships carries a mesh
/// named `glowingShape` whose material resolves to the exact same shared
/// texture, `Data\Tex\colours_flashing_GLOW.tga` - not a per-ship asset, a
/// common one. The ship-specific mesh names first noticed on Feisar
/// (`underbrake_flashrightShape`/`underbrake_flashleftShape`) and Triakis
/// (`flasherShape`/`flasher1Shape`) resolve to the identical texture, which is
/// why matching by mesh name generalised badly (each ship names its extra
/// copies of this light differently, or not at all) while matching by the
/// texture it actually paints generalises to all of them.
///
/// The texture's rows turned out to be the animation itself - see
/// `docs/formats/vex.md`, "The animation is authored in the texture, on its V
/// axis", for the full survey and the capture that confirmed it
/// (`crates/render/tests/blink_lights_ground_truth.rs` checks the texture
/// match against every real ship).
///
/// Matching on the texture rather than the mesh name also means a mesh with
/// more than one material - Feisar's `self_illuminatedShape` has one batch on
/// this texture and another on the ship's own steady-lit skin - is judged
/// batch by batch instead of being wrongly all-or-nothing.
///
/// Kept as its own predicate, rather than folded into [`animated_v_cycles`],
/// because the ship claim is evidenced far more strongly than any track entry:
/// this one is worth naming and citing separately even though the table would
/// match the same label. `blink_lights_ground_truth.rs` asserts it against every
/// real ship.
#[must_use]
pub fn is_blink_light_texture(label: &str) -> bool {
    label.to_ascii_lowercase().contains("flashing_glow")
}

#[cfg(test)]
mod blink_tests;
