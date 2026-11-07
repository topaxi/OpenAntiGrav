//! The composition root's half of the sound crate.
//!
//! [`oag_sound`] decodes soundtracks and effects and plays them over the
//! mixer, and by design knows nothing of the game: it cannot open a source as
//! whichever title it is, cannot see a [`Race`], and cannot search the player's
//! image directories. This module is what supplies those, and nothing else:
//!
//! - [`GameLibrary`] opens a source through [`oag_source::title::open_source`] and
//!   lists the disc images [`oag_source::source::search_path`] names, which is what
//!   [`oag_sound::library::Library`] asks of its host;
//! - [`hold_race_song`] arms the race song for the title's start rule;
//! - [`race_tick`] turns a race's state into the plain-data
//!   [`oag_sound::sfx::RaceFrame`] the effects read, so the sound crate never
//!   depends on `Race`;
//! - [`listener_of`] and the craft positions the frame carries.

use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_sound::Audio;
use oag_sound::library::{Library, Opened};
use oag_sound::sfx::RaceFrame;

use oag_raceplay::Race;

/// Opens sources as whichever title they are, and searches the player's image
/// directories for a second release.
#[derive(Debug, Clone, Copy, Default)]
pub struct GameLibrary;

impl Library for GameLibrary {
    fn open(&self, source: &str) -> Option<Opened> {
        let opened = oag_source::title::open_source(source, Vec::new(), Vec::new()).ok()?;
        Some(Opened {
            title: opened.title,
            archives: opened.archives,
        })
    }

    fn containers(&self) -> Vec<PathBuf> {
        let mut found = Vec::new();
        for directory in oag_source::source::search_path() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };
            let mut candidates: Vec<PathBuf> = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_file() && oag_source::source::is_container(path))
                .collect();
            // Alphabetical, so two runs of the same directory pick the same
            // image - the same reason `oag_source::source::first_image` sorts.
            candidates.sort();
            found.extend(candidates);
        }
        found
    }

    fn sources(&self) -> Vec<PathBuf> {
        oag_source::source::candidates()
    }
}

/// The front-end cue a menu's navigation sound is played as.
///
/// The one place the menus' vocabulary meets the sound crate's, so a pad press,
/// a click and a selection screen all end in the same cues. `pad` is the
/// direction the pad edge of this tick pointed, for a screen that reported a
/// move without saying which way; a title with one cue for every direction
/// (Pulse) never reads it.
#[must_use]
pub fn menu_cue(
    nav: oag_ui::menu::nav::Nav,
    pad: Option<oag_ui::menu::nav::Dir>,
) -> oag_gameplay::cue::Cue {
    use oag_gameplay::cue::Cue;
    use oag_ui::menu::nav::{Dir, Nav};
    match nav {
        Nav::Moved(dir) => match dir.or(pad).unwrap_or(Dir::Down) {
            Dir::Up => Cue::MenuUp,
            Dir::Down => Cue::MenuDown,
            Dir::Left => Cue::MenuLeft,
            Dir::Right => Cue::MenuRight,
        },
        Nav::Stepped(dir) => match dir.or(pad) {
            Some(Dir::Left) => Cue::MenuStepLeft,
            _ => Cue::MenuStepRight,
        },
        Nav::Accept => Cue::MenuAccept,
        Nav::Decline => Cue::MenuDecline,
    }
}

/// Reads the front end's navigation sounds, voices and results lines out of
/// `title`'s own banks. **The one place the load's arguments are decided**: a
/// boot and the ground-truth tests that hold its result to the disc all come
/// through here, so the zone tick and the Fury style cannot drift between them.
/// `fury` is the style the served front end is in, which picks HD's
/// `accept_fury`/`reject_fury`.
pub fn load_menu_sfx(
    archives: &mut oag_assets::source::Archives,
    title: &oag_title::Title,
    fury: bool,
) -> oag_sound::sfx::MenuSfx {
    oag_sound::sfx::MenuSfx::load(
        archives,
        title.race.sounds,
        title
            .race
            .zone_announcer
            .map_or(oag_title::SequenceTick::Unknown, |z| z.tick),
        fury,
    )
}

/// Starts the cue `nav` calls for on the front-end sound path, and says which
/// cue it was and whether it began. What the session's `play_navs` does for
/// each nav a screen logged, so a test drives exactly that.
pub fn play_nav(
    sfx: &mut oag_sound::sfx::MenuSfx,
    audio: &Audio,
    nav: oag_ui::menu::nav::Nav,
    pad: Option<oag_ui::menu::nav::Dir>,
) -> (oag_gameplay::cue::Cue, bool) {
    let cue = menu_cue(nav, pad);
    (cue, sfx.play(audio, cue))
}

/// Arms the race song for the title about to race: when its original starts the song at the
/// release (`oag_title::Music::race_song_start`), the race hand-off holds it until
/// [`race_tick`] sees the race leave its countdown. Called before every fresh race starts its
/// music, whichever way it was reached; a resume mid-race does not call it. Also sets the title's
/// effects and speech volume laws ([`Audio::set_effects_volume_law`],
/// [`Audio::set_speech_volume_law`], [`Audio::set_speech_follows_effects`]) and its master trim ([`Audio::set_master_trim_db`]) for that race.
pub fn hold_race_song(audio: &mut Audio, title: &oag_title::Title) {
    audio.hold_race_song(title.music.is_some_and(oag_title::Music::song_waits_for_go));
    audio.set_effects_volume_law(title.effects_volume_law.value);
    audio.set_speech_volume_law(title.speech_volume_law.value);
    audio.set_speech_follows_effects(title.speech_follows_effects.value);
    audio.set_master_trim_db(title.master_trim_db.value);
}

/// Arms the rear low-pass of the race about to start: the layout the title's original reads
/// (`oag_title::Title::rear_occlusion`) and whether the circuit's own `AudioConfig.xml` turns the
/// filter off (`TrackEmitters::low_pass_off`). Called after the race is built, on every fresh start.
pub fn arm_rear_low_pass(
    audio: &mut Audio,
    title: &oag_title::Title,
    emitters: &oag_sound::sfx::TrackEmitters,
) {
    audio.set_rear_occlusion_layout(title.rear_occlusion.map(|layout| layout.value));
    audio.set_circuit_low_pass_off(emitters.low_pass_off);
}

/// Starts decoding `race`'s flyby voice-over, where it has one, off this thread: the clip is ready
/// by the time the flyby's counter reaches zero. See [`Audio::prepare_pre_race_voice`].
pub fn prepare_intro_voice(audio: &mut Audio, discs: &oag_sound::MusicDiscs, race: &Race) {
    if let Some(entry) = race.intro_voice_entry() {
        audio.prepare_pre_race_voice(discs, entry, &oag_source::cache::default_audio_cache_dir());
    }
}

/// Forwards the audio calls `race`'s flyby raised this tick to the audio, in order.
/// Called after every [`Race::tick_intro`].
pub fn intro_cues(audio: &mut Audio, race: &mut Race) {
    use oag_raceplay::intro_camera::IntroCue;
    for cue in race.take_intro_cues() {
        match cue {
            IntroCue::Start(_) => audio.start_pre_race_voice(),
            IntroCue::Stop => audio.stop_pre_race_voice(),
            IntroCue::SilenceEffects => audio.silence_sfx(),
            IntroCue::FadeInEffects { per_tick } => audio.fade_in_sfx(per_tick),
        }
    }
}

/// Plays one tick of `race`'s sound effects. See [`Audio::race_tick`].
///
/// Called from inside the fixed-timestep loop, immediately after `Race::tick`,
/// and also on a finished race.
pub fn race_tick(audio: &mut Audio, race: &mut Race) {
    race_tick_keeping(audio, race, |_| true);
}

/// [`race_tick`], handing the mixer only the cues `keep` accepts. A
/// verification aid: a ground-truth test renders a race twice, once with a cue
/// held back, and compares the two files to show what that cue added.
pub fn race_tick_keeping(
    audio: &mut Audio,
    race: &mut Race,
    keep: impl Fn(&oag_gameplay::cue::CueEvent) -> bool,
) {
    tick_effects(audio, race, keep, false);
}

/// A seek in a recorded race: every effect voice is stopped and the continuous ones are
/// rebuilt from the race as it stands at `tick` (`Audio::rebuild_race_sfx`). The cues of the
/// ticks the seek ran are dropped, so nothing that sounded before the target sounds late.
pub fn rebuild(audio: &mut Audio, race: &mut Race, tick: u64) {
    drop(race.drain_cues());
    drop(race.drain_announcements());
    drop(race.drain_class_announcements());
    audio.rebuild_race_sfx(frame_of(race, Vec::new(), Vec::new(), Vec::new()), tick);
}

/// One tick of the sound effects under a title's pre-race flyby, where its rule fades them in
/// (`Race::intro_plays_race_sounds`): the circuit's ambience emitters from the flyby camera's
/// ears, through the effects fade, and the craft's engines by range (inaudible until the camera
/// nears the grid, as the original's are). No cue is taken: the original starts no front-end or
/// UI cue in the window, and the song marquee is left alone. A title whose flyby does not fade its
/// effects in stays silent, as before (`docs/gameplay/race-intro.md`, "What ours does").
pub fn flyby_tick(audio: &mut Audio, race: &mut Race) {
    if race.intro_plays_race_sounds() {
        tick_effects(audio, race, |_| false, true);
    }
}

fn tick_effects(
    audio: &mut Audio,
    race: &mut Race,
    keep: impl Fn(&oag_gameplay::cue::CueEvent) -> bool,
    flyby: bool,
) {
    let (mut cues, announcements, class_announcements) = if flyby {
        (Vec::new(), Vec::new(), Vec::new())
    } else {
        (
            race.drain_cues(),
            race.drain_announcements(),
            race.drain_class_announcements(),
        )
    };
    cues.retain(|event| keep(event));
    let frame = frame_of(race, cues, announcements, class_announcements);
    audio.race_tick(frame);
    if flyby {
        return;
    }
    // The song-name marquee shows each race song once, off the same audio state a `--no-audio`
    // run reaches: the selection does not wait on a device.
    race.note_song(audio.now_playing().as_ref());
}

/// One tick of `race` as the sound effects see it, with the cues and announcements the caller
/// drained from it.
fn frame_of<'a>(
    race: &'a Race,
    cues: Vec<oag_gameplay::cue::CueEvent>,
    announcements: Vec<u16>,
    class_announcements: Vec<u32>,
) -> RaceFrame<'a> {
    RaceFrame {
        banks: race.sounds(),
        emitters: race.track_emitters(),
        announcer: race.announcer(),
        class_announcer: race.class_announcer(),
        cues,
        announcements,
        class_announcements,
        listener: listener_of(race),
        craft: craft_positions(race),
        slot_teams: std::array::from_fn(|slot| race.slot_team(slot)),
        player_slot: race.player_slot(),
        // The local player's throttle, `0..=100`: what
        // `Ship_UpdateEngineCrossfade` reads as `ctrl[+4]` for the craft whose
        // role is zero.
        throttle: race
            .sim
            .world
            .ships
            .get(race.player_slot())
            .map_or(0.0, |ship| ship.physics.thrust),
        // An owned snapshot - `Projectile` is `Copy` and the array is small.
        // Read after `Race::tick` has already run, so a bolt that ended this
        // tick is already back to `Projectile::default()` here.
        projectiles: race.sim.world.projectiles.slots,
        running: !race.finished(),
        shielded: race.shield_is_up(),
        exploding: race.craft_is_exploding(),
        autopilot_active: race.autopilot_is_active(),
        thrust_gated: oag_race::RaceState::thrust_gated(race.sim.world.tick),
        sight: race.sight_state(),
        quake_point: race.quake_point(),
        leach_beams: race.sim.world.leach_beams,
    }
}

/// The listener, off the camera the frame is actually drawn from.
///
/// `SoundManager_Update` (`0x0893a2b0`) copies the active camera's rotation
/// rows and the negation of its `+0x70` into the sound manager once a frame.
/// The negation is there because the camera stores a *negated* eye beside a
/// world-to-camera rotation whose world axes are its columns - a split
/// `docs/.../camera.md` measured from the rendering side and this reads back
/// from the audio side. Inverting `Race::view` recovers both halves at once:
/// the camera's world matrix, whose translation is the eye and whose first
/// column is the right axis the pan projects onto.
///
/// `pub` because a circuit's own emitters are placed against the same ears the
/// craft cues are, and the two must not be allowed to disagree about where the
/// listener is - see [`TrackEmitters`].
pub fn listener_of(race: &oag_raceplay::Race) -> oag_audio::Listener {
    listener_of_view(race.view())
}

/// [`listener_of`] for a world-to-camera matrix. A right-handed camera looks
/// down its own `-Z`, so the forward axis is the negated third column.
pub fn listener_of_view(view: oag_core::math::Mat4) -> oag_audio::Listener {
    let camera = view.inverse();
    oag_audio::Listener {
        position: camera.w_axis.truncate().to_array(),
        right: camera.x_axis.truncate().normalize_or_zero().to_array(),
        forward: (-camera.z_axis.truncate()).normalize_or_zero().to_array(),
    }
}

/// Every live craft's world position and speed, indexed by grid slot.
///
/// [`None`] for a slot the race did not field. Read once per tick rather than
/// per cue, because eight cues from one craft must all agree on where it was.
fn craft_positions(race: &oag_raceplay::Race) -> [Option<(Vec3, f32)>; oag_gameplay::MAX_SHIPS] {
    std::array::from_fn(|slot| {
        let ship = race.sim.world.ships.get(slot)?;
        (slot < race.ship_count() as usize && ship.active).then(|| {
            (
                ship.physics.body.position,
                ship.physics.body.linear_velocity.length(),
            )
        })
    })
}

#[cfg(test)]
mod tests {
    use oag_gameplay::cue::Cue;
    use oag_ui::menu::nav::{Dir, Nav};

    use super::{arm_rear_low_pass, listener_of_view, menu_cue};

    /// The chase camera sits behind and above its craft, so the craft is ahead
    /// of the listener and a point at the camera's back is behind it. Fails if
    /// the forward axis takes the wrong sign of the camera's third column.
    #[test]
    fn the_listener_faces_the_way_the_chase_camera_looks() {
        use oag_core::math::{Vec3, camera};
        let eye = Vec3::new(0.0, 5.0, 10.0);
        let view = camera::look_at(eye, Vec3::ZERO, Vec3::Y);
        let ears = listener_of_view(view);
        let toward = |p: Vec3| {
            let d = (p - eye).normalize();
            d.dot(Vec3::from(ears.forward))
        };
        assert!(toward(Vec3::ZERO) > 0.9, "the craft is ahead of the camera");
        assert!(
            toward(Vec3::new(0.0, 8.0, 20.0)) < -0.5,
            "behind the camera"
        );
        assert!((Vec3::from(ears.forward).length() - 1.0).abs() < 1e-5);
        assert!(Vec3::from(ears.forward).dot(Vec3::from(ears.right)).abs() < 1e-5);
    }

    #[test]
    fn a_move_takes_its_own_direction_before_the_pads() {
        assert_eq!(
            menu_cue(Nav::Moved(Some(Dir::Up)), Some(Dir::Left)),
            Cue::MenuUp
        );
        assert_eq!(menu_cue(Nav::Moved(Some(Dir::Right)), None), Cue::MenuRight);
    }

    #[test]
    fn a_screen_that_does_not_say_which_way_takes_the_pad_edge() {
        assert_eq!(menu_cue(Nav::Moved(None), Some(Dir::Left)), Cue::MenuLeft);
        assert_eq!(menu_cue(Nav::Moved(None), Some(Dir::Up)), Cue::MenuUp);
        assert_eq!(
            menu_cue(Nav::Moved(None), None),
            Cue::MenuDown,
            "a pointer move onto a lower row"
        );
        assert_eq!(
            menu_cue(Nav::Stepped(None), Some(Dir::Left)),
            Cue::MenuStepLeft
        );
        assert_eq!(menu_cue(Nav::Stepped(None), None), Cue::MenuStepRight);
        assert_eq!(menu_cue(Nav::Accept, Some(Dir::Up)), Cue::MenuAccept);
        assert_eq!(menu_cue(Nav::Decline, None), Cue::MenuDecline);
    }

    /// Arming a race puts each title's speech source on the bus: Pulse's Speech group is the SFX
    /// slider squared, HD's stays on the Speech slider. Fails if `hold_race_song` drops the field.
    #[test]
    fn arming_a_race_sets_the_titles_speech_source() {
        use oag_audio::Bus;
        use oag_sound::Volume;
        use oag_sound::settings::Settings;
        let settings = Settings {
            sfx_volume: Volume::OFFERED[2],
            ..Default::default()
        };
        let bus = |title: &oag_title::Title| {
            let mut audio =
                oag_sound::Audio::open(&settings, None, None, oag_audio::MIN_BUFFER, false);
            super::hold_race_song(&mut audio, title);
            audio.output().with_mixer(|m| m.bus_gain(Bus::Speech))
        };
        assert_eq!(bus(oag_pulse::TITLE), 0.25);
        assert_eq!(bus(oag_hd::TITLE), 1.0);
    }

    const REAR_FILE: &str = r#"<AudioConfig><RearOcclusion><Stereo max_frequency="14000.0"/></RearOcclusion>
<ParameterMaps><ParameterMapFrontEnd><Stereo><GroupVolumes music="1" user1="1" user2="1" user3="1" user4="1" user5="1" user6="1" user7="1" user8="1" user9="1" user10="1" user11="1" user12="1"/></Stereo></ParameterMapFrontEnd></ParameterMaps></AudioConfig>"#;

    /// Fails if a title's layout or a circuit's low-pass switch stops reaching the audio, or if a
    /// title that names no layout plays the law because its file carries one.
    #[test]
    fn a_race_arms_the_rear_low_pass_from_its_title_and_its_circuit() {
        let mut audio = oag_sound::Audio::open(
            &oag_sound::settings::Settings::default(),
            None,
            None,
            oag_audio::MIN_BUFFER,
            false,
        );
        audio.set_hd_mix(oag_sound::hd_mix::Maps::parse(REAR_FILE));
        let mut emitters = oag_sound::sfx::TrackEmitters::default();
        let law = |audio: &oag_sound::Audio| audio.rear_low_pass();

        arm_rear_low_pass(&mut audio, oag_hd::TITLE, &emitters);
        assert_eq!(
            law(&audio),
            Some(14_000.0),
            "HD reads its file's stereo value"
        );
        emitters.low_pass_off = true;
        arm_rear_low_pass(&mut audio, oag_hd::TITLE, &emitters);
        assert_eq!(law(&audio), None, "a circuit with its low-pass off");
        emitters.low_pass_off = false;
        arm_rear_low_pass(&mut audio, oag_pulse::TITLE, &emitters);
        assert_eq!(law(&audio), None, "Pulse names no layout");
        arm_rear_low_pass(&mut audio, oag_omega::TITLE, &emitters);
        assert_eq!(
            law(&audio),
            None,
            "Omega's file has the element and nothing reads it"
        );
    }
}
