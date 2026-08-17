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
    b.u16(sub + 0x0a, 6);
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
    for (k, index) in [0u16, 1, 2, 1, 2, 3].into_iter().enumerate() {
        b.u16(ibuf + k * 2, index);
    }
    b.bytes.resize(ibuf + 0x100, 0);
    b.bytes
}

/// The unit cube the fixture builds, as a `.vex` node would author it.
const CUBE_BOUNDS: ([f32; 3], [f32; 3]) = ([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]);

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
/// `oag_render::mesh` does with it.
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
        submeshes,
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
