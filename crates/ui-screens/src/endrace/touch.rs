//! Wipeout 2048's race-ending screens: **`RaceSummary`** and
//! **`ObjectiveSummary`**, with the `EndRace` shell's button strip under them.
//!
//! 2048 authors them in `NEWGUI/EndRace_Definition.xml` (`oag_title::EndRaceDialect::Touch`),
//! a different vocabulary from Pulse's three screens and HD's two: a 960x544
//! touch grid, a 600x347 white panel with coloured message bars and a medal on
//! it, and a row of icon tiles. See `docs/ui/endrace-2048.md` for what draws and
//! why; the decompiled law is in
//! `docs/ghidra/functions/vita-2048-eu-v104/endrace-summary.md`.
//!
//! # What is the disc's
//!
//! - Every rectangle, text position, font, colour global and texture: read off
//!   the XML, with a colour-only `Image`'s own `OffsetX`/`OffsetY` folded in
//!   ([`oag_ui::screen::Screens::from_xml_folding_fill_offsets`]).
//! - The colour a verdict paints `MessageBox`/`TotalXPBox` (`Pass2048`,
//!   `ElitePass2048`, and the literals `0xffcd0102` for a fail and
//!   `0xff525e84` for no verdict), the text colour on them, the words
//!   (`ER_CONGRAT`, `ER_END_TOUR_7`, `FE_PASS`, `FE_ELITE_PASS`, `FE_FAIL`,
//!   `IG_HUD_BESTLAP`), the medal sheet's cell (`0`, `128`, `256` texels in),
//!   the five-second page timer and the page chain
//!   (`RaceSummary` -> `ObjectiveSummary` -> `Results`): all read off
//!   `FUN_810cf5fe` and `FUN_810cd0b8` of the v1.04 eboot.
//!
//! # What is not drawn, by name
//!
//! - **Every XP widget** (`RaceXPBox`, `PassBonusXPBox`, `SpeedRaceXPBox`, the
//!   rank badge and its bar): this build keeps no XP and no rank, and the XML's
//!   own text for them is a development placeholder (`TOTAL_4325_XP_TEST`, `99`).
//!   `TotalXPBox` keeps its coloured bar and an empty label.
//! - **`Results`' table** (`<RaceResults>`, filled by native code this lane did
//!   not decompile), `Podium` and `Badges` (multiplayer only in the original),
//!   `ParadeLaps` (a free camera this build does not have) and the `Near`
//!   button (online only).
//!
//! # What is chosen, not measured (no confidence score)
//!
//! The wording of the result line (`RaceResultText`), because the string the
//! original puts there lives in a game-mode field not traced; the pad buttons
//! and the pointer's whole-panel tap.

use oag_ui::frontend::{Align, Draw, Placed, cursor_ring};
use oag_ui::language::StringTable;
use oag_ui::pointer::{Pointer, contains};
use oag_ui::screen::{Image, Screens, Text, argb_to_rgba, parse_argb};

use super::Layout;
use super::draw::fill_draw;
use crate::campaign::draw::sprite_draw;
use crate::picker::FaceScales;

/// The shell screen the pages sit in: the translucent top bar, the panel's
/// background image and the button strip.
pub const SHELL: &str = "EndRace";
/// The first page, and the only one a race without an authored objective gets.
pub const SUMMARY: &str = "RaceSummary";
/// The second page of an event that authors an objective.
pub const OBJECTIVES: &str = "ObjectiveSummary";
/// The grid the file is authored in, which is also the Vita's native one.
pub const AUTHORED_GRID: [f32; 2] = [960.0, 544.0];
/// Seconds a page holds before the next one comes up on its own.
/// Measured: `FUN_810cd0b8` compares its page timer against `5.0`.
pub const PAGE_SECONDS: f32 = 5.0;

/// The fill a failed attempt paints `MessageBox` and `TotalXPBox`.
/// Measured: the literal `0xffcd0102` in `FUN_810cf5fe`.
const FAIL_ARGB: u32 = 0xffcd_0102;
/// The fill when there is no verdict to show. Measured: `0xff525e84`.
const UNJUDGED_ARGB: u32 = 0xff52_5e84;
/// Texels into `Post_Race_Medals` each verdict's cell starts: the value
/// `FUN_810cf5fe` stores at `PostRaceMedal + 0xc8`. Measured.
const PASS_CELL: f32 = 128.0;
const ELITE_CELL: f32 = 256.0;
const FAIL_CELL: f32 = 0.0;

/// The widgets the end-of-race pages hide because this build holds nothing to
/// put in them. See the module docs.
const UNFILLED: &[&str] = &[
    "PassBonusXPBox",
    "PassBonusXPNumberBox",
    "RaceXPBox",
    "RaceXPNumberBox",
    "SpeedRaceXPBox",
    "SpeedRaceXPNumberBox",
    "TotalXPBg",
    "TotalXPFill",
    "PassBonusXPText",
    "PassBonusXP",
    "RaceXPText",
    "RaceXP",
    "SpeedRaceXPText",
    "SpeedRaceXP",
    "RankItem",
    "RankText",
];

/// What the player's attempt came to, in the three states the screens paint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Met the pass bar.
    Pass,
    /// Met the elite bar.
    Elite,
    /// Authored an objective and missed it.
    Fail,
    /// Nothing to judge against: an event with no objective, or a speed-lap
    /// attempt that did not pass.
    Unjudged,
}

/// One objective row of [`OBJECTIVES`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectiveRow {
    /// The objective, worded.
    pub text: String,
    /// Which of the row's three icons draws: `Pass` or `Elite` for a met
    /// objective of that chain, `Fail` for a missed one. Measured,
    /// `FUN_810d0c46`.
    pub state: Tone,
}

/// A race's result as these pages say it. Every string is already resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// `FE_ENDRACE_SUMMARY` and its mode siblings, resolved.
    pub title: Option<String>,
    /// The line on the coloured bar (`ER_CONGRAT`, `ER_END_TOUR_7`, ...).
    pub message: Option<String>,
    /// The pass objective, worded.
    pub objective: Option<String>,
    /// The result line. **Chosen**, see the module docs.
    pub result: Option<String>,
    /// A speed-lap attempt: the result sits bare on the panel, with no box.
    pub speed_shape: bool,
    /// The verdict.
    pub tone: Tone,
    /// The medal's caption (`FE_PASS`, `FE_ELITE_PASS`, `FE_FAIL`).
    pub medal_label: Option<String>,
    /// [`OBJECTIVES`]' rows, empty when the event authors no objective.
    pub rows: Vec<ObjectiveRow>,
}

impl Summary {
    /// Whether the event authored an objective, which is what makes
    /// [`OBJECTIVES`] a page of the chain.
    #[must_use]
    pub fn has_objectives_page(&self) -> bool {
        !self.rows.is_empty()
    }

    /// Whether the exit tile is the tick rather than the door. Measured:
    /// `FUN_810cc556` swaps `QuitTouchButton` for `QuitTouchButtonPass` once
    /// the event's own state reads as passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        matches!(self.tone, Tone::Pass | Tone::Elite)
    }
}

/// Which page is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// [`SUMMARY`].
    Summary,
    /// [`OBJECTIVES`].
    Objectives,
}

/// One of the two tiles the strip keeps in a single-player race.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    /// `RestartRaceTouchButton`: race the event again.
    Restart,
    /// `QuitTouchButton`, or `QuitTouchButtonPass` once passed: leave.
    Exit,
}

/// What a pointer did on these screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    /// A tile was tapped.
    Tile(Button),
    /// The panel was tapped. **Chosen:** the original pages with swipe zones
    /// either side, which this build answers with a tap on the panel.
    Panel,
}

/// The disc's layout for the pages, with the colours they paint.
#[derive(Debug, Clone)]
pub struct Layouts {
    /// The shell: top bar, background, the button strip.
    pub shell: Layout,
    /// [`SUMMARY`].
    pub summary: Layout,
    /// [`OBJECTIVES`], when the file authors it.
    pub objectives: Option<Layout>,
    /// `Pass2048`.
    pub pass: u32,
    /// `ElitePass2048`.
    pub elite: u32,
    /// `White2048`, the text colour on a pass or fail bar.
    pub white: u32,
    /// `Blue2048`, the tile colour and the text colour on an elite bar.
    pub blue: u32,
    /// `Orange2048`, the pad cursor.
    pub orange: u32,
}

impl Layouts {
    /// Reads the pages off a parsed `EndRace_Definition.xml`. `screens` must
    /// come from [`Screens::from_xml_folding_fill_offsets`]. `None` when the
    /// file lacks the shell or the first page, or a colour global is not
    /// declared (`globals` is the front end's own `Skin.xml` table).
    #[must_use]
    pub fn read(
        screens: &Screens,
        strings: &StringTable,
        faces: FaceScales,
        grid: [f32; 2],
    ) -> Option<Self> {
        let layout =
            |name: &str| Layout::read_authored(screens, name, strings, faces, grid, AUTHORED_GRID);
        let colour = |name: &str| {
            screens
                .resolve(&format!("FEGlobals->{name}"))
                .and_then(parse_argb)
        };
        Some(Self {
            shell: layout(SHELL)?,
            summary: layout(SUMMARY)?,
            objectives: layout(OBJECTIVES),
            pass: colour("Pass2048")?,
            elite: colour("ElitePass2048")?,
            white: colour("White2048")?,
            blue: colour("Blue2048")?,
            orange: colour("Orange2048")?,
        })
    }

    fn tone_fill(&self, tone: Tone) -> u32 {
        match tone {
            Tone::Pass => self.pass,
            Tone::Elite => self.elite,
            Tone::Fail => FAIL_ARGB,
            Tone::Unjudged => UNJUDGED_ARGB,
        }
    }

    /// The text colour on a coloured bar, `None` to keep the authored one.
    fn tone_ink(&self, tone: Tone) -> Option<u32> {
        match tone {
            Tone::Pass | Tone::Fail => Some(self.white),
            Tone::Elite => Some(self.blue),
            Tone::Unjudged => None,
        }
    }

    /// The tiles this model keeps, with their authored rectangles.
    fn tiles(&self, model: &Summary) -> Vec<(Button, [f32; 4], Option<&str>)> {
        let touch = &self.shell.screen.touch_buttons;
        let find = |name: &str| touch.iter().find(|b| b.name.as_deref() == Some(name));
        let exit = if model.passed() {
            find("QuitTouchButtonPass")
        } else {
            find("QuitTouchButton")
        };
        [
            (Button::Restart, find("RestartRaceTouchButton")),
            (Button::Exit, exit),
        ]
        .into_iter()
        .filter_map(|(button, widget)| {
            let w = widget?;
            Some((button, [w.x, w.y, w.width, w.height], w.src.as_deref()))
        })
        .collect()
    }
}

/// The face a font name draws at, against the `Default` role. `1.0` for a
/// role nothing measured. `scales` is the front end's own table.
fn face(scales: &[(String, f32)], font: &str) -> f32 {
    scales
        .iter()
        .find(|(role, _)| role.eq_ignore_ascii_case(font))
        .map_or(1.0, |(_, scale)| *scale)
}

fn text_at(
    text: &Text,
    content: &str,
    ink: Option<u32>,
    scales: &[(String, f32)],
    line: f32,
) -> Draw {
    let scale = text.scale * face(scales, &text.font);
    // `vertalign="middle"`: half a line up from the authored `y`.
    let y = if text.middle {
        text.y - 0.5 * line * scale
    } else {
        text.y
    };
    Draw::Text {
        x: text.x,
        y,
        scale,
        color: argb_to_rgba(ink.unwrap_or(text.color)),
        border: None,
        align: Align::parse(&text.align),
        text: content.to_string(),
        wrap_width: text.wrap_width,
    }
}

/// Whether a widget by this name is drawn at all on `page` of `model`.
fn shown(name: Option<&str>, model: &Summary, page: Page) -> bool {
    let Some(name) = name else { return true };
    if UNFILLED.contains(&name) {
        return false;
    }
    match name {
        // The two result shapes are alternatives.
        "ResultBox" | "RaceResultText" => !model.speed_shape,
        "SpeedResultText" => model.speed_shape,
        "PostRaceMedal" | "PostRaceMedalText" => model.tone != Tone::Unjudged,
        // The objective page draws only the icon that matches its row.
        n if n.starts_with("Pass") || n.starts_with("ElitePass") || n.starts_with("Fail") => {
            page == Page::Objectives
        }
        _ => true,
    }
}

/// The index digit a `Pass0`/`ElitePass1`/`Fail2` widget name ends in.
fn row_of(name: &str) -> Option<(usize, Tone)> {
    let digit = name.chars().last()?.to_digit(10)? as usize;
    let tone = if name.starts_with("ElitePass") {
        Tone::Elite
    } else if name.starts_with("Pass") {
        Tone::Pass
    } else if name.starts_with("Fail") {
        Tone::Fail
    } else {
        return None;
    };
    Some((digit, tone))
}

/// The text a named widget carries on `page`, or `None` to keep the
/// disc's own. A widget whose value is filled by code is never left showing the
/// XML's development placeholder.
fn content_of(name: &str, model: &Summary, page: Page) -> Option<Option<String>> {
    let some = |s: &Option<String>| Some(s.clone());
    match name {
        "RaceSummaryTitle" => Some(model.title.clone()),
        "Message" => Some(model.message.clone()),
        "Objective" => Some(model.objective.clone()),
        "RaceResultText" | "SpeedResultText" => some(&model.result),
        "PostRaceMedalText" => some(&model.medal_label),
        "TotalXP" => Some(None),
        "Objective1" | "Objective2" | "Objective3" if page == Page::Objectives => {
            let at = usize::from(name.as_bytes()[name.len() - 1] - b'1');
            Some(model.rows.get(at).map(|row| row.text.clone()))
        }
        _ => None,
    }
}

/// One page's draw list, over the race's own frame. `sprites` resolves a
/// texture name to its place on the sheet; `scales` is the front end's face
/// table and `line` the `Default` face's line height in grid units.
#[must_use]
pub fn draw_list(
    model: &Summary,
    page: Page,
    selected: Button,
    layouts: &Layouts,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    scales: &[(String, f32)],
    line: f32,
) -> Vec<Draw> {
    let mut out = Vec::new();
    let body = match page {
        Page::Objectives => layouts.objectives.as_ref().unwrap_or(&layouts.summary),
        Page::Summary => &layouts.summary,
    };
    for layout in [&layouts.shell, body] {
        for fill in &layout.screen.fills {
            if !shown(fill.name.as_deref(), model, page) {
                continue;
            }
            let mut fill = fill.clone();
            if matches!(fill.name.as_deref(), Some("MessageBox" | "TotalXPBox")) {
                fill.color = layouts.tone_fill(model.tone);
            }
            out.push(fill_draw(&fill));
        }
        for image in &layout.screen.images {
            if !shown(image.name.as_deref(), model, page) {
                continue;
            }
            if let Some(draw) = image_draw(image, model, sprites) {
                out.push(draw);
            }
        }
        for text in &layout.screen.texts {
            if !shown(text.name.as_deref(), model, page) {
                continue;
            }
            let name = text.name.as_deref().unwrap_or_default();
            let ink = if matches!(name, "Message" | "TotalXP") {
                layouts.tone_ink(model.tone)
            } else {
                None
            };
            let content = match content_of(name, model, page) {
                Some(Some(content)) => content,
                Some(None) => continue,
                None => match &text.string {
                    Some(content) if !is_placeholder(text) => content.clone(),
                    _ => continue,
                },
            };
            out.push(text_at(text, &content, ink, scales, line));
        }
    }
    out.extend(tile_draws(model, selected, layouts, sprites));
    out
}

/// A `string=` the XML authors for a widget code fills in later: development
/// text that must never reach the screen.
fn is_placeholder(text: &Text) -> bool {
    text.idstring.is_none() && text.name.is_some()
}

fn image_draw(
    image: &Image,
    model: &Summary,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Option<Draw> {
    let placed = sprites(&image.src)?;
    let name = image.name.as_deref();
    let mut image = image.clone();
    if name == Some("PostRaceMedal") {
        image.u = Some(match model.tone {
            Tone::Elite => ELITE_CELL,
            Tone::Pass => PASS_CELL,
            Tone::Fail | Tone::Unjudged => FAIL_CELL,
        });
    }
    if let Some((row, tone)) = name.and_then(row_of)
        && model.rows.get(row).map(|r| r.state) != Some(tone)
    {
        return None;
    }
    let (w, h) = (
        image.width.unwrap_or(placed.width as f32),
        image.height.unwrap_or(placed.height as f32),
    );
    let (x, y) = if image.centred {
        (image.x - w * 0.5, image.y - h * 0.5)
    } else {
        (image.x, image.y)
    };
    Some(sprite_draw(&image, placed, x, y, image.color))
}

/// The strip: a `Blue2048` box per tile, its icon centred at the texture's own
/// size, and the pad cursor's ring on the selected one - the idiom
/// `oag_ui::frontend::Frontend::draw_tile` draws the rest of the touch front
/// end in.
fn tile_draws(
    model: &Summary,
    selected: Button,
    layouts: &Layouts,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Vec<Draw> {
    let mut out = Vec::new();
    for (button, rect, src) in layouts.tiles(model) {
        out.push(Draw::Fill {
            rect,
            color: argb_to_rgba(layouts.blue),
        });
        if button == selected {
            out.extend(cursor_ring(rect, argb_to_rgba(layouts.orange)));
        }
        if let Some(placed) = src.and_then(sprites) {
            let (w, h) = (placed.width as f32, placed.height as f32);
            out.push(Draw::Sprite {
                rect: [
                    rect[0] + (rect[2] - w) * 0.5,
                    rect[1] + (rect[3] - h) * 0.5,
                    w,
                    h,
                ],
                uv: [
                    placed.x as f32,
                    placed.y as f32,
                    placed.width as f32,
                    placed.height as f32,
                ],
                color: argb_to_rgba(0xffff_ffff),
            });
        }
    }
    out
}

/// What the pointer did this tick, if it clicked something.
#[must_use]
pub fn pointer_hit(layouts: &Layouts, model: &Summary, pointer: &Pointer) -> Option<Hit> {
    if !pointer.clicked {
        return None;
    }
    let at = pointer.position()?;
    for (button, rect, _) in layouts.tiles(model) {
        if contains(rect, at) {
            return Some(Hit::Tile(button));
        }
    }
    let panel = layouts
        .shell
        .screen
        .images
        .iter()
        .find(|image| image.name.as_deref() == Some("EndRaceBackground"))?;
    let rect = [
        panel.x,
        panel.y,
        panel.width.unwrap_or(0.0),
        panel.height.unwrap_or(0.0),
    ];
    contains(rect, at).then_some(Hit::Panel)
}

#[cfg(test)]
mod tests;
