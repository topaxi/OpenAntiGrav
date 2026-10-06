//! What [`super`]'s emitter scheduling, particle pool and effect playback are
//! asserted to do.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of `psys.rs`:
//! the tests are past the 200 lines an inline test module may hold, and
//! `psys.rs` is a baselined file that may shrink but not grow. See
//! `scripts/check-file-size.py`, which is both rules as a gate.

use super::*;

pub(super) const DT: f32 = 1.0 / TICK_HZ;

pub(super) fn constant(value: f32) -> Channel {
    Channel {
        period: 0.0,
        mode: ChannelMode::Constant,
        lo: 0.0,
        hi: value,
        keys: Vec::new(),
    }
}

/// One emitter that drops a stationary particle a tick, each living ten
/// ticks - enough to tell "still emitting" from "still fading".
pub(super) fn effect(name: &str, looping: bool, duration_ticks: f32) -> std::sync::Arc<Effect> {
    std::sync::Arc::new(Effect {
        name: name.to_string(),
        emitters: vec![EmitterSpec {
            name: name.to_string(),
            duration_ticks,
            looping,
            interval_ticks: (1, 1),
            per_emission: (1, 1),
            lifetime_ticks: (10.0, 0.0),
            speed_per_tick: (0.0, 0.0),
            direction: Direction::Cone { half_angle: 0.0 },
            drag_per_tick: Vec3::ONE,
            gravity_per_tick2: 0.0,
            live_cap: 100,
            size: constant(1.0),
            alpha: constant(255.0),
            colour_divisor: ColourScale::Full.divisor(),
            palette: Box::new([[1.0; 4]; 256]),
            colour_mode: ColourMode::RandomEntry,
            render: Render::Billboard,
            streak: StreakDraw::Procedural,
            blend: Blend::Additive,
            distort_strength: 0.0,
            particle_child: None,
            death_child: None,
            spawn_probability: 1.0,
            velocity_inherit: 0.0,
            spawn: Spawn::Point,
            emission_scale: constant(1.0),
            extent_animation: None,
            playback: playback::Playback::default(),
            sprite: None,
            atlas: Atlas::SINGLE,
            frames: FrameAdvance::Still,
            sheet_rect: None,
            short_run: false,
            world_space: false,
            template: false,
            rotation: None,
        }],
        roots: vec![0],
        field: None,
        view_depth: 0.0,
        skipped_templates: 0,
    })
}

pub(super) fn run(system: &mut System, effect: &Effect, ticks: usize, rng: &mut Rng) {
    for _ in 0..ticks {
        system.advance(effect, DT, Vec3::ZERO, Vec3::Y, rng);
    }
}

/// The rocket flare's case: an authored duration far shorter than the
/// flight it has to cover, which the `LOOPING` flag says to ignore.
#[test]
fn a_looping_emitter_outlives_its_authored_duration() {
    let mut rng = Rng::new(1);
    let brief = effect("brief", false, 10.0);
    let forever = effect("forever", true, 10.0);

    let mut one_shot = System::new();
    one_shot.ignite(&brief, Vec3::ZERO, 1.0);
    run(&mut one_shot, &brief, 200, &mut rng);
    assert!(!one_shot.is_running(), "a 10-tick emitter ran for 200");

    let mut looping = System::new();
    looping.ignite(&forever, Vec3::ZERO, 1.0);
    run(&mut looping, &forever, 200, &mut rng);
    assert!(
        looping.is_running() && looping.alive_count() > 0,
        "a looping emitter stopped at its authored duration"
    );
}

#[test]
fn stopping_leaves_the_live_particles_to_finish() {
    let mut rng = Rng::new(2);
    let forever = effect("forever", true, 10.0);
    let mut system = System::new();
    system.ignite(&forever, Vec3::ZERO, 1.0);
    run(&mut system, &forever, 30, &mut rng);
    assert!(system.alive_count() > 0);

    system.stop();
    // One tick later the emitters are silent but the particles are not.
    run(&mut system, &forever, 1, &mut rng);
    assert!(system.alive_count() > 0, "stop() killed the live particles");
    // A lifetime later there is nothing left and nothing making more.
    run(&mut system, &forever, 12, &mut rng);
    assert_eq!(system.alive_count(), 0);
    assert!(!system.is_running());
}

#[test]
fn attach_keeps_instances_free_for_bursts() {
    let mut stage = Stage::new();
    let forever = effect("forever", true, 10.0);
    let mut attached = Vec::new();
    while let Some(handle) = stage.attach(&forever, Vec3::ZERO, 1.0) {
        attached.push(handle);
    }
    assert_eq!(attached.len(), MAX_INSTANCES - RESERVED_FOR_BURSTS);
    assert!(
        stage.play(&forever, Vec3::ZERO, 1.0).is_some(),
        "a burst had nowhere to play with the reserve in place"
    );
}

/// The failure this guards is invisible rather than loud: a recycled
/// flare would leave a rocket with no glow while another rocket's glow
/// jumped to it.
#[test]
fn a_burst_never_recycles_an_attached_instance() {
    let mut stage = Stage::new();
    let forever = effect("forever", true, 10.0);
    let burst = effect("burst", false, 4.0);
    let attached: Vec<_> = std::iter::from_fn(|| stage.attach(&forever, Vec3::ZERO, 1.0)).collect();

    for _ in 0..MAX_INSTANCES * 2 {
        assert!(stage.play(&burst, Vec3::ZERO, 1.0).is_some());
    }
    for handle in attached {
        assert!(
            stage.is_playing(handle),
            "a burst evicted an attached flare"
        );
    }
}

#[test]
fn a_handle_kept_past_detach_moves_nothing() {
    let mut stage = Stage::new();
    let forever = effect("forever", true, 10.0);
    let mut rng = Rng::new(3);

    let stale = stage.attach(&forever, Vec3::ZERO, 1.0).expect("attach");
    stage.detach(stale);
    assert!(
        !stage.is_playing(stale),
        "a detached handle still owns a slot"
    );

    // Let the detached instance's particles finish so the slot frees,
    // then hand it to someone else.
    for _ in 0..16 {
        stage.advance(DT, &mut rng);
    }
    let fresh = stage.attach(&forever, Vec3::ONE, 1.0).expect("re-attach");
    stage.follow(stale, Vec3::splat(100.0));
    stage.advance(DT, &mut rng);
    assert!(stage.is_playing(fresh));
    let (additive, _) = stage.vertices(Vec3::X, Vec3::Y);
    assert!(
        additive.iter().all(|vertex| vertex.position[0] < 50.0),
        "a stale handle dragged the instance that took its slot"
    );
}

#[test]
fn a_library_keeps_load_order_and_replaces_by_name() {
    let mut library = Library::new();
    assert!(library.is_empty());
    library.insert("a", (*effect("a", false, 1.0)).clone());
    library.insert("b", (*effect("b", false, 1.0)).clone());
    library.insert("a", (*effect("a2", false, 1.0)).clone());
    assert_eq!(library.len(), 2);
    assert_eq!(library.names().collect::<Vec<_>>(), ["a", "b"]);
    assert_eq!(library.get("a").expect("a").name, "a2");
    assert!(library.get("missing").is_none());
}

#[test]
fn the_effect_path_is_the_one_the_original_builds() {
    assert_eq!(
        effect_path("WO_ROCKET_EXPLO"),
        r"Data\Psys\WO_ROCKET_EXPLO.POB"
    );
}

/// Finding R1's guard: a hand-built emitter with a zero interval emits, and
/// **returns**.
///
/// `Effect::parse` clamps the interval to at least 1, so no disc asset can
/// produce this - but `EmitterSpec` is a `pub` struct with `pub` fields, and
/// before the clamp at the use site this spun for ever inside one `step` call
/// and hung the frame. A test that hangs is a worse failure than one that
/// fails, which is why the clamp is at the use site rather than only a doc
/// promise.
#[test]
fn a_zero_interval_emitter_does_not_spin() {
    let mut effect = (*effect("zero", true, 100.0)).clone();
    effect.emitters[0].interval_ticks = (0, 0);

    let mut rng = Rng::new(1);
    let mut system = System::new();
    system.ignite(&effect, Vec3::ZERO, 1.0);
    run(&mut system, &effect, 10, &mut rng);
    assert!(
        system.alive_count() > 0,
        "a zero interval should emit, not go silent"
    );
}

/// The HD capture that drew no absorb burst: a stage kept busy by running
/// one-shot bursts refuses [`Stage::attach`], and a riding burst must still
/// get a slot - by recycling a burst, never an attached instance.
#[test]
fn a_riding_burst_plays_on_a_stage_too_busy_to_attach() {
    let mut stage = Stage::new();
    let burst = effect("burst", false, 4.0);
    let forever = effect("forever", true, 10.0);
    let flare = stage.attach(&forever, Vec3::ZERO, 1.0).expect("attach");
    for _ in 0..MAX_INSTANCES {
        assert!(stage.play(&burst, Vec3::ZERO, 1.0).is_some());
    }
    assert!(
        stage.attach(&forever, Vec3::ZERO, 1.0).is_none(),
        "the fixture is meant to leave the stage too busy to attach"
    );
    let riding = stage
        .play_riding(&burst, Vec3::ZERO, 1.0)
        .expect("a riding burst was refused");
    assert!(stage.is_playing(riding) && stage.is_emitting(riding));
    stage.follow(riding, Vec3::X);
    assert!(stage.is_playing(flare), "the riding burst evicted a flare");
    stage.detach(riding);
    assert!(!stage.is_playing(riding));
}

/// An attached instance emits about the `up` [`Stage::orient`] gave it,
/// not world `+Y` - the Rocket's flare, whose frame's `+Y` is the flight
/// path. A zero-width cone at unit speed puts every particle on that axis.
#[test]
fn an_oriented_instance_emits_along_its_own_up() {
    let mut rng = Rng::new(4);
    let mut spec = (*effect("aimed", true, 10.0)).clone();
    spec.emitters[0].speed_per_tick = (1.0, 0.0);
    let aimed = std::sync::Arc::new(spec);
    let mut stage = Stage::new();
    let handle = stage.attach(&aimed, Vec3::ZERO, 1.0).expect("attach");
    stage.orient(handle, Vec3::X);
    for _ in 0..8 {
        stage.advance(DT, &mut rng);
    }
    let (additive, _) = stage.vertices(Vec3::Z, Vec3::Y);
    assert!(!additive.is_empty(), "nothing was emitted");
    let n = additive.len() as f32;
    let centre = additive
        .iter()
        .fold(Vec3::ZERO, |sum, v| sum + Vec3::from_array(v.position))
        / n;
    assert!(
        centre.x > 1.0,
        "particles did not travel along +X: {centre:?}"
    );
    assert!(
        centre.y.abs() < 0.5,
        "particles drifted along world up: {centre:?}"
    );
}

/// One live streak particle, head at `position` and tail at `origin`.
fn streak_particle(position: Vec3, origin: Vec3) -> Particle {
    Particle {
        position,
        origin,
        life: 1.0,
        max_life: 1.0,
        ..Particle::DEAD
    }
}

/// A streak spec drawn as `draw` off a sprite placed at `rect`.
fn streak_spec(draw: StreakDraw, rect: Option<[f32; 4]>) -> EmitterSpec {
    let mut spec = effect("streak", false, 10.0).emitters[0].clone();
    spec.render = Render::Streak { from_spawn: false };
    spec.streak = draw;
    spec.sheet_rect = rect;
    spec
}

fn streak_vertices(spec: &EmitterSpec, particle: &Particle) -> Vec<GpuVertex> {
    let mut out = Vec::new();
    streak::extend(&mut out, spec, particle, 1.0, [1.0; 4], Vec3::X, Vec3::Y);
    out
}

/// `ParticleSystem_DrawStreak`'s third vertex sits by the particle, not
/// its tail (`0x08916ba0`), so class 6 is a wedge: a head at the particle
/// and one vertex out past the tail.
#[test]
fn a_class_6_streak_is_a_wedge_off_its_own_sprite() {
    let spec = streak_spec(StreakDraw::Wedge, Some([0.0, 0.0, 1.0, 1.0]));
    let out = streak_vertices(&spec, &streak_particle(Vec3::ZERO, Vec3::X * 10.0));
    assert_eq!(out.len(), 6);
    let xs: Vec<f32> = out.iter().map(|v| v.position[0]).collect();
    // The head's `position -+ cap` at x = -1 and 1; the last vertex at the
    // tail plus a cap, x = 11. Nothing sits at the tail itself.
    assert!(xs.iter().all(|x| [-1.0, 1.0, 11.0].contains(x)), "{xs:?}");
    assert_eq!(xs.iter().filter(|&&x| x == 11.0).count(), 1);
    assert!(out.iter().all(|v| v.normal[0] == 1.0), "samples the sheet");
}

/// An emitter's own class 6 (`0x08917c7c`) is a rectangle from end to end: two
/// corners past each end by `aspect * size`, `size` either side.
#[test]
fn a_class_6_pool_streak_is_a_bar_capped_by_its_aspect() {
    let spec = streak_spec(StreakDraw::Bar { aspect: 3.0 }, Some([0.0, 0.0, 1.0, 1.0]));
    let out = streak_vertices(&spec, &streak_particle(Vec3::ZERO, Vec3::X * 10.0));
    assert_eq!(out.len(), 6);
    let mut xs: Vec<f32> = out.iter().map(|v| v.position[0]).collect();
    xs.sort_by(f32::total_cmp);
    xs.dedup();
    assert_eq!(xs, [-3.0, 13.0], "two corners at each end, capped by 3");
    let ys: Vec<f32> = out.iter().map(|v| v.position[1].abs()).collect();
    assert!(ys.iter().all(|y| (y - 1.0).abs() < 1e-6), "{ys:?}");
}

/// A bar that has not moved is `2 size` across and `2 aspect size` tall:
/// `WO_REPULSER`'s `0.05` draws a sliver, not a square.
#[test]
fn a_still_class_6_pool_streak_is_a_horizontal_sliver() {
    let spec = streak_spec(StreakDraw::Bar { aspect: 0.05 }, Some([0.0, 0.0, 1.0, 1.0]));
    let out = streak_vertices(&spec, &streak_particle(Vec3::ZERO, Vec3::ZERO));
    let span = |axis: usize| {
        let values = out.iter().map(|v| v.position[axis]);
        values.clone().fold(f32::MIN, f32::max) - values.fold(f32::MAX, f32::min)
    };
    assert!((span(0) - 2.0).abs() < 1e-5, "{}", span(0));
    assert!((span(1) - 0.1).abs() < 1e-5, "{}", span(1));
}

/// A template's class 6 stays the wedge.
#[test]
fn a_template_class_6_is_the_wedge() {
    assert_eq!(
        StreakDraw::of(Some(6), 0.05).for_template(),
        StreakDraw::Wedge
    );
    assert_eq!(
        StreakDraw::of(Some(6), 0.05),
        StreakDraw::Bar { aspect: 0.05 }
    );
}

/// `ParticleSystem_DrawCappedStreak` is a bar from one cap to the other;
/// it rides one quad whose along-coordinate the shader folds into `v`.
#[test]
fn a_class_7_streak_is_one_bar_with_its_cap_share() {
    let spec = streak_spec(StreakDraw::Capped, Some([0.0, 0.0, 1.0, 1.0]));
    let out = streak_vertices(&spec, &streak_particle(Vec3::ZERO, Vec3::X * 8.0));
    assert_eq!(out.len(), 6);
    for vertex in &out {
        assert_eq!(vertex.normal[0], 2.0);
        // Each cap is `half` of `length + 2 * half`: 1 of 10.
        assert!((vertex.lit - 0.1).abs() < 1e-6);
        let (x, s) = (vertex.position[0], vertex.texcoord[1]);
        assert!((x == -1.0 && s == 0.0) || (x == 9.0 && s == 1.0), "{x} {s}");
    }
}

/// Every source but Pulse on the PSP keeps the procedural rectangle, byte
/// for byte, sprite or not.
#[test]
fn a_procedural_streak_ignores_a_placed_sprite() {
    let particle = streak_particle(Vec3::ZERO, Vec3::X * 4.0);
    let rect = Some([0.0, 0.0, 1.0, 1.0]);
    let sampled = streak_vertices(&streak_spec(StreakDraw::Procedural, rect), &particle);
    let plain = streak_vertices(&streak_spec(StreakDraw::Wedge, None), &particle);
    let bytes = |v: &[GpuVertex]| bytemuck::cast_slice::<_, u8>(v).to_vec();
    assert_eq!(bytes(&sampled), bytes(&plain));
    assert!(sampled.iter().all(|v| v.normal[0] == 0.0));
}

/// `ParticleSystem_UpdateParticles` grows the frame by the rate times the
/// tick, wraps once at `frames - 0.01`, and draws the floor.
#[test]
fn a_rated_frame_walks_the_atlas_and_wraps() {
    let advance = FrameAdvance::Rate(constant(0.5));
    let mut particle = Particle::DEAD;
    let mut drawn = Vec::new();
    for _ in 0..9 {
        advance.step(&mut particle, 0.0, 0.0, 1.0, 4);
        drawn.push(particle.frame);
    }
    // 0.5, 1.0, ... 3.5, then 4.0 - 3.99 = 0.01, then 0.51.
    assert_eq!(drawn, [0, 1, 1, 2, 2, 3, 3, 0, 0]);
}

/// Under flag `0x40` the frame is the particle's age times `frames - 0.02`,
/// whatever it was before.
#[test]
fn an_over_life_frame_follows_the_age() {
    let mut particle = Particle {
        frame_at: 3.0,
        ..Particle::DEAD
    };
    FrameAdvance::OverLife.step(&mut particle, 0.0, 0.5, 1.0, 16);
    assert_eq!(particle.frame, 7);
    assert!((particle.frame_at - 7.99).abs() < 1e-5);
    FrameAdvance::Still.step(&mut particle, 0.5, 0.9, 1.0, 16);
    assert_eq!(particle.frame, 7, "a still frame never moves");
}

/// Live, on a struck craft: a template's first draw carries its size at age
/// zero (`glow` `0.75`, `shazam` `9.36` twice), and the next frame's the size
/// one tick in. The instance's own first update makes the particle and draws
/// it before anything ages it.
#[test]
fn a_templates_first_draw_is_its_size_at_age_zero() {
    let mut effect = (*effect("tpl", false, 1.0)).clone();
    {
        let spec = &mut effect.emitters[0];
        spec.template = true;
        spec.lifetime_ticks = (10.0, 0.0);
        spec.size = Channel {
            period: 0.0,
            mode: ChannelMode::Keyframed,
            lo: 1.0,
            hi: 11.0,
            keys: vec![(0.0, 0.0), (1.0, 1.0)],
        };
    }
    let mut rng = Rng::new(3);
    let mut system = System::new();
    system.ignite(&effect, Vec3::ZERO, 1.0);
    let half = |system: &System| {
        let (additive, _) = system.vertices(&effect, Vec3::X, Vec3::Y);
        let xs = additive.iter().map(|v| v.position[0]);
        (xs.clone().fold(f32::MIN, f32::max) - xs.fold(f32::MAX, f32::min)) / 2.0
    };
    run(&mut system, &effect, 1, &mut rng);
    assert!(
        (half(&system) - 1.0).abs() < 1e-4,
        "first draw {}",
        half(&system)
    );
    run(&mut system, &effect, 1, &mut rng);
    assert!(
        (half(&system) - 2.0).abs() < 1e-4,
        "second draw {}",
        half(&system)
    );
}

/// An emitter's own class 3 particle is turned by the emitter's roll channel
/// and stretched by its aspect: drop the rotation and the quad is the plain
/// square the port drew before `ParticleSystem_DrawRolledQuads`'s law was played.
#[test]
fn an_emitters_particle_is_turned_and_stretched() {
    let quads = |rate: f32, rotated: bool| {
        let mut effect = (*effect("spin", true, 10.0)).clone();
        effect.emitters[0].rotation =
            rotated.then(|| roll::Rotation::emitter(4.0, constant(rate), 0));
        let mut system = System::new();
        system.ignite(&effect, Vec3::ZERO, 1.0);
        run(&mut system, &effect, 4, &mut Rng::new(5));
        system.vertices(&effect, Vec3::X, Vec3::Y).0
    };
    let span = |quads: &[GpuVertex], axis: usize| {
        let values = quads.iter().map(|v| v.position[axis]);
        values.clone().fold(f32::MIN, f32::max) - values.fold(f32::MAX, f32::min)
    };
    let plain = quads(0.0, false);
    assert!(
        (span(&plain, 0) - 2.0).abs() < 1e-4,
        "plain {}",
        span(&plain, 0)
    );
    let stretched = quads(0.0, true);
    assert!(
        (span(&stretched, 0) - 8.0).abs() < 1e-4,
        "stretched {}",
        span(&stretched, 0)
    );
    let turning = quads(0.4, true);
    assert!(
        span(&turning, 1) > 1.2 * span(&stretched, 1),
        "the roll did not turn the quad: {} vs {}",
        span(&turning, 1),
        span(&stretched, 1)
    );
}

/// `Psys_ReleaseHandle` with `now == 0` destroys the instance and every
/// particle with it; a detach leaves them to finish.
#[test]
fn a_kill_takes_the_live_particles_and_a_detach_leaves_them() {
    let forever = effect("forever", true, 10.0);
    let mut rng = Rng::new(3);
    let held = |end: fn(&mut Stage, Playing)| {
        let mut stage = Stage::new();
        let playing = stage.attach(&forever, Vec3::ZERO, 1.0).expect("attach");
        for _ in 0..6 {
            stage.advance(DT, &mut Rng::new(3));
        }
        assert!(stage.alive_count() > 0);
        end(&mut stage, playing);
        assert!(!stage.is_playing(playing));
        stage.alive_count()
    };
    assert!(held(Stage::detach) > 0, "a detach cut its particles");
    assert_eq!(held(Stage::kill), 0, "a kill left particles behind");

    // The slot is free at once, and the dead handle moves nothing.
    let mut stage = Stage::new();
    let stale = stage.attach(&forever, Vec3::ZERO, 1.0).expect("attach");
    stage.advance(DT, &mut rng);
    stage.kill(stale);
    stage.follow(stale, Vec3::splat(100.0));
    stage.advance(DT, &mut rng);
    assert_eq!(stage.alive_count(), 0);
}

/// The emitter of `effect()` plus a long-lived template on it, the shape of
/// `WO_MISSILE_HEAD`: a `glow` for 3600 ticks beside a trail.
fn with_template() -> std::sync::Arc<Effect> {
    let mut effect = (*effect("head", true, 10.0)).clone();
    let mut template = effect.emitters[0].clone();
    template.template = true;
    template.looping = false;
    template.duration_ticks = 1.0;
    template.lifetime_ticks = (3600.0, 0.0);
    effect.roots.push(effect.emitters.len());
    effect.emitters.push(template);
    std::sync::Arc::new(effect)
}

fn templates_alive(stage: &Stage, effect: &Effect) -> Vec<Vec3> {
    let mut found = Vec::new();
    for instance in stage.instances.iter() {
        for p in instance.system.particles.iter() {
            if p.alive() && effect.emitters[usize::from(p.spec)].template {
                found.push(p.position);
            }
        }
    }
    found
}

/// `ParticleSystem_UpdateParticleFields` copies the owning instance's node
/// position into a template every update: the missile's `glow` rides the
/// missile. Measured on the disc's own `WO_MISSILE_HEAD`, a `glow` drawn 34
/// units across used to hang at the muzzle for the whole flight.
#[test]
fn a_template_rides_its_instances_anchor() {
    let effect = with_template();
    let (mut stage, mut rng) = (Stage::new(), Rng::new(1));
    let playing = stage.attach(&effect, Vec3::ZERO, 1.0).expect("attach");
    for tick in 1..=20 {
        let at = Vec3::new(tick as f32 * 10.0, 0.0, 0.0);
        stage.follow(playing, at);
        stage.advance(DT, &mut rng);
        assert_eq!(templates_alive(&stage, &effect), [at], "tick {tick}");
    }
}

/// `Psys_ReleaseHandle` with `now != 0` (`ParticleSystem_StopAndClear`) frees
/// the template list at once and leaves the emitters' own particles to
/// finish; a detach, which an instance that ran out on its own gets, cuts
/// neither.
#[test]
fn a_release_frees_the_templates_and_leaves_the_rest() {
    let effect = with_template();
    let after = |end: fn(&mut Stage, Playing)| {
        let (mut stage, mut rng) = (Stage::new(), Rng::new(1));
        let playing = stage.attach(&effect, Vec3::ZERO, 1.0).expect("attach");
        for _ in 0..6 {
            stage.advance(DT, &mut rng);
        }
        end(&mut stage, playing);
        (templates_alive(&stage, &effect).len(), stage.alive_count())
    };
    let (kept, alive) = after(Stage::detach);
    assert_eq!(kept, 1, "a detach cut the template");
    let (freed, survivors) = after(Stage::release);
    assert_eq!(freed, 0, "a release left the template");
    assert_eq!(survivors, alive - 1, "a release cut more than the template");
}

/// `effect`, with its one emitter's lifetime and size channel replaced.
fn effect_with(lifetime: (f32, f32), size: Channel) -> std::sync::Arc<Effect> {
    let mut built = (*effect("probe", true, 10.0)).clone();
    built.emitters[0].lifetime_ticks = lifetime;
    built.emitters[0].size = size;
    std::sync::Arc::new(built)
}

/// The measured law (`Particle::fresh`): an emitter's particle is drawn at age
/// 0 on the tick it spawns, so a one-tick life is drawn once and a two-tick
/// life twice - never zero times, which is what the Rocket's
/// `WO_ROCKET_SHAZZAM` flash did when the spawn tick aged the particle too.
#[test]
fn an_emitters_particle_is_drawn_from_the_tick_it_spawns() {
    let mut rng = Rng::new(3);
    for (life, expected) in [(1.0, 1), (2.0, 2), (3.0, 3)] {
        let effect = effect_with((life, 0.0), constant(1.0));
        let mut system = System::new();
        system.ignite(&effect, Vec3::ZERO, 1.0);
        for tick in 0..8 {
            system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
            // One spawn a tick: steady from the first tick, since the particle
            // that is spent is removed by the update that finds it so.
            assert_eq!(
                system.alive_count(),
                expected.min(tick + 1),
                "life {life}, tick {tick}"
            );
        }
    }
}

/// The flare's own numbers: its first draw is the size channel at age 0
/// (`1.05` from `lo 1 + (hi 8 - lo) * 0.0068`), the next one a tick on.
#[test]
fn the_first_draw_is_the_size_at_age_zero() {
    let mut rng = Rng::new(4);
    let size = Channel {
        period: 0.0,
        mode: ChannelMode::Keyframed,
        lo: 1.0,
        hi: 8.0,
        keys: vec![(0.0, 0.0068), (1.0, 1.0)],
    };
    let effect = effect_with((30.0, 0.0), size);
    let mut system = System::new();
    system.ignite(&effect, Vec3::ZERO, 1.0);
    let mut halves = Vec::new();
    for _ in 0..3 {
        system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
        let (additive, _) = system.vertices(&effect, Vec3::X, Vec3::Y);
        let width = additive
            .iter()
            .map(|v| v.position[0])
            .fold(f32::MIN, f32::max)
            - additive
                .iter()
                .map(|v| v.position[0])
                .fold(f32::MAX, f32::min);
        halves.push(width / 2.0);
    }
    assert!((halves[0] - 1.0476).abs() < 1e-3, "{halves:?}");
    assert!((halves[1] - 1.2765).abs() < 2e-2, "{halves:?}");
}

/// The extent's animated co-factor (`attribute` record, selector 2): a ring that is born
/// `10` units out at the start of the run is born `20` out at its end (`18` after nine of ten ticks), which is how the
/// Bomb's smoke ring widens over its twenty emitting ticks.
#[test]
fn an_animated_extent_widens_the_ring_over_the_emitters_run() {
    let mut built = (*effect("ring", false, 10.0)).clone();
    built.emitters[0].spawn = Spawn::Ring {
        extent: 10.0,
        spread: 0.0,
        mode: 0,
        arc: std::f32::consts::TAU,
    };
    built.emitters[0].extent_animation = Some(Channel {
        period: 0.0,
        mode: ChannelMode::Keyframed,
        lo: 1.0,
        hi: 2.0,
        keys: vec![(0.0, 0.0), (1.0, 1.0)],
    });
    let effect = std::sync::Arc::new(built);
    let mut rng = Rng::new(21);
    let mut system = System::new();
    system.ignite(&effect, Vec3::ZERO, 1.0);
    let widest = |system: &System| {
        let (additive, _) = system.vertices(&effect, Vec3::X, Vec3::Y);
        additive
            .chunks(6)
            .map(|quad| {
                let centre = quad
                    .iter()
                    .fold(Vec3::ZERO, |s, v| s + Vec3::from(v.position))
                    / 6.0;
                centre.x.hypot(centre.z)
            })
            .fold(0.0f32, f32::max)
    };
    system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
    let first = widest(&system);
    for _ in 0..8 {
        system.advance(&effect, DT, Vec3::ZERO, Vec3::Y, &mut rng);
    }
    let last = widest(&system);
    assert!((first - 10.0).abs() < 0.2, "{first}");
    // Born at the ninth tick, 80 % through a ten-tick run: `10 * (1 + 0.8)`.
    assert!((last - 18.0).abs() < 0.2, "{last}");
}

#[test]
fn a_titles_effect_directory_replaces_data_psys_and_nothing_else() {
    assert_eq!(
        effect_path_in(r"Data\Psys", "WO_QUAKE"),
        effect_path("WO_QUAKE")
    );
    assert_eq!(
        effect_path_in(r"Data\Particles2048", "WO_QUAKE"),
        r"Data\Particles2048\WO_QUAKE.POB"
    );
}
