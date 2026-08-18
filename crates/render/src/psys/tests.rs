//! What [`super`]'s emitter scheduling, particle pool and effect playback are
//! asserted to do.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of `psys.rs`:
//! the tests are past the 200 lines an inline test module may hold, and
//! `psys.rs` is a baselined file that may shrink but not grow. See
//! `scripts/check-file-size.py`, which is both rules as a gate.

use super::*;

const DT: f32 = 1.0 / TICK_HZ;

fn constant(value: f32) -> Channel {
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
fn effect(name: &str, looping: bool, duration_ticks: f32) -> std::sync::Arc<Effect> {
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
            blend: Blend::Additive,
            particle_child: None,
            death_child: None,
            spawn_probability: 1.0,
            velocity_inherit: 0.0,
        }],
        roots: vec![0],
    })
}

fn run(system: &mut System, effect: &Effect, ticks: usize, rng: &mut Rng) {
    for _ in 0..ticks {
        system.advance(effect, DT, Vec3::ZERO, rng);
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
