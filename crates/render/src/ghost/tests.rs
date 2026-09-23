use super::*;

/// The law off `ghost.md`, at the points that pin its shape: nothing inside
/// 5 units, a linear ramp to half the far weight at 15, and the jump there.
#[test]
fn the_fade_is_invisible_close_ramps_to_half_and_jumps_at_fifteen() {
    assert_eq!(
        fade(0.0),
        Fade {
            weight: 0.0,
            glow: 0
        }
    );
    assert_eq!(
        fade(5.0),
        Fade {
            weight: 0.0,
            glow: 0
        }
    );
    let ten = fade(10.0);
    assert!((ten.weight - 5.0 * 0.55 / 20.0).abs() < 1e-6);
    assert_eq!(ten.glow, 63);
    let just_under = fade(14.999);
    assert!(just_under.weight < 0.276 && just_under.weight > 0.274);
    assert_eq!(just_under.glow, 127);
    assert_eq!(
        fade(15.0),
        Fade {
            weight: 0.55,
            glow: 255
        }
    );
    assert_eq!(
        fade(400.0),
        Fade {
            weight: 0.55,
            glow: 255
        }
    );
}

#[test]
fn the_static_offsets_are_hundredths_below_one() {
    let mut rng = Rng::new(3);
    for _ in 0..1_000 {
        for offset in static_offsets(&mut rng) {
            assert!((0.0..=0.99).contains(&offset));
            assert!((offset * 100.0 - (offset * 100.0).round()).abs() < 1e-4);
        }
    }
}

/// The three pipelines build against a race-shaped target - colour, velocity,
/// depth, 4x MSAA - on whatever adapter this machine has, and none of them
/// is rejected by validation. Skipped without an adapter.
#[test]
fn the_three_passes_build_against_a_race_target() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        return;
    };
    let Ok((device, queue)) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
    else {
        return;
    };
    let static_glow = crate::exhaust::FlareTexture {
        width: 2,
        height: 2,
        rgba: vec![255; 16],
    };
    for (format, stamps) in [
        (wgpu::TextureFormat::Rgba8Unorm, true),
        (wgpu::TextureFormat::Rgba16Float, false),
    ] {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let pipeline = Pipeline::new(&device, &queue, format, 4, stamps, Some(&static_glow));
        let error = pollster::block_on(scope.pop());
        assert!(error.is_none(), "{format:?}: {error:?}");
        assert_eq!(pipeline.stamps_glow(), stamps);
        assert_eq!(pipeline.stamp.is_some(), stamps);
    }
}
