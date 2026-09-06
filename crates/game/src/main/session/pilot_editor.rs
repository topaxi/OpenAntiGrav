//! The AI PILOTS page's own wiring: PILOT/AXIS/LOW/HIGH and SAVE/NEW FROM
//! PILOT.
//!
//! Split into its own file rather than folded into [`super::menus`], which is
//! close enough to this project's 1,000-line ceiling
//! (`scripts/check-file-size.py`) that a feature this size landing there
//! would have pushed it over. The seam is the same one [`super::remix_menu`]
//! already draws: pure per-feature logic here, called from one line in
//! [`super::menus::Session::open_menus`] and a couple of match arms in
//! [`super::apply`]/[`super::menus`].
//!
//! **All four rows are read live off the menu itself, never cached on
//! [`Session`].** `Entry::chosen` already answers "what is this row
//! currently set to" - see `oag_game::menu::Entry::chosen` - so there is no
//! second copy of "the pending edit" to keep in step with the rows a player
//! is looking at.
//!
//! **A consequence of that: SAVE writes the one axis AXIS currently names,
//! and moving AXIS before pressing it discards an edit to the axis just left,
//! silently.** [`Session::resupply_pilot_bounds`] re-derives LOW/HIGH from
//! [`Session::pilot_roster`]'s own stored span the moment AXIS (or PILOT)
//! moves, because that is the only value there is to derive them from - there
//! is nowhere an in-progress, not-yet-saved edit to a *different* axis could
//! live without becoming the second copy the paragraph above deliberately
//! avoids. One axis at a time is a defensible first cut; the silence is
//! pinned as chosen rather than accidental by
//! `moving_axis_discards_an_untouched_low_edit_without_saving` below, not
//! merely left undocumented.

use log::{error, info};

use oag_game::menu::{self, Value};
use oag_game::pilots;

use crate::stage::Stage;

use super::Session;

/// How many steps [`axis_choices`] spreads across an axis's own range,
/// before the pilot's actual value is folded in.
///
/// Coarse enough that stepping across the whole range takes a few seconds of
/// button presses, not a few hundred - this is a pad-driven list, not a
/// slider a mouse can drag.
const STEPS: usize = 40;

/// The row that edits `setting`'s currently chosen text, if any row does and
/// it is holding text.
///
/// Every pilot-editor row is a `choice`, never a `toggle`, so [`Value::Flag`]
/// never applies here - the same way [`menu::Menu::held`] this mirrors is used
/// elsewhere in this crate for a text-valued setting.
fn held_text(model: &menu::Menu, setting: &str) -> Option<String> {
    model
        .definition()
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .find(|entry| entry.setting() == Some(setting))
        .and_then(menu::Entry::chosen)
        .and_then(|value| match value {
            Value::Text(text) => Some(text),
            Value::Flag(_) => None,
        })
}

/// Rounds a generated step to a sensible number of decimal places for its own
/// magnitude, so spreading `STEPS` evenly across a range never surfaces as
/// float noise in a saved file.
///
/// **Never applied to a pilot's own current value** - only to the round
/// steps [`axis_choices`] manufactures around it. `min + span * i / STEPS` is
/// three `f32` operations, and landing one ULP off a tidy decimal turns
/// `0.1725` into `0.17250001`; that string is both what the row shows and
/// what a SAVE with nothing else touched writes to the file, which is
/// exactly the tidiness this feature exists to protect.
fn quantize(value: f32) -> f32 {
    let decimals = if value.abs() >= 100.0 {
        1
    } else if value.abs() >= 10.0 {
        2
    } else {
        4
    };
    let scale = 10f32.powi(decimals);
    (value * scale).round() / scale
}

/// A discretized, sorted, deduplicated set of numbers `LOW`/`HIGH` may be set
/// to: [`STEPS`] steps evenly spread across `[min, max]`, plus `current`
/// itself.
///
/// **`current` is always in the list, and never [`quantize`]d.** This is
/// what lets [`menu::Menu::seed`] land the row on the exact value the file
/// already held - a hand-authored `0.427` is not one of the round steps, and
/// losing it the moment the row is drawn would mean SAVE without touching
/// LOW at all still moved the number.
fn axis_choices(min: f32, max: f32, current: f32) -> Vec<menu::Choice> {
    let span = (max - min).max(f32::EPSILON);
    let mut values: Vec<f32> = (0..=STEPS)
        .map(|i| quantize(min + span * (i as f32) / (STEPS as f32)))
        .collect();
    values.push(current.clamp(min, max));
    values.sort_by(f32::total_cmp);
    values.dedup_by(|a, b| (*a - *b).abs() < span * 1e-4);
    values
        .into_iter()
        .map(|value| menu::Choice::plain(format!("{value}")))
        .collect()
}

/// One label for the `PILOT` row: the name, with a mark for a built-in - see
/// [`oag_game::pilots::Entry::from_file`].
///
/// Plain text baked into the row's own list rather than a second `Value`
/// variant: a built-in is a fact about the *pilot*, not a boolean setting a
/// `toggle` row's [`Value::Flag`] could carry.
fn pilot_choice(entry: &pilots::Entry) -> menu::Choice {
    let label = if entry.from_file {
        entry.name.clone()
    } else {
        format!("{} (built-in)", entry.name)
    };
    menu::Choice::labelled(&entry.name, label)
}

impl Session {
    /// Populates the PILOT and AXIS rows from [`Session::pilot_roster`], and
    /// LOW/HIGH for whichever one they land on - called once, when the AI
    /// PILOTS page's own [`menu::Menu`] is first built.
    ///
    /// Mirrors [`super::menus::Session::resupply_tracks_for_mode`]'s own
    /// shape: supplied before anything is seeded, because a value cannot be
    /// seeded onto a list that is not there yet, and there is nothing to seed
    /// PILOT or AXIS *from* - unlike a settings-backed row, which pilot and
    /// which axis are on screen is not persisted, so both simply open on
    /// whichever their list's first entry is.
    pub(crate) fn supply_pilot_menu(&mut self, model: &mut menu::Menu) {
        let pilots: Vec<menu::Choice> = self
            .pilot_roster
            .entries()
            .iter()
            .map(pilot_choice)
            .collect();
        model.supply(menu::ValueSource::Pilots, &pilots);
        let axes: Vec<menu::Choice> = pilots::AXES
            .iter()
            .map(|(name, _)| menu::Choice::plain(*name))
            .collect();
        model.supply(menu::ValueSource::PilotAxes, &axes);
        resupply_bounds(model, &self.pilot_roster);
    }

    /// Re-supplies LOW/HIGH from whichever pilot and axis PILOT/AXIS
    /// currently hold - called whenever either one moves.
    ///
    /// A no-op while the menus are not the stage on screen, which cannot
    /// happen through the row that fires it but is cheap to make true
    /// unconditionally - the same guard [`super::menus::Session::back`]'s
    /// siblings use throughout this crate.
    pub(crate) fn resupply_pilot_bounds(&mut self) {
        let roster = &self.pilot_roster;
        if let Stage::Menu(stage) = &mut self.stage {
            resupply_bounds(&mut stage.menu, roster);
        }
    }

    /// Writes PILOT's file with AXIS set to `[LOW, HIGH]`, through
    /// [`pilots::set_axis`] - so every other axis, and every comment a player
    /// wrote by hand, survives.
    ///
    /// **A pilot with no file yet - any untouched built-in - starts from
    /// [`pilots::template`] of its own resolved numbers, not an empty
    /// document.** Writing only the edited axis there would leave every
    /// other axis silently pulled from `balanced` the next time this pilot
    /// loads - see [`pilots::template`]'s own doc.
    ///
    /// Failure is logged and left on screen, the same way
    /// `settings::save`'s own failure is throughout this crate
    /// (`Session::apply_setting`) - there is no toast in this menu layer, and
    /// a write that did not happen is not worth losing the player's place in
    /// the menus over.
    pub(crate) fn save_pilot(&mut self) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let model = &stage.menu;
        let (Some(name), Some(axis), Some(low), Some(high)) = (
            held_text(model, "pilot.selected"),
            held_text(model, "pilot.axis"),
            held_text(model, "pilot.low").and_then(|text| text.parse::<f32>().ok()),
            held_text(model, "pilot.high").and_then(|text| text.parse::<f32>().ok()),
        ) else {
            error!("cannot save: the pilot editor's own rows are not all holding a value");
            return;
        };

        let Some(dir) = pilots::directory() else {
            error!("cannot save {name}: no config directory on this platform");
            return;
        };
        let text = match pilots::read_pilot_text(&dir, &name) {
            Ok(text) => text,
            Err(_) => {
                let base = self
                    .pilot_roster
                    .entries()
                    .iter()
                    .find(|entry| entry.name == name)
                    .map_or(oag_ai::Pilot::BALANCED, |entry| entry.pilot);
                pilots::template(&base)
            }
        };
        let edited = match pilots::set_axis(&text, &axis, low, high) {
            Ok(text) => text,
            Err(e) => {
                error!("cannot save {name}: {e:#}");
                return;
            }
        };
        if let Err(e) = pilots::write_pilot(&dir, &name, &edited) {
            error!("cannot save {name}: {e:#}");
            return;
        }
        info!("saved {name}");
        // No `select`: PILOT is already on `name`, and `Menu::supply`'s own
        // "keep the value it was already on" rule leaves it there. What
        // does change is the row's label, if this was a built-in's first
        // save - `pilot_choice` reads the reloaded roster's `from_file`,
        // which just flipped.
        self.reload_pilot_roster(None);
    }

    /// Creates a new pilot file from whichever one PILOT currently holds,
    /// auto-named `pilot-1`, `pilot-2`, ... - the create-from-template flow
    /// that keeps this cut free of text entry.
    pub(crate) fn new_pilot(&mut self) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(source_name) = held_text(&stage.menu, "pilot.selected") else {
            return;
        };
        let Some(dir) = pilots::directory() else {
            error!("cannot create a new pilot: no config directory on this platform");
            return;
        };
        let base = self
            .pilot_roster
            .entries()
            .iter()
            .find(|entry| entry.name == source_name)
            .map_or(oag_ai::Pilot::BALANCED, |entry| entry.pilot);

        let taken: std::collections::BTreeSet<&str> = self
            .pilot_roster
            .entries()
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        let mut n = 1u32;
        let name = loop {
            let candidate = format!("pilot-{n}");
            if !taken.contains(candidate.as_str()) {
                break candidate;
            }
            n += 1;
        };

        if let Err(e) = pilots::write_pilot(&dir, &name, &pilots::template(&base)) {
            error!("cannot create {name}: {e:#}");
            return;
        }
        info!("created {name} from {source_name}");
        // `Some(&name)`, unlike `save_pilot`: PILOT was on `source_name`,
        // which is still in the reloaded list, so without this the row
        // would stay put and the only sign a new pilot exists at all would
        // be the console line above.
        self.reload_pilot_roster(Some(&name));
    }

    /// Re-reads [`Session::pilot_roster`] off disk and re-supplies the AI
    /// PILOTS page from it, so a save or a create-from-template shows up on
    /// screen without the player having to leave and reopen the page.
    ///
    /// `select`, when given, moves the PILOT row onto that name - see
    /// [`supply_pilot_choices`] for why [`Session::new_pilot`] needs this and
    /// [`Session::save_pilot`] does not.
    fn reload_pilot_roster(&mut self, select: Option<&str>) {
        match pilots::load() {
            Ok(roster) => self.pilot_roster = roster,
            Err(e) => error!("could not reload pilots after saving: {e:#}"),
        }
        let roster = self.pilot_roster.clone();
        if let Stage::Menu(stage) = &mut self.stage {
            supply_pilot_choices(&mut stage.menu, &roster, select);
        }
    }
}

/// The PILOT supply [`Session::supply_pilot_menu`] does, without needing
/// `&mut Session` - shared with [`Session::reload_pilot_roster`], which
/// already holds a disjoint borrow of `self.stage` by the time it needs this.
///
/// `select`, when given, moves PILOT onto that name **after** the list is
/// supplied and **before** LOW/HIGH are resupplied - `create-from-template`
/// is why this exists: without it, `Menu::supply`'s own "keep the value it
/// was already on" rule leaves PILOT exactly where it was, so a fresh
/// `pilot-1` exists on disk and in the list with nothing on screen saying
/// so.
fn supply_pilot_choices(model: &mut menu::Menu, roster: &pilots::Roster, select: Option<&str>) {
    let pilots: Vec<menu::Choice> = roster.entries().iter().map(pilot_choice).collect();
    model.supply(menu::ValueSource::Pilots, &pilots);
    if let Some(name) = select {
        model.seed("pilot.selected", &Value::Text(name.to_string()));
    }
    resupply_bounds(model, roster);
}

/// The shared half of [`Session::supply_pilot_menu`] and
/// [`Session::resupply_pilot_bounds`]: reads whichever pilot and axis PILOT
/// and AXIS currently hold and re-supplies LOW/HIGH for that one.
///
/// A free function, not a method: it needs `&pilots::Roster` and
/// `&mut menu::Menu` at once, which are two different fields of [`Session`] -
/// passing both in directly is simpler than splitting the borrow inside a
/// method that only ever reaches one field through `&mut self`.
fn resupply_bounds(model: &mut menu::Menu, roster: &pilots::Roster) {
    let Some(pilot_name) = held_text(model, "pilot.selected") else {
        return;
    };
    let Some(axis) = held_text(model, "pilot.axis") else {
        return;
    };
    let Some(entry) = roster
        .entries()
        .iter()
        .find(|entry| entry.name == pilot_name)
    else {
        return;
    };
    let Some(accessor) = pilots::axis_accessor(&axis) else {
        return;
    };
    let span = accessor(&entry.pilot);
    let (min, max) = pilots::limit(&axis);

    model.supply(
        menu::ValueSource::PilotAxisLow,
        &axis_choices(min, max, span.low),
    );
    model.supply(
        menu::ValueSource::PilotAxisHigh,
        &axis_choices(min, max, span.high),
    );
    model.seed("pilot.low", &Value::Text(format!("{}", span.low)));
    model.seed("pilot.high", &Value::Text(format!("{}", span.high)));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A menu with just the four rows this file cares about, so a test can
    /// exercise the free functions above without a real window, GPU or disc.
    fn fixture_menu() -> menu::Menu {
        let text = "\
version = 1
root = \"pilots\"
[[page]]
id = \"pilots\"
[[page.entry]]
kind = \"choice\"
label = \"PILOT\"
setting = \"pilot.selected\"
values_from = \"pilots\"
[[page.entry]]
kind = \"choice\"
label = \"AXIS\"
setting = \"pilot.axis\"
values_from = \"pilot_axes\"
[[page.entry]]
kind = \"choice\"
label = \"LOW\"
setting = \"pilot.low\"
values_from = \"pilot_axis_low\"
[[page.entry]]
kind = \"choice\"
label = \"HIGH\"
setting = \"pilot.high\"
values_from = \"pilot_axis_high\"
";
        let strings = oag_game::language::StringTable::default();
        let definition = menu::Definition::parse(text, &strings).expect("a tiny valid definition");
        menu::Menu::new(definition)
    }

    /// A directory holding one pilot file, cleaned up on drop - the same
    /// idiom `pilots`' own tests use, kept local rather than shared so this
    /// file does not reach into `oag_game::pilots`' private test helpers.
    struct Scratch(std::path::PathBuf);

    impl Scratch {
        fn with(name: &str, body: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "oag-pilot-editor-{}-{:p}",
                std::process::id(),
                name.as_ptr()
            ));
            std::fs::create_dir_all(&dir).expect("a scratch directory");
            std::fs::write(dir.join(format!("{name}.toml")), body).expect("a scratch pilot");
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Lands PILOT on `name` and AXIS on `axis`, the way opening the real
    /// page does through [`Session::supply_pilot_menu`] - reproduced here
    /// with the two free functions directly, since there is no [`Session`]
    /// in this test.
    fn land_on(model: &mut menu::Menu, roster: &pilots::Roster, name: &str, axis: &str) {
        supply_pilot_choices(model, roster, Some(name));
        model.supply(menu::ValueSource::PilotAxes, &[menu::Choice::plain(axis)]);
        model.seed("pilot.axis", &Value::Text(axis.to_string()));
        resupply_bounds(model, roster);
    }

    /// The generated steps must never carry float noise into a saved file -
    /// see [`quantize`]'s own doc for the `0.17250001` this exists to catch.
    #[test]
    fn quantized_steps_never_print_more_than_a_handful_of_decimals() {
        for choice in axis_choices(0.1, 3.0, 1.7) {
            assert!(
                choice.value.len() <= 7,
                "{:?} looks like float noise, not a step",
                choice.value
            );
        }
    }

    /// The exact current value is always offered, even when it is not one
    /// of the round steps - a hand-authored `0.427` must still seed exactly.
    #[test]
    fn the_pilots_own_exact_value_is_always_one_of_the_choices() {
        let choices = axis_choices(0.1, 3.0, 0.427);
        assert!(choices.iter().any(|choice| choice.value == "0.427"));
    }

    /// SAVE writes AXIS's own axis alone. Pinned here: an edit to a
    /// *different* axis, left untouched when AXIS moves on to another one,
    /// is discarded without a word - see this file's own module doc.
    #[test]
    fn moving_axis_discards_an_untouched_low_edit_without_saving() {
        let scratch = Scratch::with("winston", "commitment = [0.90, 1.00]\n");
        let roster = pilots::load_from(&scratch.0).expect("a readable directory");
        let mut model = fixture_menu();
        land_on(&mut model, &roster, "winston", "commitment");
        assert_eq!(held_text(&model, "pilot.low"), Some("0.9".to_string()));

        // The player nudges LOW - "0.5" is `commitment`'s own floor, always
        // one of `axis_choices`' round steps, so it is on the list without
        // depending on how the steps are spread.
        model.seed("pilot.low", &Value::Text("0.5".to_string()));
        assert_eq!(
            held_text(&model, "pilot.low"),
            Some("0.5".to_string()),
            "the seed above did not land - this test is not exercising what it claims to"
        );

        // AXIS moves - to the same axis is enough, since `resupply_bounds`
        // does not know or care that it "moved"; it only re-derives from the
        // roster's own stored span, which is the whole of the behaviour
        // being pinned.
        land_on(&mut model, &roster, "winston", "commitment");
        assert_eq!(
            held_text(&model, "pilot.low"),
            Some("0.9".to_string()),
            "the untouched-file value should have won back over the discarded edit"
        );
    }
}
