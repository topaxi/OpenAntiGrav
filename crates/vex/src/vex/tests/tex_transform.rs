//! The texture-transform keyframe block: how it parses, how it samples, and
//! when a material has no transform at all.

/// A minimal mesh payload carrying the boost plume's keyframe block as it sits on
/// disc (`docs/formats/vex.md`, "The texture-transform keyframe block"), covering
/// the parser and sampler without a disc; `boost_plume_ground_truth.rs` pins the
/// values against every team's real file.
#[test]
fn tex_transform_block_parses_and_samples_like_the_engine() {
    let mut payload = vec![0u8; 0x30];
    payload[2] = 1; // material_count
    payload.extend([0u8; 0x14]); // one material
    payload[0x30] = 0x10; // ...carrying the engine's animate-me flag
    let block = payload.len(); // 0x44
    payload.extend([0u8; 0x30]); // the block itself
    let w = |p: &mut Vec<u8>, at: usize, v: u32| p[at..at + 4].copy_from_slice(&v.to_le_bytes());
    payload[block..block + 2].copy_from_slice(&2u16.to_le_bytes()); // offset keys
    payload[block + 2..block + 4].copy_from_slice(&1u16.to_le_bytes()); // scale keys
    // key data appended after the block, offsets relative to the block
    let scale_times = payload.len() - block;
    payload.extend(90u16.to_le_bytes());
    let scale_values = payload.len() - block;
    payload.extend(256u16.to_le_bytes());
    payload.extend(256u16.to_le_bytes());
    let offset_times = payload.len() - block;
    payload.extend(1u16.to_le_bytes());
    payload.extend(90u16.to_le_bytes());
    let offset_values = payload.len() - block;
    for v in [2i16, 0, 253, 0] {
        payload.extend(v.to_le_bytes());
    }
    w(&mut payload, block + 0x04, offset_times as u32);
    w(&mut payload, block + 0x08, scale_times as u32);
    w(&mut payload, block + 0x10, offset_values as u32);
    w(&mut payload, block + 0x14, scale_values as u32);

    let transform = crate::vex::mesh_tex_transform(&payload).expect("block parses");
    assert_eq!(transform.offset.times, vec![1, 90]);
    assert_eq!(transform.offset.values, vec![(2, 0), (253, 0)]);
    assert_eq!(transform.scale.sample(45.0), (1.0, 1.0));
    // Clamp below the first key and past the last, lerp between - the
    // engine evaluator's own behaviour (`TexAnim_EvalKeyframes`).
    assert_eq!(transform.offset.sample(0.0), (2.0 / 256.0, 0.0));
    assert_eq!(transform.offset.sample(90.0), (253.0 / 256.0, 0.0));
    assert_eq!(transform.offset.sample(500.0), (253.0 / 256.0, 0.0));
    let (u, v) = transform.offset.sample(1.0 + 89.0 / 2.0);
    assert!((u - (2.0 + 251.0 / 2.0) / 256.0).abs() < 1e-6);
    assert_eq!(v, 0.0);
    assert_eq!(transform.offset.period(), 90.0);
}

/// Two materials, two blocks, and the second one's key offsets resolving
/// off the **array base** rather than off its own block.
///
/// Shaped after `16_Track`'s hologram panels (two materials scrolling V one tile
/// per 60 and per 120 frames). Read off the wrong base, material 1 decodes to
/// noise (key times in the tens of thousands) rather than failing, hence pinned
/// here, not left to the ground-truth test.
#[test]
fn a_second_materials_block_resolves_its_keys_off_the_array_base() {
    let mut payload = vec![0u8; 0x30];
    payload[2] = 2; // material_count
    payload.extend([0u8; 0x28]); // two materials
    payload[0x30] = 0x10; // ...both carrying the animate-me flag
    payload[0x44] = 0x10;
    let base = payload.len();
    payload.extend([0u8; 0x80]); // two blocks
    let w = |p: &mut Vec<u8>, at: usize, v: u32| p[at..at + 4].copy_from_slice(&v.to_le_bytes());

    // One shared pool of key data after both blocks. Each block's own
    // times/values are two keys of V scroll over `frames`.
    let author = |p: &mut Vec<u8>, block: usize, frames: u16, loop_word: u32| {
        p[block..block + 2].copy_from_slice(&2u16.to_le_bytes()); // offset keys
        p[block + 2..block + 4].copy_from_slice(&1u16.to_le_bytes()); // scale keys
        let scale_times = p.len() - base;
        p.extend(frames.to_le_bytes());
        let scale_values = p.len() - base;
        p.extend(256u16.to_le_bytes());
        p.extend(256u16.to_le_bytes());
        let offset_times = p.len() - base;
        p.extend(0u16.to_le_bytes());
        p.extend(frames.to_le_bytes());
        let offset_values = p.len() - base;
        for v in [0i16, 0, 0, 256] {
            p.extend(v.to_le_bytes());
        }
        w(p, block + 0x04, offset_times as u32);
        w(p, block + 0x08, scale_times as u32);
        w(p, block + 0x0c, (1.0f32 / 60.0).to_bits());
        w(p, block + 0x10, offset_values as u32);
        w(p, block + 0x14, scale_values as u32);
        w(p, block + 0x2c, loop_word);
    };
    author(&mut payload, base, 60, 1.0f32.to_bits());
    author(&mut payload, base + 0x40, 120, 2.0f32.to_bits());

    let blocks = crate::vex::mesh_tex_transforms(&payload);
    assert_eq!(blocks.len(), 2);
    let first = blocks[0].as_ref().expect("material 0 authors a track");
    let second = blocks[1].as_ref().expect("material 1 authors a track");
    assert_eq!(first.offset.times, vec![0, 60]);
    assert_eq!(second.offset.times, vec![0, 120]);
    assert_eq!(second.offset.values, vec![(0, 0), (0, 256)]);
    assert_eq!(first.loop_seconds, 1.0);
    assert_eq!(second.loop_seconds, 2.0);
    // `mesh_tex_transform` is material 0's block, unchanged.
    assert_eq!(
        crate::vex::mesh_tex_transform(&payload).as_ref(),
        Some(first)
    );
}

/// The loop period is the block's own `+0x2c`, not the last key time, and
/// bit 0 of that word snaps the sample to the preceding key.
///
/// Shaped after `16_Track`'s flicker sequences: key pairs one frame apart, ending
/// at frame 12, over an authored 50-frame loop. Reading the period as `period()`
/// would run them at four times speed and destroy the phase interleave with their
/// siblings.
#[test]
fn the_loop_period_and_step_flag_come_from_the_block_not_the_keys() {
    let mut payload = vec![0u8; 0x30];
    payload[2] = 1;
    payload.extend([0u8; 0x14]);
    payload[0x30] = 0x10;
    let block = payload.len();
    payload.extend([0u8; 0x40]);
    let w = |p: &mut Vec<u8>, at: usize, v: u32| p[at..at + 4].copy_from_slice(&v.to_le_bytes());
    payload[block..block + 2].copy_from_slice(&4u16.to_le_bytes());
    let offset_times = payload.len() - block;
    for t in [3u16, 4, 7, 8] {
        payload.extend(t.to_le_bytes());
    }
    let offset_values = payload.len() - block;
    for v in [0i16, 0, 0, -64, 0, -64, 0, -128] {
        payload.extend(v.to_le_bytes());
    }
    w(&mut payload, block + 0x04, offset_times as u32);
    w(&mut payload, block + 0x10, offset_values as u32);
    w(&mut payload, block + 0x0c, (1.0f32 / 60.0).to_bits());
    // 0.8333 s with bit 0 set: the float's low mantissa bit *is* the flag.
    w(&mut payload, block + 0x2c, (50.0f32 / 60.0).to_bits() | 1);

    let t = crate::vex::mesh_tex_transform(&payload).expect("block parses");
    assert!(t.step);
    assert!((t.loop_seconds - 50.0 / 60.0).abs() < 1e-6);
    assert_eq!(t.offset.period(), 8.0, "the keys end long before the loop");

    // Stepped: frame 5 sits between keys 4 and 7 and holds key 4's value
    // rather than sliding towards key 7's.
    let (_, offset) = t.sample(5.0 / 60.0);
    assert_eq!(offset, [0.0, -64.0 / 256.0]);
    // Past the last key it clamps, all the way to the wrap.
    let (_, late) = t.sample(45.0 / 60.0);
    assert_eq!(late, [0.0, -128.0 / 256.0]);
    // And the wrap is the block's period, not the last key time: one loop
    // on lands back at the start, where 8 frames on would not.
    let (_, wrapped) = t.sample(50.0 / 60.0);
    assert_eq!(wrapped, t.sample(0.0).1);
    // An absent scale track evaluates to the engine's identity default.
    assert_eq!(t.sample(5.0 / 60.0).0, [1.0, 1.0]);
}

/// A material without the `& 0x10` flag has no transform, whatever the
/// bytes where its block would sit happen to say.
///
/// The engine's own gate (`Mesh_UpdateTextureTransforms` evaluates only flagged
/// materials), which keeps a non-animated payload from picking up an animation:
/// arbitrary bytes read as a plausible block often enough that "the counts are
/// non-zero" is not a safe predicate alone.
#[test]
fn a_material_without_the_flag_has_no_transform_however_the_bytes_read() {
    let mut payload = vec![0u8; 0x30];
    payload[2] = 1;
    payload.extend([0u8; 0x14]);
    let block = payload.len();
    payload.extend([0u8; 0x40]);
    // A block that would parse: two offset keys pointing at real data.
    payload[block..block + 2].copy_from_slice(&2u16.to_le_bytes());
    let times = payload.len() - block;
    payload.extend(0u16.to_le_bytes());
    payload.extend(60u16.to_le_bytes());
    let values = payload.len() - block;
    for v in [0i16, 0, 0, 256] {
        payload.extend(v.to_le_bytes());
    }
    let w = |p: &mut Vec<u8>, at: usize, v: u32| p[at..at + 4].copy_from_slice(&v.to_le_bytes());
    w(&mut payload, block + 0x04, times as u32);
    w(&mut payload, block + 0x10, values as u32);

    assert_eq!(
        crate::vex::mesh_tex_transform(&payload),
        None,
        "flag is clear"
    );
    payload[0x30] = 0x10;
    assert!(
        crate::vex::mesh_tex_transform(&payload).is_some(),
        "the same bytes with the flag set do parse - so the flag is what \
             decided it, not a malformed block"
    );
}

/// An empty block - both counts zero - is the engine's identity default,
/// not a parse failure, and a payload too short for a block is `None` too.
#[test]
fn tex_transform_block_absent_or_empty_is_none() {
    let mut payload = vec![0u8; 0x30];
    payload[2] = 1;
    payload.extend([0u8; 0x14]);
    payload.extend([0u8; 0x30]); // block present, counts 0/0
    assert_eq!(crate::vex::mesh_tex_transform(&payload), None);
    assert_eq!(crate::vex::mesh_tex_transform(&payload[..0x50]), None);
    assert_eq!(crate::vex::mesh_tex_transform(&[0u8; 0x10]), None);
}
