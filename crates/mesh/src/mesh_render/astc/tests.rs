use super::*;

/// An RGBA8 picture ramping red across and green down, opaque.
fn gradient(width: u32, height: u32) -> Vec<u8> {
    let mut texels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            texels.extend_from_slice(&[
                (x * 255 / width.max(2).saturating_sub(1)) as u8,
                (y * 255 / height.max(2).saturating_sub(1)) as u8,
                128,
                255,
            ]);
        }
    }
    texels
}

/// BC1 blocks for a `width` by `height` texture, each block its own pair of
/// endpoints so the picture is not flat.
fn bc1(width: u32, height: u32) -> Vec<u8> {
    let blocks = (width / 4) * (height / 4);
    (0..blocks)
        .flat_map(|block| {
            let c0 = (0xF800u16).wrapping_sub(block as u16 * 0x0841);
            let c1 = (0x001Fu16).wrapping_add(block as u16 * 0x0400);
            let mut out = Vec::with_capacity(8);
            out.extend_from_slice(&c0.to_le_bytes());
            out.extend_from_slice(&c1.to_le_bytes());
            out.extend_from_slice(&0xE4E4_E4E4u32.wrapping_add(block).to_le_bytes());
            out
        })
        .collect()
}

fn inputs(source: &[u8]) -> KeyInputs<'_> {
    KeyInputs {
        format: BlockFormat::Bc1,
        source,
        width: 16,
        height: 16,
        levels: 5,
        block: BLOCK,
        quality: QUALITY,
    }
}

/// A fresh directory under the system's temporary one, unique to this test process.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-astc-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn only_a_device_without_bc_that_samples_astc_takes_the_re_encode() {
    let bc = wgpu::Features::TEXTURE_COMPRESSION_BC;
    let astc = wgpu::Features::TEXTURE_COMPRESSION_ASTC;
    assert!(takes_astc(astc), "a phone: ASTC and no BC");
    assert!(!takes_astc(bc), "a desktop: BC keeps the disc's blocks");
    assert!(
        !takes_astc(bc | astc),
        "both: the disc's blocks win, the setting is inert"
    );
    assert!(
        !takes_astc(wgpu::Features::empty()),
        "neither: nothing to encode to"
    );
}

#[test]
fn the_key_moves_with_every_encoder_setting_and_the_source() {
    let source = bc1(16, 16);
    let base = Key::of(&inputs(&source));
    assert_eq!(
        base,
        Key::of(&inputs(&source)),
        "the same inputs, the same key"
    );
    let mut changed_source = source.clone();
    changed_source[3] ^= 1;
    let variants = [
        Key::of(&KeyInputs {
            block: 6,
            ..inputs(&source)
        }),
        Key::of(&KeyInputs {
            quality: 98.0,
            ..inputs(&source)
        }),
        Key::of(&KeyInputs {
            format: BlockFormat::Bc3,
            ..inputs(&source)
        }),
        Key::of(&KeyInputs {
            width: 8,
            height: 32,
            ..inputs(&source)
        }),
        Key::of(&KeyInputs {
            levels: 4,
            ..inputs(&source)
        }),
        Key::of(&inputs(&changed_source)),
    ];
    for (n, key) in variants.iter().enumerate() {
        assert_ne!(*key, base, "variant {n} kept the key");
    }
    assert_eq!(base.file_name().len(), 64 + ".astc".len());
}

#[test]
fn an_encode_decodes_back_close_to_its_source() {
    let mut encoder = Encoder::new().expect("astcenc configures");
    let source = gradient(16, 16);
    let blocks = encoder.encode(16, 16, &source).expect("encodes");
    assert_eq!(blocks.len(), level_len(16, 16));
    let back = encoder.decode(16, 16, &blocks).expect("decodes");
    let error = error(&source, &back);
    // Measured, not a byte hash: astcenc's output is ISA-dependent
    // (`ASTCENC_NO_INVARIANCE`), its quality is not.
    assert!(
        error.psnr > 40.0,
        "a smooth gradient survives 4x4: {error:?}"
    );

    let flat = vec![90u8; 8 * 8 * 4];
    let blocks = encoder.encode(8, 8, &flat).expect("encodes");
    let back = encoder.decode(8, 8, &blocks).expect("decodes");
    assert_eq!(back, flat, "a constant picture is exact");

    // A level smaller than a block still fills one.
    let small = gradient(2, 2);
    let blocks = encoder.encode(2, 2, &small).expect("encodes");
    assert_eq!(blocks.len(), 16);
}

#[test]
fn an_entry_reads_back_only_at_the_sizes_it_was_written_for() {
    let dims = level_dims(8, 4, 4);
    assert_eq!(dims, [(8, 4), (4, 2), (2, 1), (1, 1)]);
    let levels: Vec<Vec<u8>> = dims
        .iter()
        .map(|&(w, h)| vec![7; level_len(w, h)])
        .collect();
    let entry = write_entry(&levels);
    assert_eq!(parse_entry(&entry, &dims), Some(levels));
    assert_eq!(
        parse_entry(&entry, &dims[..3]),
        None,
        "a level count it was not written with"
    );
    assert_eq!(
        parse_entry(&entry, &level_dims(16, 4, 4)),
        None,
        "a base it was not written for"
    );
    assert_eq!(
        parse_entry(&entry[..entry.len() - 1], &dims),
        None,
        "a torn file"
    );
    let mut trailing = entry.clone();
    trailing.push(0);
    assert_eq!(
        parse_entry(&trailing, &dims),
        None,
        "bytes past the last level"
    );
    assert_eq!(parse_entry(b"not an entry", &dims), None);
}

#[test]
fn a_miss_is_queued_encoded_and_found_by_the_next_load() {
    let dir = scratch("fill");
    let cache = Arc::new(Cache::new(dir.clone()));
    let source = bc1(16, 16);
    let levels = 5;
    let key = Key::of(&inputs(&source));
    let dims = level_dims(16, 16, levels);
    assert!(
        cache.load(&key, &dims).is_none(),
        "a cold cache has nothing"
    );
    let job = || Job {
        key,
        format: BlockFormat::Bc1,
        source: source.clone(),
        width: 16,
        height: 16,
        levels,
    };
    cache.request(job());
    // Queued twice, encoded once: the second request finds the key pending.
    cache.request(job());
    let path = dir.join(key.file_name());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !path.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let entry = cache
        .load(&key, &dims)
        .expect("the encoder wrote the entry");
    assert_eq!(entry.len(), levels);

    // The entry is the decoded blocks' own box-filtered chain, encoded.
    let rgba = BlockFormat::Bc1
        .decode_level(&source, 16, 16)
        .expect("BC1 decodes")
        .into_flattened();
    let mut encoder = Encoder::new().expect("astcenc configures");
    let back = encoder.decode(16, 16, &entry[0]).expect("decodes");
    let base = error(&rgba, &back);
    assert!(
        base.psnr > 30.0,
        "the base level is the source's picture: {base:?}"
    );
    let chain = crate::mesh_render::texture::mip_chain(16, 16, &rgba);
    let back = encoder.decode(2, 2, &entry[3]).expect("decodes");
    let low = error(&chain[3].2, &back);
    assert!(low.psnr > 30.0, "a low level is the box filter's: {low:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_cache_is_off_until_configured_and_off_again_with_none() {
    let dir = scratch("configure");
    configure(Some(dir.clone()));
    let first = ACTIVE.lock().unwrap().clone().expect("on");
    configure(Some(dir.clone()));
    let second = ACTIVE.lock().unwrap().clone().expect("still on");
    assert!(
        Arc::ptr_eq(&first, &second),
        "the same directory keeps its cache"
    );
    configure(None);
    assert!(ACTIVE.lock().unwrap().is_none(), "original closes it");
}
