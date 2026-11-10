//! Our own rocket effect behind the player's `graphics.own_rockets`
//! (`docs/rendering/own-rockets.md`).
//!
//! Three parametric effects of this project's, in the format a mod's effect is written in,
//! served by the fallback layer (`assets/fallback/Data/Psys/OAGRocket*.toml`) and played on the
//! Rocket's own triggers in place of what the disc authors: the flight effect rides the rocket
//! (flame, smoke trail and the launch flash it starts with), and one burst each for an impact
//! on the track and on a craft. The puff sprites they name are the fallback layer's too.
//! Nothing is read from a disc. With the setting off, or on a title that is not offered it,
//! nothing is read and no trigger moves.

use std::sync::Arc;

use oag_fx::psys::{self, parametric};
use oag_title::Trigger;

use super::Options;

/// Each effect's trigger and its fallback-layer entry, which is also its key in the race's
/// library.
pub const EFFECTS: [(Trigger, &str); 3] = [
    (Trigger::RocketFlare, r"Data\Psys\OAGRocketFlight.toml"),
    (Trigger::TrackBlast, r"Data\Psys\OAGRocketImpactTrack.toml"),
    (Trigger::CraftBlast, r"Data\Psys\OAGRocketImpactCraft.toml"),
];

/// Whether this race draws our rocket: the player's setting, on a title that is offered it.
pub(super) fn on(options: &Options, craft_title: &oag_title::Title) -> bool {
    options.own_rockets && craft_title.looks.own_rockets.applies_everywhere()
}

/// One effect file and the sprites it names, through `read`.
fn effect(
    name: &str,
    read: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> Result<psys::Effect, String> {
    let bytes = read(name).ok_or("not in the archive set")?;
    let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
    let mut texture = |wanted: &parametric::TextureRef<'_>| {
        let parametric::TextureRef::Mod(entry) = *wanted else {
            return Err("a circuit's texture is not ours to name".to_string());
        };
        read(entry)
            .and_then(|blob| psys::sprite::Sprite::from_mask(&blob))
            .ok_or_else(|| "missing or not an OAGMASK".to_string())
    };
    parametric::parse(&text, &mut texture)
        .map(|parsed| parsed.effect)
        .map_err(|e| e.to_string())
}

/// Our effects, placed in `effects` (so their sprites join the shared sheet), each with the
/// trigger it takes over. Empty when [`on`] says no. An effect that does not load is named in
/// `report` and its trigger keeps the disc's.
pub(super) fn load(
    options: &Options,
    craft_title: &oag_title::Title,
    read: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
    effects: &mut psys::Library,
    report: &mut Vec<String>,
) -> Vec<(Trigger, Arc<psys::Effect>)> {
    if !on(options, craft_title) {
        return Vec::new();
    }
    let mut loaded = Vec::new();
    for (trigger, name) in EFFECTS {
        match effect(name, read) {
            Ok(parsed) => {
                effects.insert(name, parsed);
                report.extend(crate::assets::sheet_note(name, effects));
                if let Some(effect) = effects.get(name).cloned() {
                    loaded.push((trigger, effect));
                }
            }
            Err(why) => report.push(format!(
                "own rockets: {name} left out: {why}; the disc's effect plays there"
            )),
        }
    }
    report.push(format!(
        "own rockets: {} of {} effects of this project's (ours) play in place of {}'s rocket \
         flare and blasts (graphics.own_rockets)",
        loaded.len(),
        EFFECTS.len(),
        craft_title.name
    ));
    loaded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(own_rockets: bool) -> Options {
        Options {
            own_rockets,
            ..Options::default()
        }
    }

    /// The fallback layer's files, off the tree it is embedded from.
    fn tracked(name: &str) -> Option<Vec<u8>> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/fallback")
            .join(name.replace('\\', "/"));
        std::fs::read(path).ok()
    }

    /// Off reads nothing and says nothing: a race loads exactly as it did.
    #[test]
    fn off_reads_nothing_and_adds_no_report_line() {
        for title in [oag_pulse::TITLE, oag_pure::TITLE] {
            let (mut effects, mut report) = (psys::Library::new(), Vec::new());
            let mut read = |name: &str| -> Option<Vec<u8>> { panic!("off read {name}") };
            let loaded = load(&options(false), title, &mut read, &mut effects, &mut report);
            assert!(loaded.is_empty() && report.is_empty() && effects.is_empty());
        }
    }

    /// On, Pulse and Pure get all three effects, each with particles to draw; every other
    /// title reads none whatever the setting says.
    #[test]
    fn on_takes_the_three_rocket_triggers_on_pulse_and_pure_alone() {
        for title in [oag_pulse::TITLE, oag_pure::TITLE] {
            let (mut effects, mut report) = (psys::Library::new(), Vec::new());
            let loaded = load(
                &options(true),
                title,
                &mut tracked,
                &mut effects,
                &mut report,
            );
            let triggers: Vec<Trigger> = loaded.iter().map(|(trigger, _)| *trigger).collect();
            assert_eq!(
                triggers,
                [
                    Trigger::RocketFlare,
                    Trigger::TrackBlast,
                    Trigger::CraftBlast
                ],
                "{}: {report:?}",
                title.name
            );
            assert!(loaded.iter().all(|(_, effect)| !effect.emitters.is_empty()));
            assert!(
                report.iter().all(|line| !line.contains("left out")),
                "{report:?}"
            );
        }
        for title in [oag_hd::TITLE, oag_2048::TITLE, oag_omega::TITLE] {
            let (mut effects, mut report) = (psys::Library::new(), Vec::new());
            let mut read = |name: &str| -> Option<Vec<u8>> { panic!("{} read {name}", title.name) };
            let loaded = load(&options(true), title, &mut read, &mut effects, &mut report);
            assert!(loaded.is_empty() && report.is_empty() && effects.is_empty());
        }
    }

    /// The flight effect is what strings out behind the rocket: it must keep emitting for
    /// the whole flight, start with a launch flash that does not, and lay its smoke with a
    /// puff sprite rather than the soft disc.
    #[test]
    fn the_flight_effect_loops_opens_with_a_one_shot_flash_and_lays_puffs() {
        let (mut effects, mut report) = (psys::Library::new(), Vec::new());
        let loaded = load(
            &options(true),
            oag_pulse::TITLE,
            &mut tracked,
            &mut effects,
            &mut report,
        );
        let flight = &loaded[0].1;
        let emitter = |name: &str| {
            flight
                .emitters
                .iter()
                .find(|emitter| emitter.name == name)
                .unwrap_or_else(|| panic!("no emitter {name}"))
        };
        assert!(!emitter("launch-flash").looping);
        assert!(emitter("flame").looping);
        assert!(emitter("smoke").looping && emitter("smoke").sprite.is_some());
    }

    /// A file the layer does not answer leaves that trigger to the disc and says so.
    #[test]
    fn a_missing_file_is_named_and_its_trigger_stays_the_discs() {
        let (mut effects, mut report) = (psys::Library::new(), Vec::new());
        let mut read = |name: &str| (!name.contains("Flight")).then(|| tracked(name)).flatten();
        let loaded = load(
            &options(true),
            oag_pulse::TITLE,
            &mut read,
            &mut effects,
            &mut report,
        );
        assert!(
            loaded
                .iter()
                .all(|(trigger, _)| *trigger != Trigger::RocketFlare)
        );
        assert_eq!(loaded.len(), 2);
        assert!(
            report
                .iter()
                .any(|line| line.contains("OAGRocketFlight.toml left out"))
        );
    }
}
