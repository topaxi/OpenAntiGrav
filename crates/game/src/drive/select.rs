//! Moving focus to a named row, and a choice row to a named value, by pressing buttons.
//!
//! The rule never sets focus directly: it presses what a player would and reads the state the
//! game reports back, so a launch driven this way goes through the code a pad reaches. Which
//! direction moves focus is read from the screen when it says (a picker does) and found by
//! pressing when it does not, so no list of pages or layouts is kept here.

use oag_gameplay::input::Button;

use super::state::{Row, Screen};

/// What to do next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    /// It is there.
    Done,
    /// Tap this button, then ask again.
    Press(Button),
    /// Not yet, and not an error yet: a list that is filled by a thread (the custom tracks) may
    /// offer the value a frame from now. The caller keeps ticking until its budget is gone and
    /// then reports this reason.
    Wait(String),
}

/// The order a screen that does not say which way is forward is probed in. Down first because
/// most lists run down; Right is last among the forward pair because on a choice row it changes
/// the value, which only a strip layout (rows are all navigation) reaches after Down fails.
const PROBES: [Button; 4] = [Button::Down, Button::Right, Button::Up, Button::Left];

fn named(button: &str) -> Option<Button> {
    oag_core::buttons::button_from_name(button)
}

fn rows_listing(screen: &Screen) -> String {
    let rows: Vec<String> = screen
        .rows
        .iter()
        .map(|row| {
            let name = row
                .id
                .as_deref()
                .or(row.setting.as_deref())
                .or(row.action.as_deref())
                .unwrap_or(&row.label);
            format!("{name:?}")
        })
        .collect();
    format!(
        "{} {:?} has rows [{}]",
        screen.kind,
        screen.id,
        rows.join(", ")
    )
}

/// The row `target` names: a row id, a setting key, an action name, a label (case-insensitive) or `#index`.
fn find<'a>(screen: &'a Screen, target: &str) -> Option<&'a Row> {
    if let Some(index) = target
        .strip_prefix('#')
        .and_then(|n| n.parse::<usize>().ok())
    {
        return screen.rows.get(index);
    }
    let by = |wanted: &dyn Fn(&Row) -> bool| screen.rows.iter().find(|row| wanted(row));
    by(&|row| row.id.as_deref() == Some(target))
        .or_else(|| by(&|row| row.setting.as_deref() == Some(target)))
        .or_else(|| by(&|row| row.action.as_deref() == Some(target)))
        .or_else(|| by(&|row| row.label.eq_ignore_ascii_case(target)))
}

/// Focus to a row.
#[derive(Debug, Clone)]
pub struct Select {
    target: String,
    /// The focus and the button pressed on the last ask, to tell whether it moved.
    last: Option<(usize, Button)>,
    /// The forward button once one moved focus.
    forward: Option<Button>,
    probe: usize,
    presses: usize,
}

impl Select {
    #[must_use]
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            last: None,
            forward: None,
            probe: 0,
            presses: 0,
        }
    }

    /// The row this selects, on `screen`.
    #[must_use]
    pub fn row<'a>(&self, screen: &'a Screen) -> Option<&'a Row> {
        find(screen, &self.target)
    }

    /// What to press given the state after the last press.
    ///
    /// # Errors
    ///
    /// The row is not on the screen, the screen reports no focus, or pressing never moves it
    /// onto the row - each says what the screen does have.
    pub fn next(&mut self, screen: &Screen) -> Result<Next, String> {
        let Some(row) = find(screen, &self.target) else {
            return Err(format!(
                "no row {:?}: {}",
                self.target,
                rows_listing(screen)
            ));
        };
        let Some(focus) = screen.focus.as_ref().map(|f| f.index) else {
            return Err(format!(
                "{} {:?} reports no focus to move",
                screen.kind, screen.id
            ));
        };
        if focus == row.index {
            return Ok(Next::Done);
        }
        if let Some((from, pressed)) = self.last.take() {
            if focus != from {
                self.forward = Some(pressed);
            } else if self.forward.is_none() {
                self.probe += 1;
            }
        }
        let button = match (
            screen.step.and_then(|(forward, _)| named(forward)),
            self.forward,
        ) {
            (Some(button), _) | (None, Some(button)) => button,
            (None, None) => *PROBES.get(self.probe).ok_or_else(|| {
                format!(
                    "pressing {PROBES:?} never moved focus off row {focus} of {}",
                    rows_listing(screen)
                )
            })?,
        };
        self.presses += 1;
        if self.presses > screen.rows.len() + PROBES.len() {
            return Err(format!(
                "{} presses of {button:?} did not bring focus (now row {focus}) to {:?}: {}",
                self.presses,
                self.target,
                rows_listing(screen)
            ));
        }
        self.last = Some((focus, button));
        Ok(Next::Press(button))
    }
}

/// A focused choice row, stepped until it reads a value.
#[derive(Debug, Clone)]
pub struct SetChoice {
    row: String,
    value: String,
    presses: usize,
}

impl SetChoice {
    #[must_use]
    pub fn new(row: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            row: row.into(),
            value: value.into(),
            presses: 0,
        }
    }

    /// What to press given the row as it reads now.
    ///
    /// # Errors
    ///
    /// The row is gone, or stepping never reaches the value. A value the row does not offer
    /// (yet) is [`Next::Wait`].
    pub fn next(&mut self, screen: &Screen) -> Result<Next, String> {
        let Some(row) = find(screen, &self.row) else {
            return Err(format!("no row {:?}: {}", self.row, rows_listing(screen)));
        };
        // A path value (a custom track file) is also named by its file name.
        let reads = |value: &str, label: &str| {
            let wanted = self.value.as_str();
            value.eq_ignore_ascii_case(wanted)
                || label.eq_ignore_ascii_case(wanted)
                || value
                    .to_ascii_lowercase()
                    .ends_with(&format!("/{}", wanted.to_ascii_lowercase()))
        };
        let toggle = row.kind == "toggle";
        if toggle {
            if row
                .value
                .as_deref()
                .is_some_and(|v| v.eq_ignore_ascii_case(&self.value))
            {
                return Ok(Next::Done);
            }
        } else {
            let Some(wanted) = row.values.iter().find(|opt| reads(&opt.value, &opt.label)) else {
                let offered: Vec<String> = row
                    .values
                    .iter()
                    .map(|opt| format!("{}={:?}", opt.value, opt.label))
                    .collect();
                return Ok(Next::Wait(format!(
                    "row {:?} does not offer {:?}; it offers [{}]",
                    self.row,
                    self.value,
                    offered.join(", ")
                )));
            };
            if let Some(why) = &wanted.unavailable {
                return Ok(Next::Wait(format!(
                    "row {:?} offers {:?} but cannot use it yet: {why}",
                    self.row, self.value
                )));
            }
            if row.value.as_deref() == Some(wanted.value.as_str()) {
                return Ok(Next::Done);
            }
        }
        self.presses += 1;
        if self.presses > row.values.len().max(2) + 1 {
            return Err(format!(
                "{} steps right did not set row {:?} to {:?} (it reads {:?})",
                self.presses, self.row, self.value, row.value
            ));
        }
        Ok(Next::Press(Button::Right))
    }
}
