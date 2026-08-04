//! Decodes **every PSP team's** `shipboost.vex` off the disc and checks what
//! came out - the plume model `oag_game::race::load` reads beside `Ship.vex`.
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
//! `oag_render::mesh::build_with_textures` path `oag_game::race::load` uses.
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
//! # Only eight of the thirteen candidate teams
//!
//! `Auricom`, `Harimau` and `Icaras` carry no `Ship.vex` - and so no
//! `shipboost.vex` either - on the **PSP** disc at all; confirmed PS2-only
//! (they resolve on that disc - see `ps2_source_ground_truth.rs`'s own
//! `TEAMS`). Two more candidate names, `Mirage` and `Van_Uber`, resolve on
//! **neither** disc under this project's current name-mining - an open gap,
//! not evidence either way about which platform (or neither) ships them; see
//! `docs/formats/handling-stats.md` and `exhaust.md`'s "eight teams" note for
//! the same eight-of-thirteen count independently arrived at over
//! `handlingstats.xml` and `Engine Flare`. This file only asserts what is
//! actually established: the eight PSP-resolving teams' plume shape, and the
//! three *confirmed* PS2-only teams' absence - not the other two's status,
//! which stays open. Confirmed directly here too: querying any of the three
//! confirmed-absent teams' `Ship.vex` by name hash comes back "no entry
//! hashing to ...", not a decode failure - see [`TEAMS`].

use std::path::{Path, PathBuf};

use oag_assets::pulse;
use oag_formats::vex;
use oag_render::mesh;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// The eight teams the PSP disc actually ships a `Ship.vex` for - see this
/// file's own doc comment, and `docs/formats/handling-stats.md`'s independent
/// count of eight `handlingstats.xml` files. Not the full eleven-team roster
/// `ps2_source_ground_truth.rs`'s `TEAMS` checks, which is PS2-specific.
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
    let mut archives =
        pulse::Archives::open(&image.display().to_string()).expect("opening the PSP archives");

    for team in TEAMS {
        let name = format!(r"Data\Ships\{team}\shipboost.vex");
        let blob = archives
            .read_name(&name)
            .unwrap_or_else(|e| panic!("reading {name}: {e}"));
        let model = mesh::build_with_textures(&name, &blob, None, mesh::Lod::Both)
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
/// re-breaking the falloff `mesh.wgsl`'s `lit_texel` alpha fix depends on.
///
/// Exact float equality is safe here: `GpuVertex::colour` is decoded as
/// `byte as f32 / 255.0` (`oag_render::mesh`'s batch builder), which is
/// exact at both `0` and `255`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_psp_teams_boost_plume_vertex_alpha_is_bimodal() {
    let Some(image) = image() else {
        return;
    };
    let mut archives =
        pulse::Archives::open(&image.display().to_string()).expect("opening the PSP archives");

    for team in TEAMS {
        let name = format!(r"Data\Ships\{team}\shipboost.vex");
        let blob = archives
            .read_name(&name)
            .unwrap_or_else(|e| panic!("reading {name}: {e}"));
        let model = mesh::build_with_textures(&name, &blob, None, mesh::Lod::Both)
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

/// The three PS2-only teams have no `shipboost.vex` on the PSP disc - not a
/// decode failure, an absent entry, which is what `Loaded::boost_model`'s
/// `None` path exists for.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_three_ps2_only_teams_have_no_psp_boost_plume() {
    let Some(image) = image() else {
        return;
    };
    let mut archives =
        pulse::Archives::open(&image.display().to_string()).expect("opening the PSP archives");

    for team in ["Auricom", "Harimau", "Icaras"] {
        let name = format!(r"Data\Ships\{team}\shipboost.vex");
        assert!(
            archives.read_name(&name).is_err(),
            "{name} resolved on the PSP disc; the eight-team PSP roster in \
             this file's TEAMS needs updating"
        );
    }
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
    let mut archives =
        pulse::Archives::open(&image.display().to_string()).expect("opening the PSP archives");

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
