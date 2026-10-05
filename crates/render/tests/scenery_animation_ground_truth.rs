//! What animates trackside on a Pulse circuit, and what the renderer reaches.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(scenery_animation_ground_truth)'
//! ```
//!
//! # What this is for
//!
//! `authored_uv_ground_truth.rs` pins one surface - Talon's Junction's turn
//! arrows - through the per-material keyframe block. This file asks the
//! coverage question instead, over all twelve circuits at once: does *every*
//! material the engine's own `& 0x10` gate marks reach the shader's table, and
//! what animation does a circuit author that this engine does not play?
//!
//! The two answers, measured 2026-08-18 and asserted below:
//!
//! 1. **The texture-transform path is complete.** 922 gated materials on the
//!    twelve circuits, 922 parsed into a track, 922 on a batch that is drawn,
//!    and no circuit comes near `ANIM_TRACK_LIMIT` (18 distinct tracks at the
//!    worst).
//! 2. **`Anim Transform` (`0x3c0`) is read too, as of 2026-08-18.** It was the
//!    gap: 474 meshes hang off one, and treating the class as the identity used
//!    to drop its placement along with its animation, leaving 245 of them at
//!    the world origin - 38 while still replaying their own texture scroll.
//!    All 393 payloads decode and tile exactly, and the renderer plays them.
//!    See `docs/rendering/scenery-animation.md`.

use std::path::PathBuf;

use oag_mesh::mesh::{self, ANIM_TRACK_LIMIT, AnimTrack, Model, NODE_ANIM_LIMIT};
use oag_vex::vex;

/// The `Anim Transform` class, from the class table in `docs/formats/vex.md`.
const CLASS_ANIM_TRANSFORM: u32 = 0x3c0;
/// The `Mesh` class, version 6.
const CLASS_MESH: u32 = 0x125;
/// The `animationTrigger` class.
const CLASS_ANIMATION_TRIGGER: u32 = 0x3dc;

/// The twelve circuit directories `Data\Environments` carries. Not `01..=12`:
/// the numbering has gaps, and the mapping from directory number to circuit
/// name is not guessable - see `oag_pulse::race::DEFAULT_TRACK`.
const CIRCUITS: &[u32] = &[1, 2, 3, 4, 5, 6, 7, 9, 10, 13, 14, 16];

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn track_name(circuit: u32) -> String {
    format!(r"Data\Environments\{circuit:02}_Track\track.vex")
}

/// Every child index of every node, by parent.
fn children_of(nodes: &[vex::Node]) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for (i, node) in nodes.iter().enumerate() {
        if let Some(parent) = node.parent {
            out[parent].push(i);
        }
    }
    out
}

/// Texture label and animation slot of every drawn batch that carries one.
fn animated_draws(model: &Model) -> Vec<(String, u32)> {
    let mut out: Vec<(String, u32)> = Vec::new();
    for list in [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ] {
        for draw in list.iter() {
            let Some(&first) = model.indices.get(draw.range.start as usize) else {
                continue;
            };
            let Some(vertex) = model.vertices.get(first as usize) else {
                continue;
            };
            if vertex.anim == 0 {
                continue;
            }
            let label = draw
                .texture
                .and_then(|slot| model.textures.get(slot))
                .and_then(Option::as_ref)
                .map_or_else(|| "?".to_string(), |t| t.label.clone());
            if !out.contains(&(label.clone(), vertex.anim)) {
                out.push((label, vertex.anim));
            }
        }
    }
    out
}

/// Every material the engine's `& 0x10` gate marks becomes a track the shader
/// can index, on every circuit, and no circuit saturates the table.
///
/// The coverage half of the port. `mesh::build` deduplicates tracks by value
/// and stops at [`ANIM_TRACK_LIMIT`] `- 1`; a circuit authoring more distinct
/// tracks than that would draw the overflow frozen with nothing said, so the
/// headroom is asserted rather than assumed.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_gated_material_reaches_the_shaders_table() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    let (mut gated, mut parsed) = (0usize, 0usize);
    for &circuit in CIRCUITS {
        let name = track_name(circuit);
        let data = archives.read_name(&name).expect("reading track.vex");
        let mesh_class = vex::classes_of(&data)
            .expect("class table")
            .mesh
            .expect("mesh class");
        let nodes = vex::nodes(&data).expect("walking the node tree");

        for node in nodes.iter().filter(|n| n.class_id == mesh_class) {
            let payload = &data[node.payload()];
            let count = usize::from(u16_at(payload, 2));
            let transforms = vex::mesh_tex_transforms(payload);
            // Which material indices a drawn batch actually names. A gated
            // material no batch names would parse and never be seen.
            let mut named = vec![false; count];
            for list in [0u8, 1u8] {
                let Ok(batches) = vex::mesh_batches(payload, list) else {
                    continue;
                };
                for batch in batches {
                    if let Some(slot) = named.get_mut(usize::from(batch.material_index)) {
                        *slot = true;
                    }
                }
            }
            for (i, drawn) in named.iter().enumerate() {
                let at = 0x30 + i * 0x14;
                if at + 2 > payload.len() || u16_at(payload, at) & 0x10 == 0 {
                    continue;
                }
                gated += 1;
                assert!(
                    transforms.get(i).is_some_and(Option::is_some),
                    "{name}: material {i} carries the & 0x10 gate and parses to no track"
                );
                assert!(
                    *drawn,
                    "{name}: material {i} is animated and drawn by no batch"
                );
                parsed += 1;
            }
        }

        let model = mesh::build(&name, &data).expect("decoding track.vex");
        assert!(
            model.anim_tracks.len() < ANIM_TRACK_LIMIT - 1,
            "{name} authors {} distinct tracks, at the {} the shader holds - \
             the overflow draws frozen",
            model.anim_tracks.len(),
            ANIM_TRACK_LIMIT - 1
        );
        assert!(
            !animated_draws(&model).is_empty(),
            "{name} draws no animated batch at all"
        );
    }
    assert_eq!(gated, parsed);
    assert_eq!(
        gated, 922,
        "the gated-material count on the twelve circuits"
    );
}

/// Trackside advertising is **not** static: the banners scroll, and the disc
/// says so in its own authored blocks.
///
/// `docs/formats/vex.md`'s "Trackside advertising is static, and that is
/// measured on both axes" predates the keyframe blocks and is wrong.
/// `hub_banner_GLOW` - which that section names as static - sweeps four whole
/// tiles in `u` and one half-tile step in `v` over a ten-second loop, on both
/// circuits that carry it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_banners_scroll_and_the_disc_authors_it() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    let mut seen = 0;
    for &circuit in [9u32, 16].iter() {
        let name = track_name(circuit);
        let data = archives.read_name(&name).expect("reading track.vex");
        let model = mesh::build(&name, &data).expect("decoding track.vex");
        for (label, slot) in animated_draws(&model) {
            if !label.starts_with("hub_banner_GLOW") {
                continue;
            }
            seen += 1;
            let AnimTrack::Psp(track) = &model.anim_tracks[slot as usize - 1] else {
                panic!("a PSP circuit's own track is never Rcs")
            };
            assert_eq!(track.offset.times, vec![0, 240, 300, 540, 598]);
            assert_eq!(
                track.offset.values,
                vec![(0, 0), (512, 0), (512, 128), (1024, 128), (1024, 251)],
                "four tiles of u and half a tile of v, in 1/256 units"
            );
            assert!((track.loop_seconds - 10.0).abs() < 1e-4);
            assert!(!track.step, "a continuous scroll, not a stepped flicker");
        }
    }
    assert_eq!(seen, 2, "hub_banner_GLOW is drawn on both 09 and 16");
}

/// Every circuit authors `Anim Transform` nodes, they all decode, and the
/// meshes below them are placed rather than piled at the world origin.
///
/// **This was the gap.** `vex::world_transforms` composed only the `Transform`
/// class and treated every other class as the identity, which is right for a
/// `Mesh` or a `Skycube` and wrong for `Anim Transform` - it dropped the node's
/// placement along with its animation, and left 245 of these 474 meshes at the
/// world origin. The class is read now; see
/// `docs/rendering/scenery-animation.md`.
///
/// The eight still at the origin are there because their own authored keys put
/// them there, not because anything was dropped.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn anim_transform_nodes_are_read_and_their_meshes_are_placed() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    let (mut anim_nodes, mut meshes_under, mut at_origin) = (0usize, 0usize, 0usize);
    let (mut decoded, mut busiest) = (0usize, 0usize);
    for &circuit in CIRCUITS {
        let name = track_name(circuit);
        let data = archives.read_name(&name).expect("reading track.vex");
        let nodes = vex::nodes(&data).expect("walking the node tree");
        let world = vex::world_transforms(&data, &nodes);
        let children = children_of(&nodes);

        let mut on_this_circuit = 0usize;
        for (i, node) in nodes.iter().enumerate() {
            if node.class_id != CLASS_ANIM_TRANSFORM {
                continue;
            }
            anim_nodes += 1;
            on_this_circuit += 1;
            if vex::anim_transform_of(&data, node).is_some() {
                decoded += 1;
            }
            let mut stack = children[i].clone();
            while let Some(child) = stack.pop() {
                stack.extend(children[child].iter().copied());
                if nodes[child].class_id != CLASS_MESH {
                    continue;
                }
                meshes_under += 1;
                let m = world[child];
                if m[12] == 0.0 && m[13] == 0.0 && m[14] == 0.0 {
                    at_origin += 1;
                }
            }
        }
        assert!(
            on_this_circuit > 0,
            "{name} authors no Anim Transform - the class is not universal after all"
        );
        busiest = busiest.max(on_this_circuit);
    }

    assert_eq!(
        anim_nodes, 393,
        "Anim Transform nodes on the twelve circuits"
    );
    assert_eq!(meshes_under, 474, "meshes below one of them");
    assert_eq!(
        at_origin, 8,
        "meshes still at the world origin, which their own keys put there"
    );
    assert_eq!(decoded, 393, "payloads that decoded");
    assert!(
        busiest < NODE_ANIM_LIMIT - 1,
        "{busiest} Anim Transform nodes on one circuit, at the {} the shader \
         holds - the overflow draws frozen",
        NODE_ANIM_LIMIT - 1
    );
}

/// The payloads decode with nothing left over: the six key arrays tile
/// `[0x50, len)` exactly on all 393, which is what pins the field map.
///
/// A field map that is merely *plausible* reads a few payloads and stops. This
/// one is falsifiable on every node of every circuit at once, and one wrong
/// offset or stride breaks the tiling immediately.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_payload_tiles_exactly_under_the_field_map() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    let mut total = 0usize;
    for &circuit in CIRCUITS {
        let name = track_name(circuit);
        let data = archives.read_name(&name).expect("reading track.vex");
        let nodes = vex::nodes(&data).expect("walking the node tree");
        for node in nodes.iter().filter(|n| n.class_id == CLASS_ANIM_TRANSFORM) {
            let payload = &data[node.payload()];
            let anim = vex::anim_transform(payload, vex::byte_order(&data)).expect("decodes");
            total += 1;

            assert_eq!(
                anim.flags, 0,
                "{name}: a non-zero +0x34 selects an evaluator this decode never read"
            );
            assert!(
                (anim.seconds_per_key - 1.0 / 60.0).abs() < 1e-6,
                "{name}: key times are not 60 Hz frames"
            );

            let at = |o: usize| {
                u32::from_le_bytes(payload[o..o + 4].try_into().expect("4 bytes")) as usize
            };
            // A count of zero still stores one key, so the spans are what the
            // arrays actually occupy rather than what the counts imply.
            let keys = |n: usize| n.max(1);
            let (rot, tra, scl) = (
                keys(anim.rotation.times.len()),
                keys(anim.translation.times.len()),
                keys(anim.scale.times.len()),
            );
            let mut spans = [
                (at(0x08), rot * 2),
                (at(0x0c), tra * 2),
                (at(0x1c), rot * 6),
                (at(0x2c), tra * 6),
                (at(0x38), scl * 2),
                (at(0x40), scl * 6),
            ];
            spans.sort_unstable();
            let mut cursor = 0x50usize;
            for (start, len) in spans {
                assert!(
                    start >= cursor && start <= cursor + 1,
                    "{name}: an array at {start:#x} leaves a gap after {cursor:#x}"
                );
                cursor = start + len;
            }
            assert!(
                cursor <= payload.len() && payload.len() - cursor < 16,
                "{name}: the arrays tile to {cursor:#x} of a {:#x}-byte payload",
                payload.len()
            );
        }
    }
    assert_eq!(total, 393);
}

/// The renderer's split bake is exact: baking a vertex in its anchor's space
/// and multiplying by the anchor's world matrix lands it where composing the
/// whole chain would.
///
/// The half `mesh::build` cannot check for itself. `vex::anim_anchors` splits
/// the chain at each `Anim Transform` and `vex::anchor_world` resolves the part
/// that moves; if the two do not compose back to `world_transforms_at`, every
/// animated mesh draws somewhere subtly wrong and nothing else notices.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_split_bake_reproduces_the_whole_chain() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    let mut worst = 0.0f32;
    for &circuit in CIRCUITS {
        let name = track_name(circuit);
        let data = archives.read_name(&name).expect("reading track.vex");
        let nodes = vex::nodes(&data).expect("walking the node tree");
        let anchors = vex::anim_anchors(&data, &nodes);
        // Four times, not one: an anchor under another anchor only diverges
        // once both are off their first key.
        for seconds in [0.0f32, 1.5, 7.25, 63.0] {
            let whole = vex::world_transforms_at(&data, &nodes, seconds);
            let moving = vex::anchor_world(&data, &nodes, seconds);
            for (index, anchored) in anchors.iter().enumerate() {
                let split = match anchored.anchor {
                    Some(a) => vex::multiply(
                        &anchored.local,
                        &moving[a].expect("an anchor resolves to a world matrix"),
                    ),
                    None => anchored.local,
                };
                for k in 0..16 {
                    let scale = whole[index][k].abs().max(1.0);
                    worst = worst.max((split[k] - whole[index][k]).abs() / scale);
                }
            }
        }
    }
    assert!(
        worst < 1e-5,
        "the split bake diverges from the composed chain by {worst:e}"
    );
}

/// The two populations overlap: 52 meshes both scroll their texture **and**
/// hang under an `Anim Transform`, and they are placed now.
///
/// This is the sentence the two counts above cannot say on their own, and it
/// was the clearest single symptom of the gap: a surface replaying its authored
/// `TEXOFFSET` perfectly while sitting at the world origin, 38 of them, 34 on
/// `07_Track` alone. Two are left, and their own keys put them there.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_meshes_that_both_scroll_and_move_are_placed() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    let (mut both, mut at_origin) = (0usize, 0usize);
    for &circuit in CIRCUITS {
        let name = track_name(circuit);
        let data = archives.read_name(&name).expect("reading track.vex");
        let nodes = vex::nodes(&data).expect("walking the node tree");
        let world = vex::world_transforms(&data, &nodes);
        let children = children_of(&nodes);

        for (i, node) in nodes.iter().enumerate() {
            if node.class_id != CLASS_ANIM_TRANSFORM {
                continue;
            }
            let mut stack = children[i].clone();
            while let Some(child) = stack.pop() {
                stack.extend(children[child].iter().copied());
                if nodes[child].class_id != CLASS_MESH {
                    continue;
                }
                let payload = &data[nodes[child].payload()];
                let count = usize::from(u16_at(payload, 2));
                let animated = (0..count).any(|m| {
                    let at = 0x30 + m * 0x14;
                    at + 2 <= payload.len() && u16_at(payload, at) & 0x10 != 0
                });
                if !animated {
                    continue;
                }
                both += 1;
                let m = world[child];
                if m[12] == 0.0 && m[13] == 0.0 && m[14] == 0.0 {
                    at_origin += 1;
                }
            }
        }
    }
    assert_eq!(both, 52, "meshes in both populations");
    assert_eq!(
        at_origin, 2,
        "of those, still at the world origin - down from 38 before the class was read"
    );
}

/// `animationTrigger` `0x3dc` is authored nowhere this project has looked.
///
/// Bounded deliberately: 44 `.vex` files - the twelve `track.vex`, the twelve
/// `start_grid.vex` and twenty ship files - not the disc's ~340. Enough to say
/// the trigger is not a sibling of the `Anim Transform` nodes; not enough to say
/// Pulse never authors one.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn animation_trigger_is_absent_from_every_vex_checked() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    let mut names: Vec<String> = Vec::new();
    for &circuit in CIRCUITS {
        names.push(track_name(circuit));
        names.push(format!(
            r"Data\Environments\{circuit:02}_Track\start_grid.vex"
        ));
    }
    for team in [
        "Feisar", "Assegai", "Auricom", "Goteki45", "Harimau", "Mirage", "Piranha", "Qirex",
        "Triakis", "EG-X",
    ] {
        for file in [
            "Ship.vex",
            "shipboost.vex",
            "shipwreck.vex",
            "shipshield.vex",
        ] {
            names.push(format!(r"Data\Ships\{team}\{file}"));
        }
    }

    let mut checked = 0usize;
    for name in &names {
        let Ok(data) = archives.read_name(name) else {
            continue;
        };
        let Ok(nodes) = vex::nodes(&data) else {
            continue;
        };
        checked += 1;
        assert_eq!(
            nodes
                .iter()
                .filter(|n| n.class_id == CLASS_ANIMATION_TRIGGER)
                .count(),
            0,
            "{name} authors an animationTrigger after all"
        );
    }
    assert_eq!(checked, 44, "files that resolved and parsed");
}

/// Nothing lit hangs under a non-uniformly scaled `Anim Transform`, which is
/// what lets `mesh.wgsl` skip the inverse transpose on the node matrix.
///
/// The scale keys really are per-axis - 129 of the 920 are - so the shader's
/// simplification is not "the data is uniform", it is "the geometry that would
/// notice is prelit". That is a property of this disc and not of the format, so
/// it is asserted rather than trusted: a title that lights one of these meshes
/// turns this test red before it turns a frame wrong.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn no_lit_mesh_hangs_under_a_non_uniform_scale() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    let (mut keys, mut non_uniform, mut under, mut lit) = (0usize, 0usize, 0usize, 0usize);
    for &circuit in CIRCUITS {
        let name = track_name(circuit);
        let data = archives.read_name(&name).expect("reading track.vex");
        let nodes = vex::nodes(&data).expect("walking the node tree");
        let children = children_of(&nodes);

        for (i, node) in nodes.iter().enumerate() {
            if node.class_id != CLASS_ANIM_TRANSFORM {
                continue;
            }
            let anim = vex::anim_transform(&data[node.payload()], vex::byte_order(&data))
                .expect("decodes");
            keys += anim.scale.values.len();
            let skewed = anim
                .scale
                .values
                .iter()
                .filter(|v| v[0] != v[1] || v[1] != v[2])
                .count();
            non_uniform += skewed;
            if skewed == 0 {
                continue;
            }
            let mut stack = children[i].clone();
            while let Some(child) = stack.pop() {
                stack.extend(children[child].iter().copied());
                if nodes[child].class_id != CLASS_MESH {
                    continue;
                }
                under += 1;
                // A batch with no vertex colour is lit by the rig, and its
                // normal is what the node matrix would skew.
                let payload = &data[nodes[child].payload()];
                let is_lit = [0u8, 1u8].iter().any(|&list| {
                    vex::mesh_batches(payload, list).is_ok_and(|batches| {
                        batches
                            .iter()
                            .any(|b| b.vertices.iter().any(|v| v.colour.is_none()))
                    })
                });
                if is_lit {
                    lit += 1;
                }
            }
        }
    }
    assert_eq!(keys, 920, "authored scale keys");
    assert_eq!(non_uniform, 129, "of those, per-axis rather than uniform");
    assert_eq!(under, 37, "meshes below a node that authors one");
    assert_eq!(
        lit, 0,
        "a lit mesh under a non-uniform scale needs the inverse transpose \
         mesh.wgsl deliberately skips"
    );
}
