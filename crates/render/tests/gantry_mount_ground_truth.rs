//! Measures the start gantry's mount on every circuit that authors one.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this test:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all gantry
//! ```
//!
//! # What it is for
//!
//! `docs/rendering/start-gantry.md` has the gantry's mount as a **centroid**
//! on two circuits, `(34,-36,-185)` on `16_Track` and `(-37,-2,-219)` on
//! `05_Track`, with the ~165-unit distance to each file's own `Start Position`
//! repeating to within 2 units as the load-bearing signal. A centroid is not a
//! basis and two circuits are not eleven, so this measures the whole disc:
//! per circuit, the surface's plane, its extent, its distance and bearing from
//! the start line, and whether the two co-located textures agree.
//!
//! It **prints** far more than it asserts, deliberately. The assertions are
//! the ones that would invalidate `oag_render::gantry`'s design rather than
//! merely disappoint it: that the surface is a plane at all, that the two
//! textures name the same one, and that the mount is found on more than the
//! two circuits it was discovered on.

use std::path::PathBuf;

use oag_core::math::{Mat4, Vec3};
use oag_render::gantry::{self, BACKPLATE_TEXTURE, SLOT8_TEXTURE};
use oag_render::mesh::{self, Lod};
use oag_vex::vex;

/// The gantry model slot 8 names on every circuit that authors a manifest.
const GANTRY: &str = r"Data\Environments\321_Go\321Go_StartFinish.vex";

/// Track directories to probe, a superset - ids not on the disc are skipped.
const TRACK_IDS: &[&str] = &[
    "01_Track", "02_Track", "03_Track", "04_Track", "05_Track", "06_Track", "07_Track", "08_Track",
    "09_Track", "10_Track", "11_Track", "12_Track", "13_Track", "14_Track", "15_Track", "16_Track",
    "17_Track", "18_Track", "19_Track", "20_Track",
];

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

fn degrees_between(a: Vec3, b: Vec3) -> f32 {
    let d = a
        .normalize_or_zero()
        .dot(b.normalize_or_zero())
        .clamp(-1.0, 1.0);
    d.acos().to_degrees()
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_circuit_authors_the_same_gantry_mount() {
    let Some(image) = image("pulse-psp-usa.chd") else {
        return;
    };
    let spec = image.display().to_string();
    let mut archives = oag_pulse::open(&spec).expect("opening archives");

    // The model first: its own board plane is what a mount's basis has to be
    // matched against, and it is the same file on every circuit.
    let blob = archives.read_name(GANTRY).expect("the gantry model");
    let model =
        mesh::build_with_textures(GANTRY, &blob, None, Lod::default()).expect("decoding it");
    println!(
        "{GANTRY}: {} tri, radius {:.2}, centre {:?}",
        model.indices.len() / 3,
        model.radius,
        model.centre
    );
    let (lo, hi) = world_bounds(&model, 0.0);
    println!(
        "  world extents at t=0: x {:.2}..{:.2} ({:.2} wide)  y {:.2}..{:.2} ({:.2} tall)  \
         z {:.2}..{:.2} ({:.2} deep)",
        lo.x,
        hi.x,
        hi.x - lo.x,
        lo.y,
        hi.y,
        hi.y - lo.y,
        lo.z,
        hi.z,
        hi.z - lo.z,
    );
    for (index, name) in node_names(&blob) {
        let Some(plane) = gantry::node_plane(&model, index) else {
            continue;
        };
        println!(
            "  node {index:>3} {name:<44} centre {:>7.2},{:>7.2},{:>7.2}  \
             {:>6.2} wide x {:>6.2} tall x {:>5.2} deep  normal {:>6.3},{:>6.3},{:>6.3}",
            plane.centre.x,
            plane.centre.y,
            plane.centre.z,
            plane.width,
            plane.height,
            plane.thickness,
            plane.normal.x,
            plane.normal.y,
            plane.normal.z,
        );
    }
    let glyphs = glyph_plane(&model, &blob).expect("the 321go glyph node has a plane");
    println!(
        "  glyph board: centre {:.2},{:.2},{:.2}  normal {:.3},{:.3},{:.3}  \
         outward {:.3},{:.3},{:.3}  {:.2} wide x {:.2} tall x {:.2} deep",
        glyphs.centre.x,
        glyphs.centre.y,
        glyphs.centre.z,
        glyphs.normal.x,
        glyphs.normal.y,
        glyphs.normal.z,
        glyphs.outward.x,
        glyphs.outward.y,
        glyphs.outward.z,
        glyphs.width,
        glyphs.height,
        glyphs.thickness,
    );

    let mut found = 0usize;
    let mut agreements = Vec::new();
    for id in TRACK_IDS {
        let entry = format!(r"Data\Environments\{id}\track.vex");
        let Ok(blob) = archives.read_name(&entry) else {
            continue;
        };
        let track =
            mesh::build_with_textures(&entry, &blob, None, Lod::default()).expect("decoding");
        let backplate = gantry::surface(&track, BACKPLATE_TEXTURE);
        let stub = gantry::surface(&track, SLOT8_TEXTURE);
        let Some(mount) = backplate.or(stub) else {
            println!("{id}: no backplate and no slot-8 stub - nothing to mount a gantry on");
            continue;
        };
        found += 1;

        let start = start_position(&blob);
        let (distance, above, bearing) = match &start {
            Some(s) => {
                let p = Vec3::from(s.position);
                let f = Vec3::from(s.forward);
                let delta = mount.centre - p;
                (
                    delta.length(),
                    mount.centre.y - p.y,
                    delta.normalize_or_zero().dot(f),
                )
            }
            None => (f32::NAN, f32::NAN, f32::NAN),
        };
        println!(
            "{id}: node {:?}  centre {:.1},{:.1},{:.1}  {} verts\n  \
             normal {:.3},{:.3},{:.3}  up {:.3},{:.3},{:.3}  \
             {:.2} wide x {:.2} tall x {:.3} deep\n  \
             from Start Position: {distance:.1} units, {above:.1} above, \
             ahead-ness {bearing:+.2}",
            mount.node,
            mount.centre.x,
            mount.centre.y,
            mount.centre.z,
            mount.vertices,
            mount.normal.x,
            mount.normal.y,
            mount.normal.z,
            mount.up.x,
            mount.up.y,
            mount.up.z,
            mount.width,
            mount.height,
            mount.thickness,
        );
        if let Some(s) = &start {
            let forward = Vec3::from(s.forward);
            println!(
                "  plane vs start-line forward: {:.1} deg between the resolved normal and \
                 forward; authored vertex normals {:.1} deg off it",
                degrees_between(mount.facing(forward), forward),
                degrees_between(mount.outward, forward),
            );
        }

        // The open item from the previous pass: two textures a unit or so
        // apart, and whether the choice between them matters for the *basis*.
        if let (Some(a), Some(b)) = (backplate, stub) {
            let gap = (a.centre - b.centre).length();
            let raw = degrees_between(a.normal, b.normal);
            let tilt = raw.min(180.0 - raw);
            println!(
                "  backplate vs billboard8: centres {gap:.2} apart, planes {tilt:.2} deg apart"
            );
            agreements.push((id.to_string(), gap, tilt));
        }
        // **The claim `race::gantry` makes about scale.** The gantry is drawn
        // at 1.0, so the panel the track authors has to be able to hold the
        // model's own countdown board. That is the *glyph* board - 34.02
        // across - and not the model's widest piece: `polySurfaceShape7`, the
        // chequered state, is 43.25 and overhangs a narrow panel. It is one of
        // the pieces `gantry::clip_to_panel` drops, so its overhang never
        // reaches the screen, but the distinction belongs in the assertion.
        println!(
            "  panel / glyph board: {:.3}  (panel {:.2} against 34.02 of glyphs)",
            mount.width / glyphs.width,
            mount.width,
        );
        assert!(
            mount.width >= glyphs.width,
            "{id}: the authored panel is {:.2} wide and the gantry's own countdown board \
             is {:.2} - it cannot be drawn at 1.0 here",
            mount.width,
            glyphs.width,
        );
        // Which parts of the model the panel keeps, so a circuit whose panel
        // clips something it should not is visible in the sweep rather than in
        // a screenshot.
        let mut kept = model.clone();
        let dropped = gantry::clip_to_panel(&mut kept, mount.width / 2.0, 0.0);
        println!(
            "  clip to panel: {dropped} of {} draw(s) parked outside it",
            model.draws.len() + model.alpha_tested_draws.len() + model.transparent_draws.len(),
        );
        // Every circuit measures 6 or 7 of the model's 15 - the parked
        // `FINAL LAP` and chequered states, plus on some circuits one further
        // draw whose own extent straddles the panel edge. The band is wider
        // than that so it reads as "the parked states and nothing like the
        // whole model" rather than pinning this asset's exact draw count.
        assert!(
            (5..=8).contains(&dropped),
            "{id}: clipping to a {:.2}-wide panel dropped {dropped} of 15 draws - the \
             parked FINAL LAP and chequered states are 6 or 7 of them and nothing else \
             should go",
            mount.width,
        );
        // The panel has to actually be a panel for its normal to mean
        // anything - a fit over a lump reports a normal just as confidently.
        assert!(
            mount.thickness < mount.width * 0.35,
            "{id}: {:.2} deep against {:.2} wide is not a panel",
            mount.thickness,
            mount.width
        );
        // **What `gantry::mount` must return, and the invariant a reversed
        // preference would break.** `14_Track` puts its `billboard8` stub 354
        // units and 54 degrees away from the backplate, so the two are not
        // interchangeable and the stub is not a safe fallback: a build that
        // reached for it first would stand that circuit's gantry somewhere
        // else entirely.
        if let Some(backplate) = backplate {
            let chosen = gantry::mount(&track).expect("a mount was found above");
            assert!(
                (chosen.centre - backplate.centre).length() < 1e-3,
                "{id}: `mount` returned {:?}, not the backplate's {:?}",
                chosen.centre,
                backplate.centre,
            );
        }
    }

    println!("mount found on {found} circuit(s)");
    assert!(
        found > 2,
        "only {found} circuit(s) author a mount - the two it was discovered on are not a convention"
    );
    // **The two co-located surfaces are not interchangeable, and the disc
    // says which.** On eleven circuits the stub sits a unit or two from the
    // backplate on the same plane, so the choice would be noise; on
    // `14_Track` the two are hundreds of units and tens of degrees apart, so
    // a build that reached for `billboard8.tga` first would put that
    // circuit's gantry somewhere else entirely. `gantry::mount` prefers the
    // backplate for this reason, and this loop is what would catch that
    // preference being reversed.
    let disagreeing: Vec<&(String, f32, f32)> = agreements
        .iter()
        .filter(|(_, _, tilt)| *tilt > 5.0)
        .collect();
    println!(
        "{} of {} circuit(s) place the stub on the backplate's own plane; \
         disagreeing: {:?}",
        agreements.len() - disagreeing.len(),
        agreements.len(),
        disagreeing.iter().map(|(id, ..)| id).collect::<Vec<_>>(),
    );
    assert!(
        !disagreeing.is_empty(),
        "the stub agrees with the backplate everywhere - `gantry::mount`'s preference \
         for the backplate is then untested by this sweep, and the reason it exists \
         (`14_Track`) has gone missing from the disc or from the measurement",
    );
}

/// The model's own axis-aligned extents at `seconds`, `Anim Transform`s applied.
///
/// Every mesh in `321Go_StartFinish.vex` hangs off an `Anim Transform`, so its
/// vertices are baked in node space and `Model::centre`/`radius` describe none
/// of them. Sampling the node table is the only way to ask how big the object
/// actually is - which is the question a scale has to be answered from.
fn world_bounds(model: &mesh::Model, seconds: f32) -> (Vec3, Vec3) {
    let matrices = model.sample_anim_nodes(seconds);
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for v in &model.vertices {
        let p = Vec3::from(v.position);
        let p = match v.xform.checked_sub(1) {
            Some(slot) => match matrices.get(slot as usize) {
                Some(m) => Mat4::from_cols_array(m).transform_point3(p),
                None => p,
            },
            None => p,
        };
        lo = lo.min(p);
        hi = hi.max(p);
    }
    (lo, hi)
}

/// The plane of the gantry model's own `321go` glyph node.
///
/// Matched by node name rather than by texture, because `321go_NOMIP.tga` is
/// the palette for the `FINAL LAP` board and the backlight too. **Matched on
/// the node that actually draws**, not the first node whose name matches: the
/// glyph mesh sits under an `Anim Transform` of the same name, and picking
/// that one finds a node with no draws at all.
fn glyph_plane(model: &mesh::Model, blob: &[u8]) -> Option<gantry::Mount> {
    let nodes = vex::nodes(blob).ok()?;
    let node = model
        .draws
        .iter()
        .chain(&model.transparent_draws)
        .find_map(|draw| {
            let index = draw.node?;
            let name = nodes.get(index as usize)?.name.as_deref()?;
            name.contains("start_light_321go").then_some(index)
        })?;
    gantry::node_plane(model, node)
}

/// Every node index that has a name, in file order.
fn node_names(blob: &[u8]) -> Vec<(u32, String)> {
    vex::nodes(blob)
        .map(|nodes| {
            nodes
                .iter()
                .enumerate()
                .filter_map(|(i, n)| Some((u32::try_from(i).ok()?, n.name.clone()?)))
                .collect()
        })
        .unwrap_or_default()
}

fn start_position(blob: &[u8]) -> Option<oag_vex::track::StartPosition> {
    let nodes = vex::nodes(blob).ok()?;
    let class = vex::classes_of(blob).ok()?.start_position?;
    let node = nodes.iter().find(|node| node.class_id == class)?;
    oag_vex::track::start_position(blob.get(node.payload())?, vex::byte_order(blob))
}
