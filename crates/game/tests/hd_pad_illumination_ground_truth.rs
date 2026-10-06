//! Speed pad and weapon pad geometry on Wipeout HD, off a real disc image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all hd_pad_illumination
//! ```
//!
//! # What this pins that PSP's own pad test does not
//!
//! `crates/game/tests/race_ground_truth.rs::the_weapon_pads_are_drawn_where_they_trigger`
//! already checks a PSP track's pad geometry against its trigger volumes. HD
//! needed a second pass entirely - `mesh::rcs::build_pads`/`build_weapon_pads`
//! read a `.rcsmodel` chunk through a node-ordered walk that has nothing in
//! common with PSP's inline `.vex` mesh payload - so this file is that same
//! check run against HD's own builder, plus the failure mode unique to it:
//! a pad chunk is, structurally, also a chunk `mesh::rcs::build_scene`'s
//! world-space pass would draw on its own unless explicitly excluded. See
//! [`hd_pads_are_not_drawn_twice`].

use std::collections::HashSet;
use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_mesh::mesh;
use oag_raceplay as race;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

/// A single race on the default track (Talon's Junction) - the one mode
/// HD's own pads draw and trigger in, see `Mode::weapons_enabled`.
fn load() -> Option<race::Loaded> {
    load_track(None)
}

/// [`load`], on `track` instead of the default when given one. See
/// [`the_speedup_pads_are_drawn_where_they_trigger_on_a_circuit_that_bakes_them`]
/// for why the speed pad check needs this and the weapon pad one does not.
fn load_track(track: Option<&str>) -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: track.map(str::to_string),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// Every trigger volume in `volumes` has drawn geometry sitting in it. Ten
/// units of slack because a pad's mesh is a flat plate and its volume is a
/// box around the craft that crosses it, so the two are the same *place*
/// rather than the same extent - the same check and the same tolerance
/// `race_ground_truth.rs::the_weapon_pads_are_drawn_where_they_trigger` uses
/// for PSP.
fn assert_pads_drawn_where_they_trigger(
    label: &str,
    model: &mesh::Model,
    volumes: &[oag_vex::pads::PadVolume],
) {
    assert!(!model.indices.is_empty(), "{label}: no geometry drawn");
    assert!(!volumes.is_empty(), "{label}: the track decodes no volumes");
    println!(
        "{label}: {} volume(s), {} triangles drawn",
        volumes.len(),
        model.indices.len() / 3
    );
    for (index, pad) in volumes.iter().enumerate() {
        let centre = Vec3::from_array(pad.centre());
        let nearest = model
            .vertices
            .iter()
            .map(|v| (Vec3::from_array(v.position) - centre).length())
            .fold(f32::INFINITY, f32::min);
        assert!(
            nearest < 10.0,
            "{label} {index} triggers at {centre:?} and the nearest drawn \
             vertex is {nearest:.2} away"
        );
    }
}

/// The HD counterpart of PSP's own weapon-pad test. `mesh::rcs::build_weapon_pads`
/// reads a `.rcsmodel` chunk through a pass entirely separate from
/// `mesh::rcs::build_scene`'s own, so a wrong hash lookup or a wrong
/// assumption about the chunk's own space (see `mesh::rcs::is_world_baked`)
/// would show up here exactly as a wrong transform would on PSP.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_weapon_pads_are_drawn_where_they_trigger_on_hd() {
    let Some(loaded) = load() else { return };
    let model = loaded
        .weapon_pad_model
        .as_ref()
        .expect("Talon's Junction authors Weapon Pad geometry");
    assert_pads_drawn_where_they_trigger("HD weapon pad", model, &loaded.setup.weapon_pads);
}

/// The same check for `Speedup Pad` rather than `Weapon Pad`, on `12_sol_2`
/// rather than the default Talon's Junction - see
/// [`talons_junction_speedup_pad_chunks_are_an_unbaked_content_donor`] for why
/// Talon's Junction itself cannot be this test. PSP's own speed pads have no
/// equivalent test because `mesh::build_pads` shares its whole implementation
/// with `mesh::build_weapon_pads`, where HD's two pad classes are each built
/// by their own call into `mesh::rcs::build_pad_class`.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_speedup_pads_are_drawn_where_they_trigger_on_a_circuit_that_bakes_them() {
    let Some(loaded) = load_track(Some("/data/environments/12_sol_2/track.vex")) else {
        return;
    };
    let model = loaded
        .pad_model
        .as_ref()
        .expect("12_sol_2 authors Speedup Pad geometry");
    assert_pads_drawn_where_they_trigger("HD speedup pad", model, &loaded.setup.speedup_pads);
}

/// **Talon's Junction's own `Speedup Pad` chunks are a content donor its own
/// file never baked** - see `docs/formats/hd-status.md`'s section of the
/// same name for the finding in full, including the second-circuit check
/// that rules out a reader bug. Pinned here rather than left as a paragraph:
/// all 18 nodes read a well-formed chunk hash and none resolves against
/// `talons_junction/track.rcsmodel`, where `Weapon Pad`'s own 9 resolve in
/// full on the same file. `Loaded::pad_model` is `None` as a result - an
/// honest absence, not an invented plate - and this is the test that turns
/// "found nothing" from a silent gap into a fact this project can notice
/// changing, the moment a donor circuit or a second `.rcsmodel` closes it.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn talons_junction_speedup_pad_chunks_are_an_unbaked_content_donor() {
    let Some(loaded) = load() else { return };
    assert!(
        !loaded.setup.speedup_pads.is_empty(),
        "the trigger volumes should still be there even though the geometry is not"
    );
    assert!(
        loaded.pad_model.is_none(),
        "Talon's Junction's Speedup Pad chunks resolved - the finding this test pins may have \
         closed; update it and docs/formats/hd-status.md together rather than deleting this test"
    );
}

/// **An HD pad stays on the circuit's own lighting path, and nothing
/// recolours it.** This is the regression that shipped on 2026-09-02 and was
/// reverted on 2026-09-03: to make a flat invented tint *visible* on HD, both
/// pad models were taken off the authored branch (`vertex_colour_is_light`
/// forced `false`, every vertex's `lit` forced `0.0`), which is what routes
/// `mesh.wesl` to its stand-in shading where vertex colour multiplies. A pad
/// lit by its own baked light rig then read as a flat plate in a colour the
/// disc does not author anywhere.
///
/// Three facts pinned, each of which alone would have caught it:
///
/// 1. `vertex_colour_is_light` is `true`, as on every other PS3 model.
/// 2. Every pad vertex is `lit`, as on every other PS3 model.
/// 3. The authored colour set is what the file carries - a flat `0,0,0` on
///    all 18 chunks across both classes, HD's "this chunk is lightmapped, it
///    has no vertex light of its own" value. A tint pass writing a palette
///    entry into that slot shows up here immediately.
///
/// What a pad *looks* like on HD is the texture's own job: `ds_speedup_cs`
/// paints the blue chevron and `ds_weaponup_cs` the red cross. See
/// `docs/rendering/pads.md`.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_pads_keep_their_authored_light_and_are_never_recoloured() {
    let Some(loaded) = load_track(Some("/data/environments/12_sol_2/track.vex")) else {
        return;
    };
    for (label, model) in [
        ("speedup", &loaded.pad_model),
        ("weapon", &loaded.weapon_pad_model),
    ] {
        let model = model
            .as_ref()
            .unwrap_or_else(|| panic!("12_sol_2 authors {label} pad geometry"));
        assert!(
            model.vertex_colour_is_light,
            "{label} pads left HD's authored lighting path - `colour` is a light \
             term there, not a tint, see Drawable::tint_weapon_pads"
        );
        assert!(
            model.vertices.iter().all(|v| v.lit == 1.0),
            "{label} pads were forced onto mesh.wesl's stand-in shading branch"
        );
        assert!(
            model
                .vertices
                .iter()
                .all(|v| v.colour[..3] == [0.0, 0.0, 0.0]),
            "{label} pads no longer carry the file's own flat colour set - either \
             the reader changed or something recoloured them"
        );
    }
}

/// **The double-draw guard.** `mesh::rcs::build_scene`'s world-space pass
/// draws every `.rcsmodel` chunk no `Mesh`-class node references, and a
/// `Weapon Pad`/`Speedup Pad` chunk is exactly such a chunk unless
/// `build_scene` excludes it on purpose - see `mesh::rcs::pad_chunk_hashes`.
/// If that exclusion ever regressed, every HD pad would draw twice: once
/// through its own node-ordered pass and once more as part of the plain
/// track model - coincident geometry that would look almost right and
/// differ only by draw order, which is
/// exactly the failure mode `crates/render/examples/hd_double_submit_check.rs`
/// exists for on the *ordinary* `Mesh` side of this same mechanism.
///
/// Checked by chunk identity rather than position: `loaded.track_model` and
/// both pad models are built from the same `.rcsmodel`, so a `DrawCall`'s
/// `chunk` index means the same chunk in every one of them, and a pad chunk
/// index appearing in the track model's own draw lists is unambiguous
/// evidence of a double submit - unlike a position-based check, which could
/// not tell a duplicated pad plate from the ordinary road geometry a pad
/// always sits on top of.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_pads_are_not_drawn_twice() {
    let Some(loaded) = load() else { return };
    let pad_chunks: HashSet<u32> = [&loaded.pad_model, &loaded.weapon_pad_model]
        .into_iter()
        .flatten()
        .flat_map(|model| {
            model
                .draws
                .iter()
                .chain(&model.alpha_tested_draws)
                .chain(&model.transparent_draws)
        })
        .filter_map(|draw| draw.chunk)
        .collect();
    assert!(!pad_chunks.is_empty(), "no pad chunk was drawn at all");

    let track_chunks: Vec<u32> = loaded
        .track_model
        .draws
        .iter()
        .chain(&loaded.track_model.alpha_tested_draws)
        .chain(&loaded.track_model.transparent_draws)
        .filter_map(|draw| draw.chunk)
        .collect();
    let doubled: Vec<u32> = track_chunks
        .iter()
        .filter(|c| pad_chunks.contains(c))
        .copied()
        .collect();
    assert!(
        doubled.is_empty(),
        "{} chunk(s) drawn both as track geometry and as a pad: {doubled:?}",
        doubled.len()
    );
}
