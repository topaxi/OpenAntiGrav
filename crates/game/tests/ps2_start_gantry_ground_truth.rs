//! PS2 Pulse's start gantry: the same asset name and the same countdown
//! mechanism PSP's disc authors, skinned from an external texture set rather
//! than an embedded one, and with the `3`/`2`/`1` digits and the `GO` word
//! split across two mesh nodes where PSP carries all four glyphs on one.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The tests skip with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves nothing.
//!
//! # What this reproduces
//!
//! `crates/vex/tests/start_gantry_ground_truth.rs` proves the mechanism on
//! PSP's disc: one shared `TEXOFFSET` walked across a palette staircase,
//! sampled straight off the file's own embedded texture. Reported from play:
//! the PS2 disc drew the gantry as a blank cream slab with no legible digits -
//! placed and animated, but untextured, because a PS2 `.vex`'s `Texture`
//! nodes carry no pixels by design (`docs/formats/ps2-texture.md`). The
//! palette lives in the archive entry directly before the model, the same
//! directory-position rule already fixed for the hull, the plume and the
//! shield models (`crate::race::gantry::ps2_skin`). This file is the same
//! mechanism test pointed at that external skin, so a regression in either
//! half - the offset track or the external resolve - fails here rather than
//! only in a screenshot.
//!
//! # A real content difference from PSP, not a decode gap
//!
//! `oag-view --nodes` on both discs shows it directly: PSP's glyph node is
//! `Jons321go:Jons321go:start_light_321goShape`, one `Mesh` carrying all four
//! UV cells (`3`, `2`, `1`, `GO`) this project's own `docs/rendering/start-gantry.md`
//! documents. **The PS2 disc splits that one node into two** -
//! `start_light_321Shape` (three UV cells, the digits) and a separate
//! `GoShape` (`Anim Transform` node `Go`) - ten `Mesh` nodes against PSP's
//! nine. Both still bind the same six-texture material set and the same
//! `TEXOFFSET`-driven staircase mechanism this file's own tests reproduce for
//! the three digits; `GoShape`'s own track is not asserted here. A matched
//! pair of `--race --ticks 260` captures against PSP's own disc is the
//! evidence it plays correctly anyway - see `docs/rendering/start-gantry.md`'s
//! "Implemented on PS2" section - since the renderer walks every node's own
//! authored track generically rather than assuming PSP's single-node shape.

use std::path::PathBuf;

use oag_mesh::mesh;
use oag_vex::vex;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-ps2-eu.chd")
}

/// The gantry, named by slot 8 of every PS2 Pulse circuit that authors a
/// manifest - the same spelling the PSP disc uses.
const GANTRY: &str = r"Data\Environments\321_Go\321Go_StartFinish.vex";

/// Matches PSP's `Jons321go:Jons321go:start_light_321goShape` and PS2's own
/// `start_light_321Shape` alike - see this file's own doc comment on why the
/// two are not the same node.
const GLYPH_NODE: &str = "start_light_321";

const FPS: f32 = 60.0;

/// Same phases `start_gantry_ground_truth.rs::DIGIT_PHASES` pins on PSP.
const DIGIT_PHASES: &[(f32, &str)] = &[(1.10, "3"), (1.85, "2"), (2.60, "1")];

fn gantry() -> Option<(Vec<u8>, mesh::Ps2TextureSet)> {
    let path = image()?;
    let mut archives =
        oag_pulse::open(path.to_str().expect("image path is utf-8")).expect("open as Pulse");
    let blob = archives.read_name(GANTRY).expect("the gantry resolves");
    let preceding = archives
        .read_preceding(GANTRY)
        .expect("the preceding archive entry reads");
    let set = mesh::Ps2TextureSet::parse(&preceding)
        .expect("the preceding archive entry is a texture set");
    Some((blob, set))
}

/// One glyph: the UV cell its vertices share, and where they sit.
///
/// The same extraction `start_gantry_ground_truth.rs::glyphs` does on PSP's
/// disc, duplicated rather than shared across crates: that test lives in
/// `oag-vex`'s own test binary, and this one additionally needs `oag-game`'s
/// archive and PS2 texture-set types.
#[derive(Debug)]
struct Glyph {
    uv: [f32; 2],
    x_min: f32,
    x_max: f32,
}

fn glyph_node(nodes: &[vex::Node]) -> &vex::Node {
    nodes
        .iter()
        .find(|n| {
            n.name
                .as_deref()
                .is_some_and(|name| name.contains(GLYPH_NODE) && name.ends_with("Shape"))
        })
        .expect("the glyph node is present")
}

fn glyphs(data: &[u8], nodes: &[vex::Node]) -> Vec<Glyph> {
    let payload = &data[glyph_node(nodes).payload()];
    let mut cells: Vec<Glyph> = Vec::new();
    for list in 0u8..2 {
        let Ok(batches) = vex::mesh_batches(payload, list) else {
            continue;
        };
        for batch in &batches {
            for vertex in &batch.vertices {
                let Some(uv) = vertex.texcoord else { continue };
                let x = vertex.position[0];
                match cells
                    .iter_mut()
                    .find(|c| (c.uv[0] - uv[0]).abs() < 1e-4 && (c.uv[1] - uv[1]).abs() < 1e-4)
                {
                    Some(cell) => {
                        cell.x_min = cell.x_min.min(x);
                        cell.x_max = cell.x_max.max(x);
                    }
                    None => cells.push(Glyph {
                        uv,
                        x_min: x,
                        x_max: x,
                    }),
                }
            }
        }
    }
    // By `u`, the axis the offset track walks - see the PSP test's own note
    // on why sorting by `x` mislabels `GO`.
    cells.sort_by(|a, b| a.uv[0].total_cmp(&b.uv[0]));
    cells
}

fn glyph_track(data: &[u8], nodes: &[vex::Node]) -> vex::TexTransform {
    vex::mesh_tex_transforms(&data[glyph_node(nodes).payload()])
        .into_iter()
        .flatten()
        .next()
        .expect("the glyph material authors a texture transform")
}

fn sample(texture: &mesh::ModelTexture, rgba: &[u8], u: f32, v: f32) -> [u8; 4] {
    let (w, h) = (texture.width as i32, texture.height as i32);
    let wrap = |value: f32, size: i32| ((value * size as f32).floor() as i32).rem_euclid(size);
    let (x, y) = (wrap(u, w), wrap(v, h));
    let at = ((y * w + x) * 4) as usize;
    [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]
}

fn is_lit(texel: [u8; 4]) -> bool {
    texel[3] == 255 && texel[0] > 200 && texel[1] > 200 && texel[2] > 200
}

fn lit_glyphs<'a>(
    cells: &'a [Glyph],
    track: &vex::TexTransform,
    texture: &mesh::ModelTexture,
    rgba: &[u8],
    seconds: f32,
) -> Vec<&'a str> {
    // Only the three digits: PS2's `start_light_321Shape` carries no `GO`
    // cell at all - see this file's own doc comment on the node split.
    let names = ["3", "2", "1"];
    let (du, dv) = track.offset.sample_with(seconds * FPS, track.step);
    cells
        .iter()
        .enumerate()
        .filter(|(_, cell)| is_lit(sample(texture, rgba, cell.uv[0] + du, cell.uv[1] + dv)))
        .map(|(index, _)| names[index])
        .collect()
}

/// **`start_light_321Shape` authors three UV cells, one per digit - not
/// PSP's four.** `GO` is a separate node on this disc (`GoShape`, see this
/// file's own doc comment); this test is scoped to the node the digit
/// mechanism actually lives on.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn the_ps2_disc_authors_three_digit_glyph_cells() {
    let Some((data, _set)) = gantry() else {
        return;
    };
    let nodes = vex::nodes(&data).expect("the gantry parses");
    let cells = glyphs(&data, &nodes);
    assert_eq!(
        cells.len(),
        3,
        "one UV cell per digit on start_light_321Shape - GO is a separate node on PS2"
    );
}

/// **The whole mechanism, end to end, off the PS2's own external skin.**
///
/// Mirrors `start_gantry_ground_truth.rs::one_glyph_is_lit_at_each_phase_in_order`,
/// with the palette resolved through [`mesh::Ps2TextureSet`] instead of
/// [`vex::textures`] - the PS2 disc embeds no pixels for this file at all, so
/// the embedded path would find nothing to sample.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn one_glyph_is_lit_at_each_phase_in_order_on_ps2() {
    let Some((data, set)) = gantry() else {
        return;
    };
    let nodes = vex::nodes(&data).expect("the gantry parses");
    let cells = glyphs(&data, &nodes);
    assert_eq!(
        cells.len(),
        3,
        "three digit cells - see the file doc comment"
    );

    let classes = vex::classes_of(&data).expect("the class table parses");
    let texture_name = nodes
        .iter()
        .find(|n| Some(n.class_id) == classes.texture)
        .and_then(|n| vex::texture_asset_path(&data[n.payload()]))
        .expect("the first Texture node names the palette");
    let texture = set
        .resolve(&texture_name)
        .unwrap_or_else(|| panic!("{texture_name}: not in the preceding archive entry"));
    // **32x32, not PSP's 16x32** - measured with
    // `examples/ps2_gantry_palette_probe.rs`: the staircase is re-authored at
    // double the column width (four columns per digit instead of two) and
    // the same three row bands (28-29, 26-27, 24-25). The mechanism is
    // resolution-independent - [`sample`] wraps against the texture's own
    // measured size - so this only pins that the file really is a different
    // asset from PSP's, not a decode fault reading it as one.
    assert_eq!(
        (texture.width, texture.height),
        (32, 32),
        "the PS2 palette is a different, wider staircase than PSP's"
    );
    let rgba = texture.rgba().expect("the palette decodes to RGBA8");

    let track = glyph_track(&data, &nodes);
    assert!(
        (track.seconds_per_key - 1.0 / FPS).abs() < 1e-6,
        "key times are 60Hz frames on PS2 too"
    );
    assert!(
        (track.loop_seconds - 6.0).abs() < 1e-3,
        "the countdown's own loop is six seconds on PS2 too"
    );

    for (seconds, expected) in DIGIT_PHASES {
        let lit = lit_glyphs(&cells, &track, &texture, rgba, *seconds);
        println!("t={seconds:.2}s  lit {lit:?}");
        assert_eq!(
            lit,
            vec![*expected],
            "exactly `{expected}` is lit at {seconds:.2}s on the PS2-skinned model"
        );
    }
}
