//! The session's half of the pause: the rules are [`oag_game::pause`], this is
//! what reads the live devices, the race and the clock into them and draws the
//! line.

use log::info;
use oag_game::pause::{
    Line, State, Table, clock, go, line, present_seats, press_cue, several_people, step, strings,
};
use oag_gameplay::cue::Cue;

use crate::stage::Stage;

/// Where the line sits, in the overlay grid (272 high): how far above the bottom
/// edge its top is, and its scale. **Chosen.**
const LINE_ABOVE_BOTTOM: f32 = 44.0;
const LINE_SCALE: f32 = 1.0;
/// How much of the width the line may take before it wraps.
const LINE_WIDTH: f32 = 0.9;
/// The band behind it, so it reads over any frame.
const BAND: [f32; 4] = [0.0, 0.0, 0.0, 0.6];

/// The project's text for `id` in `language`, or its English where the language file
/// leaves it untranslated (French names it under `[untranslated]`): a gap draws a
/// word, never the id.
pub(crate) fn say(language: Option<&str>, id: &str) -> String {
    let own = oag_ui::strings::project_table(language);
    let english = oag_ui::strings::project_table(None);
    own.get(id)
        .or_else(|| english.get(id))
        .unwrap_or(id)
        .to_string()
}

impl super::Session {
    /// The line under the pause as a draw list in the overlay grid `grid`
    /// (width, height), or nothing: in a race with one person, while the race
    /// runs, and through the countdown, which has no line: its art is the title's
    /// own where it has one, and where it has none nothing is drawn (never a
    /// stand-in for the art).
    pub(crate) fn pause_line(&self, grid: (f32, f32)) -> Vec<oag_ui::frontend::Draw> {
        use oag_ui::frontend::{Align, Draw};
        if !matches!(self.stage, Stage::Race(_)) || !several_people(self.controls.assignment()) {
            return Vec::new();
        }
        let table = Table {
            present: present_seats(self.controls.assignment()),
            countdown: true,
        };
        let Some(line) = line(self.pause, self.wall, table) else {
            return Vec::new();
        };
        let language = self.settings.language.as_deref();
        let text = |id: &str, fills: &[String]| {
            fills
                .iter()
                .fold(say(language, id), |text, fill| text.replacen("%s", fill, 1))
        };
        let number = |slot: usize| (slot + 1).to_string();
        let text = match line {
            Line::Owned {
                player,
                seconds_left,
            } => text(strings::OWNED, &[number(player), clock(seconds_left)]),
            Line::Open { player: Some(player) } => text(strings::OPEN, &[number(player)]),
            Line::Open { player: None } => text(strings::OPEN_UNOWNED, &[]),
            Line::Resuming => return Vec::new(),
        };
        let top = grid.1 - LINE_ABOVE_BOTTOM;
        vec![
            Draw::Fill {
                rect: [0.0, top - 6.0, grid.0, 16.0 * LINE_SCALE + 12.0],
                color: BAND,
            },
            Draw::Text {
                x: grid.0 / 2.0,
                y: top,
                scale: LINE_SCALE,
                color: [1.0, 1.0, 1.0, 1.0],
                border: None,
                align: Align::Centre,
                text,
                wrap_width: Some(grid.0 * LINE_WIDTH),
            },
        ]
    }


    /// Whether the race is held for any reason, paused or counting down.
    pub(crate) fn paused(&self) -> bool {
        self.pause.frozen()
    }

    /// One tick of the pause rule against the live session, with this tick's Start
    /// `presses`, raising the start voice's cues on a title that has them.
    ///
    /// A resume while the race is still counting down its own start (or flying
    /// the pre-race camera) happens at once: that countdown is on screen, and a
    /// second one would draw over it.
    pub(crate) fn step_pause(&mut self, presses: u8) {
        let Stage::Race(stage) = &self.stage else {
            return;
        };
        // Start does not pause a network race, for the reason `window_away` gives.
        if self.net.is_some() {
            return;
        }
        let start_counted = oag_race::RaceState::thrust_gated(stage.race.sim.world.tick)
            || (!self.no_intro && stage.race.intro_to_play());
        let assignment = self.controls.assignment();
        let table = Table {
            present: present_seats(assignment),
            countdown: several_people(assignment) && !start_counted,
        };
        let before = self.pause;
        let after = step(before, presses, self.wall, table);
        if after == before {
            return;
        }
        self.pause = after;
        info!("pause: {before:?} -> {after:?}");
        // Measured on Pulse (USA): START plays `ACCEPT` both on pausing a race
        // and on resuming it (`ConfirmButton_Update`). Only a press: the
        // countdown running out is not one.
        self.play_navs(press_cue(before, after, presses));
        let cue = match (before, after) {
            (State::Paused { .. }, State::Resuming { .. }) => Some(Cue::Ready),
            _ if go(before, after) => {
                self.go_at = Some(self.wall);
                Some(Cue::Go)
            }
            _ => None,
        };
        if let (Some(cue), Stage::Race(stage)) = (cue, &mut self.stage) {
            stage.race.raise_start_cue(cue);
        }
    }
}

