//! Wipeout 2048's campaign map: `SP.xml`'s events, on the `newFEshell`
//! screen the disc's own `<TouchCampaign>` and `<FE3DCanvas>` live on, each
//! a tap away from `Launch 2048`.
//!
//! # What is authored, and what is not
//!
//! **Authored**: every event and everything said about it - its name, its
//! circuit, its kind, class and lap count, and its `M_X`/`M_Y` map cell -
//! comes off `Data\xml\SP.xml` through `oag_2048::campaign`
//! (`docs/formats/2048-campaign.md`); the screen it sits on, and the
//! `redirect="Launch 2048"` a tap fires, are `NEWGUI/Definition.xml`'s
//! (`docs/formats/2048-frontend.md`). The scrollable area is the shell's own
//! `<TouchScroll>`: a 960x544 view over `MaxScrollX="960"
//! maxscrolly="544"`, so a canvas twice the screen each way.
//!
//! **Chosen, not measured - no confidence score**: how a cell maps to a
//! pixel. `M_X`/`M_Y` are real `GameModeBase` fields the executable does
//! deserialise (`docs/ghidra/functions/vita-2048-eu-v104/
//! frontend-campaign-map.md`'s 2026-09-21 section: struct offsets `0x2c4`/
//! `0x2c8`, found after an earlier pass the same day missed two
//! non-auto-stringified `FUN_812dab08` registrations), but they land in a
//! *different* struct offset from the one the DLC tiers' own hotspot
//! functions read (`+0x15c`/`+0x160`) - some unfound conversion between the
//! two is where the real formula lives, not a bare read of `m_x`/`m_y`
//! itself. A live Vita3K capture the same pass
//! (`data/reference/2048-frontend/README.md`, frames `12`-`14`) confirms the
//! real map is not this grid at all: a **hexagonal** tessellation of
//! icon-on-hexagon markers (chequered flag / stopwatch, coloured by
//! locked/passed state), with a season title card
//! (`A·G·R·C 2048`/`2049`/`2050`) drawn inline on the same scrollable canvas
//! at each season's own cluster - not a `MenuSkin`-style corner widget, and
//! not any of the `<CanvasLabel>`s `2048-frontend.md` already accounts for.
//! Neither the exact per-event anchor formula nor the season-card asset was
//! recovered this pass (`M_BUTTONSHAPE`'s own `CanvasButtonShape` enum is a
//! plausible icon selector, `M_CANVASTWEAK_X`/`_Y` a plausible small pixel
//! nudge, neither confirmed - no consuming function found for either), so
//! this build still lays cells out on an even grid - [`PITCH`] units per
//! cell from [`ORIGIN`], sized so the file's own `x` 1-34 and `y` 1-25 fill
//! the authored 1920x1088 canvas - and draws each event as a plain
//! [`MARKER`]-sized square, coloured by its own progress (see this module's
//! "Progression" section below) rather than a hexagon shape nothing here has
//! a decoded source for. The city behind the real map does not
//! exist as a 3D backdrop either way: the Vita3K capture's own tiles sit on
//! a flat light triangle-outline background, corroborating
//! `2048-frontend.md`'s "not a real 3D scene" finding from the opposite
//! direction. The panel under the map naming the selected event is this
//! build's own chrome, in the skin's own colours.
//!
//! # Progression: locked, open, passed, elite
//!
//! **Resolved 2026-09-21.** [`MapEvent::requires`] is authored data - the
//! single event `oag_2048::campaign::unlock_gates` names as this one's own
//! prerequisite, folding its `M_PNEXTEVENT`/`M_PBRANCHEVENT` chain edge and
//! its `M_PEVENTREQUIRED` field into one name (see that function's own doc
//! for why one name is enough for every case `SP.xml` authors). What tier a
//! finished attempt at an event earned is not authored here at all - a
//! player's own save, read by whatever calls
//! [`Frontend::refresh_campaign_progress`] once at boot (this crate carries
//! no persistence of its own, and must not: nothing gameplay- or
//! session-facing may depend on `oag-ui`, so the medal lookup arrives as a
//! plain closure rather than a `Store` reference). [`ProgressState`] folds
//! the two together: an event whose own [`MapEvent::requires`] has not been
//! passed draws [`ProgressState::Locked`] and refuses a launch
//! ([`Frontend::launch_selected_event`]); everything open from the start, or
//! opened by a since-passed prerequisite, draws [`ProgressState::Open`],
//! [`ProgressState::Passed`] or [`ProgressState::Elite`] off whatever the
//! closure answers for its own name.
//!
//! The four colours this draws with are the disc's own -
//! `Skin.xml`'s `Grey2048`/`Blue2048`/`Pass2048`/`ElitePass2048` globals,
//! already reachable through [`Frontend::global_colour`] - never an invented
//! palette. `HardcorePass2048`, the fifth global that file declares, is not
//! used: nothing in `SP.xml` authors a third objective tier alongside
//! `M_PASSOBJECTIVE`/`M_ELITEOBJECTIVE` (measured - see
//! `oag_2048::campaign::objective_type`'s own doc comment), so that colour
//! most plausibly belongs to a Hardcore *difficulty* flag this pass found no
//! authored data for, not a rung this map ever needs to draw.

use crate::pointer::{Pointer, contains};

use super::touch::{LABEL_SCALE, Launch, PEN_ABOVE_CAPS};
use super::*;
use oag_2048::frontend::states as w2048;

/// The authored scroll range plus the view: the shell's `<TouchScroll>`.
const CANVAS: (f32, f32) = (1920.0, 1088.0);
/// Units per map cell, chosen so the file's cells fill the canvas.
const PITCH: (f32, f32) = (54.0, 42.0);
/// Where cell `(1, 1)` lands. Chosen.
const ORIGIN: (f32, f32) = (24.0, 24.0);
/// A marker's side. Chosen.
const MARKER: f32 = 36.0;
/// The detail panel's rect, in the view. Chosen.
const PANEL: [f32; 4] = [16.0, 448.0, 928.0, 80.0];

/// One campaign event, in the terms the map draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEvent {
    /// The `SP.xml` instance name - what `Launch::Event` carries.
    pub name: String,
    /// `M_X`/`M_Y`, the file's own cell.
    pub x: i32,
    pub y: i32,
    /// What to say about it: the circuit's display name, the kind, the
    /// class and the laps, already resolved to text by the caller.
    pub detail: String,
    /// The single other event whose completion opens this one -
    /// `oag_2048::campaign::unlock_gates`'s own name for it, `None` for an
    /// event open from the start. Authored data, read once at boot; see
    /// this module's own "Progression" section.
    pub requires: Option<String>,
}

/// A finished attempt's own two-tier result - this crate's copy of
/// `oag_2048::campaign::Tier`, kept separate so `oag-ui` never depends on
/// `oag_2048` or `oag_game::records`. See
/// [`Frontend::refresh_campaign_progress`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarnedTier {
    /// Cleared the event's own pass bar.
    Pass,
    /// Cleared the harder elite bar.
    Elite,
}

/// Where an event sits once its own gate and a player's own save have both
/// been folded in - see this module's own "Progression" section.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProgressState {
    /// [`MapEvent::requires`] names an event that has not been passed yet.
    /// Refuses a launch.
    Locked,
    /// Open, not yet finished with at least a pass. The starting state for
    /// an event with no gate, before any progress has been read in at all.
    #[default]
    Open,
    /// Finished with at least [`EarnedTier::Pass`].
    Passed,
    /// Finished with [`EarnedTier::Elite`].
    Elite,
}

/// Where the map is: which event the cursor is on and how far the view has
/// scrolled.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CampaignMap {
    events: Vec<MapEvent>,
    /// [`EarnedTier`] per event, indexed the same as `events` - `None`
    /// until [`Frontend::refresh_campaign_progress`] runs, and for every
    /// event a save has no result for yet. Absence here reads exactly like
    /// "never played", which is correct both before the first refresh and
    /// after it.
    earned: Vec<Option<EarnedTier>>,
    selected: usize,
    scroll: (f32, f32),
}

impl CampaignMap {
    fn marker(event: &MapEvent) -> [f32; 4] {
        [
            ORIGIN.0 + (event.x - 1) as f32 * PITCH.0,
            ORIGIN.1 + (event.y - 1) as f32 * PITCH.1,
            MARKER,
            MARKER,
        ]
    }

    /// Scrolls so the selected marker is as close to the middle of the
    /// view as the canvas allows.
    fn follow(&mut self, view: (f32, f32)) {
        let Some(event) = self.events.get(self.selected) else {
            return;
        };
        let [x, y, w, h] = Self::marker(event);
        self.scroll = (
            (x + w * 0.5 - view.0 * 0.5).clamp(0.0, (CANVAS.0 - view.0).max(0.0)),
            (y + h * 0.5 - view.1 * 0.5).clamp(0.0, (CANVAS.1 - view.1).max(0.0)),
        );
    }

    /// [`ProgressState`] for the event at `index` - `Locked` when
    /// [`MapEvent::requires`] names an event that has not been passed yet
    /// (an unknown name, e.g. one filtered out of the map entirely, reads as
    /// not passed rather than as open by default - see this module's own
    /// "Never invent" rule), the tier `earned` carries otherwise.
    fn state_of(&self, index: usize) -> ProgressState {
        let Some(event) = self.events.get(index) else {
            return ProgressState::Locked;
        };
        let open = event.requires.as_deref().is_none_or(|gate| {
            self.events
                .iter()
                .position(|other| other.name == gate)
                .and_then(|at| self.earned.get(at).copied())
                .flatten()
                .is_some()
        });
        if !open {
            return ProgressState::Locked;
        }
        match self.earned.get(index).copied().flatten() {
            Some(EarnedTier::Elite) => ProgressState::Elite,
            Some(EarnedTier::Pass) => ProgressState::Passed,
            None => ProgressState::Open,
        }
    }
}

impl Frontend {
    /// Gives the map its events, in the order the file lists them. The
    /// cursor starts on the first one named `2048 - Event 1` when there is
    /// one - the first season's first event by name, which is a reading of
    /// the names and not of the unlock graph - and on the first event
    /// otherwise.
    ///
    /// Every event reads [`ProgressState::Open`] or [`ProgressState::Locked`]
    /// (off its own [`MapEvent::requires`] alone, nothing earned yet) until
    /// [`Self::refresh_campaign_progress`] is called - a caller with a save
    /// to read should call it once, straight after this.
    pub fn set_campaign(&mut self, events: Vec<MapEvent>) {
        self.campaign.selected = events
            .iter()
            .position(|event| event.name == "2048 - Event 1")
            .unwrap_or(0);
        self.campaign.earned = vec![None; events.len()];
        self.campaign.events = events;
        self.campaign.scroll = (0.0, 0.0);
        self.campaign.follow(self.space.size);
    }

    /// Folds a player's own save into the map: `earned(name)` is asked once
    /// per event and answers the tier that event's own best-ever result
    /// earned, or `None` for one never finished with at least a pass.
    ///
    /// A plain closure rather than a `Store` reference, on purpose - see
    /// this module's own "Progression" section for why `oag-ui` cannot
    /// depend on `oag_game::records` at all.
    pub fn refresh_campaign_progress(&mut self, earned: impl Fn(&str) -> Option<EarnedTier>) {
        self.campaign.earned = self
            .campaign
            .events
            .iter()
            .map(|event| earned(&event.name))
            .collect();
    }

    /// The events on the map, in the order they were given.
    #[must_use]
    pub fn campaign_events(&self) -> &[MapEvent] {
        &self.campaign.events
    }

    /// The event the cursor is on.
    #[must_use]
    pub fn selected_event(&self) -> Option<&MapEvent> {
        self.campaign.events.get(self.campaign.selected)
    }

    /// Where `name` sits right now - `None` when it names no event on the
    /// map at all. A caller that only wants to check a save's own effect
    /// (a ground-truth test, most plausibly) can read this directly rather
    /// than driving the pad or the pointer to find out.
    #[must_use]
    pub fn campaign_event_state(&self, name: &str) -> Option<ProgressState> {
        let index = self
            .campaign
            .events
            .iter()
            .position(|event| event.name == name)?;
        Some(self.campaign.state_of(index))
    }

    /// The pad on the map: the d-pad moves to the nearest event that way,
    /// cross or start launches the one under the cursor.
    pub(super) fn update_campaign_map(&mut self, input: &mut Input) {
        if self.campaign.events.is_empty() {
            return;
        }
        for (button, direction) in [
            (Button::Right, (1.0, 0.0)),
            (Button::Left, (-1.0, 0.0)),
            (Button::Down, (0.0, 1.0)),
            (Button::Up, (0.0, -1.0)),
        ] {
            if input.is_pressed(button) {
                input.consume_press(button);
                if let Some(next) = self.nearest_event(direction) {
                    self.campaign.selected = next;
                    self.campaign.follow(self.space.size);
                }
                return;
            }
        }
        if input.is_pressed(Button::Cross) || input.is_pressed(Button::Start) {
            input.consume_press(Button::Cross);
            input.consume_press(Button::Start);
            self.launch_selected_event();
        }
    }

    /// The event nearest the cursor in `direction`, by cell distance,
    /// among those that lie at least one cell that way.
    fn nearest_event(&self, direction: (f32, f32)) -> Option<usize> {
        let from = self.campaign.events.get(self.campaign.selected)?;
        self.campaign
            .events
            .iter()
            .enumerate()
            .filter(|(at, _)| *at != self.campaign.selected)
            .filter_map(|(at, event)| {
                let dx = (event.x - from.x) as f32;
                let dy = (event.y - from.y) as f32;
                let along = dx * direction.0 + dy * direction.1;
                if along < 1.0 {
                    return None;
                }
                // Distance, weighted against drifting sideways.
                let across = dx * direction.1 - dy * direction.0;
                Some((at, along + 2.0 * across.abs()))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(at, _)| at)
    }

    fn launch_selected_event(&mut self) {
        let Some(event) = self.campaign.events.get(self.campaign.selected) else {
            return;
        };
        if matches!(
            self.campaign.state_of(self.campaign.selected),
            ProgressState::Locked
        ) {
            self.notes.push(format!(
                "{}: {:?} is locked, refusing to launch",
                w2048::NEW_FE_SHELL,
                event.name
            ));
            return;
        }
        self.notes.push(format!(
            "{}: {:?} tapped, firing {}",
            w2048::NEW_FE_SHELL,
            event.name,
            w2048::LAUNCH_2048
        ));
        self.touch.launch = Some(Launch::Event(event.name.clone()));
        self.machine.fire(w2048::LAUNCH_2048);
    }

    /// The pointer on the map: hovering selects, a click on the selected
    /// event launches it, a click elsewhere selects.
    pub(super) fn campaign_map_pointer(&mut self, pointer: &Pointer) -> bool {
        if pointer.is_idle() {
            return true;
        }
        let Some(at) = pointer.at else {
            return true;
        };
        let at = (at.0 + self.campaign.scroll.0, at.1 + self.campaign.scroll.1);
        let hit = self
            .campaign
            .events
            .iter()
            .position(|event| contains(CampaignMap::marker(event), at));
        // Against the selection *before* this tick's hover moved it: a tap
        // arrives with `moved` and `clicked` set together, and reading the
        // click against the hover it came with would launch on one tap.
        let was = self.campaign.selected;
        if pointer.moved
            && let Some(hit) = hit
        {
            self.campaign.selected = hit;
        }
        if pointer.clicked
            && let Some(hit) = hit
        {
            if hit == was {
                self.launch_selected_event();
            } else {
                self.campaign.selected = hit;
                self.campaign.follow(self.space.size);
            }
        }
        true
    }

    /// The map: every marker in view, the cursor ring, and the panel.
    ///
    /// Each marker's own colour is one of the disc's own four -
    /// [`ProgressState::Locked`] draws `Grey2048`, [`ProgressState::Open`]
    /// the same `Blue2048` this always drew, [`ProgressState::Passed`]
    /// `Pass2048` and [`ProgressState::Elite`] `ElitePass2048` - see this
    /// module's own "Progression" section for why no fifth colour is drawn.
    pub(super) fn draw_campaign_map(&self, out: &mut Vec<Draw>) {
        let (width, height) = self.space.size;
        let open = self.global_colour("Blue2048");
        let cursor = self.global_colour("Orange2048");
        let (sx, sy) = self.campaign.scroll;
        for (at, event) in self.campaign.events.iter().enumerate() {
            let [x, y, w, h] = CampaignMap::marker(event);
            let rect = [x - sx, y - sy, w, h];
            if rect[0] + w < 0.0 || rect[1] + h < 0.0 || rect[0] > width || rect[1] > height {
                continue;
            }
            let color = match self.campaign.state_of(at) {
                ProgressState::Locked => self.global_colour("Grey2048"),
                ProgressState::Open => open,
                ProgressState::Passed => self.global_colour("Pass2048"),
                ProgressState::Elite => self.global_colour("ElitePass2048"),
            };
            out.push(Draw::Fill { rect, color });
            if at == self.campaign.selected {
                self.draw_cursor_ring(rect, cursor, out);
            }
        }
        let Some(event) = self.selected_event() else {
            out.push(Draw::Text {
                x: width * 0.5,
                y: height * 0.5,
                scale: LABEL_SCALE,
                color: open,
                border: None,
                align: Align::Centre,
                text: "no campaign events were loaded".to_string(),
                wrap_width: None,
            });
            return;
        };
        out.push(Draw::Fill {
            rect: PANEL,
            color: open,
        });
        let line = self.default_line_height.unwrap_or(37.0) * LABEL_SCALE;
        let top = PANEL[1] + 12.0 - PEN_ABOVE_CAPS * LABEL_SCALE;
        for (row, text) in [event.name.as_str(), event.detail.as_str()]
            .into_iter()
            .enumerate()
        {
            out.push(Draw::Text {
                x: PANEL[0] + 16.0,
                y: top + row as f32 * (line + 6.0),
                scale: LABEL_SCALE,
                color: [1.0, 1.0, 1.0, 1.0],
                border: None,
                align: Align::Left,
                text: text.to_string(),
                wrap_width: Some(PANEL[2] - 32.0),
            });
        }
    }
}
