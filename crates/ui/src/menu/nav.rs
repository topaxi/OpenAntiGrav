//! The navigation sounds a menu calls for, and the three row operations that
//! raise them: [`Menu::adjust`], [`Menu::activate`] and [`Menu::back`].
//!
//! The menus do not know a mixer exists. They log a [`Nav`] for each thing the
//! original's front end plays a cue for, and the composition root drains the
//! log with [`Menu::take_nav`] and plays whichever cue the title's front-end
//! bank carries. The four cues and when Pulse plays them are in
//! `docs/ghidra/functions/psp-pulse-usa/menu-sounds.md`.

use super::{Entry, Menu, MenuEvent, Value};

/// One navigation sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    /// The cursor moved to another row or entry (`UPDOWN`).
    UpDown,
    /// A row's value stepped (`LEFTRIGHT`).
    LeftRight,
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

impl Menu {
    /// The navigation sounds called for since the last call, oldest first.
    ///
    /// Drained by the caller each tick, after [`Self::update`] and
    /// [`Self::pointer`]; a menu nobody drains just keeps the log, which a
    /// stage that does not play them should not leave unbounded - the
    /// composition root drains it either way.
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
                self.nav.push(Nav::LeftRight);
                Some(MenuEvent::Changed {
                    setting: setting.clone(),
                    // The stored value, not the label: a settings file holds
                    // `16_Track`, never the words a player read.
                    value: Value::Text(values[*current].value.clone()),
                })
            }
            Entry::Toggle { setting, on, .. } => {
                *on = !*on;
                self.nav.push(Nav::LeftRight);
                Some(MenuEvent::Changed {
                    setting: setting.clone(),
                    value: Value::Flag(*on),
                })
            }
            _ => None,
        }
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
