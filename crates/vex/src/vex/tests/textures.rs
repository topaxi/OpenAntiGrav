//! `Texture` and material nodes: the slots they keep when they cannot be
//! decoded, where a texture's name comes from, and the fields a material
//! carries.

use crate::vex::*;

/// A texture this build cannot decode must hold its place, because materials
/// name a texture by its ordinal. Dropping it renumbers every later one, and
/// the symptom is a model wearing the wrong skins rather than an error.
#[test]
fn an_undecodable_texture_keeps_its_slot() {
    // Three Texture nodes, the middle one at an unsupported 16bpp. Each
    // carries a 16-byte payload declaring a 4-byte palette and 4 texels.
    let mut tree: Vec<u8> = Vec::new();
    for bpp in [8u8, 16, 8] {
        tree.extend(CLASS_TEXTURE.to_le_bytes());
        tree.extend(0x20u16.to_le_bytes());
        tree.extend(0u16.to_le_bytes());
        tree.extend(0x10u32.to_le_bytes()); // payload length
        tree.extend(0u32.to_le_bytes()); // no children
        tree.extend([0u8; 0x10]); // name area
        // payload: 1x4 pixels, the given depth, 4-byte clut, 4 texels
        tree.extend(1u16.to_le_bytes());
        tree.extend(4u16.to_le_bytes());
        tree.push(bpp);
        tree.push(1); // mip count
        tree.extend([0u8, 0]);
        tree.extend(4u32.to_le_bytes());
        tree.extend(4u32.to_le_bytes());
    }

    let mut data = Vec::new();
    data.extend(6u32.to_le_bytes());
    data.extend((tree.len() as u32).to_le_bytes());
    data.extend((3u32 * 8).to_le_bytes());
    data.extend(MAGIC);
    data.extend(tree);
    // Three palette-and-texel blocks, distinguishable by their first byte.
    for tag in [0x11u8, 0x22, 0x33] {
        data.extend([tag, 0, 0, 255]); // one palette entry
        data.extend([0u8; 4]); // texels
    }

    let slots = textures(&data).expect("textures");
    assert_eq!(slots.len(), 3, "one slot per Texture node");
    assert!(slots[1].is_none(), "16bpp is not decodable here");

    // The third texture must have read *its own* block, which only happens
    // if the undecodable one still advanced the cursor.
    let third = slots[2].as_ref().expect("third texture");
    assert_eq!(third.palette[0], [0x33, 0, 0, 255]);
    let first = slots[0].as_ref().expect("first texture");
    assert_eq!(first.palette[0], [0x11, 0, 0, 255]);
}

/// A track's `Texture` nodes use the short 32-byte header with the name zeroed,
/// so the node name is `None` and the only name is the runtime asset path at
/// payload `+0x38`. Reading the header alone left every track texture unnamed,
/// so the blink-light heuristic never matched track geometry.
#[test]
fn a_texture_names_itself_from_the_payload_when_the_header_does_not() {
    let path = br"Data\Environments\07_Track\Textures\07_Pulse_light_BLEND_GLOW.TGA";
    // 0x38 for the fixed fields, the path, and its NUL.
    let mut payload = vec![0u8; 0x38 + path.len() + 1];
    payload[0..2].copy_from_slice(&1u16.to_le_bytes()); // width
    payload[2..4].copy_from_slice(&4u16.to_le_bytes()); // height
    payload[4] = 8; // bits per pixel
    payload[5] = 1; // mip count
    payload[8..12].copy_from_slice(&4u32.to_le_bytes()); // clut_size
    payload[12..16].copy_from_slice(&4u32.to_le_bytes()); // texel_size
    payload[0x38..0x38 + path.len()].copy_from_slice(path);

    let mut tree: Vec<u8> = Vec::new();
    tree.extend(CLASS_TEXTURE.to_le_bytes());
    // The short header: no name area at all, which is the whole point.
    tree.extend(0x10u16.to_le_bytes());
    tree.extend(0u16.to_le_bytes());
    tree.extend((payload.len() as u32).to_le_bytes());
    tree.extend(0u32.to_le_bytes()); // no children
    tree.extend(&payload);

    let mut data = Vec::new();
    data.extend(6u32.to_le_bytes());
    data.extend((tree.len() as u32).to_le_bytes());
    data.extend(8u32.to_le_bytes()); // clut + texels
    data.extend(MAGIC);
    data.extend(tree);
    data.extend([0x44u8, 0, 0, 255]); // one palette entry
    data.extend([0u8; 4]); // texels

    let slots = textures(&data).expect("textures");
    let texture = slots[0].as_ref().expect("a decodable texture");
    assert_eq!(texture.name, None, "the header carries no name");
    assert_eq!(
        texture.asset_path.as_deref(),
        Some(std::str::from_utf8(path).expect("ascii")),
        "the payload does"
    );
}

/// PS2's `.vex` scenes declare a zero-length texture block: `Texture` nodes carry
/// real dimensions and non-zero `clut_size`/`texel_size` but no palette or texel
/// bytes follow the tree. Reading them as if present walks past the end of the
/// file (what broke on `Data\Ships\Feisar\Ship.vex` on a real PS2 image); the
/// header's declared texture length is the signal to trust.
#[test]
fn a_zero_length_texture_block_returns_none_for_every_slot() {
    let mut tree: Vec<u8> = Vec::new();
    tree.extend(CLASS_TEXTURE.to_le_bytes());
    tree.extend(0x20u16.to_le_bytes());
    tree.extend(0u16.to_le_bytes());
    tree.extend(0x10u32.to_le_bytes()); // payload length
    tree.extend(0u32.to_le_bytes()); // no children
    tree.extend([0u8; 0x10]); // name area
    // A payload that looks exactly like a real, decodable 8bpp texture:
    // this is the point. Only the header's texture_len says otherwise.
    tree.extend(64u16.to_le_bytes());
    tree.extend(64u16.to_le_bytes());
    tree.push(8);
    tree.push(1);
    tree.extend([0u8, 0]);
    tree.extend(1024u32.to_le_bytes()); // clut_size
    tree.extend(4096u32.to_le_bytes()); // texel_size

    let mut data = Vec::new();
    data.extend(6u32.to_le_bytes());
    data.extend((tree.len() as u32).to_le_bytes());
    data.extend(0u32.to_le_bytes()); // texture block length: none
    data.extend(MAGIC);
    data.extend(tree);
    // No palette or texel bytes follow, matching the zero-length header.

    let slots = textures(&data).expect("a zero-length block is not an error");
    assert_eq!(slots.len(), 1, "one slot per Texture node");
    assert!(
        slots[0].is_none(),
        "no bytes to read, whatever the node claims"
    );
}

#[test]
fn a_truncated_material_keeps_its_slot() {
    // material_count says two, but the payload only holds one.
    let mut payload = vec![0u8; 0x30 + 0x14];
    payload[2..4].copy_from_slice(&2u16.to_le_bytes());
    payload[0x30 + 4..0x30 + 8].copy_from_slice(&7u32.to_le_bytes());

    let materials = mesh_materials(&payload);
    assert_eq!(
        materials,
        vec![
            Some(Material {
                flags: 0,
                texture: 7,
                second_texture: 0,
            }),
            None,
        ]
    );
}

/// All three fields come out of their documented offsets, and the tail the
/// entry does not use is not mistaken for one of them.
#[test]
fn a_material_reads_flags_and_both_texture_indices() {
    let mut payload = vec![0u8; 0x30 + 0x14];
    payload[2..4].copy_from_slice(&1u16.to_le_bytes());
    payload[0x30..0x30 + 2].copy_from_slice(&0x2001u16.to_le_bytes());
    payload[0x30 + 4..0x30 + 8].copy_from_slice(&12u32.to_le_bytes());
    payload[0x30 + 8..0x30 + 12].copy_from_slice(&34u32.to_le_bytes());

    assert_eq!(
        mesh_materials(&payload),
        vec![Some(Material {
            flags: 0x2001,
            texture: 12,
            second_texture: 34,
        })]
    );
}

/// One flagged 64x16 4-bit texture with two declared levels, as a file of the
/// given version: `class` is that version's `Texture` id (`0x3c1` at 6, `0x373`
/// at 4).
fn flagged_file(version: u32, class: u32, texels: &[u8]) -> Vec<u8> {
    texture_file(version, class, (64, 16, 4, 2, 0xe5), texels)
}

/// One texture node with `(width, height, bits per pixel, mip count, flags)` and
/// 16 palette entries, as a file of the given version.
fn texture_file(
    version: u32,
    class: u32,
    (width, height, bpp, mips, flags): (u16, u16, u8, u8, u8),
    texels: &[u8],
) -> Vec<u8> {
    let mut payload = vec![0u8; 0x10];
    payload[0..2].copy_from_slice(&width.to_le_bytes());
    payload[2..4].copy_from_slice(&height.to_le_bytes());
    payload[4] = bpp;
    payload[5] = mips;
    payload[6] = flags;
    payload[8..12].copy_from_slice(&64u32.to_le_bytes()); // 16 palette entries
    payload[12..16].copy_from_slice(&(texels.len() as u32).to_le_bytes());
    let mut tree: Vec<u8> = Vec::new();
    tree.extend(class.to_le_bytes());
    tree.extend(0x10u16.to_le_bytes());
    tree.extend(0u16.to_le_bytes());
    tree.extend((payload.len() as u32).to_le_bytes());
    tree.extend(0u32.to_le_bytes());
    tree.extend(&payload);
    let mut data = Vec::new();
    data.extend(version.to_le_bytes());
    data.extend((tree.len() as u32).to_le_bytes());
    data.extend((64 + texels.len() as u32).to_le_bytes());
    data.extend(MAGIC);
    data.extend(tree);
    data.extend((0..16u8).flat_map(|i| [i, 0, 0, 255]));
    data.extend(texels);
    data
}

fn only_texture(file: &[u8]) -> EmbeddedTexture {
    textures(file).expect("reads").remove(0).expect("decodes")
}

/// A header's mip count is a byte: 255 levels of a 64x16 texture read to the 1x1 level
/// and stop, flagged or not, without shifting a `u16` out of range.
#[test]
fn a_hostile_mip_count_reads_to_the_1x1_level_and_no_further() {
    let texels = vec![0x11u8; 16 * 1024];
    for flags in [0xe5, 0xe4] {
        let t = only_texture(&texture_file(4, 0x373, (64, 16, 4, 255, flags), &texels));
        let sizes: Vec<usize> = t.levels.iter().map(Vec::len).collect();
        assert!(sizes.len() <= 6, "flags {flags:#x}: {sizes:?}");
        assert!(sizes.iter().all(|&n| n >= 1), "{sizes:?}");
        assert!(sizes.windows(2).all(|w| w[1] <= w[0]), "{sizes:?}");
    }
    // 17 or more halvings of a 1x1 texture: nothing follows the base.
    let t = only_texture(&texture_file(4, 0x373, (1, 1, 8, 40, 0xe5), &texels));
    assert!(t.levels.is_empty(), "{:?}", t.levels.len());
}

/// A zero dimension has no levels to author, and no fabricated ones.
#[test]
fn a_zero_dimension_authors_no_levels() {
    let texels = vec![0x11u8; 256];
    for flags in [0xe5, 0xe4] {
        for (w, h) in [(0, 16), (16, 0), (0, 0)] {
            let t = only_texture(&texture_file(4, 0x373, (w, h, 4, 3, flags), &texels));
            assert!(t.levels.is_empty(), "{w}x{h} flags {flags:#x}");
            assert!(t.indices.is_empty(), "{w}x{h} flags {flags:#x}");
        }
    }
}

/// Texel `(level, y, x)` of the odd-width test's pattern; the padding nibble of a row is `0xf`
/// and never a texel of it (the pattern stays under 15).
fn pattern(level: usize, y: usize, x: usize) -> u8 {
    ((level * 7 + y * 5 + x * 3) % 15) as u8
}

/// A 4-bit level of odd width unpacks exactly its width: the last byte of a row carries a
/// padding nibble that is not the next row's first texel. 3x8 has levels 1x4, 1x2 and 1x1.
#[test]
fn a_four_bit_level_of_odd_width_unpacks_exactly_its_width() {
    let mut texels = Vec::new();
    for (level, (w, h)) in [(3usize, 8usize), (1, 4), (1, 2), (1, 1)]
        .into_iter()
        .enumerate()
    {
        for y in 0..h {
            let mut row = vec![0u8; 16];
            for x in 0..w.div_ceil(2) * 2 {
                let n = if x < w { pattern(level, y, x) } else { 0xf };
                row[x / 2] |= n << (4 * (x % 2));
            }
            texels.extend(row);
        }
    }
    let t = only_texture(&texture_file(4, 0x373, (3, 8, 4, 4, 0xe4), &texels));
    let want = |level: usize, w: usize, h: usize| -> Vec<u8> {
        (0..h)
            .flat_map(|y| (0..w).map(move |x| pattern(level, y, x)))
            .collect()
    };
    assert_eq!(t.indices, want(0, 3, 8), "the base level");
    assert_eq!(
        t.levels,
        vec![want(1, 1, 4), want(2, 1, 2), want(3, 1, 1)],
        "the levels below it"
    );
}

/// A flagged texture whose block is shorter than its base level unswizzles the blocks it has,
/// as a file without levels does, rather than reading the raw swizzled bytes.
#[test]
fn a_flagged_block_shorter_than_its_base_level_still_unswizzles_what_it_holds() {
    // 64x16 at stride 32 is 4 blocks of 128 bytes; give two of them.
    let block: Vec<u8> = (0..256u32).map(|i| (i * 11 % 251) as u8).collect();
    let t = only_texture(&texture_file(4, 0x373, (64, 16, 4, 2, 0xe5), &block));
    let rows = oag_formats::swizzle::unswizzle(&block, 32, 16);
    let want: Vec<u8> = rows[..32].iter().flat_map(|v| [v & 15, v >> 4]).collect();
    assert_eq!(t.indices[..64], want[..], "row 0 is the unswizzled one");
    assert_eq!(t.indices.len(), 64 * 16);
    assert!(t.levels.is_empty());
}

/// A flagged texture is unswizzled, and hands on its authored levels, on every version.
#[test]
fn a_flagged_texture_is_unswizzled_and_every_version_keeps_its_levels() {
    // Level 0: 64x16 at stride 32 (2 block columns, 2 block rows), level 1:
    // 32x8 at stride 16 (one block, so it is its own swizzle).
    let linear0: Vec<u8> = (0..32 * 16).map(|i| (i % 251) as u8).collect();
    let level1: Vec<u8> = (0..16 * 8).map(|i| (i * 7 % 253) as u8).collect();
    let mut swizzled0 = Vec::new();
    for block_row in 0..2 {
        for block_col in 0..2 {
            for row in 0..8 {
                let at = (block_row * 8 + row) * 32 + block_col * 16;
                swizzled0.extend_from_slice(&linear0[at..at + 16]);
            }
        }
    }
    let texels: Vec<u8> = swizzled0.iter().chain(&level1).copied().collect();
    let nibbles = |b: &[u8]| -> Vec<u8> { b.iter().flat_map(|v| [v & 15, v >> 4]).collect() };

    let six = textures(&flagged_file(6, 0x3c1, &texels))
        .expect("v6")
        .remove(0)
        .expect("decodes");
    assert_eq!(six.indices, nibbles(&linear0), "level 0 unswizzled");
    assert_eq!(
        six.levels,
        vec![nibbles(&level1)],
        "the authored level, kept"
    );

    let four = textures(&flagged_file(4, 0x373, &texels))
        .expect("v4")
        .remove(0)
        .expect("decodes");
    assert_eq!(four.indices, six.indices, "level 0 is the same on both");
    assert_eq!(four.levels, six.levels, "version 4 authors its levels too");
}
