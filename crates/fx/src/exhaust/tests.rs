//! What the engine exhaust in [`super`] is asserted to do: the speed ramp, the
//! intensity envelope, the flicker, the flare quad, and the trail ribbon.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `exhaust.rs`: the tests are 546 lines, past the 200 an inline test module
//! may hold. See `scripts/check-file-size.py`, which is the rule as a gate.

use super::*;

fn rng() -> Rng {
    Rng::new(1)
}

/// The speed term's two endpoints, straight from `Exhaust_Update`.
#[test]
fn the_speed_ramp_is_zero_at_100_kmh_and_one_at_600() {
    let mut e = Exhaust::new();
    let mut r = rng();

    // 100 km/h is the floor: the ramp contributes nothing, so with no thrust
    // the accumulator's floor is zero too.
    e.advance(SUBSTEP_DT, 0.0, RAMP_FLOOR_KMH / SPEED_TO_KMH, &mut r);
    assert_eq!(e.boost_accumulator(), 0.0);

    // 600 km/h saturates it, and the floor is 0.6 of that.
    let mut e = Exhaust::new();
    e.advance(
        SUBSTEP_DT,
        0.0,
        (RAMP_FLOOR_KMH + RAMP_SPAN_KMH) / SPEED_TO_KMH,
        &mut r,
    );
    assert!(
        (e.boost_accumulator() - RAMP_FLOOR_SHARE).abs() < 1e-6,
        "{}",
        e.boost_accumulator()
    );
}

/// Beyond 600 km/h the ramp clamps rather than growing.
#[test]
fn the_speed_ramp_clamps_above_600_kmh() {
    let mut e = Exhaust::new();
    let mut r = rng();
    e.advance(SUBSTEP_DT, 0.0, 10_000.0, &mut r);
    assert!((e.boost_accumulator() - RAMP_FLOOR_SHARE).abs() < 1e-6);
}

/// The three layers cross zero at 0, 0.25 and 0.5, and all reach full at 1.0.
///
/// This is the shape the staggered gains exist to produce, and it is the one
/// property a wrong transcription of `1.33` or `2.0` would break.
#[test]
fn the_three_layers_stagger_and_then_saturate_together() {
    let mut e = Exhaust::new();

    e.intensity = 0.0;
    assert_eq!(e.layer_alphas(), [0.0, 0.0, 0.0]);

    // Just above each start, that layer is live and the later ones are not.
    e.intensity = 0.01;
    let a = e.layer_alphas();
    assert!(a[0] > 0.0 && a[1] == 0.0 && a[2] == 0.0, "{a:?}");

    e.intensity = 0.26;
    let a = e.layer_alphas();
    assert!(a[1] > 0.0 && a[2] == 0.0, "{a:?}");

    e.intensity = 0.51;
    let a = e.layer_alphas();
    assert!(a[2] > 0.0, "{a:?}");

    // At full intensity all three arrive together at the ceiling - but only
    // *nearly*, and the near-miss is the original's own.
    //
    // Layers 0 and 2 land on `LAYER_ALPHA_SCALE` exactly (`1.0 * 1.0 * 0.7`
    // and `0.5 * 2.0 * 0.7`). Layer 1 reaches `0.75 * 1.33 * 0.7 = 0.69825`,
    // 0.25 % short, because the shipped gain is the literal `1.33` and not
    // `4/3`. Asserting exact equality here failed, which is the check earning
    // its keep: had the constant been "tidied" to `1.0 / 0.75` on the way in,
    // this test would have passed and the value would have been wrong.
    e.intensity = 1.0;
    let full = e.layer_alphas();
    assert_eq!(full[0], LAYER_ALPHA_SCALE);
    assert_eq!(full[2], LAYER_ALPHA_SCALE);
    assert!(
        (full[1] - 0.698_25).abs() < 1e-5,
        "middle layer at {}",
        full[1]
    );
    for (n, alpha) in full.iter().enumerate() {
        assert!(
            (*alpha - LAYER_ALPHA_SCALE).abs() < LAYER_ALPHA_SCALE * 0.005,
            "layer {n} at {alpha} is more than 0.5% off the ceiling"
        );
    }
}

/// Intensity rises at 0.25/s and falls at 0.5/s, clamped to `[0, 1]`.
#[test]
fn intensity_rises_at_a_quarter_and_falls_at_a_half_per_second() {
    let mut e = Exhaust::new();
    let mut r = rng();

    // One second of thrust at 60 Hz.
    for _ in 0..60 {
        e.advance(1.0 / 60.0, 100.0, 0.0, &mut r);
    }
    assert!(
        (e.intensity() - INTENSITY_RISE).abs() < 1e-3,
        "{}",
        e.intensity()
    );

    // Then one second off. Falling twice as fast, it overshoots zero and
    // clamps rather than going negative.
    for _ in 0..60 {
        e.advance(1.0 / 60.0, 0.0, 0.0, &mut r);
    }
    assert_eq!(e.intensity(), 0.0);
}

/// An idle engine draws a small glow, not nothing.
///
/// The flare's intensity response is entirely in its size - the colour is
/// white at the flickered alpha regardless - so at intensity 0 the quad
/// runs at `0.4` of its saturated base. The original shows exactly this on
/// the start line before the countdown ends. (The *ribbon* still vanishes
/// cold: the layer ramps are its colours.)
#[test]
fn an_idle_engine_glows_small_rather_than_vanishing() {
    let mut e = Exhaust::new();
    let mut r = rng();
    assert_eq!(e.layer_alphas(), [0.0, 0.0, 0.0]);
    e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
    assert!(e.alpha() >= ALPHA_FLICKER.0 - 1e-6);
    let idle = e.half_size();
    assert!(idle > 0.0);

    // Saturate and compare: the size ratio is the recovered 0.4 base
    // against the full 1.0, within the flicker's spread.
    for _ in 0..600 {
        e.advance(SUBSTEP_DT, 100.0, 0.0, &mut r);
    }
    let hot = e.half_size();
    let ratio = idle / hot;
    let bound = (HALF_SIZE_BASE / (HALF_SIZE_SPAN + HALF_SIZE_BASE))
        * (FLICKER.1 / FLICKER.0).max(FLICKER.0 / FLICKER.1);
    assert!(
        ratio < bound + 1e-3,
        "idle {idle} vs hot {hot}: ratio {ratio} above {bound}"
    );
}

/// `snap` puts a thrusting craft straight at full intensity, and does not
/// depend on how many times it has been called.
#[test]
fn snap_does_not_ramp_and_is_idempotent() {
    let mut e = Exhaust::new();
    e.snap(100.0, 100.0);
    assert_eq!(e.intensity(), 1.0);
    let once = e;
    e.snap(100.0, 100.0);
    assert_eq!(once, e);
}

/// The flicker is seeded, so the same tick sequence gives the same picture.
///
/// This is what keeps `--screenshot` comparable between runs; an unseeded
/// flicker would make every capture differ and none of them wrong.
#[test]
fn the_flicker_is_reproducible_from_the_seed() {
    let run = || {
        let mut e = Exhaust::new();
        let mut r = Rng::new(7);
        for _ in 0..120 {
            e.advance(1.0 / 60.0, 100.0, 90.0, &mut r);
        }
        (e.half_size(), e.alpha())
    };
    assert_eq!(run(), run());
}

/// The flicker stays inside its recovered bounds over a long run.
#[test]
fn the_flicker_stays_within_its_bounds() {
    let mut e = Exhaust::new();
    let mut r = rng();
    for _ in 0..600 {
        e.advance(1.0 / 60.0, 100.0, 120.0, &mut r);
        assert!(
            e.alpha() >= ALPHA_FLICKER.0 - 1e-6 && e.alpha() <= 1.0,
            "{}",
            e.alpha()
        );
    }
    // At full intensity the unflickered half-size is 2.5 times the world
    // conversion, so the flicker bounds it by 0.75 and 1.25 of that.
    let steady = HALF_SIZE_GAIN * HALF_SIZE_TO_WORLD;
    assert!(
        e.half_size() >= steady * FLICKER.0 - 1e-3 && e.half_size() <= steady * FLICKER.1 + 1e-3,
        "{}",
        e.half_size()
    );
}

/// Boost widens the flare, via the timer rather than the accumulator.
#[test]
fn boost_widens_the_flare_while_its_timer_runs() {
    let mut e = Exhaust::new();
    let mut r = rng();
    for _ in 0..600 {
        e.advance(1.0 / 60.0, 100.0, 120.0, &mut r);
    }
    let steady = e.half_size();
    e.boost(1.0);
    e.advance(1.0 / 60.0, 100.0, 120.0, &mut r);
    assert!(e.half_size() > steady, "{} vs {steady}", e.half_size());
}

/// The flare is one quad, emitted every frame, so the count never changes.
///
/// It used to assert `cold == MAX_VERTICES`, which was true only while the
/// flare was the buffer's *sole* occupant. [`sprite`] gave it company, so
/// the two claims are separated here: one quad is still six vertices, and
/// six still fit.
#[test]
fn the_vertex_count_is_constant() {
    let mut e = Exhaust::new();
    let mut r = rng();
    let cold = e.vertices(Vec3::ZERO, Vec3::X, Vec3::Y).len();
    for _ in 0..300 {
        e.advance(1.0 / 60.0, 100.0, 200.0, &mut r);
    }
    assert_eq!(cold, 6, "the flare is one quad");
    assert!(cold <= MAX_VERTICES);
    assert_eq!(e.vertices(Vec3::ZERO, Vec3::X, Vec3::Y).len(), cold);
}

/// A caller's sprite is the same one quad, and the budget has room for the
/// flare plus a full sky of them - which is the invariant
/// `oag_raceplay`'s own budget test depends on.
#[test]
fn a_caller_sprite_is_one_quad_and_the_budget_has_room_for_the_flare_too() {
    let quad = sprite(Vec3::ZERO, Vec3::X, Vec3::Y, 1.0, 1.0);
    assert_eq!(quad.len(), 6);
    assert_eq!(MAX_VERTICES, MAX_SPRITES * 6);
}

/// The flare quad is square, because the original's is.
///
/// `ExhaustFlare_Draw` takes both extents from one register and the live
/// projection is aspect-correct, so a stretch here would be an invention -
/// see [`Exhaust::vertices`] for the three readings that retired the
/// `480 / 272` factor this test used to assert.
#[test]
fn the_flare_quad_is_square() {
    let mut e = Exhaust::new();
    let mut r = rng();
    for _ in 0..600 {
        e.advance(1.0 / 60.0, 100.0, 200.0, &mut r);
    }
    let v = e.vertices(Vec3::ZERO, Vec3::X, Vec3::Y);
    let width = v.iter().map(|v| v.position[0]).fold(f32::MIN, f32::max)
        - v.iter().map(|v| v.position[0]).fold(f32::MAX, f32::min);
    let height = v.iter().map(|v| v.position[1]).fold(f32::MIN, f32::max)
        - v.iter().map(|v| v.position[1]).fold(f32::MAX, f32::min);
    assert!(
        (width / height - 1.0).abs() < 1e-5,
        "aspect {} is not square",
        width / height
    );
}

/// The quad is built from the camera basis, so it faces the viewer.
#[test]
fn the_quad_follows_the_camera_basis() {
    let mut e = Exhaust::new();
    let mut r = rng();
    e.advance(1.0 / 60.0, 100.0, 200.0, &mut r);

    // Spanned by x and y: every vertex has z equal to the centre's.
    let v = e.vertices(Vec3::new(0.0, 0.0, 5.0), Vec3::X, Vec3::Y);
    assert!(v.iter().all(|v| (v.position[2] - 5.0).abs() < 1e-6));

    // Rotate the basis a quarter turn and the quad lies in x/z instead.
    let v = e.vertices(Vec3::new(0.0, 7.0, 0.0), Vec3::X, Vec3::Z);
    assert!(v.iter().all(|v| (v.position[1] - 7.0).abs() < 1e-6));
}

/// The ribbon draws nothing until the ring is full, which is the original's
/// own gate - `Trail_DrawRibbon` returns early on a partial ring.
#[test]
fn the_ribbon_waits_for_a_full_ring() {
    let mut e = Exhaust::new();
    for k in 0..TRAIL_SAMPLES - 1 {
        e.push_trail(Vec3::new(0.0, 0.0, -(k as f32)), -Vec3::Z);
        assert!(!e.trail_ready(), "ready after only {} samples", k + 1);
        assert!(e.trail_vertices(Vec3::X, Vec3::Y).is_empty());
    }
    e.push_trail(Vec3::new(0.0, 0.0, -9.0), -Vec3::Z);
    assert!(e.trail_ready());
    assert_eq!(
        e.trail_vertices(Vec3::X, Vec3::Y).len(),
        TRAIL_VERTICES_PER_CRAFT,
        "a full ring must emit every layer's every segment"
    );
    // And the shared buffer holds a full ring for every craft on the grid,
    // which is what makes the eighth craft's ribbon reach the screen rather
    // than being clipped off by `Pipeline::upload`'s `min`. HD's tube is the
    // larger of the two shapes today (954 against 648), so the budget is its.
    const { assert!(MAX_TRAIL_VERTICES >= MAX_TRAILS * TRAIL_VERTICES_PER_CRAFT) };
    assert_eq!(MAX_TRAIL_VERTICES, MAX_TRAILS * hd::VERTICES_PER_CRAFT);
}

/// A respawn must not leave a ribbon stretched across the track.
#[test]
fn clearing_the_trail_stops_it_drawing() {
    let mut e = Exhaust::new();
    for k in 0..TRAIL_SAMPLES {
        e.push_trail(Vec3::new(0.0, 0.0, -(k as f32)), -Vec3::Z);
    }
    assert!(e.trail_ready());
    e.clear_trail();
    assert!(!e.trail_ready());
    assert!(e.trail_vertices(Vec3::X, Vec3::Y).is_empty());
}

/// The ribbon tapers in width from head to tail, and its colour fades on
/// the recovered `powf(1 - 0.1 i, 1/0.3)` curve.
///
/// Both halves matter, and both were misread once: the taper (`1 - 0.075 i`,
/// 0.325 at sample 9) was missed entirely at first, and the colour fade was
/// declared absent while it sat baked in the ring's vertex colours.
#[test]
fn the_ribbon_tapers_in_width_and_fades_in_colour() {
    let mut e = Exhaust::new();
    let mut r = rng();
    for _ in 0..600 {
        e.advance(1.0 / 60.0, 100.0, 200.0, &mut r);
    }
    // A straight run down -z, so widths read directly off the vertex spans.
    for k in 0..TRAIL_SAMPLES {
        e.push_trail(Vec3::new(0.0, 0.0, -(k as f32) * 3.0), -Vec3::Z);
    }
    let v = e.trail_vertices(Vec3::X, Vec3::Y);

    let per_segment = TRAIL_FINS * 6;
    let per_layer = (TRAIL_SAMPLES - 1) * per_segment;

    // Fin 0's first two vertices sit at head+up*w and head+right*w, so the
    // rim radius reads off either component. Head against last segment's
    // tail end (its `b`-side vertices, offset 2 within the fin).
    let radius = |vertex: usize| {
        let p = v[vertex].position;
        (p[0].powi(2) + p[1].powi(2)).sqrt()
    };
    let head = radius(0);
    let tail = radius(per_layer - per_segment + 2);
    assert!(head > tail, "head {head} must be wider than tail {tail}");

    // The recovered taper ratio at the extremes.
    let ratio = trail_taper(TRAIL_SAMPLES - 1) / trail_taper(0);
    assert!((ratio - 0.325).abs() < 1e-3, "taper ratio {ratio}");

    // The head width is the layer width times the runtime scale - the
    // intensity-driven `0.2..0.55`, saturated here - never the authored
    // width alone.
    let scale = 1.0 * TRAIL_WIDTH_GAIN + TRAIL_WIDTH_BASE;
    assert!(
        (head - LAYER_WIDTH[0] * scale).abs() < 1e-4,
        "head {head} vs {}",
        LAYER_WIDTH[0] * scale
    );

    // Colour fades along the ribbon on the baked curve.
    let head_lum = v[0].colour[2];
    let tail_lum = v[per_layer - per_segment + 2].colour[2];
    assert!(
        tail_lum < head_lum * 0.01,
        "tail colour {tail_lum} must be under 1% of head {head_lum}"
    );

    // **Alpha is the bloom's glow mask, not an opacity, and it is not 1.0.**
    // This assertion used to read `colour[3] == 1.0` on the reasoning that
    // the additive blend ignores alpha - true of the *colour* result, and
    // the reason the value looked free to be anything. It is not free: the
    // ribbon stamps `intensity * TRAIL_GLOW_GAIN` into the target's alpha
    // for `oag_post::bloom`'s bright pass to weigh, reproducing
    // `Trail_BuildStateList`'s stencil `REPLACE`. See [`TRAIL_GLOW_GAIN`].
    assert!(
        (v[0].colour[3] - trail_glow(1.0, 0)).abs() < 1e-6,
        "the head carries the full glow value, not an opacity"
    );
    assert!(
        v[per_layer - 1].colour[3] < v[0].colour[3] * 0.2,
        "the glow ramps down along the ribbon: tail {} vs head {}",
        v[per_layer - 1].colour[3],
        v[0].colour[3]
    );

    // Layer 0 is the authored deep blue times the ramp: red stays zero.
    assert_eq!(v[0].colour[0], 0.0);
    assert!(v[0].colour[2] > v[0].colour[1], "blue must dominate green");
}

/// The taper is monotonic and never negative.
#[test]
fn the_taper_falls_monotonically_and_stays_positive() {
    let mut last = f32::INFINITY;
    for i in 0..TRAIL_SAMPLES {
        let w = trail_taper(i);
        assert!(w < last, "not falling at {i}");
        assert!(w > 0.0, "taper went non-positive at {i}");
        last = w;
    }
    assert!((trail_taper(0) - 1.0).abs() < 1e-6);
}

/// The baked fade matches the live-captured vertex bytes.
///
/// The white layer's baked colours at samples 0/2/4/6/8 read
/// `0xd2 0x63 0x26 0x09 0x00` in the emulator - `210, 99, 38, 9, 0` - and
/// the bake is `authored * fade * 255` truncated. Reproducing those bytes
/// pins both the curve and its exponent against ground truth.
#[test]
fn the_fade_reproduces_the_live_captured_vertex_bytes() {
    let authored = LAYER_COLOUR[2][0];
    let baked: Vec<u32> = (0..TRAIL_SAMPLES)
        .step_by(2)
        .map(|i| (authored * trail_fade(i) * 255.0) as u32)
        .collect();
    assert_eq!(baked, vec![210, 99, 38, 9, 0]);
}

/// The backwards stretch is zero at the head and about 3.5 units at the
/// tail - the recovered `w(t) * 200000 * 0.75`, no free scale.
#[test]
fn the_trail_stretch_is_recovered_end_to_end() {
    assert_eq!(trail_stretch(0), 0.0);
    let mut last = -1.0;
    for i in 0..TRAIL_SAMPLES {
        let w = trail_stretch(i);
        assert!(w > last, "not monotonic at {i}");
        last = w;
    }
    let tail = trail_stretch(TRAIL_SAMPLES - 1);
    assert!((tail - 3.51).abs() < 0.02, "tail stretch {tail}");
}

/// The flare is emissive, so it must not take the mesh shader's light rig.
#[test]
fn the_flare_is_never_lit() {
    let e = Exhaust::new();
    let v = e.vertices(Vec3::ZERO, Vec3::X, Vec3::Y);
    assert!(v.iter().all(|v| v.lit == 0.0));
}

/// Ticks after the arming tick until the plume hides, with no further
/// boosts crossed in between.
///
/// Derived rather than hardcoded from [`PLUME_SECONDS`]`/`[`SUBSTEP_DT`]:
/// `plume_timer` is an `f32` accumulator, and repeated addition of
/// [`SUBSTEP_DT`] does not land on exactly `90 * SUBSTEP_DT` after 90
/// additions, so a test asserting an exact tick index against the rounded
/// division was off by one. The tests below only ever compare two runs
/// driven by this same accumulation, never a hardcoded index, so they do
/// not depend on where the boundary actually falls.
fn plume_hide_tick() -> usize {
    let mut e = Exhaust::new();
    let mut r = rng();
    e.boost(BOOST_SECONDS);
    e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
    assert!(e.plume_visible(), "did not reveal on the arming tick");
    for tick in 1..200 {
        e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
        if !e.plume_visible() {
            return tick;
        }
    }
    panic!("plume never hid within 200 ticks");
}

/// The plume is latched: crossing a second pad while it is up neither
/// restarts nor extends its own 1.5 s countdown.
#[test]
fn a_second_boost_mid_plume_does_not_extend_it() {
    let baseline = plume_hide_tick();

    let mut e = Exhaust::new();
    let mut r = rng();
    e.boost(BOOST_SECONDS);
    e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
    assert!(e.plume_visible());

    for tick in 1..=baseline {
        if tick == baseline / 2 {
            // Still visible: re-arming the boost timer here must not
            // reset the plume's own countdown.
            e.boost(BOOST_SECONDS);
        }
        e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
        let should_be_hidden = tick == baseline;
        assert_eq!(
            e.plume_visible(),
            !should_be_hidden,
            "wrong visibility at tick {tick}, expected relative to \
             baseline hide tick {baseline}"
        );
    }
}

/// A pad crossed late enough that `boost_timer` is still above
/// `BOOST_GATE` when the plume's own `1.5 s` expires produces a deferred
/// back-to-back second plume: hide this tick, reveal the next. A
/// one-shot latch that swallowed the second boost would be wrong - see
/// this module's `BOOST_SECONDS` doc comment.
#[test]
fn a_late_pad_re_reveals_the_plume_the_tick_after_it_hides() {
    let baseline = plume_hide_tick();

    let mut e = Exhaust::new();
    let mut r = rng();
    e.boost(BOOST_SECONDS);
    e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
    assert!(e.plume_visible());

    for _ in 1..baseline {
        e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
    }
    assert!(
        e.plume_visible(),
        "still up one tick before its own deadline"
    );

    // Cross a second pad one tick before the deadline, freshly arming
    // `boost_timer` so it is still comfortably above `BOOST_GATE` once
    // the plume's own deadline hits.
    e.boost(BOOST_SECONDS);

    // Crosses the deadline: hides, without re-revealing on the same tick
    // even though `boost_timer` is high again.
    e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
    assert!(!e.plume_visible(), "did not hide at its own deadline");

    // The very next tick: the latch is clear again and `boost_timer` is
    // still above the gate, so it reveals again.
    e.advance(SUBSTEP_DT, 0.0, 0.0, &mut r);
    assert!(
        e.plume_visible(),
        "did not re-reveal the tick after hiding, though boost_timer \
         was still armed"
    );
}

#[test]
fn hd_trail_facing_term_is_two_sided() {
    // `hd_enginetrail_bluered.rcsmaterial`'s fragment program takes
    // `MIN |dot(n, v)|` (NV40 SRC0_ABS on word 1 bit 29 at @0x0b), so a fin
    // seen from behind fades in exactly as one seen from the front.
    let wgsl = include_str!(concat!(env!("OUT_DIR"), "/exhaust.wgsl"));
    assert!(wgsl.contains("abs(facing_dot)"), "facing term lost its abs");
}
