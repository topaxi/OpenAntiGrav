use super::*;

/// Vertex `i` at `x = i`, its alpha `i / 10`, so every vertex is told apart.
fn base(count: u32) -> Vec<mesh::GpuVertex> {
    (0..count)
        .map(|i| mesh::GpuVertex {
            position: [i as f32, 0.0, 0.0],
            colour: [0.5, 0.5, 0.5, i as f32 / 10.0],
            ..bytemuck::Zeroable::zeroed()
        })
        .collect()
}

/// What was written at each vertex: `(vertex, bytes)`.
type Written = Vec<(u32, Vec<u8>)>;

/// What the write of one pad at a time put at each vertex.
fn per_pad(
    ranges: &[std::ops::Range<u32>],
    base: &[mesh::GpuVertex],
    colours: &[[f32; 3]],
) -> Written {
    let mut out = Vec::new();
    for (range, colour) in ranges.iter().zip(colours) {
        for index in range.clone() {
            let mut v = base[index as usize];
            v.colour = [colour[0], colour[1], colour[2], v.colour[3]];
            v.position[1] += index as f32;
            out.push((index, bytemuck::bytes_of(&v).to_vec()));
        }
    }
    out
}

/// The writes `recolour_runs` makes, and the same split into `(vertex, bytes)`.
fn runs(
    ranges: &[std::ops::Range<u32>],
    base: &[mesh::GpuVertex],
    colours: &[[f32; 3]],
) -> (Vec<std::ops::Range<u32>>, Written) {
    let (mut writes, mut vertices) = (Vec::new(), Vec::new());
    recolour_runs(
        ranges,
        base,
        colours.iter().copied(),
        |index, v| v.position[1] += index as f32,
        &mut Vec::new(),
        |first, written| {
            writes.push(first..first + written.len() as u32);
            for (index, v) in (first..).zip(written) {
                vertices.push((index, bytemuck::bytes_of(v).to_vec()));
            }
        },
    );
    (writes, vertices)
}

const COLOURS: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

#[test]
fn pads_built_back_to_back_are_one_write_of_the_bytes_each_pad_wrote() {
    let base = base(9);
    let ranges = [0..4, 4..6, 6..9];
    let (writes, vertices) = runs(&ranges, &base, &COLOURS);
    assert_eq!(writes, vec![0..9]);
    assert_eq!(vertices, per_pad(&ranges, &base, &COLOURS));
}

#[test]
fn a_gap_between_pads_splits_the_write_and_is_never_written() {
    let base = base(10);
    let ranges = [0..3, 3..5, 6..10];
    let (writes, vertices) = runs(&ranges, &base, &COLOURS);
    assert_eq!(writes, vec![0..5, 6..10]);
    assert_eq!(vertices, per_pad(&ranges, &base, &COLOURS));
}

#[test]
fn a_pad_past_the_vertices_is_skipped_and_its_colour_with_it() {
    let base = base(6);
    let ranges = [0..3, 3..9, 3..6];
    let (writes, vertices) = runs(&ranges, &base, &COLOURS);
    assert_eq!(writes, vec![0..6]);
    assert_eq!(
        vertices,
        per_pad(&[0..3, 3..6], &base, &[COLOURS[0], COLOURS[2]])
    );
}
