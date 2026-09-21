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
//! pixel. `M_X`/`M_Y` looked like the obvious source, but
//! `docs/ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md`'s
//! 2026-09-21 section found they most likely never reach the shipped
//! executable at all - the runtime field-reflection table for the event
//! class registers `M_BUTTONSHAPE`/`M_CANVASTWEAK_X`/`M_CANVASTWEAK_Y` at
//! real struct offsets but never `M_X`/`M_Y`, and those two carry a
//! `parentid` no confirmed-runtime field on any typedef in the file uses -
//! the best-fitting reading is that they are the Mjolnir level-design tool's
//! own node-graph canvas position, not a game-read value. A live Vita3K
//! capture the same pass (`data/reference/2048-frontend/README.md`, frames
//! `12`-`14`) confirms the real map is not this grid at all: a **hexagonal**
//! tessellation of icon-on-hexagon markers (chequered flag / stopwatch,
//! coloured by locked/passed state), with a season title card
//! (`A·G·R·C 2048`/`2049`/`2050`) drawn inline on the same scrollable canvas
//! at each season's own cluster - not a `MenuSkin`-style corner widget, and
//! not any of the `<CanvasLabel>`s `2048-frontend.md` already accounts for.
//! Neither the exact per-event anchor formula nor the season-card asset was
//! recovered this pass (`M_BUTTONSHAPE`'s own `CanvasButtonShape` enum is
//! the leading candidate for an anchor-table index, unconfirmed - no
//! consuming function found), so this build still lays cells out on an even
//! grid - [`PITCH`] units per cell from [`ORIGIN`], sized so the file's own
//! `x` 1-34 and `y` 1-25 fill the authored 1920x1088 canvas - and draws each
//! event as a plain [`MARKER`]-sized `Blue2048` square rather than a
//! hexagon shape nothing here has a decoded source for. The city behind the
//! real map does not exist as a 3D backdrop either way: the Vita3K capture's
//! own tiles sit on a flat light triangle-outline background, corroborating
//! `2048-frontend.md`'s "not a real 3D scene" finding from the opposite
//! direction. The panel under the map naming the selected event is this
//! build's own chrome, in the skin's own colours.
//!
//! Every event is offered, whatever the unlock graph says: there is no save
//! to read progress from, and hiding events behind a graph nothing walks
//! would be inventing a locked state.

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
}

/// Where the map is: which event the cursor is on and how far the view has
/// scrolled.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CampaignMap {
    events: Vec<MapEvent>,
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
}

impl Frontend {
    /// Gives the map its events, in the order the file lists them. The
    /// cursor starts on the first one named `2048 - Event 1` when there is
    /// one - the first season's first event by name, which is a reading of
    /// the names and not of the unlock graph - and on the first event
    /// otherwise.
    pub fn set_campaign(&mut self, events: Vec<MapEvent>) {
        self.campaign.selected = events
            .iter()
            .position(|event| event.name == "2048 - Event 1")
            .unwrap_or(0);
        self.campaign.events = events;
        self.campaign.scroll = (0.0, 0.0);
        self.campaign.follow(self.space.size);
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
    pub(super) fn draw_campaign_map(&self, out: &mut Vec<Draw>) {
        let (width, height) = self.space.size;
        let tile = self.global_colour("Blue2048");
        let cursor = self.global_colour("Orange2048");
        let (sx, sy) = self.campaign.scroll;
        for (at, event) in self.campaign.events.iter().enumerate() {
            let [x, y, w, h] = CampaignMap::marker(event);
            let rect = [x - sx, y - sy, w, h];
            if rect[0] + w < 0.0 || rect[1] + h < 0.0 || rect[0] > width || rect[1] > height {
                continue;
            }
            out.push(Draw::Fill { rect, color: tile });
            if at == self.campaign.selected {
                self.draw_cursor_ring(rect, cursor, out);
            }
        }
        let Some(event) = self.selected_event() else {
            out.push(Draw::Text {
                x: width * 0.5,
                y: height * 0.5,
                scale: LABEL_SCALE,
                color: tile,
                border: None,
                align: Align::Centre,
                text: "no campaign events were loaded".to_string(),
                wrap_width: None,
            });
            return;
        };
        out.push(Draw::Fill {
            rect: PANEL,
            color: tile,
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
