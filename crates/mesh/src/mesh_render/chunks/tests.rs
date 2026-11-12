use super::*;

/// `count` quads of four vertices each, one draw per quad.
fn quads(count: u32) -> (u32, Vec<u32>, Vec<Range<u32>>) {
    let mut indices = Vec::new();
    let mut draws = Vec::new();
    for quad in 0..count {
        let start = indices.len() as u32;
        indices.extend([0, 1, 2, 0, 2, 3].map(|i| quad * 4 + i));
        draws.push(start..start + 6);
    }
    (count * 4, indices, draws)
}

const fn bytes(vertices: u64) -> u64 {
    vertices * VERTEX_BYTES
}

#[test]
fn a_model_that_fits_is_one_chunk() {
    let (vertices, indices, draws) = quads(8);
    let chunks =
        VertexChunks::plan_ranges(vertices, &indices, draws.into_iter(), bytes(32)).unwrap();
    assert!(chunks.is_whole());
    assert_eq!(chunks.ranges(), [0..32]);
}

#[test]
fn a_model_past_the_cap_is_cut_between_draws_and_tiles_every_vertex() {
    let (vertices, indices, draws) = quads(8);
    let chunks =
        VertexChunks::plan_ranges(vertices, &indices, draws.into_iter(), bytes(10)).unwrap();
    // Two quads (8 vertices) fit under 10 vertices; three (12) do not.
    assert_eq!(chunks.ranges(), [0..8, 8..16, 16..24, 24..32]);
    assert!(chunks.ranges().iter().all(|r| r.len() as u64 * VERTEX_BYTES <= bytes(10)));
}

#[test]
fn no_cut_lands_inside_a_draw() {
    // The last draw reaches back from vertex 5 to vertex 3, so the cut at 4 is
    // not allowed: the first chunk ends at 8 although 4 is the nearer boundary.
    let (vertices, mut indices, mut draws) = quads(4);
    indices.extend([3, 4, 5]);
    draws.push(24..27);
    let chunks =
        VertexChunks::plan_ranges(vertices, &indices, draws.iter().cloned(), bytes(8)).unwrap();
    assert_eq!(chunks.ranges(), [0..8, 8..16]);
    for draw in draws {
        let span = &indices[draw.start as usize..draw.end as usize];
        let chunk = chunks.chunk_of(span[0]);
        assert!(
            span.iter().all(|&v| chunks.range(chunk).contains(&v)),
            "draw {draw:?} straddles {:?}",
            chunks.ranges()
        );
    }
    assert!(chunks.len() > 1);
}

#[test]
fn a_draw_wider_than_the_cap_is_refused_by_name() {
    let (vertices, indices, mut draws) = quads(4);
    draws.push(0..24);
    let error =
        VertexChunks::plan_ranges(vertices, &indices, draws.into_iter(), bytes(8)).unwrap_err();
    assert_eq!(error.cap, 8);
    assert!(error.to_string().contains("straddle"));
}

#[test]
fn a_write_is_split_at_the_chunk_boundaries() {
    let chunks = VertexChunks {
        ranges: vec![0..8, 8..16, 16..24],
    };
    let pieces: Vec<_> = chunks.split(6, 12).collect();
    assert_eq!(
        pieces,
        [(0, 6, 0..2), (1, 0, 2..10), (2, 0, 10..12)],
        "vertices 6..18 over three chunks"
    );
    assert_eq!(chunks.split(8, 8).collect::<Vec<_>>(), [(1, 0, 0..8)]);
    assert_eq!(chunks.base_vertex(2), -16);
}
