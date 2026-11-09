//! The trail-hit burst behind the player's `graphics.trail_hits`.
//!
//! Pulse and Pure have no `WO_TRAIL_HITSHIP` of their own (`docs/formats/pulse-absent-effects.md`).
//! With the player's setting on, the race reads the effect off another source that does author
//! it, through that source's own title data (effect directory, colour scale, sprite and extent
//! laws), and plays it for a craft flying in a rival's trail. With no such source it plays
//! nothing and the loader report says so: no stand-in is drawn.
//!
//! 2048 and Omega author the burst, but their executables never spawn it
//! (`docs/ghidra/functions/ps4-omega-eu/trail-hits.md`): [`gate`] keeps their own effect off the
//! race unless the setting is on. Wipeout HD plays it whatever the setting.

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
        from.effects.far_bars.contains(&name),
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

/// `handles` without the title's own trail-hit burst when the title's original never fires it
/// (`oag_title::Effects::trail_hits_native`) and the player's setting is off. A lent burst only
/// exists with the setting on, so it is never taken back here.
pub(super) fn gate(
    options: &Options,
    craft_title: &oag_title::Title,
    handles: crate::EffectHandles,
) -> crate::EffectHandles {
    if craft_title.effects.trail_hits_native || matches!(options.trail_hits, TrailHits::On { .. }) {
        return handles;
    }
    handles
        .without(Trigger::TrailHitship)
        .without(Trigger::TrailHitshipRed)
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

    fn both_bursts() -> crate::EffectHandles {
        let mut handles = crate::EffectHandles::default();
        for trigger in [Trigger::TrailHitship, Trigger::TrailHitshipRed] {
            let pob = crate::tests::respawn::one_emitter_pob("WO_TRAIL_HITSHIP", 0);
            let effect = psys::Effect::parse(&pob, psys::ColourScale::Full).expect("parses");
            handles.insert(trigger, effect);
        }
        handles
    }

    fn plays(handles: &crate::EffectHandles) -> [bool; 2] {
        [Trigger::TrailHitship, Trigger::TrailHitshipRed].map(|t| handles.get(t).is_some())
    }

    /// 2048 and Omega author the burst but never fire it: their own effect plays only with the
    /// setting on.
    #[test]
    fn a_title_whose_original_never_fires_the_burst_plays_it_only_with_the_setting() {
        for title in [oag_2048::TITLE, oag_omega::TITLE] {
            assert!(!title.effects.plays_trail_hits_natively(), "{}", title.name);
            let off = gate(&Options::default(), title, both_bursts());
            assert_eq!(
                plays(&off),
                [false, false],
                "{} with the setting off",
                title.name
            );
            let on = gate(&on(None), title, both_bursts());
            assert_eq!(
                plays(&on),
                [true, true],
                "{} with the setting on",
                title.name
            );
        }
    }

    /// Wipeout HD fires the burst itself, so the setting changes nothing.
    #[test]
    fn hd_plays_its_burst_whatever_the_setting() {
        assert!(oag_hd::TITLE.effects.plays_trail_hits_natively());
        for options in [Options::default(), on(None)] {
            let handles = gate(&options, oag_hd::TITLE, both_bursts());
            assert_eq!(plays(&handles), [true, true]);
        }
    }

    /// A title that authors its own burst is never lent another's, whatever the setting.
    #[test]
    fn a_title_with_its_own_burst_ignores_the_setting() {
        for title in [oag_hd::TITLE, oag_2048::TITLE, oag_omega::TITLE] {
            let (mut effects, mut report) = (psys::Library::new(), Vec::new());
            let lent = lend(&on(None), title, &mut effects, &mut report);
            assert!(lent.is_none() && report.is_empty(), "{}", title.name);
        }
    }
}
