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
    /// The best medal the watch has raised so far, as `RaceStage::earned_medal`.
    pub earned: std::cell::Cell<Option<oag_tables::race_campaign::Medal>>,
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
    if let Some(weapon) = options.give
        && race.sim.world.ships[0].pickup.weapon.is_none()
    {
        race.sim.world.ships[0].pickup.weapon = Some(weapon);
    }
    race.tick(&oag_gameplay::PlayerInputs::single(snapshot));
    // After the tick, as an impact arms it: the frame this tick ends on shows
    // the shake at progress 0.
    if let Some((at, severity)) = options.force_shake
        && at == tick
    {
        race.force_shake(severity);
    }
    for &(at, percent) in &options.force_shield {
        if at == tick {
            let ship = &mut race.sim.world.ships[0];
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
        crate::medal_watch::tick(cell, *difficulty, &mut earned, race);
        options.medals.earned.set(earned);
    }
    if let Some((at, slot)) = options.force_wreck
        && at == tick
    {
        race.force_destroy(slot);
    }
    if let Some((at, slot)) = options.force_hit
        && at == tick
    {
        race.force_weapon_hit(slot);
    }
    if let Some((at, slot)) = options.force_bomb_trip
        && at == tick
        && !race.force_bomb_trip(slot)
    {
        log::warn!(
            "--force-bomb-trip: no Bomb laid by another craft yet, or slot {slot} is not racing (pass --opponents)"
        );
    }
    if let Some((at, slot)) = options.force_missile_hit
        && at == tick
        && !race.force_missile_hit(slot)
    {
        log::warn!(
            "--force-missile-hit: no Missile in flight from another craft yet, or slot {slot} is not racing (pass --opponents)"
        );
    }
    if let Some((at, target)) = options.force_leach_lock
        && at == tick
        && !race.force_leach_lock(0, target as u8)
    {
        log::warn!(
            "--force-leach-lock: slot {target} is not racing (pass --opponents), or the title authors no LeachBeam"
        );
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
            .sim
            .world
            .tick
            .is_multiple_of(u64::from(options.log_every))
    {
        println!("{}", describe(&race.telemetry()));
    }
}
