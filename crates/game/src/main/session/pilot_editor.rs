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

/// A discretized, sorted, deduplicated set of numbers `LOW`/`HIGH` may be set
/// to: [`STEPS`] steps evenly spread across `[min, max]`, plus `current`
/// itself.
///
/// **`current` is always in the list**, which is what lets
/// [`menu::Menu::seed`] land the row on the exact value the file already
/// held - a hand-authored `0.427` is not one of the round steps, and losing
/// it the moment the row is drawn would mean SAVE without touching LOW at
/// all still moved the number.
fn axis_choices(min: f32, max: f32, current: f32) -> Vec<menu::Choice> {
    let span = (max - min).max(f32::EPSILON);
    let mut values: Vec<f32> = (0..=STEPS)
        .map(|i| min + span * (i as f32) / (STEPS as f32))
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
        self.reload_pilot_roster();
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
        self.reload_pilot_roster();
    }

    /// Re-reads [`Session::pilot_roster`] off disk and re-supplies the AI
    /// PILOTS page from it, so a save or a create-from-template shows up on
    /// screen without the player having to leave and reopen the page.
    fn reload_pilot_roster(&mut self) {
        match pilots::load() {
            Ok(roster) => self.pilot_roster = roster,
            Err(e) => error!("could not reload pilots after saving: {e:#}"),
        }
        let roster = self.pilot_roster.clone();
        if let Stage::Menu(stage) = &mut self.stage {
            supply_pilot_choices(&mut stage.menu, &roster);
        }
    }
}

/// The PILOT/AXIS supply [`Session::supply_pilot_menu`] does, without needing
/// `&mut Session` - shared with [`Session::reload_pilot_roster`], which
/// already holds a disjoint borrow of `self.stage` by the time it needs this.
fn supply_pilot_choices(model: &mut menu::Menu, roster: &pilots::Roster) {
    let pilots: Vec<menu::Choice> = roster.entries().iter().map(pilot_choice).collect();
    model.supply(menu::ValueSource::Pilots, &pilots);
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
