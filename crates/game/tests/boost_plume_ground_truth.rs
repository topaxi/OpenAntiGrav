//! Decodes **every team's** boost plume off the disc and checks what came out -
//! the model `oag_raceplay::load` reads beside `Ship.vex`.
//!
//! **Two discs, and they do not ship the same model.** Most of this file is the
//! PSP's `shipboost.vex`, which is where the plume's look was measured; the
//! last two tests are the PS2's, which is a differently authored file that
//! happens to sit under the same name. What differs, measured across all 24 of
//! the PS2 disc's plume files (twelve teams, `shipboost` and `Zoneboost` each)
//! and all 16 of the PSP's:
//!
//! | | PSP | PS2 |
//! | --- | --- | --- |
//! | triangles | 113 | 142 |
//! | `Texture` nodes | 1, `Data\Tex\pulse_boost2_ADD.tga` | 2, the team's own `Textures\*_GLOW.tga` |
//! | texture pixels | embedded in the file | the archive entry **before** it |
//! | `Mesh` parent | `World` directly | an `Anim Transform` each |
//! | authored motion | a UV scroll over 90 frames | a UV scroll over 100 frames **and** a 67-key anchor scale flicker looping at 1.6667 s |
//!
//! Both differences are load-bearing rather than trivia: a PS2 plume built
//! through the PSP's path draws untextured, with both meshes collapsed onto the
//! craft's origin several units ahead of the engines, because geometry under an
//! `Anim Transform` is baked in that node's space. See `livery::ps2_skin` and
//! `race::scene::frame`'s draw loop.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(boost_plume_ground_truth)'
//! ```
//!
//! # What this pins
//!
//! Reading one team's `shipboost.vex` by eye, through `oag-view --mesh` and a
//! `--nodes` dump, found: two `Mesh` nodes (`bflare1Shape`/`bflare2Shape` on
//! Assegai, a per-team-varying pair of names elsewhere), one `Texture` node
//! (`Data\Tex\pulse_boost2_ADD.tga`), both direct children of `World` with no
//! `Transform` between them - which is why the boost plume is drawn once in
//! ship space rather than mounted on a locator, see `Loaded::boost_model`'s
//! doc comment. This file checks the same shape holds for every team rather
//! than the one that happened to be looked at, through the same
//! `oag_mesh::mesh::build_with_textures` path `oag_raceplay::load` uses.
//!
//! It also pins that those transparent-draw batches are what they need to be
//! for `race::Scene`'s additive blend override to matter at all: if the
//! authored `pass_mask` had classified them opaque or alpha-tested instead,
//! `exhaust::TRAIL_BLEND` would never run and the plume would draw with no
//! blending, silently, rather than with the wrong one. Measured, not
//! inferred from how a screenshot looked: every team's four batches land in
//! `Model::transparent_draws`, none in the other two lists.
//!
//! **Not every team's `shipboost.vex` is otherwise identical.** A full
//! `--nodes` dump (not filtered to `Mesh`/`Texture`, which is how an earlier
//! pass missed this) shows Assegai carrying one extra `Airbrake` node
//! (`0x3c5`) beside its two meshes, and Goteki carrying a `Transform` node
//! plus *two* `Airbrake` nodes - both still direct children of `World`, so
//! neither sits between it and either `Mesh` node. The other six teams carry
//! only the three `0x000` junk nodes described below. None of it affects the
//! assertions here, since `mesh::build_with_textures` only reads `Mesh` and
//! `Texture` nodes - but a claim that every team's file is shaped identically
//! would have been wrong.
//!
//! # Only eight of the twelve teams are on the disc, and the other four are DLC
//!
//! `Auricom`, `Harimau`, `Icaras` and `Mantis` carry no `Ship.vex` - and so no
//! `shipboost.vex` either - on the **PSP** disc. They are not PS2-only, which
//! is what this file used to say: they are the four teams Pulse sold as
//! **downloadable content** for the PSP, and each one's ship is in its pack.
//! See `docs/formats/dlc-pack.md`, and `dlc_ground_truth.rs`, which loads all
//! four off the American disc with the European packs mounted.
//!
//! `Mantis` is why the old reading held for so long. The pack is sold as
//! Mirage, the folder is `Mantis`, and every search for `Mirage` on either
//! disc missed - so a team that is on the PS2 disc and in a PSP pack looked
//! absent from both.
//!
//! `Van_Uber` is the one name that was unaccounted for here, and it is a
//! *Pure* team rather than a Pulse one, confirmed inside Pure's Gamma Pack as
//! team id `Vanuber` (no underscore) - see `docs/formats/dlc-pack.md`.
//!
//! The PSP tests here assert only what a bare disc offers: the eight teams'
//! plume shape, and the four DLC teams' absence *from the disc itself*, by
//! name-hash miss rather than decode failure - see [`TEAMS`]. That absence is
//! now the evidence that they are downloadable, so it is worth keeping rather
//! than deleting. The PS2 tests read [`PS2_TEAMS`], all twelve, because that
//! disc ships all twelve outright.

use std::path::PathBuf;

use oag_mesh::mesh;
use oag_pulse as pulse;
use oag_vex::vex;

/// The PS2 release, Europe-only, `SCES-54748` - the same image
/// `ps2_source_ground_truth.rs` reads.
const PS2_IMAGE: &str = "pulse-ps2-eu.chd";

/// The full twelve-team roster the PS2 disc bundles, against [`TEAMS`]'s eight.
/// The four the PSP sold as downloadable content are on this disc outright -
/// see `docs/formats/dlc-pack.md`, and this file's own doc comment.
const PS2_TEAMS: [&str; 12] = [
    "AG_Systems",
    "Assegai",
    "Auricom",
    "EGX",
    "Feisar",
    "Goteki",
    "Harimau",
    "Icaras",
    "Mantis",
    "Piranha",
    "Qirex",
    "Triakis",
];

/// Both plume files every PS2 team ships, in the pairing
/// `oag_livery::entry::boost_entry_name` picks between on mode.
const PS2_STEMS: [&str; 2] = ["shipboost", "Zoneboost"];

fn image() -> Option<PathBuf> {
    image_named("pulse-psp-usa.chd")
}

fn image_named(file: &str) -> Option<PathBuf> {
    oag_testdata::image(file)
}

/// The eight teams the PSP disc actually ships a `Ship.vex` for - see this
/// file's own doc comment, and `docs/formats/handling-stats.md`'s independent
/// count of eight `handlingstats.xml` files. Not the full twelve-team roster
/// `ps2_source_ground_truth.rs`'s `TEAMS` checks: the PS2 disc bundles the four
/// teams the PSP sold as downloadable content.
const TEAMS: [&str; 8] = [
    "AG_Systems",
    "Assegai",
    "EGX",
    "Feisar",
    "Goteki",
    "Piranha",
    "Qirex",
    "Triakis",
];

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_psp_teams_boost_plume_decodes_with_two_meshes_and_its_texture() {
    let Some(image) = image() else {
        return;
    };
    let mut archives = pulse::open(&image.display().to_string()).expect("opening the PSP archives");

    for team in TEAMS {
        let name = format!(r"Data\Ships\{team}\shipboost.vex");
        let blob = archives
            .read_name(&name)
            .unwrap_or_else(|e| panic!("reading {name}: {e}"));
        let model = mesh::build_with_textures(&name, &blob, None)
            .unwrap_or_else(|e| panic!("decoding {name}: {e}"));
        println!(
            "{name}: {} mesh(es), {} triangle(s), {} texture slot(s), \
             {} opaque / {} cutout / {} transparent draw(s)",
            model.mesh_count,
            model.indices.len() / 3,
            model.textures.len(),
            model.draws.len(),
            model.alpha_tested_draws.len(),
            model.transparent_draws.len()
        );
        assert_eq!(
            model.mesh_count, 2,
            "{name} carries {} mesh(es), not the two the plume is built from",
            model.mesh_count
        );
        assert!(
            !model.textures.is_empty() && model.textures.iter().all(Option::is_some),
            "{name}: {} of {} texture slot(s) resolved",
            model.textures.iter().filter(|t| t.is_some()).count(),
            model.textures.len()
        );
        // `race::Scene`'s boost `Drawable` only reaches `exhaust::TRAIL_BLEND`
        // through `blend_pipeline`, which draws `Model::transparent_draws`.
        // If the authored `pass_mask` classified this model's batches as
        // opaque or alpha-tested instead, `exhaust::TRAIL_BLEND` never runs
        // and the plume draws through the ordinary opaque pipeline with no
        // blending at all - not merely the wrong (lerp) blend the plan's
        // trap warns about, but none.
        assert!(
            !model.transparent_draws.is_empty(),
            "{name}: {} opaque / {} cutout / 0 transparent draw(s) - \
             exhaust::TRAIL_BLEND never runs for this model",
            model.draws.len(),
            model.alpha_tested_draws.len()
        );
    }
}

/// Pins the boost plume's baked vertex alpha as bimodal - `0.0` or `1.0`
/// only, never a partial value - across every PSP team, not just the
/// Assegai sample an earlier pass measured by hand (63 vertices at
/// `[1,1,1,1]`, 58 at `[1.0, 0.384, 0.0196, 0.0]`). Guards against a
/// future re-export of the asset changing this shape and silently
/// re-breaking the falloff `mesh.wesl`'s `lit_texel` alpha fix depends on.
///
/// Exact float equality is safe here: `GpuVertex::colour` is decoded as
/// `byte as f32 / 255.0` (`oag_mesh::mesh`'s batch builder), which is
/// exact at both `0` and `255`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_psp_teams_boost_plume_vertex_alpha_is_bimodal() {
    let Some(image) = image() else {
        return;
    };
    let mut archives = pulse::open(&image.display().to_string()).expect("opening the PSP archives");

    for team in TEAMS {
        let name = format!(r"Data\Ships\{team}\shipboost.vex");
        let blob = archives
            .read_name(&name)
            .unwrap_or_else(|e| panic!("reading {name}: {e}"));
        let model = mesh::build_with_textures(&name, &blob, None)
            .unwrap_or_else(|e| panic!("decoding {name}: {e}"));

        let (mut zero, mut one, mut other) = (0usize, 0usize, Vec::new());
        for v in &model.vertices {
            match v.colour[3] {
                0.0 => zero += 1,
                1.0 => one += 1,
                a => other.push(a),
            }
        }
        assert!(
            other.is_empty(),
            "{name}: {} vertex/vertices carry a partial baked alpha (e.g. {:?}), not just \
             0.0/1.0 - the plume's falloff shape has changed since this test was written",
            other.len(),
            &other[..other.len().min(5)]
        );
        assert!(
            zero > 0 && one > 0,
            "{name}: expected both a fully-transparent and a fully-opaque baked vertex \
             bucket, got {zero} at 0.0 and {one} at 1.0"
        );
    }
}

/// The four DLC teams have no `shipboost.vex` on the PSP disc - not a decode
/// failure, an absent entry, which is what `Loaded::boost_model`'s `None` path
/// exists for.
///
/// **This is the disc on its own**, deliberately: with a pack mounted all four
/// resolve, and `dlc_ground_truth.rs` is where that is asserted. Keeping both
/// is what pins the pack as the thing that supplies them.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_four_dlc_teams_have_no_boost_plume_on_the_disc_alone() {
    let Some(image) = image() else {
        return;
    };
    let mut archives = pulse::open(&image.display().to_string()).expect("opening the PSP archives");

    for team in ["Auricom", "Harimau", "Icaras", "Mantis"] {
        let name = format!(r"Data\Ships\{team}\shipboost.vex");
        assert!(
            archives.read_name(&name).is_err(),
            "{name} resolved on the PSP disc; the eight-team PSP roster in \
             this file's TEAMS needs updating"
        );
    }
}

/// Pulse names a standalone boost model, unlike Pure - see
/// `pure_race_ground_truth.rs`'s sibling assertion for the other half of this
/// pairing. Regression for `oag_title::race::ShipPaths::boost` reaching
/// `livery::plume` correctly through the *full* load path: a title with the
/// seat filled must still find and draw its plume, not just report that the
/// seat exists.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulses_load_report_still_finds_its_boost_plume() {
    let Some(image) = image() else {
        return;
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        ..oag_raceplay::Options::default()
    })
    .expect("loading the race");
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("drawn additively while the plume is up")),
        "no boost plume line in the report: {:#?}",
        loaded.report
    );
}

/// Pins the finding in `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`:
/// every PSP team's `shipboost.vex` batches take `Mesh_SetBatchDrawState`'s
/// pure-additive (`Batch::is_additive_blend`) branch, never the "replace"
/// one. A separate one-off measurement during that pass found the same true
/// for a `Ship.vex` per team and both `01_Track`/`16_Track` - so this is not
/// plume-specific, but only the plume's blend pipeline choice
/// (`race::Scene`'s `exhaust::TRAIL_BLEND` override) depends on it, which is
/// why only the plume gets a permanent regression test. If a future
/// re-export ever ships a "replace"-branch batch here, that override stops
/// being correct for it and this test is what should catch that, not a
/// screenshot.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_psp_teams_boost_plume_batches_are_additive_blend() {
    let Some(image) = image() else {
        return;
    };
    let mut archives = pulse::open(&image.display().to_string()).expect("opening the PSP archives");

    for team in TEAMS {
        let name = format!(r"Data\Ships\{team}\shipboost.vex");
        let blob = archives
            .read_name(&name)
            .unwrap_or_else(|e| panic!("reading {name}: {e}"));
        let nodes = vex::nodes(&blob).expect("nodes");

        let mut checked = 0usize;
        for node in nodes.iter().filter(|n| n.class_id == vex::CLASS_MESH) {
            let payload = &blob[node.payload()];
            for list in 0..2u8 {
                let batches = vex::mesh_batches(payload, list)
                    .unwrap_or_else(|e| panic!("{name}: decoding batch list {list}: {e}"));
                for batch in batches {
                    assert!(
                        batch.is_additive_blend(),
                        "{name}: a batch (pass_mask {:#06x}, header_flags {:#04x}) takes \
                         Mesh_SetBatchDrawState's \"replace\" branch - exhaust::TRAIL_BLEND \
                         is no longer the right override for this model, see \
                         docs/ghidra/functions/psp-pulse-usa/mesh-draw.md",
                        batch.pass_mask,
                        batch.header_flags
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 0, "{name}: decoded zero batches to check");
    }
}

/// Pins the plume's texture coordinates as **authored**, not as an artefact of
/// how `vex::mesh_batches` reads them.
///
/// Each team's plume is four batches: two of 51 vertices and two of 9 or 10.
/// The short pair decodes to a *single* texture coordinate - `u` `0.0078`, `v`
/// `0.0312` on every one of their vertices - which reads exactly like a stride
/// or vertex-format misread. It is not one. The texcoord field of all 9 (or 10)
/// vertices is byte-identical in the file, the literal pair `01 04`, so no
/// stride, scale or format change can turn it into a sweep; the layout it is
/// read at is corroborated disc-wide by
/// `crates/formats/tests/vex_batch_layout_ground_truth.rs`.
///
/// The part worth pinning is what that means on the actual texture, because it
/// is **not** confined to the short batches. `pulse_boost2_ADD` is 64x16 and
/// ramps left to right, white to black. Every batch of the plume, long pair
/// included, carries `u <= 1/128` - texel column 0 - and column 0 is one flat
/// colour down all sixteen rows, so the long batches' `v` sweep
/// (`{4, 5, 114, 122} / 128`) lands on that same texel four times over. So the
/// short batches are not the only ones the baked coordinates give no texture
/// detail to; a stripe texture substituted for this one *does* band on the long
/// pair, but only because a stripe varies along `v` where this one does not,
/// which is what made the short batches look like the odd ones out.
///
/// **What the original samples on screen is not this, and that is now
/// settled.** These are the *baked* coordinates, and a transparent batch never
/// reads them: `Mesh_BeginTransparentPass` sets `TEXMAPMODE` uvgen 2 -
/// environment (shade) mapping from the vertex normal and lights 0/1 - and
/// nothing in the batch loop re-emits `0xc0` to undo it. So the ramp is not a
/// no-op on the original; the GE walks it with coordinates it generates per
/// vertex. This test therefore pins the *file*, not the picture, and the
/// reimplementation generates its own through
/// [`oag_render::texgen`](../../render/src/texgen.rs) - which is what turned
/// the plume from two flat orange wedges into streaks. See
/// `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`.
///
/// The `v` sweep landing on one texel is worth keeping for a second reason
/// this page did not have when it was written: at any column *other* than 0
/// the rows alternate hard between a bright magenta `(241, 110, 253)` and a
/// dark navy `(20, 5, 122)`, so `v` is the texture's streak pattern rather
/// than a second ramp, and column 0 is the one column where that alternation
/// vanishes.
///
/// If a future change to the decoder ever makes these coordinates sweep, this
/// test fails and the finding above needs re-deriving rather than the constants
/// here being widened.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_psp_teams_boost_plume_samples_one_texture_column() {
    /// Largest `u` any plume vertex carries, as the raw `u8` the file stores.
    /// `1 / 128` lands mid-texel in column 0 of a 64-wide texture.
    const MAX_U: f32 = 1.0 / 128.0;

    let Some(image) = image() else {
        return;
    };
    let mut archives = pulse::open(&image.display().to_string()).expect("opening the PSP archives");

    for team in TEAMS {
        let name = format!(r"Data\Ships\{team}\shipboost.vex");
        let blob = archives
            .read_name(&name)
            .unwrap_or_else(|e| panic!("reading {name}: {e}"));
        let nodes = vex::nodes(&blob).expect("nodes");

        let texture = vex::textures(&blob)
            .expect("textures")
            .into_iter()
            .flatten()
            .next()
            .unwrap_or_else(|| panic!("{name}: no embedded texture"));
        let width = usize::from(texture.width);
        let rgba = texture.to_rgba();
        let column: Vec<_> = (0..usize::from(texture.height))
            .map(|y| &rgba[y * width * 4..y * width * 4 + 4])
            .collect();
        assert!(
            column.iter().all(|c| *c == column[0]),
            "{name}: texel column 0 of the {}x{} texture is not one flat colour \
             ({:?}), so the plume's v sweep does reach texture content and this \
             test's premise is wrong",
            texture.width,
            texture.height,
            column,
        );

        // And the other columns are not flat, which is the whole point: `v` is
        // a streak pattern everywhere except at `u = 0`. Measured on Assegai at
        // column 20 - alternating `(241, 110, 253)` and `(20, 5, 122)` - and
        // asserted here as "more than one distinct row", which is the claim
        // `oag_render::texgen` rests on without pinning any team's exact art.
        let mid = width / 2;
        let mid_column: Vec<_> = (0..usize::from(texture.height))
            .map(|y| &rgba[(y * width + mid) * 4..(y * width + mid) * 4 + 4])
            .collect();
        assert!(
            mid_column.iter().any(|c| *c != mid_column[0]),
            "{name}: column {mid} of the {}x{} texture is flat down every row, so the \
             generated v coordinate has no streak pattern to land in and \
             oag_render::texgen's premise is wrong",
            texture.width,
            texture.height,
        );

        let mut short_batches = 0usize;
        let mut batches = 0usize;
        for node in nodes.iter().filter(|n| n.class_id == vex::CLASS_MESH) {
            let payload = &blob[node.payload()];
            for list in 0..2u8 {
                for batch in vex::mesh_batches(payload, list).expect("batches") {
                    batches += 1;
                    let uvs: Vec<[f32; 2]> =
                        batch.vertices.iter().filter_map(|v| v.texcoord).collect();
                    assert_eq!(
                        uvs.len(),
                        batch.vertices.len(),
                        "{name}: a batch decodes without texture coordinates"
                    );
                    let worst = uvs.iter().map(|t| t[0]).fold(0.0f32, f32::max);
                    assert!(
                        worst <= MAX_U,
                        "{name}: a batch reaches u {worst}, past column 0 of the \
                         texture - the plume now samples texture content and the \
                         finding this test pins has changed"
                    );
                    if batch.vertices.len() < 20 {
                        short_batches += 1;
                        assert!(
                            uvs.iter().all(|t| *t == uvs[0]),
                            "{name}: a {}-vertex batch carries more than one \
                             texture coordinate ({uvs:?}); the single authored \
                             point this test pins is gone",
                            batch.vertices.len()
                        );
                    }
                }
            }
        }
        assert_eq!(
            batches, 4,
            "{name}: {batches} batch(es), not the plume's four"
        );
        assert_eq!(
            short_batches, 2,
            "{name}: {short_batches} short batch(es), not the plume's two"
        );
    }
}

/// Every team's plume authors the **same** texture-transform keyframe track,
/// on both meshes: `u` ramping `2/256 -> 253/256` over key times 1..90
/// (60 Hz frames), `v` fixed at 0, scale constant `(1.0, 1.0)`.
///
/// This is the scroll `race::Scene` samples per frame and applies to the
/// plume's authored UVs - the recovered replacement for environment-mapped
/// generation, after the plume's compiled list was read to be replayed under
/// `TEXMAPMODE` 0. Mechanism and evidence:
/// `docs/ghidra/functions/psp-pulse-usa/texture-animation.md`, "The values
/// gap is closed"; on-disc layout: `docs/formats/vex.md`, "The
/// texture-transform keyframe block". The values here were verified live
/// against PSP RAM on Assegai; this test pins that the other seven teams
/// author the same track rather than a per-team variant.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_psp_teams_boost_plume_authors_the_same_uv_scroll_track() {
    let Some(image) = image() else {
        return;
    };
    let mut archives = pulse::open(&image.display().to_string()).expect("opening the PSP archives");

    for team in TEAMS {
        let name = format!(r"Data\Ships\{team}\shipboost.vex");
        let blob = archives
            .read_name(&name)
            .unwrap_or_else(|e| panic!("reading {name}: {e}"));
        let nodes = vex::nodes(&blob).expect("nodes");

        let mut meshes = 0usize;
        for node in nodes.iter().filter(|n| n.class_id == vex::CLASS_MESH) {
            let payload = &blob[node.payload()];
            let transform = vex::mesh_tex_transform(payload)
                .unwrap_or_else(|| panic!("{name}: a plume mesh carries no keyframe block"));
            assert_eq!(
                transform.offset.times,
                vec![1, 90],
                "{name}: offset key times differ from the recovered track"
            );
            assert_eq!(
                transform.offset.values,
                vec![(2, 0), (253, 0)],
                "{name}: offset key values differ from the recovered track"
            );
            assert_eq!(
                transform.scale.values,
                vec![(256, 256)],
                "{name}: the scale track is not the constant 1.0 the plume authors"
            );
            // The law the renderer runs: clamped at both ends, linear between.
            let at = |t: f32| transform.offset.sample(t).0;
            assert_eq!(at(0.0), 2.0 / 256.0, "{name}: below-first-key clamp");
            assert_eq!(at(90.0), 253.0 / 256.0, "{name}: past-end clamp");
            let mid = at(45.5);
            let expected = (2.0 + (253.0 - 2.0) * (44.5 / 89.0)) / 256.0;
            assert!(
                (mid - expected).abs() < 1e-6,
                "{name}: lerp at t=45.5 gives {mid}, expected {expected}"
            );
            assert_eq!(
                transform.offset.sample(45.5).1,
                0.0,
                "{name}: v moved; the plume's track holds it at 0"
            );
            meshes += 1;
        }
        assert_eq!(meshes, 2, "{name}: {meshes} mesh(es) with a track, not 2");
    }
}

/// The PS2 plume's shape, which is not the PSP plume's: two meshes hung off
/// two `Anim Transform` anchors, each carrying an authored scale track.
///
/// **This is what makes the model unusable through the PSP's path.**
/// `mesh::anim_node::placement` bakes geometry under an `Anim Transform` in
/// that node's own space and leaves the world placement to the shader's node
/// table - so a caller that never writes the table draws both meshes at the
/// craft's origin, on top of each other, several units ahead of the engines.
/// `race::scene::frame`'s draw loop is what writes it; this test is what says
/// the anchors it depends on are really there, on every team, rather than on
/// the one that was opened by hand.
///
/// The scale track is pinned too, and it is the reason that loop needs a clock
/// at all: the anchors flicker in `z` rather than sitting still. Its period is
/// what the clock choice was argued from - see the draw loop's comment for
/// what is measured and what is a reading.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_ps2_teams_boost_plume_hangs_two_meshes_off_two_animated_anchors() {
    let Some(image) = image_named(PS2_IMAGE) else {
        return;
    };
    let mut archives = pulse::open(&image.display().to_string()).expect("opening the PS2 archives");

    for team in PS2_TEAMS {
        for stem in PS2_STEMS {
            let name = format!(r"Data\Ships\{team}\{stem}.vex");
            let blob = archives
                .read_name(&name)
                .unwrap_or_else(|e| panic!("reading {name}: {e}"));
            let model = mesh::build_with_textures(&name, &blob, None)
                .unwrap_or_else(|e| panic!("decoding {name}: {e}"));

            assert_eq!(
                model.mesh_count, 2,
                "{name} carries {} mesh(es), not the two the plume is built from",
                model.mesh_count
            );
            assert_eq!(
                model.anim_nodes.len(),
                2,
                "{name} carries {} Anim Transform node(s), not the two its meshes hang \
                 off - race::scene::frame's node-anim write no longer places this plume",
                model.anim_nodes.len()
            );

            // The anchors are where the nozzles are, and "behind the craft" is
            // the part that matters: a bake that lost them would put the plume
            // at `z == 0`. Asserted as a sign and a bound rather than as
            // twelve exact triples, which would be shipped design data.
            let nodes = vex::nodes(&blob).expect("nodes");
            let placed: Vec<[f32; 16]> = vex::anchor_world(&blob, &nodes, 0.0)
                .into_iter()
                .flatten()
                .collect();
            assert_eq!(placed.len(), 2, "{name}: {} anchor(s) placed", placed.len());
            for anchor in &placed {
                assert!(
                    anchor[14] < -1.0,
                    "{name}: an anchor sits at z {:.3}, not behind the craft - the \
                     plume would draw through the hull",
                    anchor[14]
                );
            }

            for (index, anim) in model.anim_nodes.iter().enumerate() {
                // A PS2 plume is a `.vex` node; the rig form is 2048's and
                // has no business here.
                let oag_mesh::mesh::Motion::Vex(transform) = &anim.transform else {
                    panic!("{name}: anchor {index} is not a .vex Anim Transform");
                };
                assert!(
                    transform.rotation.is_empty(),
                    "{name}: anchor {index} carries a rotation track, which nothing here \
                     accounts for"
                );
                assert!(
                    !transform.scale.is_empty(),
                    "{name}: anchor {index} carries no scale track - the plume's authored \
                     flicker has gone, and the draw loop's clock has nothing left to drive"
                );
                // The clock argument in `race::scene::frame` rests on this
                // period outlasting a reveal, so the flicker plays one
                // monotonic pass and never wraps. `exhaust::PLUME_SECONDS` is
                // 1.5; a track that dropped below it would wrap mid-boost and
                // that argument would need re-making.
                assert!(
                    transform.loop_seconds > 1.5,
                    "{name}: anchor {index} loops at {:.4} s, inside the {:.1} s a plume \
                     stays up - see race::scene::frame on why the reveal timer was chosen",
                    transform.loop_seconds,
                    oag_fx::exhaust::PLUME_SECONDS,
                );
            }
        }
    }
}

/// The PS2 plume's pixels are the archive entry directly before it, and both
/// of its slots fill from that one entry.
///
/// A third model type answering to the directory-position rule
/// (`docs/formats/ps2-texture.md`), after the hull and the track - and the
/// first one asserted for *every* file rather than extrapolated from one,
/// because the rule is positional and each of these 24 entries has its own
/// neighbour.
///
/// **Both halves are asserted, and the second is the one that catches a
/// regression quietly.** That the preceding entry decodes says the lookup
/// found something; that rebuilding through it leaves no empty slot says the
/// ordinals lined up. A set shorter than the slot count would pass the first
/// and fail the second, and would draw a plume half-skinned rather than
/// visibly broken.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_ps2_teams_boost_plume_is_skinned_by_the_entry_before_it() {
    let Some(image) = image_named(PS2_IMAGE) else {
        return;
    };
    let mut archives = pulse::open(&image.display().to_string()).expect("opening the PS2 archives");

    for team in PS2_TEAMS {
        for stem in PS2_STEMS {
            let name = format!(r"Data\Ships\{team}\{stem}.vex");
            let blob = archives
                .read_name(&name)
                .unwrap_or_else(|e| panic!("reading {name}: {e}"));
            let bare = mesh::build_with_textures(&name, &blob, None)
                .unwrap_or_else(|e| panic!("decoding {name}: {e}"));

            // The gate `livery::plume` takes this branch on. A PS2 model
            // declares its texture nodes and embeds none of their pixels; a
            // PSP one embeds all of them and must never reach here.
            assert!(
                !bare.textures.is_empty() && bare.textures.iter().all(Option::is_none),
                "{name}: {} of {} texture slot(s) came out of the file itself, so this is \
                 not the empty-block PS2 signature livery::ps2_skin gates on",
                bare.textures.iter().filter(|t| t.is_some()).count(),
                bare.textures.len()
            );

            let preceding = archives
                .read_preceding(&name)
                .unwrap_or_else(|e| panic!("{name}: reading the entry before it: {e}"));
            let set = mesh::Ps2TextureSet::parse(&preceding).unwrap_or_else(|e| {
                panic!("{name}: the entry before it is not a texture set ({e})")
            });
            assert_eq!(
                set.entry_count(),
                bare.textures.len(),
                "{name}: the preceding entry holds {} texture(s) against {} declared \
                 slot(s) - the ordinals a material indexes would not line up",
                set.entry_count(),
                bare.textures.len()
            );

            let skinned = mesh::build_with_textures(&name, &blob, Some(&set))
                .unwrap_or_else(|e| panic!("re-skinning {name}: {e}"));
            assert!(
                skinned.textures.iter().all(Option::is_some),
                "{name}: {} of {} slot(s) still empty after re-skinning - the plume draws \
                 partly untextured",
                skinned.textures.iter().filter(|t| t.is_none()).count(),
                skinned.textures.len()
            );
        }
    }
}

/// The PS2 plume's batches are classified **opaque**, and this is the premise
/// `race::Drawable::draw_additive` exists to override rather than a licence to
/// draw them opaque.
///
/// The classification is real: `pass_mask & 0x0700 == 0` on all four, so
/// `Model::transparent_draws` is empty and the ordinary draw path would submit
/// them through the opaque pipeline. It is also the exact opposite of what
/// `every_psp_teams_boost_plume_decodes_with_two_meshes_and_its_texture`
/// asserts for the PSP model, which is why the PSP-shaped expectation cannot
/// simply be reused here.
///
/// **What the classification does *not* settle is how the original blends
/// it.** A PCSX2 capture of a real PS2 boost shows soft plumes with no geometry
/// edge and the hull visible through them; drawn opaque, the same model puts a
/// hard hexagon over each nozzle. So the original blends it, and the dispatch
/// that says so has not been found - see
/// `docs/ghidra/functions/ps2-pulse-eu/batch-draw-state.md`, which records both
/// the reading and its refutation.
///
/// **`pass_mask & 0xc0` is the part that survived**: the PS2 plume sets it and
/// the PSP one does not, and on the original it stamps the alpha channel
/// through `KEEP, KEEP, REPLACE`, which the bloom reads. That is why
/// `race::Scene` giving the plume `GlowMask::Written` is right on both discs.
///
/// Pinned rather than left to a screenshot so that the override above keeps a
/// stated premise: if a re-export ever tags these batches `0x200`, this test
/// fails and `draw_additive` becomes unnecessary rather than silently
/// redundant.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_ps2_teams_boost_plume_is_authored_opaque_and_glow_masked() {
    let Some(image) = image_named(PS2_IMAGE) else {
        return;
    };
    let mut archives = pulse::open(&image.display().to_string()).expect("opening the PS2 archives");

    for team in PS2_TEAMS {
        for stem in PS2_STEMS {
            let name = format!(r"Data\Ships\{team}\{stem}.vex");
            let blob = archives
                .read_name(&name)
                .unwrap_or_else(|e| panic!("reading {name}: {e}"));
            let model = mesh::build_with_textures(&name, &blob, None)
                .unwrap_or_else(|e| panic!("decoding {name}: {e}"));
            assert!(
                model.transparent_draws.is_empty() && model.alpha_tested_draws.is_empty(),
                "{name}: {} transparent / {} cutout draw(s) - the PS2 plume used to be \
                 wholly opaque-classified, which is the premise Drawable::draw_additive \
                 overrides; if that changed, the override may no longer be needed",
                model.transparent_draws.len(),
                model.alpha_tested_draws.len()
            );
            assert!(
                !model.draws.is_empty(),
                "{name}: no opaque draw(s) at all, so nothing draws"
            );

            let classes = vex::classes_of(&blob).expect("class table");
            let nodes = vex::nodes(&blob).expect("nodes");
            let mut checked = 0usize;
            for node in nodes.iter().filter(|n| Some(n.class_id) == classes.mesh) {
                let payload = &blob[node.payload()];
                for list in 0..2u8 {
                    for batch in vex::mesh_batches(payload, list)
                        .unwrap_or_else(|e| panic!("{name}: batch list {list}: {e}"))
                    {
                        assert!(
                            !batch.is_transparent(),
                            "{name}: a batch carries a 0x0700 class bit (pass_mask {:#06x}), \
                             so it would pick its own blend class and no longer need \
                             Drawable::draw_additive's override",
                            batch.pass_mask
                        );
                        assert!(
                            batch.pass_mask & 0x00c0 != 0,
                            "{name}: a batch (pass_mask {:#06x}) does not set the glow-mask \
                             bits - without them the plume is opaque geometry the bloom never \
                             picks up, which is a solid hexagon and nothing else",
                            batch.pass_mask
                        );
                        assert!(
                            batch.is_additive_blend(),
                            "{name}: a batch (header_flags {:#04x}) takes the \"replace\" \
                             branch rather than the additive one",
                            batch.header_flags
                        );
                        checked += 1;
                    }
                }
            }
            assert!(checked > 0, "{name}: decoded zero batches to check");
        }
    }
}
