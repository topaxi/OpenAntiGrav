//! [`advance_one_tick`]: one tick of a headless capture, moved out of
//! `capture.rs` under the 1,000-line rule in `scripts/check-file-size.py`; a
//! move, with no behaviour change.

use super::*;

/// The medal lines a capture can show: forced ones, and the real watch over a
/// campaign cell's own targets.
#[derive(Debug, Clone, Default)]
pub struct Medals {
    /// Every `--force-medal TICK:TIER`: a phrase id, raised as a HUD message.
    pub forced: Vec<(u32, &'static str)>,
    /// `--campaign-cell`: the cell whose targets the race is judged against,
    /// at the rung it is judged on. [`crate::medal_watch::tick`] then runs
    /// every tick exactly as the windowed game's `RaceStage` runs it.
    pub cell: Option<(
        oag_tables::race_campaign::Cell,
        oag_tables::race_campaign::Difficulty,
    )>,
    /// The title's [`oag_title::Campaign::counting_modes`], the law [`Self::cell`] is judged by.
    pub counting_modes: &'static [&'static str],
    /// The best medal the watch has raised so far, as `RaceStage::earned_medal`.
    pub earned: std::cell::Cell<Option<oag_tables::race_campaign::Medal>>,
}

/// One `--force-rocket-hit-at`: see [`Race::force_rocket_hit_at`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForcedRocketHit {
    /// The blasts play at the end of this tick, so they are drawn from the next, as a hit
    /// forced by `--force-rocket-hit` at the same tick is.
    pub tick: u32,
    pub at: oag_core::math::Vec3,
    pub count: usize,
}

/// One simulation tick of a capture: `tick`'s input, the race step, and the
/// audio that rides it.
///
/// Extracted from the tick loop so a motion blur capture can defer exactly
/// one tick past the primer render without duplicating the input logic -
/// a second copy of the script/pulse arithmetic is how the two would drift.
pub(super) fn advance_one_tick(
    race: &mut Race,
    held: &mut HeldButtons,
    audio: &mut oag_sound::Audio,
    options: &CaptureOptions,
    tick: u32,
) {
    held.advance(
        options.input_script.as_ref(),
        options.pressed,
        options.held,
        tick,
    );
    let snapshot = held.snapshot();
    // Before the tick, so `spend_pickup` can fire it on this tick's edge.
    // Every viewed slot, which is slot 0 alone without a split.
    for slot in 0..race.view_state().viewport_count() {
        if let Some(weapon) = options.give {
            race.give_weapon(slot, weapon);
        }
    }
    let held_pose =
        (options.pose_hold == pose_hold::Hold::EveryTick).then(|| pose_hold::StartPose::of(race));
    match (&options.net, &options.remote_correction) {
        (Some(net), _) => net.drive_to(race, u64::from(tick) + 1, snapshot),
        (None, Some(scene)) => scene.tick(race, &snapshot, tick),
        (None, None) if options.shot.restage.is_some() => {
            let record = options
                .shot
                .restage
                .as_deref()
                .expect("matched by the guard");
            for event in record.replay.events_at(u64::from(tick)) {
                if let Err(why) = race.core.sim.apply_event(event) {
                    log::warn!("--from-png: {why}");
                }
            }
            race.tick(&record.replay.inputs_at(u64::from(tick)));
        }
        (None, None) => {
            race.tick(&oag_gameplay::PlayerInputs::single(snapshot));
        }
    }
    if let Some(pose) = held_pose {
        pose.restore_at_rest(race);
    }
    // After the tick, as an impact arms it: the frame this tick ends on shows
    // the shake at progress 0.
    if let Some((at, severity)) = options.force_shake
        && at == tick
    {
        race.force_shake(severity);
    }
    // After the tick too, so the frame this tick ends on is the first drawn
    // through the wider field.
    if let Some((at, open)) = options.force_boost_kick
        && at == tick
    {
        race.force_boost_kick(open);
    }
    for &(at, percent) in &options.force_shield {
        if at == tick {
            let ship = &mut race.core.sim.world.ships[0];
            ship.physics.shield = ship.handling.dimensions.shield * percent / 100.0;
        }
    }
    for &(at, id) in &options.medals.forced {
        if at == tick {
            race.raise_message(id, true);
        }
    }
    if let Some((cell, difficulty)) = &options.medals.cell {
        let mut earned = options.medals.earned.get();
        crate::medal_watch::tick(
            cell,
            *difficulty,
            options.medals.counting_modes,
            &mut earned,
            race,
        );
        options.medals.earned.set(earned);
    }
    race.set_weapon_lights(!options.no_weapon_lights);
    if let Some((at, slot)) = options.force_wreck
        && at == tick
    {
        race.core.sim.force_destroy(slot);
    }
    if let Some((at, score)) = options.force_detonator_score
        && at == tick
    {
        race.core.sim.world.detonator.cannon.score = score;
    }
    if let Some((at, slot)) = options.force_hit
        && at == tick
    {
        race.force_weapon_hit(slot);
    }
    if let Some((at, slot)) = options.force_wall_slide
        && at == tick
    {
        race.core.sim.force_wall_slide(slot);
    }
    if let Some((at, slot)) = options.force_bomb_trip
        && at == tick
        && !race.core.sim.force_bomb_trip(slot)
    {
        log::warn!(
            "--force-bomb-trip: no Bomb laid by another craft yet, or slot {slot} is not racing (pass --opponents)"
        );
    }
    if let Some((at, slot)) = options.force_missile_hit
        && at == tick
        && !race.core.sim.force_missile_hit(slot)
    {
        log::warn!(
            "--force-missile-hit: no Missile in flight from another craft yet, or slot {slot} is not racing (pass --opponents)"
        );
    }
    if let Some((at, slot)) = options.force_rocket_hit
        && at == tick
    {
        if !race.core.sim.force_rocket_hit(slot) {
            log::warn!(
                "--force-rocket-hit: no Rocket in flight from another craft yet, or slot {slot} is not racing (pass --opponents)"
            );
        } else if let Some(velocity) = options.force_rocket_hit_velocity {
            race.force_struck_velocity(slot, velocity);
        }
    }
    for hit in options
        .force_rocket_hit_at
        .iter()
        .filter(|h| h.tick == tick)
    {
        match race.force_rocket_hit_at(hit.at, hit.count, options.force_rocket_hit_velocity) {
            None => log::warn!("--force-rocket-hit-at: this race loaded no craft blast"),
            Some(retired) if retired < hit.count => log::debug!(
                "--force-rocket-hit-at: {retired} of {} Rockets of the player's were in flight",
                hit.count
            ),
            Some(_) => {}
        }
    }
    if let Some((at, slot)) = options.force_quake
        && at == tick
        && !race.core.force_quake(slot)
    {
        log::warn!(
            "--force-quake: slot {slot} is not an opponent racing (pass --opponents), a Quake is already up, or the title authors none"
        );
    }
    if let Some((at, target)) = options.force_leach_lock
        && at == tick
        && !race.core.sim.force_leach_lock(0, target as u8)
    {
        log::warn!(
            "--force-leach-lock: slot {target} is not racing (pass --opponents), or the title authors no LeachBeam"
        );
    }
    if let (Some((at, _)), Some(path)) = (options.force_leach_lock, &options.leach_path)
        && at == tick
        && !race.force_leach_path(path.clone())
    {
        log::warn!("--leach-path: this title draws no LeachBeam strip");
    }
    // The race's own voices, on the tick that raised them - the same call
    // the windowed loop makes immediately after `Race::tick` in
    // `main::session::frame`. Without it a `--dump-audio` capture of a race
    // carried the music and nothing else: no engines, no collisions, no
    // speech, and the cues piled up undrained in the race. The dump is the
    // only end-to-end evidence a headless run has that a sound was made at
    // all, so a capture that silently held only half the mix is worse than
    // no capture.
    crate::sound::race_tick(audio, race);
    // Beside the race step and not outside the loop, for the reason the
    // exhaust and the chase camera are advanced from inside `Race::tick`:
    // what a capture produces has to be a function of the tick count and
    // nothing else, or the same command line gives a different file on a
    // slower machine.
    audio.tick();
    // The same two calls `main::session::frame` makes, so that a headless
    // capture is a usable instrument and not only a picture. `--tap-audio`
    // needs them in particular: the recording is written from whichever loop
    // is running, and this one is the only loop a machine with no window has.
    audio.output().report_health();
    audio.output().flush_tap();
    if options.log_every > 0
        && race
            .core
            .sim
            .world
            .tick
            .is_multiple_of(u64::from(options.log_every))
    {
        println!("{}", describe(&race.core.telemetry()));
    }
}
