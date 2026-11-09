//! A trail-hit burst lent to a title that authors none (`graphics.trail_hits`).
//!
//! Pulse and Pure have no `WO_TRAIL_HITSHIP` of their own (`docs/formats/pulse-absent-effects.md`).
//! With the player's setting on, the race reads the effect off another source that does author
//! it, through that source's own title data (effect directory, colour scale, sprite and extent
//! laws), and plays it for a craft flying in a rival's trail. With no such source it plays
//! nothing and the loader report says so: no stand-in is drawn.

use std::sync::Arc;

use oag_fx::psys;
use oag_title::Trigger;

use super::{Options, assets};
use crate::TrailHits;

/// The lent effect, placed in `effects` (so its sprites join the shared sheet), or `None` when
/// the setting is off, the craft's title plays its own, or no source supplies one.
pub(super) fn lend(
    options: &Options,
    craft_title: &oag_title::Title,
    effects: &mut psys::Library,
    report: &mut Vec<String>,
) -> Option<Arc<psys::Effect>> {
    let TrailHits::On { lender } = &options.trail_hits else {
        return None;
    };
    if craft_title.effect_on(Trigger::TrailHitship).is_some() {
        return None;
    }
    let Some(lender) = lender else {
        report.push(format!(
            "trail hits: switched on, but no source that authors WO_TRAIL_HITSHIP (Wipeout HD, \
             2048 or Omega) is mounted, so {} plays no burst for a craft in a rival's trail",
            craft_title.name
        ));
        return None;
    };
    let opened = match oag_source::title::open_source(lender, Vec::new(), Vec::new()) {
        Ok(opened) => opened,
        Err(why) => {
            report.push(format!(
                "trail hits: {lender} would not open ({why}), so no burst plays"
            ));
            return None;
        }
    };
    let (mut archives, from) = (opened.archives, opened.title);
    let Some(spec) = from.effect_on(Trigger::TrailHitship) else {
        report.push(format!(
            "trail hits: {} authors no trail-hit burst, so none plays",
            from.name
        ));
        return None;
    };
    let platform = archives.layout.platform;
    let name = spec.effect;
    let loaded = assets::particle_effect(
        &mut archives,
        from.race.effect_dir_for(""),
        name,
        from.looks.particle_laws.applies(platform),
        from.effects.extents.contains(&name),
        from.effects.sprites.contains(&name),
    );
    match loaded {
        Ok((effect, note)) => {
            report.push(note);
            effects.insert(name, effect);
            report.push(format!(
                "trail hits: {name} read off {}, played for a craft flying in a rival's trail; an \
                 enhancement of this project's, {} authors no such burst",
                from.name, craft_title.name
            ));
            effects.get(name).cloned()
        }
        Err(why) => {
            report.push(format!("{why} - {name} will not be drawn"));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(lender: Option<&str>) -> Options {
        Options {
            trail_hits: TrailHits::On {
                lender: lender.map(str::to_string),
            },
            ..Options::default()
        }
    }

    /// Off reads nothing and says nothing: a Pulse or Pure race loads exactly as it did.
    #[test]
    fn off_lends_nothing_and_adds_no_report_line() {
        let (mut effects, mut report) = (psys::Library::new(), Vec::new());
        let lent = lend(
            &Options::default(),
            oag_pulse::TITLE,
            &mut effects,
            &mut report,
        );
        assert!(lent.is_none() && report.is_empty() && effects.is_empty());
    }

    /// On with no source that authors the burst: nothing plays, and the report names why.
    #[test]
    fn on_with_no_source_lends_nothing_and_the_report_says_why() {
        for title in [oag_pulse::TITLE, oag_pure::TITLE] {
            let (mut effects, mut report) = (psys::Library::new(), Vec::new());
            let lent = lend(&on(None), title, &mut effects, &mut report);
            assert!(lent.is_none() && effects.is_empty(), "{}", title.name);
            assert_eq!(report.len(), 1, "{report:?}");
            assert!(
                report[0].contains("no source that authors WO_TRAIL_HITSHIP")
                    && report[0].contains(title.name),
                "{}",
                report[0]
            );
        }
    }

    /// A source that will not open is a report line, not a panic and not a stand-in.
    #[test]
    fn a_lender_that_will_not_open_is_reported_and_lends_nothing() {
        let (mut effects, mut report) = (psys::Library::new(), Vec::new());
        let lent = lend(
            &on(Some("does/not/exist.iso")),
            oag_pulse::TITLE,
            &mut effects,
            &mut report,
        );
        assert!(lent.is_none() && effects.is_empty());
        assert!(report[0].contains("would not open"), "{report:?}");
    }

    /// A title that plays its own burst ignores the setting entirely.
    #[test]
    fn a_title_with_its_own_burst_ignores_the_setting() {
        for title in [oag_hd::TITLE, oag_2048::TITLE, oag_omega::TITLE] {
            let (mut effects, mut report) = (psys::Library::new(), Vec::new());
            let lent = lend(&on(None), title, &mut effects, &mut report);
            assert!(lent.is_none() && report.is_empty(), "{}", title.name);
        }
    }
}
