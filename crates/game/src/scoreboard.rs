//! The table a finished race leaves behind: who came where, on which lap, at
//! what time - and the overlay that draws it over the frozen scene.
//!
//! # This is ours, and the original's own race-end chain is not built
//!
//! The disc ends a race in a *sequence* of screens, and the names are recovered:
//! `"Race End Photo"`, `"Race End Save"`, `"Race End Records"`,
//! `"Race End Proceed"` and `"Race End Alone"` are `.rodata` state names in
//! `docs/ghidra/functions/psp-pulse-usa/main-loop.md`, and `Zone_UpdateResults`
//! (`0x0882f438`) fires `"EndRace_Results"`. **None of them has had its screen,
//! its layout or its transitions read**, so none of them is reproduced here.
//!
//! What this draws instead is the same kind of thing `crate::menu` is: our own
//! presentation, built from state the simulation already holds, standing in for
//! a recovered screen nobody has recovered yet. It is deliberately plain - a
//! list of positions in the built-in grid - rather than a plausible imitation of
//! the original's results screen, because a stand-in that *looks* recovered is
//! how a wrong picture survives review. See the do-not-invent rule in
//! `CLAUDE.md`.
//!
//! What would retire it: reading `Race End Records`' layout out of the front-end
//! XML the way `crate::hud` reads `Arcade_HUD.xml`, and the transitions that
//! reach it out of the state machine.
//!
//! # What is not in a row, and why
//!
//! **No team name.** A craft's team reaches the renderer as
//! [`crate::livery::Livery::team`], which is an *id* (`Feisar`) and says so; the
//! display name lives in the string table, which a `--race` run never loads.
//! Neither is reachable from [`crate::race::Race`], which is what holds the
//! standings, so a row names its grid slot and nothing else. Threading the
//! liveries into the race is its own change.
//!
//! **A best lap per craft, and a race time only for finishers.** Every craft
//! times its own laps - `oag_race::Standing` carries `best_lap_ticks` beside the
//! lap count - so the `BEST` column is that craft's own quickest lap, not the
//! player's clock repeated eight times. The `TIME` column is the tick a craft
//! crossed for the last time, and empty for one the race ended under.

use crate::frontend::{Align, Draw};
use crate::hud::{Precision, format_lap_time};
use crate::records;

#[cfg(test)]
mod tests;

/// One craft, as the caller knows it before the board is ordered.
///
/// Taken rather than a `&World` so that [`build`] is a pure function of the
/// numbers and can be tested without a disc, a track or a physics step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Craft {
    /// Grid slot, zero-based. Slot 0 is the player.
    pub slot: u8,
    /// Race position, 1-based, as `oag_race::places` ordered it.
    pub place: u8,
    /// The lap this craft was on, 1-based, exactly as `oag_race::Standing`
    /// means it.
    pub lap: u32,
    /// The tick it crossed the line for the last time, or `None` if it had not.
    pub finish_tick: Option<u64>,
    /// Its own quickest lap, in ticks, or `None` if it never completed one.
    pub best_lap_ticks: Option<u32>,
}

/// One line of the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Race position, 1-based.
    pub place: u8,
    /// Grid slot, zero-based.
    pub slot: u8,
    /// Laps completed, which is one less than the lap being driven - and capped
    /// at the target for a craft that has finished, whose lap counter is one
    /// past it.
    pub laps_completed: u32,
    /// The tick it finished on, or `None` for a craft still racing when the
    /// race ended.
    pub finish_tick: Option<u64>,
    /// Its own quickest lap, in ticks, or `None` if it never completed one.
    ///
    /// The craft's own clock (`oag_race::Standing::best_lap_ticks`) rather than
    /// the player's, which is what makes this a column rather than a footer.
    pub best_lap_ticks: Option<u32>,
    /// Whether this is the player's row.
    pub player: bool,
}

impl Row {
    /// Whether this craft finished the race.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.finish_tick.is_some()
    }
}

/// A finished race's results, ordered by place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    /// Every craft that was on the grid, first place first.
    pub rows: Vec<Row>,
    /// Laps the race ran to, or `None` for a mode that never ends on its own.
    pub laps_target: Option<u32>,
    /// The tick the race ended on - the player's own last crossing.
    pub tick: u64,
}

impl Board {
    /// The player's row, which is the one a caller usually wants.
    #[must_use]
    pub fn player(&self) -> Option<&Row> {
        self.rows.iter().find(|row| row.player)
    }

    /// Whether there is a *place* to show at all.
    ///
    /// **The HUD's own rule, deliberately the same one.** A place is a position
    /// among opponents, and a time trial, a speed lap and a Zone run all grid
    /// the player alone: `crate::hud::Readout` omits its position widget on a
    /// field of one because `1 / 1` is arithmetic rather than a standing, and a
    /// results table that printed `POS 1` under it would contradict the screen
    /// the player was looking at a tick earlier.
    #[must_use]
    pub fn shows_place(&self) -> bool {
        self.rows.len() > 1
    }
}

/// Orders the field into a board.
///
/// The ordering is **not** redone here: `crafts` carries the place
/// `oag_race::places` already assigned, and this sorts on it, so the table and
/// the HUD's position readout cannot disagree. Two craft cannot share a place -
/// that function breaks every tie, down to the slot index - so the sort is
/// total.
#[must_use]
pub fn build(crafts: &[Craft], laps_target: Option<u32>, tick: u64) -> Board {
    let mut rows: Vec<Row> = crafts
        .iter()
        .map(|craft| Row {
            place: craft.place,
            slot: craft.slot,
            // A finished craft is on `target + 1` by the time its finish tick is
            // written, so completed laps would read one too many without the cap.
            laps_completed: match (craft.finish_tick, laps_target) {
                (Some(_), Some(target)) => target,
                _ => craft.lap.saturating_sub(1),
            },
            finish_tick: craft.finish_tick,
            best_lap_ticks: craft.best_lap_ticks,
            player: craft.slot == 0,
        })
        .collect();
    rows.sort_by_key(|row| (row.place, row.slot));
    Board {
        rows,
        laps_target,
        tick,
    }
}

/// The heading, drawn once whatever the mode.
const TITLE: &str = "RACE COMPLETE";

/// What leaves the board.
///
/// **Deliberately non-committal about where it lands**, because that depends on
/// the route: leaving a race started from the menus hands the window back to
/// them, and leaving one `--race` started quits, there being no level behind it.
/// The composition root keeps those apart as two separate hints
/// (`ESC_TO_MENU` and `ESC_QUITS`), and a caption that named one of them would
/// be wrong on the other path - which is the path this build's own screenshots
/// are taken from.
const HINT: &str = "X OR ESCAPE LEAVES THE RACE";

/// Where the panel sits in the 480x272 grid: left edge, top edge and width.
const PANEL_X: f32 = 40.0;
const PANEL_Y: f32 = 20.0;
const PANEL_W: f32 = 400.0;
/// Breathing room inside the panel, and between its sections.
const PAD: f32 = 8.0;

/// The five columns' anchors, and how each is aligned about them.
const POS_X: f32 = PANEL_X + PAD + 22.0;
const CRAFT_X: f32 = PANEL_X + PAD + 34.0;
const LAPS_X: f32 = PANEL_X + 190.0;
const BEST_X: f32 = PANEL_X + 300.0;
const TIME_X: f32 = PANEL_X + PANEL_W - PAD;

/// The panel, the ordinary row, the player's row, and the quieter captions.
const PANEL: [f32; 4] = [0.0, 0.0, 0.0, 0.78];
const TEXT: [f32; 4] = [0.92, 0.95, 1.0, 1.0];
const PLAYER: [f32; 4] = [1.0, 0.83, 0.24, 1.0];
const CAPTION: [f32; 4] = [0.62, 0.68, 0.78, 1.0];
/// The outline the two HUD fonts bake into their atlas needs a colour of its
/// own, or a glyph fills as a solid box - see `crate::font::Atlas::luma`.
const BORDER: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// What the craft column's own words are for a medal: the disc-inspired,
/// all-caps convention every other label on this panel already uses (`POS`,
/// `LAPS`, `YOU`). See [`records::Medal`] for the law behind which tier a
/// race earns - nothing here recomputes it, only names it.
fn medal_label(medal: records::Medal) -> &'static str {
    match medal {
        records::Medal::Gold => "GOLD",
        records::Medal::Silver => "SILVER",
        records::Medal::Bronze => "BRONZE",
    }
}

/// How many extra footer lines [`draw_list`] owes [`PersonalBest`] - zero,
/// one or two, depending on which of its two rows have anything to show.
/// Read before the panel height is fixed, so a race with no stored lap yet
/// and no medal in play draws exactly the panel [`crate::scoreboard`] always
/// drew, with nothing tacked on underneath it - see the do-not-invent rule in
/// `CLAUDE.md`: an absent personal best is a shorter panel, never a blank or
/// placeholder line.
///
/// [`PersonalBest`]: crate::records::PersonalBest
fn personal_best_lines(personal_best: Option<&records::PersonalBest>) -> usize {
    let Some(pb) = personal_best else {
        return 0;
    };
    usize::from(pb.best_lap_ticks.is_some()) + usize::from(pb.best_medal.is_some())
}

/// Builds the draw list for a board, in the 480x272 grid every other overlay
/// here draws in.
///
/// `line_height` is the font's own, so the table spaces itself to whatever face
/// the source supplied - the disc's `HUDSmall` where there is one, the built-in
/// 5x7 set where there is not.
///
/// `personal_best` is `Session::frame`'s finish-transition arm's own
/// [`records::PersonalBest::compare`], read once at the same capture site
/// that persists a race's `Observation` into `records::Store` - never
/// recomputed here. `None` on every path that has no `records::Store` to
/// compare against at all, which draws the panel exactly as it did before
/// this existed.
#[must_use]
pub fn draw_list(
    board: &Board,
    line_height: f32,
    personal_best: Option<&records::PersonalBest>,
) -> Vec<Draw> {
    let line = (line_height + 2.0).max(10.0);
    let header_y = PANEL_Y + PAD + line * 1.6;
    let first_row_y = header_y + line * 1.4;
    let footer_y = first_row_y + board.rows.len() as f32 * line + PAD;
    let hint_y = footer_y + line * (1 + personal_best_lines(personal_best)) as f32;
    let panel_h = hint_y + line + PAD - PANEL_Y;

    let mut out = vec![Draw::Fill {
        rect: [PANEL_X, PANEL_Y, PANEL_W, panel_h],
        color: PANEL,
    }];

    let text = |x: f32, y: f32, align: Align, color: [f32; 4], text: String| Draw::Text {
        x,
        y,
        scale: 1.0,
        color,
        border: Some(BORDER),
        align,
        text,
        wrap_width: None,
    };

    out.push(text(
        PANEL_X + PANEL_W / 2.0,
        PANEL_Y + PAD,
        Align::Centre,
        TEXT,
        TITLE.to_string(),
    ));
    // The place column goes on a field of one - see [`Board::shows_place`] -
    // and its caption goes with it rather than heading an empty column.
    for (x, align, caption) in [
        (POS_X, Align::Right, "POS"),
        (CRAFT_X, Align::Left, "CRAFT"),
        (LAPS_X, Align::Right, "LAPS"),
        (BEST_X, Align::Right, "BEST"),
        (TIME_X, Align::Right, "TIME"),
    ] {
        if caption == "POS" && !board.shows_place() {
            continue;
        }
        out.push(text(x, header_y, align, CAPTION, caption.to_string()));
    }

    for (index, row) in board.rows.iter().enumerate() {
        let y = first_row_y + index as f32 * line;
        let color = if row.player { PLAYER } else { TEXT };
        if board.shows_place() {
            out.push(text(
                POS_X,
                y,
                Align::Right,
                color,
                format!("{}", row.place),
            ));
        }
        out.push(text(CRAFT_X, y, Align::Left, color, craft_label(row)));
        out.push(text(
            LAPS_X,
            y,
            Align::Right,
            color,
            match board.laps_target {
                Some(target) => format!("{}/{target}", row.laps_completed),
                None => format!("{}", row.laps_completed),
            },
        ));
        out.push(text(
            BEST_X,
            y,
            Align::Right,
            color,
            match row.best_lap_ticks {
                Some(ticks) => format_lap_time(u64::from(ticks), Precision::Hundredths),
                // A craft that never completed a lap has no best one. The race
                // ends after three, so this is the grid's back half on a circuit
                // long enough that the flag caught them on lap 1.
                None => "-".to_string(),
            },
        ));
        out.push(text(
            TIME_X,
            y,
            Align::Right,
            color,
            match row.finish_tick {
                Some(tick) => format_lap_time(tick, Precision::Hundredths),
                // Not `DNF`: the craft did not retire, the race ended under it
                // when the player crossed. A dash says "no time" without
                // claiming a rule this build does not implement.
                None => "-".to_string(),
            },
        ));
    }

    // The race's own length, once. The player's best lap used to be here too
    // and is now the `BEST` cell on their own row, where it is one of eight
    // rather than a figure that looked like the field's.
    out.push(text(
        PANEL_X + PAD,
        footer_y,
        Align::Left,
        CAPTION,
        format!(
            "RACE TIME {}",
            format_lap_time(board.tick, Precision::Hundredths)
        ),
    ));

    // The personal-best lines, one row under the race time and above the
    // hint - present only for what `personal_best` actually has to say, per
    // [`personal_best_lines`]. Highlighted in the same colour a player's own
    // row already uses when this race is the one that set the figure, so
    // "new" reads the same way here as it does in the standings above.
    let mut y = footer_y + line;
    if let Some(pb) = personal_best {
        if let Some(ticks) = pb.best_lap_ticks {
            out.push(text(
                PANEL_X + PAD,
                y,
                Align::Left,
                if pb.lap_improved { PLAYER } else { CAPTION },
                format!(
                    "PERSONAL BEST LAP {}{}",
                    format_lap_time(u64::from(ticks), Precision::Hundredths),
                    if pb.lap_improved { " - NEW!" } else { "" }
                ),
            ));
            y += line;
        }
        if let Some(medal) = pb.best_medal {
            out.push(text(
                PANEL_X + PAD,
                y,
                Align::Left,
                if pb.medal_improved { PLAYER } else { CAPTION },
                format!(
                    "BEST MEDAL {}{}",
                    medal_label(medal),
                    if pb.medal_improved { " - NEW!" } else { "" }
                ),
            ));
        }
    }

    out.push(text(
        PANEL_X + PANEL_W / 2.0,
        hint_y,
        Align::Centre,
        CAPTION,
        HINT.to_string(),
    ));

    out
}

/// What the craft column says: the player is named, everyone else is their grid
/// slot.
///
/// **A slot number and not a team**, for the reason the module header gives: the
/// team a slot flies is an id held by the renderer's liveries, not by the race.
fn craft_label(row: &Row) -> String {
    if row.player {
        "YOU".to_string()
    } else {
        format!("SLOT {}", row.slot + 1)
    }
}

/// Draws a board over whatever is already in the target.
///
/// A renderer of its own rather than a method on [`crate::hud::Overlay`]: that
/// one draws a *layout* read off the disc and this draws a table of ours, and
/// the two share nothing but a font. It is built from the same
/// [`crate::hud::Assets`] the HUD is, so a source whose `HUDSmall` face is
/// unreadable draws this in the built-in 5x7 set and says so once, where the HUD
/// already said it.
pub struct Overlay {
    renderer: crate::render::Renderer,
    line_height: f32,
}

impl std::fmt::Debug for Overlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Overlay")
            .field("line_height", &self.line_height)
            .finish()
    }
}

impl Overlay {
    /// Builds the renderer the board is drawn with.
    ///
    /// # Errors
    ///
    /// Propagates pipeline creation.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        assets: &crate::hud::Assets,
    ) -> anyhow::Result<Self> {
        let renderer = crate::render::Renderer::new(
            device,
            queue,
            format,
            None,
            assets.small_font.clone(),
            &assets.sheet,
        )?;
        Ok(Self {
            renderer,
            line_height: assets.small_font.line_height,
        })
    }

    /// Draws the board, over the frame and inside the same target the HUD is
    /// composited into.
    ///
    /// `personal_best` is [`draw_list`]'s own parameter of the same name -
    /// `None` on every caller with no `records::Store` to compare against.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        board: &Board,
        personal_best: Option<&records::PersonalBest>,
        viewport: (f32, f32, f32, f32),
    ) {
        let list = draw_list(board, self.line_height, personal_best);
        self.renderer
            .overlay(device, queue, encoder, view, &list, viewport);
    }
}
