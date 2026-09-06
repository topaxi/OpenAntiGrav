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

use oag_game::language::StringTable;
use oag_game::menu::{self, Value};
use oag_game::pilots;
use oag_game::prompt::{self, Outcome};

use crate::menu_stage::MenuStage;
use crate::overlay::{Finished, Prompt, Purpose};
use crate::stage::Stage;

use super::Session;

/// One project-owned string, or the English written beside its id.
///
/// **The first caller of `assets/ui/strings/english.toml`'s second kind of
/// entry** - an id for text this project invented rather than an override of
/// a disc idstring. Every label the pilot prompts draw goes through here, so
/// the prompt models themselves hold no table and no English: see
/// `oag_game::prompt`'s own module doc on that seam.
///
/// The fallback is not politeness. A language with no file yet overlays
/// nothing (`crate::strings::built_in` ships English alone today), so an id
/// that resolves to nothing has to read as English rather than as a blank
/// line - the same rule `menu::definition::resolve` applies to a row's own
/// `string_id`.
///
/// `Option`, because the table lives on [`super::Shell`] and a `--race` run
/// has no shell at all. Nothing can open one of these prompts without menus,
/// so `None` is unreachable from the pilot editor - but it is the honest
/// signature, and it costs one `and_then`.
fn say(strings: Option<&StringTable>, id: &str, english: &str) -> String {
    strings
        .and_then(|table| table.get(id))
        .unwrap_or(english)
        .to_string()
}

/// [`say`] with the pilot's name substituted for every `%s`.
///
/// `%s` rather than Rust's own `{}`: this is a *string table* entry, which a
/// translator edits, and the disc's own tables already spell a substitution
/// that way. `{}` would also make an accidental brace in a translation a
/// formatting error rather than a brace.
fn say_of(strings: Option<&StringTable>, id: &str, english: &str, name: &str) -> String {
    say(strings, id, english).replace("%s", name)
}

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

    /// Opens the on-screen keyboard on PILOT's current name.
    ///
    /// **Refused for a pilot with no file of its own.** An untouched built-in
    /// lives in the binary and there is nothing on disk to move; the way to
    /// get one a file is to save an edit to it, which the same page already
    /// does. Gated on [`pilots::Entry::from_file`] rather than on the name
    /// being one of the four, because the moment that save happens a file
    /// exists and renaming *it* is perfectly legitimate - a name-based gate
    /// would go on refusing for ever.
    pub(crate) fn rename_pilot(&mut self) {
        let Some(name) = self.selected_pilot() else {
            return;
        };
        if !self.pilot_has_a_file(&name) {
            error!(
                "{name} has no file of its own to rename - save an edit to it first, \
                 which is what creates one"
            );
            return;
        }
        let labels = prompt::Labels {
            title: say(self.table(), "OAG_PILOT_RENAME_TITLE", "RENAME PILOT"),
            delete: say(self.table(), "OAG_KEYBOARD_DELETE", "DEL"),
            accept: say(self.table(), "OAG_KEYBOARD_ACCEPT", "OK"),
            hint: say(
                self.table(),
                "OAG_KEYBOARD_HINT",
                "CROSS TYPE   SQUARE DELETE   START ACCEPT   CIRCLE CANCEL",
            ),
        };
        let keyboard = prompt::Keyboard::new(labels, &name, pilots::MAX_NAME);
        if let Stage::Menu(stage) = &mut self.stage {
            stage.prompt = Some(Prompt::typing(
                Purpose::RenamePilot { from: name },
                keyboard,
            ));
        }
    }

    /// Asks whether to delete PILOT's file, and says what deleting it will
    /// actually do.
    ///
    /// **The wording is the feature here.** For a file named after one of the
    /// four built-ins, deleting it does not remove a pilot: the file was
    /// *replacing* the built-in, so removing it restores the built-in and the
    /// pilot goes on existing with the numbers the binary ships. A player who
    /// deletes `aggressive` and finds it still on the list, unexplained, has
    /// been misled by a menu - see `pilots::is_built_in_name`.
    pub(crate) fn delete_pilot(&mut self) {
        let Some(name) = self.selected_pilot() else {
            return;
        };
        if !self.pilot_has_a_file(&name) {
            error!("{name} has no file of its own to delete - it is the built-in");
            return;
        }
        let message = if pilots::is_built_in_name(&name) {
            say_of(
                self.table(),
                "OAG_PILOT_DELETE_BUILT_IN",
                "%s IS BUILT IN AND YOUR FILE REPLACES IT. \
                 DELETING THE FILE RESTORES THE BUILT-IN INSTEAD OF REMOVING %s.",
                &name,
            )
        } else {
            say_of(
                self.table(),
                "OAG_PILOT_DELETE_ASK",
                "DELETE %s? THIS CANNOT BE UNDONE.",
                &name,
            )
        };
        let confirm = prompt::Confirm::new(prompt::ConfirmLabels {
            title: say(self.table(), "OAG_PILOT_DELETE_TITLE", "DELETE PILOT"),
            message,
            yes: say(self.table(), "OAG_PILOT_DELETE_YES", "DELETE"),
            no: say(self.table(), "OAG_PILOT_DELETE_NO", "KEEP"),
        });
        if let Stage::Menu(stage) = &mut self.stage {
            stage.prompt = Some(Prompt::asking(Purpose::DeletePilot { name }, confirm));
        }
    }

    /// The chosen language's string table, or `None` on a `--race` run with
    /// no shell. See [`say`].
    fn table(&self) -> Option<&StringTable> {
        self.shell.as_ref().map(|shell| &shell.strings)
    }

    /// Whatever PILOT currently holds, or `None` off the AI PILOTS page.
    fn selected_pilot(&self) -> Option<String> {
        let Stage::Menu(stage) = &self.stage else {
            return None;
        };
        held_text(&stage.menu, "pilot.selected")
    }

    /// Whether `name` has a file on disk, as opposed to being a built-in
    /// nobody has retuned. The gate both operations above need.
    fn pilot_has_a_file(&self, name: &str) -> bool {
        self.pilot_roster
            .entries()
            .iter()
            .any(|entry| entry.name == name && entry.from_file)
    }

    /// Does what a finished prompt asked for.
    ///
    /// Takes [`Finished`] by value rather than reading the model back off the
    /// stage: the model is gone by now, which is what lets this run with
    /// `&mut self` and no borrow of the stage still live.
    pub(crate) fn finish_prompt(&mut self, finished: Finished) {
        let Some(dir) = pilots::directory() else {
            error!("no config directory on this platform, so nothing can be written");
            return;
        };
        match finished.purpose {
            Purpose::RenamePilot { from } => {
                let to = finished.text;
                if to == from {
                    return;
                }
                if let Err(e) = pilots::rename_pilot(&dir, &from, &to) {
                    // Left on screen the way `save_pilot`'s own failure is:
                    // a rename that did not happen is not worth losing the
                    // player's place in the menus over, and the list below
                    // still shows the old name, which is the truth.
                    error!("cannot rename {from}: {e:#}");
                    return;
                }
                info!("renamed {from} to {to}");
                // `Some(&to)`, for `new_pilot`'s reason: `from` is gone from
                // the reloaded list, so without this PILOT would fall to
                // whichever entry happened to be first.
                self.reload_pilot_roster(Some(&to));
            }
            Purpose::DeletePilot { name } => {
                if let Err(e) = pilots::delete_pilot(&dir, &name) {
                    error!("cannot delete {name}: {e:#}");
                    return;
                }
                if pilots::is_built_in_name(&name) {
                    info!("deleted the {name} file; the built-in {name} is back");
                } else {
                    info!("deleted {name}");
                }
                // **No `select`, and the two cases differ.** A built-in name
                // is still in the reloaded list - as the built-in - so
                // `Menu::supply`'s own "keep the value it was already on"
                // rule leaves PILOT exactly where it is, which is right: the
                // pilot the player was looking at is still there. Any other
                // name is gone from the list and the same rule drops PILOT to
                // the first entry, which is also right - there is nothing
                // better to land on than the top.
                self.reload_pilot_roster(None);
            }
        }
    }

    /// Whether an on-screen **keyboard** is open - not a confirm, which has
    /// nothing to type into.
    ///
    /// Read by `app.rs` before it decides whether a raw key event is text or
    /// a game button. See [`crate::typing`].
    pub(crate) fn typing_is_open(&self) -> bool {
        let Stage::Menu(stage) = &self.stage else {
            return false;
        };
        stage
            .prompt
            .as_ref()
            .is_some_and(|prompt| matches!(prompt.model, crate::overlay::Model::Keyboard(_)))
    }

    /// Applies one decision off a desk keyboard, and says whether it was
    /// used.
    ///
    /// `false` means the event is still the abstract input layer's - see the
    /// call site in `app.rs` for why that distinction is what keeps the arrow
    /// keys navigating the grid.
    pub(crate) fn typed(&mut self, typed: crate::typing::Typed) -> bool {
        let Stage::Menu(stage) = &mut self.stage else {
            return false;
        };
        let Some(prompt) = stage.prompt.as_mut() else {
            return false;
        };
        let Some(keyboard) = prompt.keyboard_mut() else {
            // A confirm has no buffer, so a typed key is not its business.
            return false;
        };
        match typed {
            crate::typing::Typed::Edit(edit) => {
                keyboard.edit(edit);
                true
            }
            crate::typing::Typed::Accept => {
                let text = keyboard.text().to_string();
                let prompt = stage.prompt.take().expect("checked just above");
                self.finish_prompt(Finished {
                    purpose: prompt.purpose,
                    text,
                });
                true
            }
            crate::typing::Typed::Ignore => false,
        }
    }

    /// One tick of whatever prompt is open, plus the live note under it.
    ///
    /// An associated function taking the stage rather than a method on
    /// `&mut Session`, for [`resupply_bounds`]' reason one step further out:
    /// the caller in [`super::frame`] is already inside a `match &mut
    /// self.stage`, so the stage arrives borrowed and `self` cannot be.
    ///
    /// Returns what to act on once that borrow is over - see
    /// [`Session::finish_prompt`].
    pub(crate) fn tick_prompt(
        stage: &mut MenuStage,
        input: &mut oag_game::input::Input,
        roster: &pilots::Roster,
        strings: Option<&StringTable>,
    ) -> Option<Finished> {
        let prompt = stage.prompt.as_mut()?;
        // The note is set from out here every tick because whether there is
        // anything to say depends on what the text *means*, which is exactly
        // what `prompt::Keyboard` does not know.
        if let Purpose::RenamePilot { from } = &prompt.purpose {
            let from = from.clone();
            if let Some(keyboard) = prompt.keyboard_mut() {
                let note = rename_note(keyboard.text(), &from, roster, strings);
                keyboard.set_note(note);
            }
        }
        match prompt.update(input) {
            Outcome::Pending => None,
            Outcome::Cancelled => {
                stage.prompt = None;
                None
            }
            Outcome::Accepted => {
                let mut prompt = stage.prompt.take().expect("checked just above");
                let text = prompt
                    .keyboard_mut()
                    .map(|keyboard| keyboard.text().to_string())
                    .unwrap_or_default();
                Some(Finished {
                    purpose: prompt.purpose,
                    text,
                })
            }
        }
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

/// What to say under the keyboard about the name currently typed, if
/// anything.
///
/// **Live, on every keystroke, rather than only on accept.** Every one of
/// these is a reason `pilots::rename_pilot` would refuse or would surprise,
/// and finding that out after typing a name on a grid is the difference
/// between a keyboard that is merely correct and one that is bearable.
///
/// The order matters: a name that is *taken* is refused outright, so that
/// sentence wins over the built-in one even for the four built-in names -
/// which are only "taken" once a file of that name exists. A built-in with
/// no file yet is a legitimate destination and the note there says what will
/// happen rather than refusing it.
fn rename_note(
    typed: &str,
    from: &str,
    roster: &pilots::Roster,
    strings: Option<&StringTable>,
) -> Option<String> {
    if typed == from {
        return None;
    }
    if let Err(why) = pilots::check_name(typed) {
        // The writer's own sentence, which already names what is wrong.
        // Uppercased to sit with the rest of the menu rather than reworded
        // here, which would be a second place for the rule to drift.
        return Some(format!("{why}").to_uppercase());
    }
    if roster
        .entries()
        .iter()
        .any(|entry| entry.name == typed && entry.from_file)
    {
        return Some(say_of(
            strings,
            "OAG_PILOT_NAME_TAKEN",
            "THERE IS ALREADY A PILOT CALLED %s",
            typed,
        ));
    }
    if pilots::is_built_in_name(typed) {
        return Some(say_of(
            strings,
            "OAG_PILOT_RENAME_SHADOWS",
            "%s IS BUILT IN: A FILE OF THAT NAME REPLACES IT",
            typed,
        ));
    }
    None
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
#[path = "pilot_editor/tests.rs"]
mod tests;
