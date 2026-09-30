use super::*;

/// A 16x16 BC7 `Thin_1DThin` `.gnf` whose blocks all carry a valid mode, with
/// `levels` mip levels (1 or 2; each level here is one 8x8-block tile).
fn gnf(levels: u32) -> Vec<u8> {
    let contents_len = 8 + 36;
    let data_offset = 8 + contents_len;
    let mut bytes = vec![0u8; data_offset + levels as usize * 1024];
    bytes[0..4].copy_from_slice(b"GNF ");
    bytes[4..8].copy_from_slice(&(contents_len as u32).to_le_bytes());
    bytes[8] = 2;
    bytes[9] = 1;
    bytes[10] = 8;
    let word1 = 0x29u32 << 20;
    let word2 = 15u32 | (15u32 << 14);
    let word3 = (0x0du32 << 20) | ((levels - 1) << 16);
    let word4 = 15u32 << 13;
    for (offset, word) in [(4, word1), (8, word2), (12, word3), (16, word4)] {
        bytes[16 + offset..16 + offset + 4].copy_from_slice(&word.to_le_bytes());
    }
    for block in bytes[data_offset..].chunks_exact_mut(16) {
        block[0] = 1;
    }
    bytes
}

#[test]
fn a_gnf_chain_keeps_its_blocks_and_its_levels() {
    let texture = ModelTexture::from_gnf("chain", &gnf(2)).expect("decodes");
    let Texels::Blocks { format, levels } = &texture.texels else {
        panic!("a two-level BC7 chain must stay in blocks");
    };
    assert_eq!(*format, BlockFormat::Bc7);
    assert_eq!(levels.iter().map(Vec::len).collect::<Vec<_>>(), [256, 64]);
    assert_eq!(texture.cpu_bytes(), 320);
    assert_eq!(BlockFormat::Bc7.wgpu(), wgpu::TextureFormat::Bc7RgbaUnorm);
}

#[test]
fn a_single_level_gnf_decodes_to_rgba_so_the_renderer_can_mip_it() {
    let texture = ModelTexture::from_gnf("flat", &gnf(1)).expect("decodes");
    assert_eq!(texture.rgba().map(<[u8]>::len), Some(16 * 16 * 4));
}

#[test]
fn a_gnf_the_decoder_refuses_draws_nothing() {
    let mut bytes = gnf(2);
    bytes[8 + 8 + 4 + 3] = 0x20 << 2; // surface format 0x3f: no decoder
    assert!(ModelTexture::from_gnf("bad", &bytes).is_none());
}

#[test]
fn blocks_decode_back_to_the_same_texels_the_rgba_path_draws() {
    let bytes = gnf(2);
    let chain = ModelTexture::from_gnf("chain", &bytes).unwrap();
    let flat = ModelTexture::from_gnf("flat", &gnf(1)).unwrap();
    assert_eq!(chain.to_rgba().unwrap(), flat.to_rgba().unwrap());
}

#[test]
fn releasing_a_model_keeps_its_slots_and_frees_its_texels() {
    let texture = std::sync::Arc::new(ModelTexture::from_gnf("chain", &gnf(2)).unwrap());
    let mut model = crate::mesh::Model::none("m");
    model.textures = vec![Some(texture.clone()), None, Some(texture.clone())];
    model.lightmaps = vec![Some(texture.clone()), None, None];
    model.release_texels();
    assert_eq!(model.textures.len(), 3);
    assert!(model.textures[1].is_none());
    let kept = model.textures[0].as_ref().unwrap();
    assert_eq!(
        (kept.label.as_str(), kept.width, kept.height),
        ("chain", 16, 16)
    );
    assert_eq!(kept.cpu_bytes(), 0);
    assert!(std::sync::Arc::ptr_eq(
        kept,
        model.textures[2].as_ref().unwrap()
    ));
    assert_eq!(model.lightmaps[0].as_ref().unwrap().cpu_bytes(), 0);
    assert_eq!(
        texture.cpu_bytes(),
        320,
        "another owner's copy is untouched"
    );
}
