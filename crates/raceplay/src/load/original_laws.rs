//! The original driver's variant for this race: the title's
//! [`oag_title::original_driver::OriginalDriver`] (Pulse's driver where it has
//! none), with its halved-lookahead row resolved to this circuit and class.
//!
//! HD's rule names a `PI_Track` id (`05_Track`), and a race is handed a
//! `.vex` entry, so the id is looked up in the title's own plugin definition:
//! the entry with the race's environment directory and direction. HD spells a
//! race's path `/data/environments/<dir>/track.vex` and its definition
//! `Data\Environments\<Dir>`, so both are compared lower-cased with one kind
//! of separator.

use oag_ai::original::Laws;

/// `path` lower-cased, with `/` as `\` and no leading separator.
fn normal(path: &str) -> String {
    path.replace('/', "\\")
        .trim_start_matches('\\')
        .to_ascii_lowercase()
}

/// The `PI_Track` id of the circuit the `.vex` entry `track` races on.
fn track_id(tracks: &[crate::catalogue::Track], track: &str) -> Option<String> {
    let track = normal(track);
    let (directory, file) = track.rsplit_once('\\')?;
    let reversed = file.contains("reversed");
    tracks
        .iter()
        .find(|t| normal(&t.location) == directory && t.reversed == reversed)
        .map(|t| t.id.clone())
}

/// This race's laws, said in `report` when the title has a variant.
pub(super) fn resolve(
    archives: &mut oag_assets::source::Archives,
    title: &'static oag_title::Title,
    track: &str,
    class: &str,
    report: &mut Vec<String>,
) -> Laws {
    let Some(driver) = title.race.original_driver else {
        return Laws::PULSE;
    };
    let halved_lookahead = if driver.halved_lookahead.is_empty() {
        None
    } else {
        let tracks = archives
            .read_name(title.plugin_definition)
            .ok()
            .and_then(|blob| oag_tables::fexml::text(&blob).ok())
            .map(|xml| crate::catalogue::tracks(&xml))
            .unwrap_or_default();
        match track_id(&tracks, track) {
            Some(id) => driver.halved_lookahead_for(&id, class),
            None => {
                report.push(format!(
                    "original AI: no PI_Track in {} matches {track}; its full lookahead is kept",
                    title.plugin_definition
                ));
                None
            }
        }
    };
    let laws = Laws {
        finished_spread: driver.finished_spread.value,
        finished_wander_to_place: driver.finished_wander_to_place.value,
        eliminator_drops_when_behind: driver.eliminator_drops_when_behind.value,
        round_robin_pre_increment: driver.round_robin_pre_increment.value,
        halved_lookahead,
    };
    report.push(format!(
        "original AI: this title's driver variant, {laws:?}"
    ));
    laws
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: &str, location: &str, reversed: bool) -> crate::catalogue::Track {
        crate::catalogue::Track {
            id: id.into(),
            location: location.into(),
            reversed,
            available_in_zone: false,
            unlock_grid: None,
            sound_register: None,
            foreign: None,
        }
    }

    #[test]
    fn an_hd_path_finds_its_definition_entry_and_direction() {
        let tracks = [
            track("05_Track", r"Data\Environments\05_Ubermall", false),
            track("06_Track", r"Data\Environments\05_Ubermall", true),
        ];
        assert_eq!(
            track_id(&tracks, "/data/environments/05_ubermall/track.vex").as_deref(),
            Some("05_Track")
        );
        assert_eq!(
            track_id(&tracks, r"Data\Environments\05_Ubermall\track_reversed.vex").as_deref(),
            Some("06_Track")
        );
        assert_eq!(
            track_id(&tracks, "/data/environments/amphiseum/track.vex"),
            None
        );
    }
}
