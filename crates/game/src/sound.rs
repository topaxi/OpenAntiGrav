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
) -> oag_sound::sfx::Cue {
    use oag_sound::sfx::Cue;
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
    keep: impl Fn(&oag_sound::sfx::CueEvent) -> bool,
) {
    let mut cues = race.drain_cues();
    cues.retain(|event| keep(event));
    let announcements = race.drain_announcements();
    let class_announcements = race.drain_class_announcements();
    let frame = RaceFrame {
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
        // The local player's throttle, `0..=100`: what
        // `Ship_UpdateEngineCrossfade` reads as `ctrl[+4]` for the craft whose
        // role is zero.
        throttle: race
            .sim
            .world
            .ships
            .first()
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
        leach_beam: race.sim.world.leach_beam,
    };
    audio.race_tick(frame);
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
    let camera = race.view().inverse();
    oag_audio::Listener {
        position: camera.w_axis.truncate().to_array(),
        right: camera.x_axis.truncate().normalize_or_zero().to_array(),
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
    use oag_sound::sfx::Cue;
    use oag_ui::menu::nav::{Dir, Nav};

    use super::menu_cue;

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
}
