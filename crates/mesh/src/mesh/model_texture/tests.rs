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
    for block in bytes[data_offset..].as_chunks_mut::<16>().0 {
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
fn the_form_a_gnf_took_is_counted_so_a_loader_can_say_how_many_fell_back() {
    let (_, chain) = ModelTexture::from_gnf_form("chain", &gnf(2)).expect("decodes");
    let (_, flat) = ModelTexture::from_gnf_form("flat", &gnf(1)).expect("decodes");
    assert_eq!((chain, flat), (GnfForm::Blocks, GnfForm::SingleLevel));
    let mut counts = GnfCounts::default();
    assert_eq!(counts.describe(), "", "a build with no .gnf says nothing");
    for form in [chain, flat, flat, GnfForm::OffGrid, GnfForm::BlocksRefused] {
        counts.record(form);
    }
    assert_eq!(counts.fell_back(), 4);
    assert_eq!(
        counts.describe(),
        "; 1 .gnf texture(s) kept as BC7 blocks, 4 decoded to RGBA8 with a synthesised chain \
         (2 single-level, 1 off the block grid, 1 refused as blocks)"
    );
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

/// An uploaded texture holds no texels to release, and `release_texels` must
/// keep its view: the uploader hands that view to every drawable that binds it.
#[test]
fn releasing_texels_keeps_an_uploaded_texture_s_view() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        return;
    };
    let Ok((device, queue)) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
    else {
        return;
    };
    let _scope = crate::mesh_render::TextureSinkScope::open(&device, &queue);
    let texture = crate::mesh_render::offer_to_texture_sink(ModelTexture::rgba8(
        "uploaded".into(),
        2,
        2,
        vec![9; 16],
        None,
    ));
    assert!(matches!(texture.released().texels, Texels::Uploaded { .. }));
}
