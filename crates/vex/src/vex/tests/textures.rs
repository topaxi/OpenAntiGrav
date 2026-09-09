//! `Texture` and material nodes: the slots they keep when they cannot be
//! decoded, where a texture's name comes from, and the fields a material
//! carries.
//!
//! Split out of `vex.rs`'s `#[cfg(test)] mod tests`, which was 1,108
//! lines - past the 200 an inline test module may hold, and past the 1,000
//! a file may. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

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

/// A track's `Texture` nodes use the short 32-byte header with the name
/// field zeroed, so the node name is `None` and the only name the texture
/// has is the runtime asset path at payload `+0x38`. Reading the header
/// alone is why every track texture used to come back unnamed, and why the
/// blink-light heuristic could never match track geometry.
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

/// PS2's `.vex` scenes declare a zero-length texture block: `Texture`
/// nodes carry real dimensions and non-zero `clut_size`/`texel_size`
/// fields, but no palette or texel bytes follow the tree at all. Reading
/// them as if the bytes were there walks past the end of the file, which
/// is what broke on a real PS2 disc image (`Data\Ships\Feisar\Ship.vex`,
/// see HANDOVER.md). The header's own declared texture length is the
/// signal to trust instead of the node's.
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
