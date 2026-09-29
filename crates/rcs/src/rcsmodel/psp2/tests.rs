//! What the 2048 `.rcsmodel` reader in [`super`] is asserted to do.

use super::material::PS4_SAMPLER_STRIDE;
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

/// Builds a one-submesh image like [`build`], but pokes a specific 3-byte
/// normal into every vertex at [`NORMAL_OFFSET`] instead of leaving the tail
/// zeroed - proves the decode reads the field this module claims, at the
/// offset it claims, not a neighbour or a zeroed pad.
fn build_with_normal(vertex_count: usize, stride: usize, normal: [u8; 3]) -> Vec<u8> {
    let mut file = build(&[(vertex_count * 3, vertex_count, stride)]);
    let decoded = parse(&file).expect("the plain fixture parses");
    let mesh = &decoded.submeshes[0];
    assert_eq!(mesh.positions.len(), vertex_count);
    let gpu = decoded.sections[1];
    let base = gpu.at + gpu.len - vertex_count * stride;
    for v in 0..vertex_count {
        let at = base + v * stride + NORMAL_OFFSET;
        file[at..at + 3].copy_from_slice(&normal);
    }
    file
}

/// Builds an image like [`build`], but each submesh's own record spells out
/// which of [`KNOWN_BUFFER_POINTER_GAPS`] separates its two buffer pointers -
/// the shape a real sky file mixes (`SKY_BUFFER_POINTER_GAP` on the dome
/// itself, [`BUFFER_POINTER_GAP`] on whatever else the record graph
/// carries), which [`build`] alone cannot exercise since every one of its
/// records is the ordinary gap.
fn build_with_gaps(submeshes: &[(usize, usize, usize, usize)]) -> Vec<u8> {
    let record_len = INDEX_POINTER + submeshes.iter().map(|s| s.3).max().unwrap_or(0) + 4;
    let mut cpu = vec![0u8; submeshes.len() * record_len];
    let mut gpu = Vec::new();
    let mut relocations = Vec::new();
    for (i, &(index_count, vertex_count, stride, gap)) in submeshes.iter().enumerate() {
        let record = i * record_len;
        cpu[record..record + 4].copy_from_slice(&(index_count as u32).to_le_bytes());
        cpu[record + 4..record + 8].copy_from_slice(&(vertex_count as u32).to_le_bytes());

        let index_at = gpu.len() as u32;
        for k in 0..index_count {
            gpu.extend_from_slice(&((k % vertex_count) as u16).to_le_bytes());
        }
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

        let index_ptr_at = record + INDEX_POINTER;
        let vertex_ptr_at = record + INDEX_POINTER + gap;
        cpu[index_ptr_at..index_ptr_at + 4].copy_from_slice(&index_at.to_le_bytes());
        cpu[vertex_ptr_at..vertex_ptr_at + 4].copy_from_slice(&vertex_at.to_le_bytes());
        relocations.push(index_ptr_at as u32);
        relocations.push(vertex_ptr_at as u32);
    }

    let header_len = DESCRIPTOR_BASE + 2 * DESCRIPTOR_LEN + relocations.len() * RELOCATION_LEN;
    let mut out = vec![0u8; header_len];
    out[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    out[0x08..0x0c].copy_from_slice(&2u32.to_le_bytes());
    out[0x0c..0x10].copy_from_slice(&(header_len as u32).to_le_bytes());
    out[0x24..0x28].copy_from_slice(&(cpu.len() as u32).to_le_bytes());
    out[0x28..0x2c].copy_from_slice(&0u32.to_le_bytes());
    out[0x2c..0x30].copy_from_slice(&0u32.to_le_bytes());
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

/// A [`SKY_BUFFER_POINTER_GAP`] pair is found on its own, the same as an
/// ordinary [`BUFFER_POINTER_GAP`] one - proves the second gap is genuinely
/// tried, not just accepted by coincidence when it happens to sit next to an
/// ordinary one.
#[test]
fn a_sky_gap_submesh_is_found_on_its_own() {
    let decoded = parse(&build_with_gaps(&[(9, 4, 20, SKY_BUFFER_POINTER_GAP)])).expect("parses");
    assert_eq!(decoded.submeshes.len(), 1);
    assert_eq!(decoded.submeshes[0].stride, 20);
    assert_eq!(decoded.unpaired_pointers, 0);
}

/// The real shape a six-submesh sky file carries: one [`SKY_BUFFER_POINTER_GAP`]
/// record then five ordinary ones. **The regression this guards**: pairing
/// greedily by index (`i += 2` only on a match) must not let a matched
/// sky-gap pair shift where the scan resumes, or every downstream ordinary
/// pair desynchronises - which would either miss real submeshes or, worse,
/// pair two pointers that were never a submesh together.
#[test]
fn a_sky_gap_record_does_not_desynchronise_the_ordinary_pairs_after_it() {
    let specs = [
        (9, 4, 20, SKY_BUFFER_POINTER_GAP),
        (6, 3, 28, BUFFER_POINTER_GAP),
        (12, 5, 28, BUFFER_POINTER_GAP),
    ];
    let decoded = parse(&build_with_gaps(&specs)).expect("parses");
    assert_eq!(decoded.submeshes.len(), 3);
    let counts: Vec<(usize, usize)> = decoded
        .submeshes
        .iter()
        .map(|m| (m.indices.len(), m.positions.len()))
        .collect();
    assert_eq!(counts, vec![(9, 4), (6, 3), (12, 5)]);
    assert_eq!(decoded.unpaired_pointers, 0);
}

/// A gap that is neither known value is left unpaired, the same as any other
/// pointer pair that does not check out - the corpus-safety property the
/// second gap must not weaken: an arbitrary gap must still not be read as a
/// submesh.
#[test]
fn an_unknown_gap_is_not_taken_as_a_submesh() {
    let decoded = parse(&build_with_gaps(&[(9, 4, 20, 100)])).expect("parses");
    assert!(decoded.submeshes.is_empty());
    assert_eq!(decoded.unpaired_pointers, 2);
}

#[test]
fn unpack_normal_decodes_signed_bytes_over_127() {
    assert_eq!(unpack_normal([127, 0, 0]), [1.0, 0.0, 0.0]);
    assert_eq!(unpack_normal([0, 127, 0]), [0.0, 1.0, 0.0]);
    assert_eq!(unpack_normal([0, 0, 127]), [0.0, 0.0, 1.0]);
    // -128 two's-complement, not -127 - the encoder never emits it (every
    // shipped normal byte measured is unit-length within float rounding),
    // but the decode itself must not panic or wrap.
    let n = unpack_normal([0x80, 0, 0]);
    assert!((n[0] + 128.0 / 127.0).abs() < 1e-6, "{n:?}");
}

/// The normal decodes from the declared offset, on a stride that carries one
/// (28, the commonest declared layout) and on the smallest stride this
/// reading has ever seen carry one (16).
#[test]
fn a_submeshs_normal_decodes_at_the_declared_offset() {
    for stride in [16, 20, 28] {
        let file = build_with_normal(4, stride, [127, 0, 0]);
        let decoded = parse(&file).expect("parses");
        let mesh = &decoded.submeshes[0];
        assert_eq!(mesh.normals.len(), 4, "stride {stride}");
        for n in &mesh.normals {
            assert_eq!(*n, [1.0, 0.0, 0.0], "stride {stride}");
        }
    }
}

/// A stride too small to hold a normal past the position leaves `normals`
/// empty rather than reading into the next vertex - not observed on any
/// shipped file (16 is the smallest declared stride), but not assumed away.
#[test]
fn too_small_a_stride_leaves_normals_empty() {
    let file = build(&[(9, 4, 12)]);
    let decoded = parse(&file).expect("parses");
    assert!(decoded.submeshes[0].normals.is_empty());
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

/// The PS4 Omega Collection's record puts its two buffer pointers
/// [`PS4_BUFFER_POINTER_GAP`] apart, and is found on its own like the other
/// two shapes. On `ag_systems\ship.rcsmodel` nine of the twelve records are this
/// shape and a reader without it found the other three only.
#[test]
fn a_ps4_gap_submesh_is_found_on_its_own() {
    let decoded = parse(&build_with_gaps(&[(9, 4, 24, PS4_BUFFER_POINTER_GAP)])).expect("parses");
    assert_eq!(decoded.submeshes.len(), 1);
    assert_eq!(decoded.submeshes[0].stride, 24);
    assert_eq!(decoded.unpaired_pointers, 0);
}

/// All three shapes in one file, which is what a PS4 ship is: a mirrored twin
/// in the Vita's 184-byte shape beside the 32-byte ones. None of them may
/// shift where the scan resumes for the next.
#[test]
fn the_three_gaps_mix_without_desynchronising() {
    let specs = [
        (9, 4, 20, SKY_BUFFER_POINTER_GAP),
        (6, 3, 28, PS4_BUFFER_POINTER_GAP),
        (12, 5, 24, PS4_BUFFER_POINTER_GAP),
        (6, 3, 28, BUFFER_POINTER_GAP),
    ];
    let decoded = parse(&build_with_gaps(&specs)).expect("parses");
    let counts: Vec<(usize, usize)> = decoded
        .submeshes
        .iter()
        .map(|m| (m.indices.len(), m.positions.len()))
        .collect();
    assert_eq!(counts, vec![(9, 4), (6, 3), (12, 5), (6, 3)]);
    assert_eq!(decoded.unpaired_pointers, 0);
}

/// The header word that says a file is PS4's. A Vita file has `0` there and
/// the fixture builder writes none, so the plain fixtures are Vita.
#[test]
fn a_ps4_file_is_told_from_a_vita_one_by_its_header_word() {
    let mut file = build(&[(3, 3, 24)]);
    assert!(!is_ps4(&file));
    file[4..8].copy_from_slice(&PS4_HEADER_WORD.to_le_bytes());
    assert!(is_ps4(&file));
    assert!(!is_ps4(&file[..6]), "a truncated header is not PS4");
    // The container reader does not care which it is.
    assert!(parse(&file).is_ok());
}

/// Appends one PS4 sampler entry at `at`: the name hash, then the `0x12`
/// word every measured entry carries, then a pointer [`PS4_SAMPLER_STRIDE`]
/// apart from the next entry's.
fn put_sampler(cpu: &mut [u8], at: usize, hash: u32, pointer: usize) {
    cpu[at..at + 4].copy_from_slice(&hash.to_le_bytes());
    cpu[at + 4..at + 8].copy_from_slice(&0x12u32.to_le_bytes());
    cpu[at + 0x18..at + 0x20].copy_from_slice(&(pointer as u64).to_le_bytes());
}

/// Writes `text` NUL-terminated at `at` and returns where it starts, which is
/// what a pointer to it holds. A NUL sits before it, as in the real pool.
fn put_str(cpu: &mut [u8], at: usize, text: &str) -> usize {
    cpu[at..at + text.len()].copy_from_slice(text.as_bytes());
    at
}

/// A material whose first-listed texture is not its diffuse - the road on
/// `tech_de_ra`, `track_surface_displacement2out`: a displacement map, then
/// `Diffuse`, `Normal` and the per-object `lightmap`. The diffuse comes first
/// after the read, the unnamed hash next, the other known roles last.
#[test]
fn a_ps4_material_lists_its_diffuse_texture_first() {
    use crate::rcsmaterial::name_hash;
    let mut cpu = vec![0u8; 0x400];
    let name = put_str(&mut cpu, 0x300, "data/materials/road.rcsmaterial");
    let displacement = put_str(&mut cpu, 0x330, "data/tex/displacement.gnf");
    let diffuse = put_str(&mut cpu, 0x350, "data/tex/diffuse.gnf");
    let normal = put_str(&mut cpu, 0x370, "data/tex/normal.gnf");
    let lightmap = put_str(&mut cpu, 0x390, "data/tex/lightmap-lmap.gnf");
    // The material's own name pointer, then four entries a stride apart.
    cpu[0x08..0x10].copy_from_slice(&(name as u64).to_le_bytes());
    let first = 0x20;
    put_sampler(&mut cpu, first, 0xdead_beef, displacement);
    put_sampler(
        &mut cpu,
        first + PS4_SAMPLER_STRIDE,
        name_hash("Diffuse"),
        diffuse,
    );
    put_sampler(
        &mut cpu,
        first + 2 * PS4_SAMPLER_STRIDE,
        name_hash("Normal"),
        normal,
    );
    put_sampler(
        &mut cpu,
        first + 3 * PS4_SAMPLER_STRIDE,
        name_hash("lightmap"),
        lightmap,
    );

    let materials = material::read_ps4(&cpu);
    assert_eq!(materials.len(), 1);
    assert_eq!(materials[0].name, "data/materials/road.rcsmaterial");
    assert_eq!(
        materials[0].textures,
        [
            "data/tex/diffuse.gnf",
            "data/tex/displacement.gnf",
            "data/tex/normal.gnf",
            "data/tex/lightmap-lmap.gnf",
        ]
    );
    assert_eq!(materials[0].diffuse_texture(), Some("data/tex/diffuse.gnf"));
}

/// Two materials are two entries in ascending address order, and each one's
/// scan stops at the next one's name pointer - a texture belongs to the
/// material above it, never the one below.
#[test]
fn ps4_materials_are_taken_in_address_order_and_do_not_share_textures() {
    let mut cpu = vec![0u8; 0x400];
    let first = put_str(&mut cpu, 0x300, "data/materials/a.rcsmaterial");
    let second = put_str(&mut cpu, 0x330, "data/materials/b.rcsmaterial");
    let tex_a = put_str(&mut cpu, 0x360, "data/tex/a.gnf");
    let tex_b = put_str(&mut cpu, 0x380, "data/tex/b.gnf");
    cpu[0x08..0x10].copy_from_slice(&(first as u64).to_le_bytes());
    put_sampler(&mut cpu, 0x20, 0xdead_beef, tex_a);
    cpu[0x80..0x88].copy_from_slice(&(second as u64).to_le_bytes());
    put_sampler(&mut cpu, 0xa0, 0xdead_beef, tex_b);

    let materials = material::read_ps4(&cpu);
    let names: Vec<&str> = materials.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "data/materials/a.rcsmaterial",
            "data/materials/b.rcsmaterial"
        ]
    );
    assert_eq!(materials[0].textures, ["data/tex/a.gnf"]);
    assert_eq!(materials[1].textures, ["data/tex/b.gnf"]);
}

/// A pointer into the middle of a path is not a material: the byte before a
/// real pointer's target is NUL, and without that check a truncated
/// `.rcsmaterial` suffix passes as a shorter, wrong name.
#[test]
fn a_pointer_into_the_middle_of_a_path_is_not_a_material() {
    let mut cpu = vec![0u8; 0x100];
    put_str(&mut cpu, 0x80, "data/materials/real.rcsmaterial");
    cpu[0x08..0x10].copy_from_slice(&(0x80u64 + 5).to_le_bytes());
    assert!(material::read_ps4(&cpu).is_empty());
}
