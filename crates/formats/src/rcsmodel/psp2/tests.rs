//! What the 2048 `.rcsmodel` reader in [`super`] is asserted to do.

use super::*;

/// Builds an image with one CPU section, one GPU section and `submeshes`
/// submeshes, each a fan of `(indices, vertices, stride)`.
fn build(submeshes: &[(usize, usize, usize)]) -> Vec<u8> {
    // The CPU section: one 0x40-byte record per submesh.
    const RECORD: usize = 0x40;
    let mut cpu = vec![0u8; submeshes.len() * RECORD];
    let mut gpu = Vec::new();
    let mut relocations = Vec::new();
    for (i, &(index_count, vertex_count, stride)) in submeshes.iter().enumerate() {
        let record = i * RECORD;
        cpu[record..record + 4].copy_from_slice(&(index_count as u32).to_le_bytes());
        cpu[record + 4..record + 8].copy_from_slice(&(vertex_count as u32).to_le_bytes());

        let index_at = gpu.len() as u32;
        for k in 0..index_count {
            gpu.extend_from_slice(&((k % vertex_count) as u16).to_le_bytes());
        }
        // Rounded up to four, exactly as the shipped files are.
        while gpu.len() % 4 != 0 {
            gpu.push(0);
        }
        let vertex_at = gpu.len() as u32;
        for v in 0..vertex_count {
            for a in 0..3 {
                gpu.extend_from_slice(&((v * 3 + a) as f32).to_bits().to_le_bytes());
            }
            gpu.extend(std::iter::repeat_n(0u8, stride - 12));
        }

        cpu[record + INDEX_POINTER..record + INDEX_POINTER + 4]
            .copy_from_slice(&index_at.to_le_bytes());
        cpu[record + VERTEX_POINTER..record + VERTEX_POINTER + 4]
            .copy_from_slice(&vertex_at.to_le_bytes());
        relocations.push((record + INDEX_POINTER) as u32);
        relocations.push((record + VERTEX_POINTER) as u32);
    }

    let header_len = DESCRIPTOR_BASE + 2 * DESCRIPTOR_LEN + relocations.len() * RELOCATION_LEN;
    let mut out = vec![0u8; header_len];
    out[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    out[0x08..0x0c].copy_from_slice(&2u32.to_le_bytes());
    out[0x0c..0x10].copy_from_slice(&(header_len as u32).to_le_bytes());
    // The CPU descriptor: no relocations of its own in this fixture.
    out[0x24..0x28].copy_from_slice(&(cpu.len() as u32).to_le_bytes());
    out[0x28..0x2c].copy_from_slice(&0u32.to_le_bytes());
    out[0x2c..0x30].copy_from_slice(&0u32.to_le_bytes());
    // The GPU descriptor, whose table is every pointer above.
    out[0x44..0x48].copy_from_slice(&(gpu.len() as u32).to_le_bytes());
    out[0x48..0x4c].copy_from_slice(&0u32.to_le_bytes());
    out[0x4c..0x50].copy_from_slice(&(relocations.len() as u32).to_le_bytes());
    let table = DESCRIPTOR_BASE + 2 * DESCRIPTOR_LEN;
    for (i, &offset) in relocations.iter().enumerate() {
        let at = table + i * RELOCATION_LEN;
        out[at..at + 4].copy_from_slice(&offset.to_le_bytes());
        out[at + 4..at + 8].copy_from_slice(&0u32.to_le_bytes());
    }
    out.extend_from_slice(&cpu);
    out.extend_from_slice(&gpu);
    out
}

#[test]
fn it_reads_a_submesh_out_of_the_relocation_table() {
    let decoded = parse(&build(&[(9, 4, 28)])).expect("the fixture parses");
    assert_eq!(decoded.sections.len(), 2);
    assert_eq!(decoded.submeshes.len(), 1);
    let mesh = &decoded.submeshes[0];
    assert_eq!(mesh.indices.len(), 9);
    assert_eq!(mesh.triangle_count(), 3);
    assert_eq!(mesh.positions.len(), 4);
    assert_eq!(mesh.stride, 28);
    assert_eq!(mesh.positions[0], [0.0, 1.0, 2.0]);
    assert_eq!(mesh.positions[3], [9.0, 10.0, 11.0]);
    assert_eq!(decoded.unpaired_pointers, 0);
}

/// **The stride is not in a field**: it comes from the buffer's own length
/// divided by the vertex count, which is what makes several strides in one file
/// an ordinary state rather than a contradiction.
#[test]
fn the_stride_comes_from_the_buffers_own_length() {
    let decoded = parse(&build(&[(3, 3, 16), (6, 5, 24), (9, 7, 40)])).expect("parses");
    let strides: Vec<usize> = decoded.submeshes.iter().map(|m| m.stride).collect();
    assert_eq!(strides, vec![16, 24, 40]);
    for mesh in &decoded.submeshes {
        assert_eq!(mesh.positions.len() * mesh.stride % mesh.stride, 0);
    }
}

#[test]
fn it_refuses_the_wipeout_hd_container() {
    // HD's opens with a big-endian version word and no magic at all, which is
    // exactly the confusion this check exists to stop.
    let mut hd = vec![0u8; 0x80];
    hd[0..4].copy_from_slice(&crate::rcsmodel::VERSION.to_be_bytes());
    assert!(matches!(parse(&hd), Err(Error::BadMagic { .. })));
}

/// A file whose sections do not add up to its own length is refused, which is
/// the closure all 993 shipped files satisfy.
#[test]
fn it_refuses_sections_that_do_not_add_up() {
    let mut file = build(&[(3, 3, 16)]);
    file.push(0);
    assert!(matches!(
        parse(&file),
        Err(Error::SectionsDoNotClose { .. })
    ));
    let mut file = build(&[(3, 3, 16)]);
    file.truncate(file.len() - 1);
    assert!(matches!(
        parse(&file),
        Err(Error::SectionsDoNotClose { .. })
    ));
}

#[test]
fn it_refuses_a_header_that_runs_past_the_end() {
    let file = build(&[(3, 3, 16)]);
    for cut in [4, 12, 0x20, 0x50] {
        assert!(parse(&file[..cut]).is_err(), "{cut} bytes should not parse");
    }
}

/// One section and no GPU block is an ordinary state - 43 of the corpus's 993 -
/// because `RcsModel_Load` skips the whole second allocation when its size is
/// zero. It reads as a model with no geometry, not as a failure.
#[test]
fn a_model_with_no_gpu_section_has_no_geometry_and_is_not_an_error() {
    let header_len = DESCRIPTOR_BASE + DESCRIPTOR_LEN;
    let mut out = vec![0u8; header_len];
    out[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    out[0x08..0x0c].copy_from_slice(&1u32.to_le_bytes());
    out[0x0c..0x10].copy_from_slice(&(header_len as u32).to_le_bytes());
    out[0x24..0x28].copy_from_slice(&8u32.to_le_bytes());
    out.extend_from_slice(&[0u8; 8]);
    let decoded = parse(&out).expect("a model with no geometry still parses");
    assert!(!decoded.has_geometry());
    assert!(decoded.submeshes.is_empty());
}

/// A pointer pair that is not a submesh is left alone and counted, rather than
/// read as one - the check that makes finding records through the relocation
/// table safe.
#[test]
fn a_pair_that_does_not_check_out_is_counted_not_decoded() {
    let mut file = build(&[(9, 4, 28)]);
    // Break the index count so it no longer divides by three: the record sits
    // at the very start of the CPU section, which follows the header.
    let header_len = u32::from_le_bytes(file[0x0c..0x10].try_into().unwrap()) as usize;
    file[header_len..header_len + 4].copy_from_slice(&7u32.to_le_bytes());
    let decoded = parse(&file).expect("the container still parses");
    assert!(decoded.submeshes.is_empty());
    assert_eq!(decoded.unpaired_pointers, 2);
}
