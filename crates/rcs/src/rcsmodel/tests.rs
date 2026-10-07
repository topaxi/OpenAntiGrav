//! What [`super`] is asserted to do, on hand-built files.
//!
//! The disc-backed half is `crates/formats/tests/rcsmodel_ground_truth.rs`,
//! which is where the claims about the *format* are checked. What is here is
//! the reader's own behaviour: that it refuses a blob that is not one of these,
//! and that the stride search says "no" rather than guessing.

use super::*;

/// Big-endian writer, because that is the only order this format has.
#[derive(Default)]
struct Builder {
    bytes: Vec<u8>,
}

impl Builder {
    fn at(&mut self, at: usize, bytes: &[u8]) {
        if self.bytes.len() < at + bytes.len() {
            self.bytes.resize(at + bytes.len(), 0);
        }
        self.bytes[at..at + bytes.len()].copy_from_slice(bytes);
    }
    fn u32(&mut self, at: usize, v: u32) {
        self.at(at, &v.to_be_bytes());
    }
    fn u16(&mut self, at: usize, v: u16) {
        self.at(at, &v.to_be_bytes());
    }
    fn f32(&mut self, at: usize, v: f32) {
        self.at(at, &v.to_be_bytes());
    }
    fn i16(&mut self, at: usize, v: i16) {
        self.at(at, &v.to_be_bytes());
    }
}

/// A one-mesh, one-submesh file of `VERTICES` positions spanning a unit cube,
/// laid out at `stride` with the spare bytes filled by `filler`.
///
/// **The filler is the whole point of the fixture.** A real `.rcsmodel` vertex
/// carries 8 to 16 bytes of normal, tangent and texture coordinate after its
/// position, stored normalised across the full `i16` range - so reading a
/// position at the wrong stride lands on one of those and comes out hundreds of
/// units from the model. That is the mechanism [`Mesh::solve_stride`] relies
/// on, and a fixture padded with zeroes would not have it.
fn model(stride: usize, filler: i16) -> Vec<u8> {
    const CHUNK: usize = 0x100;
    const VBUF: usize = 0x400;
    const VERTICES: usize = 240;

    let ibuf = VBUF + stride * VERTICES + 0x10;
    let mut b = Builder::default();
    b.u32(0x00, VERSION);
    b.u32(0x1c, 1);
    b.u32(0x20, 0x80);
    b.u32(0x80, CHUNK as u32);

    b.u32(CHUNK, 0xdead_beef);
    // Byte +0x06 selects the layout: this fixture is the described one.
    b.u32(CHUNK + 0x04, u32::from(LAYOUT_DESCRIBED) << 8 | 0x02);
    for i in 0..3 {
        b.f32(CHUNK + 0x30 + i * 4, 0.0);
        b.f32(CHUNK + 0x40 + i * 4, 1.0 / 128.0);
    }
    b.u32(CHUNK + 0x50, 1);
    let sub = CHUNK + SUBMESH_BASE;
    b.u16(sub + 0x08, VERTICES as u16);
    b.u16(sub + 0x0a, VERTICES as u16);
    b.u32(sub + 0x10, ibuf as u32);
    b.u32(sub + 0x18, VBUF as u32);

    // A cheap deterministic spread, plus the eight corners so the box is
    // exactly filled rather than approximately.
    let mut seed: u32 = 0x1234_5678;
    for k in 0..VERTICES {
        let at = VBUF + k * stride;
        for i in 0..3 {
            let v = if k < 8 {
                if (k >> i) & 1 == 1 { 128 } else { -128 }
            } else {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                i16::try_from((seed >> 16) % 257).unwrap_or(0) - 128
            };
            b.i16(at + i * 2, v);
        }
        for j in (POSITION_LEN..stride).step_by(2) {
            b.i16(at + j, filler);
        }
    }
    // **Names every vertex.** A stride is judged on the vertices some triangle
    // references - `Mesh::drawn_extent` - so a fixture whose indices reach only
    // a few of them describes a box the mesh does not have, and would decide
    // both the tightness test and the ambiguity one below on the fixture rather
    // than on the stride.
    for k in 0..VERTICES {
        b.u16(ibuf + k * 2, u16::try_from(k).unwrap_or(0));
    }
    b.bytes.resize(ibuf + VERTICES * 2 + 0x100, 0);
    b.bytes
}

/// The unit cube the fixture builds, as a `.vex` node would author it.
const CUBE_BOUNDS: ([f32; 3], [f32; 3]) = ([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]);

/// [`model`] with a two-entry material table, and its chunk pointing at
/// whichever of the two `pick` names.
fn model_with_materials(pick: u32) -> Vec<u8> {
    const CHUNK: usize = 0x100;

    let mut b = Builder {
        bytes: model(14, 30_000),
    };
    // Past everything `model` laid out, so the table cannot land on a chunk
    // header or a vertex buffer.
    let table = b.bytes.len();
    let records = [table + 0x10, table + 0x90];
    let names = [table + 0x110, table + 0x140];

    b.u32(0x2c, 2);
    b.u32(0x30, table as u32);
    for (i, (&record, &name)) in records.iter().zip(&names).enumerate() {
        b.u32(table + i * 4, record as u32);
        b.u32(record + 0x04, name as u32);
        // Slot 0 is an opaque road surface holding the default pair nothing
        // consumes; slot 1 is glass, blended over what is behind it.
        b.u32(record + 0x10, if i == 0 { 0x007c } else { 0x0039 });
        b.u16(record + 0x14, 0x0302);
        b.u16(record + 0x16, 0x0303);
    }
    b.at(names[0], b"data/env/track_surface.rcsmaterial\0");
    b.at(names[1], b"data/env/glass_texture.rcsmaterial\0");
    b.u32(CHUNK + 0x20, pick);
    b.bytes
}

/// The table walks, and a chunk's `+0x20` picks an entry out of it.
#[test]
fn a_chunk_points_at_one_entry_of_the_material_table() {
    let data = model_with_materials(1);
    let parsed = Model::parse(&data).expect("a model");
    assert_eq!(parsed.materials.len(), 2);
    assert_eq!(
        parsed.materials[0].name,
        "data/env/track_surface.rcsmaterial"
    );
    let mesh = &parsed.meshes[0];
    assert_eq!(mesh.material, 1);
    assert_eq!(
        parsed.material_of(mesh).map(|m| m.name.as_str()),
        Some("data/env/glass_texture.rcsmaterial")
    );
}

/// The low two state bits are what say see-through, and the factor pair alone
/// does not: both fixture materials carry the identical `0302`/`0303` pair.
#[test]
fn the_state_word_and_not_the_factor_pair_is_what_says_see_through() {
    let data = model_with_materials(0);
    let parsed = Model::parse(&data).unwrap();
    let [road, glass] = [&parsed.materials[0], &parsed.materials[1]];
    assert_eq!(
        (road.src_factor, road.dst_factor),
        (glass.src_factor, glass.dst_factor),
        "the fixture gives both the same equation on purpose"
    );
    assert_eq!(road.transparency(), Some(Transparency::Opaque));
    assert!(!road.is_see_through());
    assert_eq!(road.blend(), Blend::Opaque);
    assert_eq!(glass.transparency(), Some(Transparency::Blended));
    assert!(glass.is_see_through());
    assert_eq!(
        glass.blend(),
        Blend::Factors {
            src: material::Factor::SrcAlpha,
            dst: material::Factor::OneMinusSrcAlpha,
        }
    );
}

/// Mode 2 answers an alpha **test**, and it does so while carrying the very
/// factor pair a blended material carries.
///
/// That coincidence is the whole reason the two were drawn identically: 211 of
/// the disc's 212 mode-2 materials author `0302`/`0303`, so keying on the
/// equation can never separate them and only the state word can.
#[test]
fn mode_two_is_an_alpha_test_and_not_the_pair_it_still_carries() {
    let cutout = Material {
        name: String::new(),
        state: 2,
        src_factor: material::FACTOR_SRC_ALPHA,
        dst_factor: material::FACTOR_ONE_MINUS_SRC_ALPHA,
        alpha_func: 0x0204,
        alpha_ref: 0.5,
        texture: String::new(),
        second_texture: None,
        texture_sampler: 0,
        second_texture_sampler: None,
        samplers: Vec::new(),
        parameters: Vec::new(),
        curve: None,
    };
    assert_eq!(cutout.transparency(), Some(Transparency::Mode2));
    assert!(cutout.is_see_through());
    assert_eq!(
        cutout.blend(),
        Blend::AlphaTest,
        "bit 1 enables the alpha test and leaves blending off"
    );
}

/// **Both** factors come through, and a value outside the four the disc uses is
/// reported rather than folded onto the nearest one.
///
/// The source used to be dropped here - three different sources against a
/// destination of `GL_ONE` all read as one class - which is what made a renderer
/// draw `0001`/`0001` as `SrcAlpha`/`One`.
#[test]
fn both_factors_come_through_and_an_unknown_one_is_not_guessed_at() {
    let blended = |src, dst| Material {
        name: String::new(),
        state: 1,
        src_factor: src,
        dst_factor: dst,
        alpha_func: 0,
        alpha_ref: 0.0,
        texture: String::new(),
        second_texture: None,
        texture_sampler: 0,
        second_texture_sampler: None,
        samplers: Vec::new(),
        parameters: Vec::new(),
        curve: None,
    };
    for (src, named) in [
        (material::FACTOR_ONE, material::Factor::One),
        (material::FACTOR_SRC_COLOUR, material::Factor::SrcColour),
        (material::FACTOR_SRC_ALPHA, material::Factor::SrcAlpha),
    ] {
        assert_eq!(
            blended(src, material::FACTOR_ONE).blend(),
            Blend::Factors {
                src: named,
                dst: material::Factor::One,
            },
            "src {src:#06x} against a destination of GL_ONE is its own equation"
        );
    }
    assert_eq!(
        blended(material::FACTOR_SRC_COLOUR, 0x0302).blend(),
        Blend::Factors {
            src: material::Factor::SrcColour,
            dst: material::Factor::SrcAlpha,
        },
        "`hologram`'s pair is carried rather than left unmapped"
    );
    assert_eq!(
        blended(material::FACTOR_ONE, 0x0999).blend(),
        Blend::Unmapped {
            src: 0x0001,
            dst: 0x0999
        },
        "a fifth value is a fact about the data this reading has not seen"
    );
}

/// The fourth encoding of the transparency field, which nothing on the disc
/// uses, is `None` and reads as opaque rather than as some third blend.
#[test]
fn the_unused_transparency_encoding_is_not_guessed_at() {
    let odd = Material {
        name: String::new(),
        state: 0x0003,
        src_factor: 0x0302,
        dst_factor: 0x0303,
        alpha_func: 0,
        alpha_ref: 0.0,
        texture: String::new(),
        second_texture: None,
        texture_sampler: 0,
        second_texture_sampler: None,
        samplers: Vec::new(),
        parameters: Vec::new(),
        curve: None,
    };
    assert_eq!(odd.transparency(), None);
    assert!(
        !odd.is_see_through(),
        "an unknown mode draws its geometry rather than hiding it"
    );
}

#[test]
fn a_blob_that_is_not_one_of_these_is_refused_by_version() {
    assert_eq!(Model::parse(&[]), Err(Error::TooShort { got: 0 }));
    let mut data = model(14, 30_000);
    data[3] = 1;
    assert_eq!(
        Model::parse(&data),
        Err(Error::BadVersion { got: 0x000a_0001 }),
        "this container has no magic, so the version word is the whole check"
    );
}

#[test]
fn the_directory_walks_and_the_hash_is_what_addresses_a_mesh() {
    let data = model(14, 30_000);
    let model = Model::parse(&data).expect("a model");
    assert_eq!(model.meshes.len(), 1);
    assert!(model.mesh(0xdead_beef).is_some());
    assert!(
        model.mesh(0).is_none(),
        "an unknown hash is None, not slot 0"
    );
}

#[test]
fn positions_dequantise_through_the_bias_and_scale() {
    let data = model(14, 30_000);
    let model = Model::parse(&data).unwrap();
    let mesh = &model.meshes[0];
    let points = mesh
        .positions(&data, &mesh.submeshes[0], 14)
        .expect("positions");
    assert_eq!(points.len(), 240);
    for point in &points[..8] {
        for v in point {
            assert!(v.abs() == 1.0, "128 * 1/128 = 1, got {v}");
        }
    }
}

/// The invariant that says the index buffer was found: 1,274 of 1,274 real
/// submeshes satisfy it.
#[test]
fn an_index_past_the_vertex_count_is_an_error_rather_than_a_wrap() {
    let mut data = model(14, 30_000);
    let model = Model::parse(&data).unwrap();
    let sub = model.meshes[0].submeshes[0];
    data[sub.index_offset] = 3;
    data[sub.index_offset + 1] = 231;
    let model = Model::parse(&data).unwrap();
    assert_eq!(
        model.meshes[0].indices(&data, &sub),
        Err(Error::IndexOutOfRange {
            index: 999,
            vertices: 240
        })
    );
}

/// The search finds the stride the fixture was written at, at each of the three
/// widths the disc uses.
#[test]
fn the_stride_search_recovers_each_width_the_disc_ships() {
    for stride in [14, 18, 22] {
        let data = model(stride, 30_000);
        let model = Model::parse(&data).unwrap();
        assert_eq!(
            model.meshes[0].solve_stride(&data, CUBE_BOUNDS, 2.0 / 128.0),
            Some(stride),
            "stride {stride}"
        );
    }
}

/// When the spare bytes are themselves legal positions, the search reports
/// nothing rather than preferring one.
///
/// **The safe failure, and this fixture is harder than the disc.** A real
/// vertex's attribute bytes span the whole `i16` range, which is what makes a
/// wrong stride leave the box - see [`model`]. Fill them with a value that
/// dequantises to a legal position instead and several strides both fit and
/// fill the box, so there is no answer to give. It gives none: an undecodable
/// mesh draws nothing and is reported, which is what
/// `oag_mesh::mesh` does with it.
#[test]
fn a_mesh_whose_stride_is_not_determined_reports_nothing_rather_than_guessing() {
    let data = model(18, 64);
    let parsed = Model::parse(&data).unwrap();
    let mesh = &parsed.meshes[0];

    let contained: Vec<usize> = STRIDES
        .iter()
        .copied()
        .filter(|&stride| mesh.extent(&data, stride, CUBE_BOUNDS).is_some())
        .collect();
    assert!(
        contained.len() > 1,
        "the fixture is meant to be ambiguous, got {contained:?}"
    );
    assert_eq!(mesh.solve_stride(&data, CUBE_BOUNDS, 2.0 / 128.0), None);
}

/// A declared vertex count that overruns its own buffer does not condemn the
/// submesh, because no triangle names the vertices it overruns onto.
///
/// **This is a real shape and not a hypothetical.** `talons_junction`'s
/// `tanker4c1Shape` declares 1,288 vertices in a buffer 32 bytes short of
/// holding them at the stride the chunk declares, so the last two are read out
/// of the next submesh's buffer and land 100 units away. RSX fetches a vertex
/// when an index asks for it and never sweeps the array, so the original draws
/// the same picture either way - and judging the stride on those two dropped 17
/// of Talon's Junction's 117 node submeshes on the floor.
#[test]
fn vertices_no_triangle_names_do_not_decide_whether_a_submesh_fits() {
    // The fixture's own geometry: 240 vertices at stride 18 from 0x400, the
    // eight corners of the unit cube among them, all 240 indexed.
    const SUB: usize = 0x100 + SUBMESH_BASE;
    const VBUF: usize = 0x400;
    let mut data = model(18, 30_000);
    let honest = Model::parse(&data).unwrap();
    assert!(honest.meshes[0].submesh_fits(&data, &honest.meshes[0].submeshes[0], 18, CUBE_BOUNDS));

    // One vertex past the end, written far outside the box, and not indexed.
    // It lands in the padding between the two buffers, which is where a real
    // overrun lands too - and writing a second would reach into the index
    // buffer and test something else entirely.
    data[SUB + 0x08..SUB + 0x0a].copy_from_slice(&241u16.to_be_bytes());
    for i in 0..3 {
        let at = VBUF + 240 * 18 + i * 2;
        data[at..at + 2].copy_from_slice(&30_000i16.to_be_bytes());
    }
    let parsed = Model::parse(&data).unwrap();
    let mesh = &parsed.meshes[0];
    let submesh = &mesh.submeshes[0];
    assert_eq!(submesh.vertex_count, 241);

    let points = mesh.positions(&data, submesh, 18).expect("positions");
    assert!(
        points[240][0] > 100.0,
        "the fixture's overrun vertices have to be outside the box to test anything"
    );
    assert!(
        mesh.submesh_fits(&data, submesh, 18, CUBE_BOUNDS),
        "the stray vertex is not indexed, so nothing draws it"
    );
    assert_eq!(
        mesh.solve_stride(&data, CUBE_BOUNDS, 2.0 / 128.0),
        Some(18),
        "and the stride search is not thrown by them either"
    );
}

/// The other chunk layout: one submesh, its buffers named in the chunk header
/// and no descriptor table at all. 2,489 of the disc's chunks are this.
fn inline_model(vertices: usize, stride: usize) -> Vec<u8> {
    const CHUNK: usize = 0x100;
    const IBUF: usize = 0x200;
    const VBUF: usize = 0x300;

    let mut b = Builder::default();
    b.u32(0x00, VERSION);
    b.u32(0x1c, 1);
    b.u32(0x20, 0x80);
    b.u32(0x80, CHUNK as u32);

    b.u32(CHUNK, 0xfeed_face);
    b.u32(CHUNK + 0x04, u32::from(LAYOUT_INLINE) << 8 | 0x02);
    for i in 0..3 {
        b.f32(CHUNK + 0x30 + i * 4, 0.0);
        b.f32(CHUNK + 0x40 + i * 4, 1.0 / 128.0);
    }
    // The four fields that replace a descriptor.
    b.u32(CHUNK + 0x54, VBUF as u32);
    b.u32(CHUNK + 0x58, 3);
    b.u32(CHUNK + 0x5c, IBUF as u32);
    b.u16(CHUNK + 0x6c, vertices as u16);

    for (k, index) in [0u16, 1, 2].into_iter().enumerate() {
        b.u16(IBUF + k * 2, index);
    }
    for k in 0..vertices {
        for i in 0..3 {
            b.i16(VBUF + k * stride + i * 2, i16::try_from(k).unwrap_or(0));
        }
    }
    b.bytes.resize(VBUF + vertices * stride + 0x10, 0);
    b.bytes
}

/// The inline layout reads, and its counts come from the fields that replace
/// the descriptor rather than from `+0x50` - which is a file-wide pointer there
/// and read as a submesh count until 2026-08-17.
#[test]
fn the_inline_chunk_layout_names_its_buffers_in_the_chunk_header() {
    let data = inline_model(64, 18);
    let model = Model::parse(&data).expect("a model");
    let mesh = &model.meshes[0];
    assert_eq!(mesh.submeshes.len(), 1, "the inline layout is one submesh");
    let sub = mesh.submeshes[0];
    assert_eq!((sub.vertex_count, sub.index_count), (64, 3));
    assert_eq!(mesh.indices(&data, &sub), Ok(vec![0, 1, 2]));
    assert_eq!(mesh.positions(&data, &sub, 18).map(|p| p.len()), Ok(64));
}

/// The last four bytes of an inline stride-18 vertex read as a colour, one per
/// vertex, and a described chunk refuses.
#[test]
fn an_inline_chunks_tail_reads_as_four_colour_bytes() {
    let mut data = inline_model(4, 18);
    for (k, alpha) in [0x4cu8, 0x26, 0x00, 0xff].into_iter().enumerate() {
        let at = 0x300 + k * 18 + 14;
        data[at..at + 4].copy_from_slice(&[0xff, 0xff, 0xff, alpha]);
    }
    let model = Model::parse(&data).expect("a model");
    let mesh = &model.meshes[0];
    let colours = mesh
        .inline_colours(&data, &mesh.submeshes[0])
        .expect("inside the file");
    assert_eq!(
        colours.iter().map(|c| c[3]).collect::<Vec<_>>(),
        [0x4c, 0x26, 0x00, 0xff]
    );
    assert!(colours.iter().all(|c| c[..3] == [0xff; 3]));
}

/// A layout byte that is neither is reported, not read as one of them.
#[test]
fn an_unknown_chunk_layout_is_an_error_rather_than_a_guess() {
    let mut data = inline_model(8, 14);
    data[0x100 + 0x06] = 0x03;
    assert_eq!(
        Model::parse(&data),
        Err(Error::UnknownChunkLayout {
            got: 0x03,
            at: 0x100
        })
    );
}

/// A mesh whose only content is where its buffers sit, which is all
/// [`Mesh::solve_stride_by_layout`] reads.
fn packed(counts: &[usize], stride: usize) -> Mesh {
    let mut at = 0x1000;
    let submeshes = counts
        .iter()
        .map(|&vertex_count| {
            let sub = SubMesh {
                // Not read by anything the layout rule does; a real one opens
                // `83 XX 10 10 10 10 10 00`.
                format: [0x83, 0x08, 0x10, 0x10, 0x10, 0x10, 0x10, 0x00],
                vertex_count,
                vertex_offset: at,
                index_count: 3,
                index_offset: 0,
            };
            at += vertex_count * stride;
            sub
        })
        .collect();
    Mesh {
        hash: 0xdead_beef,
        bias: [0.0; 3],
        scale: [1.0 / 128.0; 3],
        material: 0,
        layout: Layout::Described,
        submeshes,
        decl: None,
        space: Space::World,
        render_flags: 0,
        extra_surfaces: Vec::new(),
    }
}

#[test]
fn packed_vertex_buffers_give_up_the_stride_they_were_written_at() {
    for stride in [14, 18, 22] {
        assert_eq!(
            packed(&[424, 376, 56, 1240], stride).solve_stride_by_layout(),
            Some(stride),
            "stride {stride}"
        );
    }
}

/// The residual the disc actually carries: a buffer that starts up to 96 bytes
/// before `count * stride` would end. Why is unrecovered; that it must not
/// change the answer is the point.
#[test]
fn a_step_short_of_the_declared_length_still_rounds_to_the_right_width() {
    let mut mesh = packed(&[1224, 1232, 1240], 22);
    mesh.submeshes[1].vertex_offset -= 16;
    mesh.submeshes[2].vertex_offset -= 16 + 96;
    assert_eq!(mesh.solve_stride_by_layout(), Some(22));
}

#[test]
fn one_submesh_is_no_step_to_measure_and_reports_nothing() {
    assert_eq!(packed(&[424], 22).solve_stride_by_layout(), None);
    assert_eq!(packed(&[], 22).solve_stride_by_layout(), None);
}

/// Equal support for two widths is not a majority, and there is no principle
/// here that breaks the tie - so it is unknown, like every other unresolved
/// stride in this module.
#[test]
fn two_widths_with_the_same_support_are_unknown_rather_than_the_first_one() {
    let mut mesh = packed(&[100, 100, 100], 14);
    // Leave pair one at 14 and put pair two at 22.
    mesh.submeshes[2].vertex_offset = mesh.submeshes[1].vertex_offset + 100 * 22;
    assert_eq!(mesh.solve_stride_by_layout(), None);
}

/// A step that is not one of the three widths the disc uses votes for nothing
/// rather than widening the set, which is the trap that decoded a circuit to
/// spikes - see [`STRIDES`].
#[test]
fn a_step_at_an_unsupported_width_votes_for_nothing() {
    assert_eq!(packed(&[100, 100], 32).solve_stride_by_layout(), None);
}

/// No stride explains the box: `None`, not a guess.
#[test]
fn a_box_nothing_explains_is_none() {
    let data = model(14, 30_000);
    let model = Model::parse(&data).unwrap();
    assert_eq!(
        model.meshes[0].solve_stride(&data, ([-9.0; 3], [9.0; 3]), 2.0 / 128.0),
        None,
        "a box far larger than the geometry is filled by nothing"
    );
}

/// A texture slot's sampler name hash and its `.gtf` path come from the same
/// entry of the material's own input table, and every sampler entry is read -
/// not the first two at hardcoded offsets.
///
/// **The pairing is the load-bearing part.** An entry is `(hash, kind, ...,
/// path)` at stride `0x20`, so the hash sitting eight bytes *after* a path
/// belongs to the next entry; reading it that way put every texture on its
/// neighbour's sampler. See `docs/formats/rcsmaterial.md`, "A texture slot
/// names its sampler".
#[test]
fn every_sampler_entry_is_read_and_paired_with_its_own_path() {
    const TABLE: usize = 0x100;
    const PATHS: [usize; 3] = [0x200, 0x220, 0x240];
    const NAME: usize = 0x260;
    let mut data = vec![0u8; 0x400];
    let mut put = |at: usize, word: u32| data[at..at + 4].copy_from_slice(&word.to_be_bytes());
    put(0x04, NAME as u32);
    put(0x10, 1);
    put(0x30, 4); // four entries, one of them a parameter
    put(0x34, TABLE as u32);
    for (i, (hash, path)) in [
        (0x3bdc_0403u32, Some(PATHS[0])),
        (0x37b5_db58, Some(PATHS[1])),
        (0x9edd_3243, None),
        (0x0000_0001, Some(PATHS[2])), // a parameter: skipped
    ]
    .into_iter()
    .enumerate()
    {
        let entry = TABLE + i * 0x20;
        put(entry, hash);
        put(
            entry + 0x04,
            if i == 3 { 0 } else { material::KIND_SAMPLER },
        );
        put(entry + 0x18, path.unwrap_or(0) as u32);
        put(entry + 0x1c, 1);
    }
    for (at, text) in [
        (NAME, "m.rcsmaterial"),
        (PATHS[0], "a.gtf"),
        (PATHS[1], "lm.gtf"),
        (PATHS[2], "not-a-sampler.gtf"),
    ] {
        data[at..at + text.len()].copy_from_slice(text.as_bytes());
    }

    let material = material::Material::parse(&data, 0).unwrap();
    assert_eq!(
        material.samplers,
        vec![
            (0x3bdc_0403, Some("a.gtf".to_string())),
            (0x37b5_db58, Some("lm.gtf".to_string())),
            (0x9edd_3243, None),
        ],
        "three sampler entries, in table order, the parameter left out"
    );
    assert_eq!(
        material.texture_sampler, 0x3bdc_0403,
        "~crc32(\"Texture1\")"
    );
    assert_eq!(
        material.second_texture_sampler,
        Some(0x37b5_db58),
        "~crc32(\"lightmap\") - the entry its own path sits in, not the next one"
    );
}

/// [`model`] with a relocation table at the header's `+0x04` and a
/// render-block record for its one chunk, carrying `flags` at the record's
/// `+0x06`.
///
/// The record sits where every other fixture puts nothing, and the table
/// lists the chunk's `+0x08` slot the way the disc's own tables do, so the
/// reader can be checked against the same shape the ground-truth test walks.
fn model_with_render_block(flags: u16) -> Vec<u8> {
    const CHUNK: usize = 0x100;
    let mut b = Builder {
        bytes: model(14, 30_000),
    };
    let table = b.bytes.len();
    let record = table + 0x20;
    b.u32(0x04, table as u32);
    b.u32(table, 3);
    b.u32(table + 4, 0x20);
    b.u32(table + 8, 0x30);
    b.u32(table + 12, (CHUNK + 0x08) as u32);
    b.u32(CHUNK + 0x08, record as u32);
    b.u16(record + 0x06, flags);
    b.bytes.resize(record + RENDER_BLOCK_LEN, 0);
    b.bytes
}

/// The flags come from the record the chunk's `+0x08` names, and the table
/// at the header's `+0x04` lists that slot.
#[test]
fn the_render_block_flags_are_read_through_the_chunk_header_word() {
    let data = model_with_render_block(0x0201);
    let model = Model::parse(&data).expect("a model");
    let mesh = &model.meshes[0];
    assert_eq!(mesh.render_flags, 0x0201);
    assert!(mesh.is_track(), "bit 0 is the track bit");
    assert_eq!(
        Model::relocations(&data),
        Ok(vec![0x20, 0x30, 0x108]),
        "the chunk's +0x08 slot is a relocated word"
    );

    let data = model_with_render_block(0x0800);
    let mesh = &Model::parse(&data).expect("a model").meshes[0];
    assert_eq!(mesh.render_flags, 0x0800);
    assert!(!mesh.is_track(), "bit 11 alone is not track");
}

/// **The layout byte is also a `+0x06`, of a different struct.** A described
/// chunk's layout byte is `0x05`, whose bit 0 is set; reading that byte in
/// place of the record's halfword would call every inline chunk track and
/// every described one too. A record authoring zero says otherwise.
#[test]
fn the_layout_byte_is_not_the_render_flags() {
    let data = model_with_render_block(0);
    let mesh = &Model::parse(&data).expect("a model").meshes[0];
    assert_eq!(mesh.layout, Layout::Described);
    assert_eq!(mesh.render_flags, 0);
    assert!(!mesh.is_track());

    // And a chunk with no record at all - the word left zero, as every
    // other fixture here leaves it - reads as the Scene set, not an error.
    let data = model(14, 30_000);
    let mesh = &Model::parse(&data).expect("a model").meshes[0];
    assert_eq!(mesh.render_flags, 0);
    assert!(!mesh.is_track());
}

/// A record the word names outside the file is the reader being handed the
/// wrong word, and is said so rather than read as zero.
#[test]
fn a_render_block_past_the_end_of_the_file_is_an_error() {
    let mut data = model_with_render_block(1);
    let len = data.len() as u32;
    data[0x108..0x10c].copy_from_slice(&(len - 8).to_be_bytes());
    assert_eq!(
        Model::parse(&data),
        Err(Error::OutOfBounds {
            what: "a chunk's render block",
            end: data.len() - 8 + RENDER_BLOCK_LEN,
            len: data.len(),
        })
    );
}

/// Two stride-18 vertices at `packed`'s buffer offset, with `early` at
/// `+0x0a` and `tail` in the last four bytes.
fn inline_eighteen(early: [u8; 4], tail: [u8; 4]) -> (Mesh, Vec<u8>) {
    let mesh = packed(&[2], 18);
    let mut data = vec![0u8; 0x1000 + 2 * 18];
    for k in 0..2 {
        let at = 0x1000 + k * 18;
        data[at + 10..at + 14].copy_from_slice(&early);
        data[at + 14..at + 18].copy_from_slice(&tail);
    }
    (mesh, data)
}

/// The LeachBall sphere's shape: a colour `ff ff ff cc` in the tail reads as
/// two `NaN` halves, so the coordinate is the four bytes after the normal.
#[test]
fn an_inline_eighteen_byte_vertex_with_a_colour_tail_reads_its_uv_after_the_normal() {
    let (mesh, data) = inline_eighteen([0x38, 0x00, 0x3c, 0x00], [0xff, 0xff, 0xff, 0xcc]);
    let uv = mesh
        .texcoords(&data, &mesh.submeshes[0], 18)
        .expect("coords");
    assert_eq!(uv, vec![[0.5, 1.0]; 2]);
}

/// A finite tail is still the coordinate, as it was for every inline chunk
/// before - and a tail and an early field both non-finite keep the tail.
#[test]
fn an_inline_eighteen_byte_vertex_keeps_its_tail_unless_the_tail_cannot_be_one() {
    let (mesh, data) = inline_eighteen([0x38, 0x00, 0x3c, 0x00], [0x34, 0x00, 0x34, 0x00]);
    let uv = mesh
        .texcoords(&data, &mesh.submeshes[0], 18)
        .expect("coords");
    assert_eq!(uv, vec![[0.25, 0.25]; 2]);

    let (mesh, data) = inline_eighteen([0xff, 0xff, 0xff, 0xff], [0xff, 0xff, 0xff, 0xcc]);
    let uv = mesh
        .texcoords(&data, &mesh.submeshes[0], 18)
        .expect("coords");
    assert!(uv.iter().all(|c| c[0].is_nan()));
}

/// Two stride-22 vertices at `packed`'s buffer offset: `early` at `+0x0a`,
/// colour fields `a` at `+0x0e` and `b` at `+0x12`.
fn inline_twenty_two(early: [u8; 4], a: [u8; 4], b: [u8; 4]) -> (Mesh, Vec<u8>) {
    let mesh = packed(&[2], 22);
    let mut data = vec![0u8; 0x1000 + 2 * 22];
    for k in 0..2 {
        let at = 0x1000 + k * 22;
        data[at + 10..at + 14].copy_from_slice(&early);
        data[at + 14..at + 18].copy_from_slice(&a);
        data[at + 18..at + 22].copy_from_slice(&b);
    }
    (mesh, data)
}

/// `hd_bomb_shockwaves`' shape (`ff 9f 00 4c` twice): the tail reads as `NaN`,
/// so the coordinate is the four bytes after the normal, and the two colour
/// fields fold into `[a.rgb, b.a]`.
#[test]
fn an_inline_twenty_two_byte_vertex_with_two_colours_reads_its_uv_after_the_normal() {
    let colour = [0xff, 0x9f, 0x00, 0x4c];
    let (mesh, data) = inline_twenty_two([0x38, 0x00, 0x3c, 0x00], colour, colour);
    let uv = mesh
        .texcoords(&data, &mesh.submeshes[0], 22)
        .expect("coords");
    assert_eq!(uv, vec![[0.5, 1.0]; 2]);
    let light = mesh
        .inline_two_colours(&data, &mesh.submeshes[0], 22)
        .expect("colours");
    assert_eq!(
        light,
        vec![
            [
                1.0,
                f32::from(0x9fu8) / 255.0,
                0.0,
                f32::from(0x4cu8) / 255.0
            ];
            2
        ]
    );
}

/// A stride-22 chunk with a real coordinate in its tail keeps it and has no
/// colour pair to give - the other eleven inline stride-22 chunks on the disc.
#[test]
fn an_inline_twenty_two_byte_vertex_with_a_finite_tail_keeps_it() {
    let (mesh, data) = inline_twenty_two(
        [0x38, 0x00, 0x3c, 0x00],
        [0x12, 0x34, 0x56, 0x78],
        [0x34, 0x00, 0x34, 0x00],
    );
    let uv = mesh
        .texcoords(&data, &mesh.submeshes[0], 22)
        .expect("coords");
    assert_eq!(uv, vec![[0.25, 0.25]; 2]);
    assert_eq!(
        mesh.inline_two_colours(&data, &mesh.submeshes[0], 22),
        Err(Error::NoTexcoord)
    );
}
