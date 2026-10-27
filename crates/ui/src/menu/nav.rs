//! The navigation sounds a menu calls for, and the three row operations that
//! raise them: [`Menu::adjust`], [`Menu::activate`] and [`Menu::back`].
//!
//! The menus do not know a mixer exists. They log a [`Nav`] for each thing the
//! original's front end plays a cue for, and the composition root drains the
//! log with [`Menu::take_nav`] and plays whichever cue the title's front-end
//! bank carries. The four cues and when Pulse plays them are in
//! `docs/ghidra/functions/psp-pulse-usa/menu-sounds.md`.

use super::{Entry, Menu, MenuEvent, Value};

/// The way a cursor moved or a value stepped.
///
/// Pulse plays one cue whichever way it went; HD plays one per direction, so
/// the menus say which and the title's data decides whether it matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    /// Towards the first row.
    Up,
    /// Towards the last row.
    Down,
    /// Towards the previous entry or a lower value.
    Left,
    /// Towards the next entry or a higher value.
    Right,
}

/// One navigation sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    /// The cursor moved to another row or entry (`UPDOWN` on Pulse). `None`
    /// is a screen that does not say which way; the composition root reads
    /// the pad edge it saw.
    Moved(Option<Dir>),
    /// A row's value stepped (`LEFTRIGHT` on Pulse). `None` as for
    /// [`Self::Moved`].
    Stepped(Option<Dir>),
    /// A choice went through: a page opened or an action fired (`ACCEPT`).
    Accept,
    /// A page was left, or a choice was refused (`DECLINE`).
    Decline,
}

/// A menu's undrained [`Nav`]s, oldest first.
#[derive(Debug, Clone, Default)]
pub(super) struct Log(Vec<Nav>);

impl Log {
    pub(super) fn push(&mut self, nav: Nav) {
        self.0.push(nav);
    }

    pub(super) fn when(&mut self, condition: bool, nav: Nav) {
        if condition {
            self.0.push(nav);
        }
    }
}

/// Where a reset moves a row to.
enum Reset {
    /// A choice row's index.
    Choice(usize),
    /// A toggle row's state.
    Toggle(bool),
}

/// The direction a value stepped in.
fn step_dir(step: i32) -> Dir {
    if step < 0 { Dir::Left } else { Dir::Right }
}

impl Menu {
    /// The navigation sounds called for since the last call, oldest first.
    ///
    /// Drained by the caller each tick, after [`Self::update`] and
    /// [`Self::pointer`]; a menu nobody drains just keeps the log, which a
    /// stage that does not play them should not leave unbounded - the
    /// composition root drains it either way.
    #[must_use]
    pub fn take_nav(&mut self) -> Vec<Nav> {
        std::mem::take(&mut self.nav.0)
    }

    /// Moves the selected row's value by `step`, wrapping.
    pub(super) fn adjust(&mut self, step: i32) -> Option<MenuEvent> {
        // A disabled row does not move, and does not report a change it did not
        // make. Checked here rather than in `update` so activating one is inert
        // too - `activate` steps an adjustable row forward.
        if self.selected_is_disabled() {
            return None;
        }
        let page = self.current();
        let row = self.cursor[page];
        let entry = self.definition.pages[page].entries.get_mut(row)?;

        match entry {
            Entry::Choice {
                setting,
                values,
                current,
                ..
            } => {
                let count = values.len() as i32;
                if count == 0 {
                    return None;
                }
                *current = (*current as i32 + step).rem_euclid(count) as usize;
                self.nav.push(Nav::Stepped(Some(step_dir(step))));
                Some(MenuEvent::Changed {
                    setting: setting.clone(),
                    // The stored value, not the label: a settings file holds
                    // `16_Track`, never the words a player read.
                    value: Value::Text(values[*current].value.clone()),
                })
            }
            Entry::Toggle { setting, on, .. } => {
                *on = !*on;
                self.nav.push(Nav::Stepped(Some(step_dir(step))));
                Some(MenuEvent::Changed {
                    setting: setting.clone(),
                    value: Value::Flag(*on),
                })
            }
            _ => None,
        }
    }

    /// Records what `setting` is set to on a fresh install, for [`Self::reset`] to go back to.
    ///
    /// Supplied by whoever owns the settings, next to [`Self::seed`] and from the same
    /// source the settings loader fills a new file from, so the menus keep no table of
    /// their own. A value is matched the way `seed` matches one: text against a choice
    /// row's stored value, a flag against a toggle.
    pub fn set_default(&mut self, setting: &str, value: Value) {
        match self.defaults.iter_mut().find(|(key, _)| key == setting) {
            Some(held) => held.1 = value,
            None => self.defaults.push((setting.to_string(), value)),
        }
    }

    /// The words of the hint drawn under the rows while the selected row is off its default
    /// ("RESET TO DEFAULT"), which is also the pointer's and the finger's target for a
    /// reset. Supplied from the string table; empty draws no hint.
    pub fn set_reset_label(&mut self, label: &str) {
        label.clone_into(&mut self.reset_label);
    }

    /// The reset hint's text for the selected row, when there is one to show: the label
    /// and the keys bound to the reset button, while the row is off its default and no
    /// warning or restart note holds the line under the rows (those win it).
    pub(super) fn reset_hint(
        &self,
        bindings: &dyn Fn(super::Button) -> Vec<&'static str>,
    ) -> Option<String> {
        if self.reset_label.is_empty() || !self.can_reset() || self.line_is_noted() {
            return None;
        }
        let keys = bindings(super::Button::Square).join(" / ");
        Some(if keys.is_empty() {
            self.reset_label.clone()
        } else {
            format!("{} ({keys})", self.reset_label)
        })
    }

    /// Whether a row on screen has a note that takes the line under the rows.
    fn line_is_noted(&self) -> bool {
        let first = self.scroll();
        self.page()
            .entries
            .iter()
            .skip(first)
            .take(self.visible_rows())
            .any(|entry| {
                self.warning(entry).is_some()
                    || self.restart_note(entry).is_some()
                    || self.unavailable(entry).is_some()
            })
    }

    /// Where the selected row's value would go on a reset: `None` for a row that holds no
    /// value, is disabled or pinned, has no recorded default, is already at it, or does
    /// not offer it (a list read off the disc may lack the default).
    fn reset_target(&self) -> Option<Reset> {
        if self.selected_is_disabled() {
            return None;
        }
        let entry = self.page().entries.get(self.cursor[self.current()])?;
        let setting = entry.setting()?;
        let default = &self.defaults.iter().find(|(key, _)| key == setting)?.1;
        match (entry, default) {
            (
                Entry::Choice {
                    values, current, ..
                },
                Value::Text(text),
            ) => {
                let at = values.iter().position(|v| &v.value == text)?;
                (at != *current).then_some(Reset::Choice(at))
            }
            (Entry::Toggle { on, .. }, Value::Flag(flag)) => {
                (flag != on).then_some(Reset::Toggle(*flag))
            }
            _ => None,
        }
    }

    /// Whether [`Self::reset`] would change the selected row. What a reset affordance asks
    /// before it shows itself.
    #[must_use]
    pub fn can_reset(&self) -> bool {
        self.reset_target().is_some()
    }

    /// Puts the selected row back on its default, reporting it as any other edit so the
    /// owner applies and persists it the same way. Nothing on a row that holds no value,
    /// is disabled, or is already there.
    pub(super) fn reset(&mut self) -> Option<MenuEvent> {
        let target = self.reset_target()?;
        let page = self.current();
        let row = self.cursor[page];
        let entry = self.definition.pages[page].entries.get_mut(row)?;
        let event = match (entry, target) {
            (
                Entry::Choice {
                    setting,
                    values,
                    current,
                    ..
                },
                Reset::Choice(at),
            ) => {
                *current = at;
                MenuEvent::Changed {
                    setting: setting.clone(),
                    value: Value::Text(values[at].value.clone()),
                }
            }
            (Entry::Toggle { setting, on, .. }, Reset::Toggle(flag)) => {
                *on = flag;
                MenuEvent::Changed {
                    setting: setting.clone(),
                    value: Value::Flag(flag),
                }
            }
            _ => return None,
        };
        self.nav.push(Nav::Stepped(None));
        Some(event)
    }

    /// Activates the selected row.
    pub(super) fn activate(&mut self) -> Vec<MenuEvent> {
        let page = self.current();
        let row = self.cursor[page];
        let Some(entry) = self.definition.pages[page].entries.get(row) else {
            return Vec::new();
        };

        match entry {
            Entry::Submenu { target, .. } => {
                self.stack.push(*target);
                self.snap_focus();
                self.nav.push(Nav::Accept);
                Vec::new()
            }
            // A row for something not built yet is focusable and refuses the confirm.
            Entry::Run { .. } if self.is_disabled(entry) => {
                self.nav.push(Nav::Decline);
                Vec::new()
            }
            Entry::Run { action, .. } => {
                self.nav.push(Nav::Accept);
                vec![MenuEvent::Fired(*action)]
            }
            Entry::Back { .. } => self.back(),
            // Activating an adjustable row steps it forward, so a player who
            // only ever presses one button can still change everything. Left
            // and right are the discoverable way; this is the forgiving one.
            Entry::Choice { .. } | Entry::Toggle { .. } => {
                let changed = self.adjust(1);
                // A refused confirm is `DECLINE`, not silence.
                self.nav.when(changed.is_none(), Nav::Decline);
                changed.into_iter().collect()
            }
            Entry::Binding { .. } => Vec::new(),
        }
    }

    /// Pops a page, or reports that the root was backed out of.
    ///
    /// Public because escape is not a game button and must not become one:
    /// mapping it onto `circle` would give the menus their back key and give a
    /// race a brake. The window layer therefore calls this directly, out of
    /// band with the tick loop, which is safe precisely because the menus hold
    /// no input state of their own - [`Self::update`] takes edges off a
    /// snapshot and this takes none at all.
    pub fn back(&mut self) -> Vec<MenuEvent> {
        self.nav.push(Nav::Decline);
        if self.stack.len() > 1 {
            self.stack.pop();
            self.snap_focus();
            Vec::new()
        } else {
            vec![MenuEvent::Closed]
        }
    }
}
