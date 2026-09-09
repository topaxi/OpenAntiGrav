//! What Wipeout HD's `321Go_StartFinish.vex` authors for its `3`, `2`, `1`,
//! `GO` gantry - and, just as load-bearing, what it does *not* author.
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
//! release check wants: a skipped ground-truth test is green and proves
//! nothing.
//!
//! # What this is for
//!
//! `docs/rendering/start-gantry.md`'s HD section makes five claims this
//! reproduces end to end, without a GPU:
//!
//! 1. HD's glyph node's geometry lives in a sibling `.rcsmodel`, not the
//!    `.vex` itself, and carries five distinct per-vertex UV cells rather
//!    than Pulse's four.
//! 2. Adding the material's own static `u` offset lands those cells' `x`
//!    positions and the palette's diagonal staircase bands in the same
//!    reading order - the evidence the layout is the same authored concept
//!    as Pulse's, at a different resolution and encoding.
//! 3. The texture is a 4x-scaled, DXT-compressed version of Pulse's
//!    staircase-and-marker-column shape, not a fresh design.
//! 4. **Negative, and the load-bearing half**: nothing in either the
//!    material's own parameter table or the node's payload carries a
//!    keyframe track for that offset - a fact about the format, checked with
//!    the same parser that recovers Pulse's `TEXOFFSET` track, not assumed
//!    from one file's silence.
//! 5. The glyph node's own `Anim Transform` moves it out of view at the same
//!    6.000 s loop-closing instant Pulse's own teleport lands on, off two
//!    independent authored tracks on two different platforms.
//! 6. **Added when the 2048 pass found the same shape in its own inherited
//!    copy of this file**: HD's own `polySurface7` (the chequered-flag node,
//!    the same name Pulse's file carries) and `pasted__Final_Lap` both carry
//!    translation keys at frame 740/741 (12.333/12.350 s), and
//!    `pasted__Final_Lap` also keys at 560/561 (9.333/9.350 s) - the same
//!    three-beat timeline (6.000/9.333/12.333 s) Pulse's own file authors,
//!    checked here against HD's own disc rather than 2048's re-export of it.
//!
//! Nothing here places the gantry, resolves whether anything writes the
//! offset at runtime, or resolves HD's slot-7 substitution - all three stay
//! open on `docs/rendering/start-gantry.md` and are not asserted here.

use std::path::{Path, PathBuf};

use oag_formats::{gtf, rcsmodel, vex};

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

/// `321go_startfinish.vex`, `321go_startfinish.rcsmodel` and `321_go_64.gtf`
/// are all in this one archive - see `docs/rendering/start-gantry.md`'s
/// "not all in one archive" paragraph for why that is worth stating rather
/// than assuming.
const ARCHIVE: &str = "DATA02.PSARC";

const VEX: &str = "/data/billboards/hd_adverts/321go/321go_startfinish.vex";
const RCSMODEL: &str = "/data/billboards/hd_adverts/321go/321go_startfinish.rcsmodel";
const TEXTURE: &str = "/data/billboards/hd_adverts/321go/321_go_64.gtf";

/// The node carrying every glyph, the same two-node `Anim Transform` -> `Mesh`
/// shape Pulse's own `start_light_321go` takes.
const GLYPH_NODE: &str = "pasted__Go_HD_start_light_321go";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

fn archive(image: &Path) -> oag_assets::psarc::Archive {
    let spec = format!("{}:PS3_GAME/USRDIR/{ARCHIVE}", image.display());
    oag_assets::psarc::Archive::open(&spec).expect("the archive opens")
}

/// The glyph `Mesh` node's payload, out of the `.vex`.
fn glyph_payload(vex_data: &[u8]) -> Vec<u8> {
    let nodes = vex::nodes(vex_data).expect("nodes");
    let classes = vex::classes_of(vex_data).expect("classes");
    let mesh_class = classes.mesh.expect("mesh class id");
    let node = nodes
        .iter()
        .find(|n| {
            n.class_id == mesh_class
                && n.name
                    .as_deref()
                    .is_some_and(|name| name.contains(GLYPH_NODE))
        })
        .expect("the glyph mesh node is present");
    vex_data[node.payload()].to_vec()
}

/// The hash a `Mesh` payload's `+0x30` word carries on a PS3 `.vex` - the tie
/// to the `.rcsmodel` chunk holding this node's actual geometry.
fn chunk_hash(payload: &[u8]) -> u32 {
    u32::from_be_bytes(payload[0x30..0x34].try_into().expect("32 bits at +0x30"))
}

/// One UV cell: the value every vertex sharing it carries, and the `x` span
/// of those vertices.
struct Cell {
    uv: [f32; 2],
    x_min: f32,
    x_max: f32,
}

/// Every distinct per-vertex UV on the glyph's `.rcsmodel` chunk, sorted by
/// `u` - the axis the material's static offset shifts.
fn glyph_cells(rcs_data: &[u8], hash: u32) -> Vec<Cell> {
    let model = rcsmodel::Model::parse(rcs_data).expect("the .rcsmodel parses");
    let chunk = model
        .mesh(hash)
        .expect("the glyph's hash resolves to an .rcsmodel chunk");
    let stride = chunk
        .declared_stride()
        .or_else(|| chunk.solve_stride_without_a_box(rcs_data))
        .expect("a vertex stride is recoverable");
    let mut cells: Vec<Cell> = Vec::new();
    for submesh in &chunk.submeshes {
        let positions = chunk
            .positions(rcs_data, submesh, stride)
            .expect("positions decode");
        let texcoords = chunk
            .texcoords(rcs_data, submesh, stride)
            .expect("the chunk declares a diffuse texture coordinate");
        for (p, uv) in positions.iter().zip(texcoords.iter()) {
            match cells
                .iter_mut()
                .find(|c| (c.uv[0] - uv[0]).abs() < 1e-3 && (c.uv[1] - uv[1]).abs() < 1e-3)
            {
                Some(c) => {
                    c.x_min = c.x_min.min(p[0]);
                    c.x_max = c.x_max.max(p[0]);
                }
                None => cells.push(Cell {
                    uv: *uv,
                    x_min: p[0],
                    x_max: p[0],
                }),
            }
        }
    }
    cells.sort_by(|a, b| a.uv[0].total_cmp(&b.uv[0]));
    cells
}

/// The glyph material's own static `uvOffset`, `(0, 0)` if it authors none.
fn uv_offset(rcs_data: &[u8], hash: u32) -> (f32, f32) {
    let model = rcsmodel::Model::parse(rcs_data).expect("parses");
    let chunk = model.mesh(hash).expect("chunk");
    let material = model
        .material_of(chunk)
        .expect("the chunk names a material");
    material
        .parameters
        .iter()
        .find(|p| p.hash == oag_formats::rcsmaterial::name_hash("uvOffset"))
        .map(|p| (p.value[0], p.value[1]))
        .unwrap_or((0.0, 0.0))
}

/// **Claim 1 and 2**: five cells, not four, and adding the material's own
/// static `u` offset puts them in the same reading-order `x` sequence a
/// digit board reads in.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_glyph_authors_five_uv_cells_aligned_by_the_static_offset() {
    let Some(image) = image() else { return };
    let mut archive = archive(&image);
    let vex_data = archive.read_path(VEX).expect("the .vex reads");
    let rcs_data = archive.read_path(RCSMODEL).expect("the .rcsmodel reads");
    let payload = glyph_payload(&vex_data);
    let hash = chunk_hash(&payload);

    let cells = glyph_cells(&rcs_data, hash);
    assert_eq!(
        cells.len(),
        5,
        "the glyph node authors five distinct UV cells, not Pulse's four - \
         a real difference this test pins rather than assumes stays four"
    );

    let (offset_u, _) = uv_offset(&rcs_data, hash);
    assert!(
        (offset_u - 0.04).abs() < 1e-6,
        "the material's own authored u offset is 0.04; docs/rendering/start-gantry.md's \
         column-alignment table is built on this exact value"
    );

    // `cells` is already ordered by `u` - the axis the material's offset
    // shifts, and per docs/rendering/start-gantry.md the axis whose order is
    // the reading order, the same lesson Pulse's own test learned sorting by
    // `x` instead. Four cells read left to right by `x_min` in that same
    // `u` order, the two near-identical middle ones (the unexplained twin)
    // included; the fifth is the wide one and comes last by `u`.
    let x_mins: Vec<f32> = cells[..4].iter().map(|c| c.x_min).collect();
    assert!(
        x_mins.windows(2).all(|w| w[0] < w[1]),
        "the four narrow cells' x_min should read left to right in u-order: {x_mins:?}"
    );
    let wide = &cells[4];
    let widest_span = cells
        .iter()
        .map(|c| c.x_max - c.x_min)
        .fold(f32::MIN, f32::max);
    assert!(
        (wide.x_max - wide.x_min - widest_span).abs() < 1e-3,
        "the last cell by u-order is expected to be the widest, the same shape as Pulse's GO"
    );
    let narrow_span = cells[3].x_max - cells[0].x_min;
    assert!(
        (wide.x_max - wide.x_min) > narrow_span * 0.5,
        "the wide cell should cover a large fraction of the whole board, the \
         same shape as Pulse's GO spanning all three digits"
    );
}

/// **Claim 3**: the texture is Pulse's staircase-and-marker-column shape,
/// scaled 4x and DXT-compressed, not a fresh design.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_texture_is_a_scaled_diagonal_staircase() {
    let Some(image) = image() else { return };
    let mut archive = archive(&image);
    let tex_data = archive.read_path(TEXTURE).expect("the .gtf reads");
    let file = gtf::Gtf::parse(&tex_data).expect("the .gtf parses");
    let tex = &file.textures[0];
    assert_eq!(
        (tex.width, tex.height),
        (64, 128),
        "4x Pulse's 16x32 in both dimensions"
    );

    let rgba = tex.to_rgba(&tex_data).expect("the .gtf decodes");
    let is_white = |x: u32, y: u32| {
        let c = rgba[(y * u32::from(tex.width) + x) as usize];
        c[3] > 200 && c[0] > 200 && c[1] > 200 && c[2] > 200
    };
    // The three diagonal 8x8 staircase bands.
    for (row_lo, row_hi, col_lo, col_hi) in [(7, 14, 0, 7), (15, 22, 8, 15), (23, 30, 16, 23)] {
        for y in row_lo..=row_hi {
            for x in col_lo..=col_hi {
                assert!(is_white(x, y), "expected opaque white at ({x}, {y})");
            }
        }
    }
    // The two unused marker columns Pulse's own palette also carries - here
    // at roughly half alpha, not fully opaque, checked at that resolution
    // rather than assumed the same as the digit bands' opaque white.
    let is_red = |x: u32, y: u32| {
        let c = rgba[(y * u32::from(tex.width) + x) as usize];
        c[3] > 100 && c[0] > 200 && c[1] < 60 && c[2] < 60
    };
    let is_green = |x: u32, y: u32| {
        let c = rgba[(y * u32::from(tex.width) + x) as usize];
        c[3] > 100 && c[0] < 60 && c[1] > 120 && c[2] < 60
    };
    assert!(
        (32..=127).all(|y| is_red(50, y)),
        "column 50 is a half-alpha red marker over the lower three quarters"
    );
    assert!(
        (0..=127).all(|y| is_green(58, y)),
        "column 58 is a half-alpha green marker over the full height"
    );
}

/// **Claim 4, the negative that matters most.** Neither the material's own
/// parameter table nor `oag_formats::vex::mesh_tex_transforms` finds a
/// keyframe track anywhere on this node - a fact about the format the same
/// parser that recovers Pulse's authored `TEXOFFSET` track confirms, not an
/// absence assumed from silence.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn nothing_on_disk_authors_a_texture_offset_track() {
    let Some(image) = image() else { return };
    let mut archive = archive(&image);
    let vex_data = archive.read_path(VEX).expect("the .vex reads");
    let rcs_data = archive.read_path(RCSMODEL).expect("the .rcsmodel reads");
    let payload = glyph_payload(&vex_data);
    let hash = chunk_hash(&payload);

    // The parser that recovers Pulse's own track finds nothing in this
    // node's `.vex` payload - there is no batch list here to carry one, only
    // a bounding box and a hash.
    assert!(
        vex::mesh_tex_transforms(&payload)
            .iter()
            .all(Option::is_none),
        "a PS3 Mesh payload carries no texture-transform block for this parser to find"
    );

    // The material's own parameter table carries a bare value, not a
    // sequence: every entry declares exactly one `vec4`, which is what a
    // static value looks like in this format's own terms.
    let model = rcsmodel::Model::parse(&rcs_data).expect("parses");
    let chunk = model.mesh(hash).expect("chunk");
    let material = model.material_of(chunk).expect("material");
    assert!(
        !material.parameters.is_empty(),
        "the material is expected to carry uvOffset/uvScale"
    );
    for p in &material.parameters {
        assert_eq!(
            p.quads, 1,
            "every parameter here is a single vec4, the shape a static value \
             takes rather than a keyed sequence"
        );
    }
}

/// **Claim 5**: the glyph teleports out of view at the same 6.000 s
/// loop-closing instant Pulse's own gantry does, off an independently
/// authored track.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_glyph_teleports_at_the_six_second_loop_close() {
    let Some(image) = image() else { return };
    let mut archive = archive(&image);
    let vex_data = archive.read_path(VEX).expect("the .vex reads");

    let nodes = vex::nodes(&vex_data).expect("nodes");
    let classes = vex::classes_of(&vex_data).expect("classes");
    let anim_class = classes.anim_transform.expect("anim transform class id");
    let node = nodes
        .iter()
        .find(|n| {
            n.class_id == anim_class && n.name.as_deref().is_some_and(|name| name == GLYPH_NODE)
        })
        .expect("the glyph's own Anim Transform node is present");
    let anim = vex::anim_transform_of(&vex_data, node).expect("it decodes");

    assert_eq!(
        anim.translation.times,
        vec![359, 360],
        "the teleport is authored as one frame's worth of interpolation \
         landing exactly on frame 360 - 6.000 s at this node's own 60 Hz rate"
    );

    let before = anim.sample(5.9);
    let after = anim.sample(6.0);
    let delta_y = after[13] - before[13];
    assert!(
        delta_y > 8.0,
        "expected a teleport of roughly ten world units in Y at the loop \
         close, got {delta_y}"
    );
    assert!(
        (before[12] - after[12]).abs() < 1e-6 && (before[14] - after[14]).abs() < 1e-6,
        "the teleport is a pure Y move, matching Pulse's own +9.99-in-y teleport's axis"
    );
}

/// **Claim 6**: HD's own `polySurface7` and `pasted__Final_Lap` nodes carry
/// the same three-beat timeline (6.000/9.333/12.333 s) Pulse's own file
/// authors - checked against HD's own disc directly, not against 2048's
/// later re-export of this same file, which is where this claim was first
/// noticed.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_three_beat_timeline_holds_on_hds_own_disc_not_only_2048s_copy_of_it() {
    let Some(image) = image() else { return };
    let mut archive = archive(&image);
    let vex_data = archive.read_path(VEX).expect("the .vex reads");
    let nodes = vex::nodes(&vex_data).expect("nodes");

    let node = |name: &str| {
        nodes
            .iter()
            .find(|n| n.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("{name} should exist in HD's own file"))
    };

    let poly7 = node("polySurface7");
    let poly7_anim = vex::anim_transform_of(&vex_data, poly7).expect("polySurface7 decodes");
    assert_eq!(
        poly7_anim.translation.times,
        vec![740, 741],
        "HD's own chequered-flag node - the same name Pulse's file carries - \
         should key at exactly frame 740/741"
    );

    let final_lap = node("pasted__Final_Lap");
    let final_lap_anim =
        vex::anim_transform_of(&vex_data, final_lap).expect("pasted__Final_Lap decodes");
    assert_eq!(
        final_lap_anim.translation.times,
        vec![560, 561, 740, 741],
        "HD's own FINAL LAP node should key at 560/561 (entrance) and \
         740/741 (exit), the same frame numbers Pulse's own Final_Lap node \
         carries"
    );
}
