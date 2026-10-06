//! What Pulse's start gantry authors for its `3`, `2`, `1`, `GO`.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! # What this is for
//!
//! `docs/rendering/start-gantry.md` claims the countdown is driven by one
//! shared UV offset walking four per-glyph cells across a palette staircase -
//! **not** by node animation, per-node visibility, a material swap or an
//! atlas-cell swap. That claim is only worth its score if the file still says
//! so, so this reproduces it end to end without a GPU:
//!
//! 1. the four UV cells exist, and ordering them by `u` - the axis the offset
//!    track walks - puts them in reading order `3`, `2`, `1`, then a wide
//!    centred `GO`;
//! 2. the palette's three white pairs sit at the three row bands the page
//!    names;
//! 3. evaluating the material's own [`TexTransform`](oag_vex::vex::TexTransform)
//!    at three phase times lights **exactly one** digit each, in the order
//!    `3`, `2`, `1`;
//! 4. over the span after that, `GO` strobes and no digit is lit at all.
//!
//! Point 3 is the one that pins the *mechanism*: it samples the real palette
//! through the real track, so a decoder that got either half wrong fails here
//! rather than in a picture nobody looks at.
//!
//! Nothing here places the gantry - slot 8's transform is still unrecovered,
//! and this test asserts what the model does, not where it stands.

use std::path::PathBuf;

use oag_vex::vex;

/// The gantry, named by slot 8 of every Pulse circuit that authors a manifest.
const GANTRY: &str = r"Data\Environments\321_Go\321Go_StartFinish.vex";

/// The node holding all four glyphs. Namespaced twice by its Maya reference.
const GLYPH_NODE: &str = "start_light_321go";

/// Frames per second the key times are in, from the block's own
/// `seconds_per_key`, asserted rather than assumed.
const FPS: f32 = 60.0;

/// When each digit is the lit one, from `docs/rendering/start-gantry.md`'s
/// table. Sampled mid-window rather than at an edge, so a one-frame decoder
/// slip does not flip the answer and a real regression still does.
///
/// **Digits only.** `GO` cannot be pinned to one instant the same way, because
/// its own palette column strobes - see
/// [`go_strobes_while_no_digit_is_lit`].
const DIGIT_PHASES: &[(f32, &str)] = &[(1.10, "3"), (1.85, "2"), (2.60, "1")];

/// The span `GO` owns, between the `u` step that hands the board over to it and
/// the panel leaving the aperture at 6.000 s.
const GO_SPAN: (f32, f32) = (3.60, 5.95);

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn gantry() -> Option<Vec<u8>> {
    let path = image()?;
    let mut archives =
        oag_pulse::open(path.to_str().expect("image path is utf-8")).expect("open as Pulse");
    Some(archives.read_name(GANTRY).expect("the gantry resolves"))
}

/// One glyph: the UV cell its vertices share, and where they sit.
#[derive(Debug)]
struct Glyph {
    uv: [f32; 2],
    x_min: f32,
    x_max: f32,
    vertices: usize,
}

/// Every distinct per-vertex UV on the glyph node, with the x span of the
/// vertices carrying it.
///
/// **Distinct UV, not distinct batch.** The node draws in three batches and
/// none of them is one glyph: the split is by material pass, so grouping by
/// batch finds three groups where the file authors four.
fn glyphs(data: &[u8], nodes: &[vex::Node]) -> Vec<Glyph> {
    let node = nodes
        .iter()
        .find(|n| {
            n.name
                .as_deref()
                .is_some_and(|name| name.contains(GLYPH_NODE) && name.ends_with("Shape"))
        })
        .expect("the glyph node is present");
    let payload = &data[node.payload()];
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
                        cell.vertices += 1;
                    }
                    None => cells.push(Glyph {
                        uv,
                        x_min: x,
                        x_max: x,
                        vertices: 1,
                    }),
                }
            }
        }
    }
    // **By `u`, not by where the glyph sits.** `u` is what the offset track
    // walks, so it is the mechanism's own order - and it is the only one that
    // works: `GO` spans the whole board, so its left edge sorts it *second* by
    // x, between `3` and `2`. Sorting by x labels two of the four wrongly and
    // the test still looks like it is measuring something.
    cells.sort_by(|a, b| a.uv[0].total_cmp(&b.uv[0]));
    cells
}

/// The RGBA the palette holds at `(u, v)`, with the wrap the sampler applies.
///
/// Nearest rather than bilinear on purpose: the claim is about *which texel* a
/// glyph reaches, and blending two neighbours would answer a softer question
/// than the one being asked.
fn sample(tex: &vex::EmbeddedTexture, rgba: &[u8], u: f32, v: f32) -> [u8; 4] {
    let (w, h) = (i32::from(tex.width), i32::from(tex.height));
    let wrap = |value: f32, size: i32| {
        let texel = (value * size as f32).floor() as i32;
        texel.rem_euclid(size)
    };
    let (x, y) = (wrap(u, w), wrap(v, h));
    let at = ((y * w + x) * 4) as usize;
    [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]
}

/// Whether a sampled texel is the opaque white that lights a glyph, as opposed
/// to the dark red it holds otherwise or the alpha-0 it holds off the ladder.
fn is_lit(texel: [u8; 4]) -> bool {
    texel[3] == 255 && texel[0] > 200 && texel[1] > 200 && texel[2] > 200
}

/// The glyph material's own texture-transform block.
fn glyph_track(data: &[u8], nodes: &[vex::Node]) -> vex::TexTransform {
    let node = nodes
        .iter()
        .find(|n| {
            n.name
                .as_deref()
                .is_some_and(|name| name.contains(GLYPH_NODE) && name.ends_with("Shape"))
        })
        .expect("the glyph node is present");
    vex::mesh_tex_transforms(&data[node.payload()])
        .into_iter()
        .flatten()
        .next()
        .expect("the glyph material authors a texture transform")
}

/// Which glyphs read an opaque-white texel at `seconds`, in `u` order.
///
/// This is the shader's own composition done on the CPU: displace each cell's
/// authored UV by the track's offset at this instant, then sample.
fn lit_glyphs<'a>(
    cells: &'a [Glyph],
    track: &vex::TexTransform,
    tex: &vex::EmbeddedTexture,
    rgba: &[u8],
    seconds: f32,
) -> Vec<&'a str> {
    let names = ["3", "2", "1", "GO"];
    let (du, dv) = track.offset.sample_with(seconds * FPS, track.step);
    cells
        .iter()
        .enumerate()
        .filter(|(_, cell)| is_lit(sample(tex, rgba, cell.uv[0] + du, cell.uv[1] + dv)))
        .map(|(index, _)| names[index])
        .collect()
}

/// **Four glyphs, one UV cell each, ordered `3` `2` `1` `GO` across the board.**
///
/// The evidence that the digits are geometry sharing one texture rather than
/// four cells of an atlas: three of the four cells have the *same* `v` and
/// differ only in `u`, so nothing about them selects a picture - they select a
/// palette column.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_gantry_authors_four_glyph_cells() {
    let Some(data) = gantry() else { return };
    let nodes = vex::nodes(&data).expect("the gantry parses");
    let cells = glyphs(&data, &nodes);

    for (index, cell) in cells.iter().enumerate() {
        println!(
            "cell {index}: uv ({:.3}, {:.3})  {} vert(s)  x {:.2}..{:.2}",
            cell.uv[0], cell.uv[1], cell.vertices, cell.x_min, cell.x_max
        );
    }
    assert_eq!(cells.len(), 4, "one UV cell per glyph");

    // The three digits share a row and differ only by column, which is what
    // makes a single shared offset able to tell them apart at all.
    for pair in cells[..3].windows(2) {
        assert!(
            (pair[0].uv[1] - pair[1].uv[1]).abs() < 1e-4,
            "the digits share one v: {:?} vs {:?}",
            pair[0],
            pair[1]
        );
        assert!(
            pair[0].x_min < pair[1].x_min,
            "reading order follows u: {:?} then {:?}",
            pair[0],
            pair[1]
        );
    }

    // `GO` is past the digits by u, sits on its own row, and spans the board
    // rather than a third of it - the three facts that say it is a word and
    // not a fourth digit.
    let go = &cells[3];
    assert!(
        (go.uv[1] - cells[0].uv[1]).abs() > 1e-3,
        "GO reads its own row: {go:?}"
    );
    let digit_width = cells[0].x_max - cells[0].x_min;
    assert!(
        go.x_max - go.x_min > digit_width * 1.5,
        "GO is wider than a digit: {:.2} against {digit_width:.2}",
        go.x_max - go.x_min
    );
}

/// **The palette is a staircase: three opaque-white pairs, one per digit.**
///
/// Asserted on the texture's own texels rather than through a render, because
/// this is what makes a single shared offset able to sequence three glyphs at
/// all. A flat ramp could not.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_palette_is_a_staircase() {
    let Some(data) = gantry() else { return };
    let slots = vex::textures(&data).expect("the textures decode");
    let tex = slots
        .first()
        .and_then(Option::as_ref)
        .expect("slot 0 is the palette");
    assert_eq!((tex.width, tex.height), (16, 32), "the palette's size");
    let rgba = tex.to_rgba();

    // Column, then the rows that column is opaque white on, over the ladder.
    let ladder: Vec<(usize, Vec<usize>)> = (0..6)
        .map(|x| {
            let rows = (23..30)
                .filter(|y| {
                    let at = (y * usize::from(tex.width) + x) * 4;
                    is_lit([rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]])
                })
                .collect();
            (x, rows)
        })
        .collect();
    for (x, rows) in &ladder {
        println!("column {x}: white rows {rows:?}");
    }

    assert_eq!(ladder[0].1, vec![28, 29], "column 0 lights the `3`");
    assert_eq!(ladder[1].1, vec![28, 29], "column 1 lights the `3`");
    assert_eq!(ladder[2].1, vec![26, 27], "column 2 lights the `2`");
    assert_eq!(ladder[3].1, vec![26, 27], "column 3 lights the `2`");
    assert_eq!(ladder[4].1, vec![24, 25], "column 4 lights the `1`");
    assert_eq!(ladder[5].1, vec![24, 25], "column 5 lights the `1`");
}

/// **The whole mechanism, end to end: one glyph is lit at each phase, in
/// order.**
///
/// The material's authored offset track is evaluated at four times, each cell's
/// own UV is displaced by it, and the palette is sampled. Exactly one cell
/// comes back opaque white each time, and which one walks `3` -> `2` -> `1` ->
/// `GO`. Nothing here hard-codes a glyph's colour: the answer comes out of the
/// file's own two halves being combined the way the shader combines them.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn one_glyph_is_lit_at_each_phase_in_order() {
    let Some(data) = gantry() else { return };
    let nodes = vex::nodes(&data).expect("the gantry parses");
    let cells = glyphs(&data, &nodes);
    assert_eq!(cells.len(), 4);

    let slots = vex::textures(&data).expect("the textures decode");
    let tex = slots
        .first()
        .and_then(Option::as_ref)
        .expect("slot 0 is the palette");
    let rgba = tex.to_rgba();

    let track = glyph_track(&data, &nodes);
    println!(
        "offset keys {:?} values {:?}, loop {:.3}s, step {}",
        track.offset.times, track.offset.values, track.loop_seconds, track.step
    );
    assert!(
        (track.seconds_per_key - 1.0 / FPS).abs() < 1e-6,
        "key times are 60Hz frames"
    );
    assert!(
        (track.loop_seconds - 6.0).abs() < 1e-3,
        "the countdown's own loop is six seconds, not the last key time"
    );

    for (seconds, expected) in DIGIT_PHASES {
        let lit = lit_glyphs(&cells, &track, tex, &rgba, *seconds);
        println!("t={seconds:.2}s  lit {lit:?}");
        assert_eq!(
            lit,
            vec![*expected],
            "exactly `{expected}` is lit at {seconds:.2}s"
        );
    }
}

/// **`GO` strobes, and no digit is lit while it does.**
///
/// The four glyphs are not symmetric and a test that treated them as such would
/// miss it: columns 0-5 are solid over the digits' rows, but the column `GO`
/// reads alternates opaque and alpha-0 row by row over rows 0-21, so scrolling
/// through it makes the word **flash** rather than fade in. Sampling one
/// instant would therefore answer "lit" or "unlit" by luck.
///
/// So this asserts the shape instead: over the span `GO` owns, it is lit
/// sometimes and unlit sometimes, and a digit is lit never - which is the claim
/// that actually matters, that the board has handed over.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn go_strobes_while_no_digit_is_lit() {
    let Some(data) = gantry() else { return };
    let nodes = vex::nodes(&data).expect("the gantry parses");
    let cells = glyphs(&data, &nodes);
    let slots = vex::textures(&data).expect("the textures decode");
    let tex = slots
        .first()
        .and_then(Option::as_ref)
        .expect("slot 0 is the palette");
    let rgba = tex.to_rgba();
    let track = glyph_track(&data, &nodes);

    let (mut on, mut off, mut samples) = (0usize, 0usize, 0usize);
    let (start, end) = GO_SPAN;
    let mut seconds = start;
    while seconds <= end {
        let lit = lit_glyphs(&cells, &track, tex, &rgba, seconds);
        assert!(
            !lit.iter().any(|g| *g != "GO"),
            "a digit is lit at {seconds:.2}s, after the board handed over: {lit:?}"
        );
        if lit.is_empty() {
            off += 1
        } else {
            on += 1
        }
        samples += 1;
        seconds += 0.05;
    }
    println!("over {start:.2}..{end:.2}s: GO lit on {on} of {samples} sample(s), dark on {off}");
    // Wide margins on purpose: the measured split is 33 lit to 14 dark, and the
    // claim is "it strobes", not "it strobes at this duty cycle" - which would
    // be a number nothing has checked against the original.
    assert!(on * 4 > samples, "GO is lit for a real part of its span");
    assert!(
        off * 6 > samples,
        "and dark for a real part of it - it strobes"
    );
}
