//! The race's decoded sound cues and its two Zone-mode voice ladders.
//!
//! Split out of `load.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// [`oag_sound::sfx::Banks`], [`oag_sound::sfx::Announcer`] and
/// [`oag_sound::sfx::ClassAnnouncer`], each loaded on the same "never
/// fails, an empty result costs nothing to carry" terms.
pub(super) fn banks_and_announcers(
    archives: &mut oag_assets::Archives,
    title: &oag_title::Title,
    mode: Mode,
    vr_fx: bool,
    slot_teams: &[String],
    no_effects: bool,
    report: &mut Vec<String>,
) -> (
    oag_sound::sfx::Banks,
    oag_sound::sfx::Announcer,
    oag_sound::sfx::ClassAnnouncer,
) {
    // Nothing is read, so nothing decodes and nothing is reported: the race has no cue to play.
    if no_effects {
        return Default::default();
    }
    let (race, lap_hud, destroyed) = (title.race, title.hud_art.lap_hud, title.destroyed.value);
    // Adding a cue is adding its name and its trigger, never a loader. Zone
    // races read `ship_zone.bnk` instead of `ship.bnk` - a different bank
    // with the same cue names.
    let tick = race
        .zone_announcer
        .map_or(oag_title::SequenceTick::Unknown, |z| z.tick);
    // A Zone race takes `FUN_08847a54`'s own branch (`ship_zone.bnk`, no weapons bank) and never
    // reads the flag, though `Data\Zone\01_Zone` authors it.
    let vr_fx = vr_fx && mode != Mode::Zone;
    let banks = race.sounds.for_circuit(vr_fx);
    match (vr_fx, race.sounds.vr) {
        (true, Some(vr)) => report.push(format!(
            "sfx: the circuit's <VrFx> is on: {} and {} in place of {} and {}",
            vr.ship, vr.weapons, race.sounds.ship, race.sounds.weapons
        )),
        (true, None) => report.push(
            "sfx: the circuit's <VrFx> is on and this title names no VR sound banks; keeping its own"
                .to_string(),
        ),
        (false, _) => {}
    }
    let mut sounds = oag_sound::sfx::Banks::load(archives, &banks, mode == Mode::Zone, tick);
    if mode == Mode::Detonator {
        sounds.load_detonator(archives, &banks, tick);
    }
    // A Zone craft's end plays `shipdestroy` out of the environment bank
    // (`oag_title::destroyed::ZoneEnd::cue`).
    if let (Mode::Zone, Some(cue)) = (mode, destroyed.zone_end.and_then(|end| end.cue)) {
        sounds.load_named_as(
            archives,
            cue.bank,
            cue.tick,
            cue.hashed,
            &[(oag_gameplay::cue::Cue::ShipDestroy, "shipdestroy")],
        );
    }
    // Pure's `EXPLSMALL` and `EXPLBIG` out of its ship bank, for every craft.
    if destroyed.sound == oag_title::destroyed::DestroyedSound::BlowupAndBlasts {
        let bank = if mode == Mode::Zone {
            banks.ship_zone
        } else {
            banks.ship
        };
        sounds.load_named(
            archives,
            bank,
            tick,
            &[
                (oag_gameplay::cue::Cue::ShipExplosionSmall, "EXPLSMALL"),
                (oag_gameplay::cue::Cue::ShipExplosion, "EXPLBIG"),
            ],
        );
    }
    // A title whose cues are Wwise events (Omega) plays those in place of the
    // named cues, which its stub banks do not hold.
    if let Some(wwise) = race.sounds.wwise {
        sounds.load_wwise_cues(archives, wwise, slot_teams.first().map(String::as_str));
    }
    // The start-of-race voice, from the speech bank this mode opened rather
    // than the title's: Zone and Eliminator each load their own. See
    // `oag_title::CountdownVoice`.
    let speech = match race.countdown_voice {
        Some(voice) => match mode {
            Mode::Zone => voice.zone_bank,
            Mode::Eliminator => voice.eliminator_bank,
            _ => race.sounds.speech,
        },
        None => race.sounds.speech,
    };
    if let Some(voice) = race.countdown_voice {
        sounds.load_countdown(archives, speech, voice);
    }
    // The lap announcer shares the mode's speech slot on Pulse
    // (`DAT_08ac1dfc`, `FUN_08829778`) and is the title's `speech.bnk` on HD.
    if let Some(lap) = lap_hud {
        sounds.load_lap_voices(archives, speech, tick, lap.voices);
        if let Some(medal) = lap.speed_lap_medal {
            sounds.load_speed_lap_medal(archives, tick, medal);
        }
    }
    // Detonator's EMP announcer lines, from the bank the title names (HD: `speech_det.bnk`).
    if mode == Mode::Detonator {
        match race.detonator.and_then(|detonator| detonator.voices) {
            Some(voices) => sounds.load_named(
                archives,
                voices.bank,
                tick,
                &[
                    (oag_gameplay::cue::Cue::EmpReady, voices.ready),
                    (oag_gameplay::cue::Cue::EmpFull, voices.full),
                ],
            ),
            None => report.push(
                "sfx: EMPREADY and EMPFULL not played: this title names no Detonator voices"
                    .to_string(),
            ),
        }
    }
    // The engine is a per-team crossfade table on HD and 2048, not a cue:
    // loaded only where the ship bank has no `~ENGINE`. 2048 addresses a bank
    // of its own and keeps a second set of tables for Zone. See
    // `oag_sound::sfx::XfadeTeam` and `oag_title::Crossfade`.
    let zone = mode == Mode::Zone;
    let source = match race.sounds.crossfade {
        Some(c) => oag_sound::sfx::XfadeSource {
            bank: if zone { c.ship_zone } else { c.ship },
            infix: c.zone_infix.filter(|_| zone).unwrap_or(""),
        },
        None => oag_sound::sfx::XfadeSource {
            bank: if zone { banks.ship_zone } else { banks.ship },
            infix: "",
        },
    };
    sounds.load_xfade(archives, source, slot_teams, report);
    report.extend(sounds.report.iter().cloned());

    // Loaded regardless of mode, on the same terms `sounds` is: an announcer
    // with nothing to say (not Zone, or a title with no recovered ladder) is
    // an empty map.
    let announcer = oag_sound::sfx::Announcer::load(archives, race.zone_announcer);
    report.extend(announcer.report.iter().cloned());

    // `announcer`'s sibling, keyed by stage instead of milestone.
    let class_announcer = oag_sound::sfx::ClassAnnouncer::load(archives, race.zone_class_announcer);
    report.extend(class_announcer.report.iter().cloned());

    (sounds, announcer, class_announcer)
}

/// The circuit's own authored sound emitters, read off the track `.vex` already
/// in hand.
///
/// Separate from [`banks_and_announcers`] because it is keyed by *circuit*
/// rather than by title: one disc's twelve tracks author twelve different sets
/// and a Zone circuit authors none, where the bank table is one per release.
/// See [`oag_sound::sfx::TrackEmitters`].
pub(super) fn track_emitters(
    archives: &mut oag_assets::Archives,
    banks: &oag_title::SoundBanks,
    track: &str,
    blob: &[u8],
    report: &mut Vec<String>,
) -> oag_sound::sfx::TrackEmitters {
    let loaded = oag_sound::sfx::TrackEmitters::load(archives, banks, track, blob);
    report.extend(loaded.report.iter().cloned());
    loaded
}
