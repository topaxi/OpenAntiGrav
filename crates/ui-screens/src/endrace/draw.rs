//! `EndRace Results`/`EndRace Rewards`/`EndRace Menu`'s own draw lists,
//! split out the way [`crate::campaign::draw`] is - one file per screen
//! family once `endrace.rs` itself would otherwise carry both the model and
//! the drawing past a comfortable read.
//!
//! The small per-widget helpers below (`fill_draw`/`image_draw`/`text_draw`)
//! duplicate [`crate::campaign::draw`]'s own rather than sharing them across
//! the two module trees, for the same reason
//! [`crate::campaign::draw::mode_event_label`] duplicates
//! [`oag_ui::menu::mode_label`] instead of reaching across that seam. They are
//! `pub(super)`, the same visibility `crate::campaign::draw`'s own copies
//! carry, so [`super::hd`] can reuse them for Wipeout HD/Fury's own screens
//! without a second copy inside this module tree.

use oag_ui::frontend::{Align, Draw, Placed};
use oag_ui::language::StringTable;
use oag_ui::menu::{Frame, Layers, Picture, Skin};
use oag_ui::screen::{Fill, Image, Text, argb_to_rgba};

use super::table::{Shown, lap_slot, table_layers};
use super::{EndRaceMenu, Headline, Layout, Results, Rewards, TournamentResults, TournamentRow};

#[cfg(test)]
mod tests;

/// [`Layout::face_scale`] is private to [`crate::campaign`] - this crate's
/// own `Layout` type is reused from there (see the parent module doc), but
/// its scaling method is not, so this reimplements the same lookup off
/// [`Layout`]'s own public `faces` field rather than widening that method's
/// visibility for one caller outside its module.
pub(super) fn face_scale(layout: &Layout, font: &str) -> f32 {
    match font.to_ascii_lowercase().as_str() {
        "default" => layout.faces.default,
        "small" => layout.faces.small,
        _ => 1.0,
    }
}

/// `Endrace Options`' own row font (`font="title"`) has no per-row height in
/// the XML, the same gap `oag_ui::frontend::rows`'s `font_line_height` closes
/// for `Language Selection`'s `Menu` widget - duplicated rather than shared
/// for the same reason as this module's other small helpers: that table is
/// a private `fn` in a different module tree.
pub(super) fn font_line_height(font: &str) -> f32 {
    match font.to_ascii_lowercase().as_str() {
        "menu" => 22.0,
        "title" | "small" | "ingame" | "stats" => 17.0,
        _ => 13.0,
    }
}

/// A selected `Endrace Options` row's own ink. **Chosen, not measured**: the
/// capture (`results-03.png`) shows every row in the same cyan family with
/// no obviously distinct selected-row colour sampled, so this brightens
/// toward white the same way a picker's own unmeasured selection cue would -
/// see `oag_ui::frontend::draw`'s own `static_selected` fallback for the
/// identical shape of gap on `Language Selection`.
const MENU_SELECTED: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// `EndRace Results`' draw list: the headline and the per-lap table.
///
/// **The `perfectlap{n}` icons never draw**: this build keeps no per-lap "perfect"
/// flag. **`boostimg` draws** as the third column's header icon, and the column counts
/// the speedup pads entered on each lap ([`super::LapSplit::boosts`]). The icon is shown
/// by `EndRaceResults_PopulateLapTable` and by no other populate; it was left undrawn
/// until 2026-09-30 on a reading of the decompile that had bit `0x4` the wrong way
/// round (`docs/ui/endrace-screens.md`).
/// Speed Lap has no totals row and no highlight; a race with no completed lap hides
/// the whole table (`table &= ~4`).
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts a menu page or a picker takes"
)]
pub fn results_draw_list(
    model: &Results,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Layers {
    let laps = model.laps.len();
    let totals = model.headline != Headline::SpeedLap;
    let shown = Shown {
        table: laps > 0,
        rows: laps + usize::from(totals),
        header_bar: true,
        boost_icon: true,
        highlight: totals.then_some(laps + 1),
    };
    table_layers(
        layout,
        skin,
        frame,
        backdrop,
        race_behind,
        sprites,
        shown,
        &|text| {
            let name = text.name.as_deref().unwrap_or("");
            if name.starts_with("lap") {
                lap_cell_text(name, model, strings, totals)
            } else if name == "Line1" {
                headline_text(model.headline, strings)
            } else {
                text.string.clone()
            }
        },
    )
}

/// Pulse's own Tournament `EndRace Results`' draw list: `BigTopText` (the
/// leg counter or `ER_END_TOUR`), `Line1` (`ER_RACE_STAN`/`ER_TOUR_STAN`,
/// whichever page is current) and the table - `PRO_POS`/`ER_TEAM`/
/// `ER_POINTS`, off [`TournamentResults::current`]. See the module doc on
/// [`super::TournamentResults`] for the law this reimplements.
///
/// **No `ER_DNF`/`ER_RACING`, ever.** `EndRaceResults_PopulateTournamentTable`
/// only shows either in place of the points column when the screen's own
/// `+0xe9` flag is set, and that flag is `0xd < g_game_mode` -
/// `EndRaceResults_OnEnter` - which is false for Tournament's own mode `4`
/// (only `16`, an unimplemented sibling this project has no evidence is
/// reachable through any launch path it drives, sets it true). A
/// non-finisher's own points read `0` (`oag_race::tournament::points_for_finish`)
/// exactly as the original's own unsorted leg row would, with no DNF text
/// standing in for it.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts a menu page or a picker takes"
)]
pub fn tournament_results_draw_list(
    model: &TournamentResults,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Layers {
    let rows = model.current();
    // The highlight steps `0x5d + 0x14 * (row - 1)` - the same literal the lap
    // table's totals row ends on, one pixel under a `tablebg`'s own `OffsetY`.
    // `boostimg` is the lap table's header icon and no other populate shows it.
    let shown = Shown {
        table: true,
        rows: rows.len(),
        header_bar: true,
        boost_icon: false,
        highlight: rows
            .iter()
            .position(|row| row.player)
            .map(|index| index + 1),
    };
    table_layers(
        layout,
        skin,
        frame,
        backdrop,
        race_behind,
        sprites,
        shown,
        &|text| {
            let name = text.name.as_deref().unwrap_or("");
            if name.starts_with("lap") {
                tournament_lap_cell_text(name, rows, strings)
            } else {
                match name {
                    "BigTopText" => Some(tournament_big_top_text(model, strings)),
                    "Line1" => Some(strings.get_or_id(model.line1_id()).to_string()),
                    _ => text.string.clone(),
                }
            }
        },
    )
}

/// `EndRace Rewards`' draw list: the medal-award phrase, the disc's own
/// hex-dash glyph on the one measured no-medal case, and the loyalty row
/// when the model carries one. See the module doc on [`super::Rewards`] for
/// the trophy (drawn by the composition root, not here) and
/// [`super::Loyalty`] for the law behind the two loyalty numbers.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts a menu page or a picker takes"
)]
pub fn rewards_draw_list(
    model: &Rewards,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Layers {
    let mut layers = Layers {
        backdrop: frame.backdrops(
            skin.space(),
            skin.background(),
            backdrop.map(Picture::draw),
            race_behind,
        ),
        ..Layers::default()
    };
    let screen = &layout.screen;
    let mut out = Vec::new();
    for fill in &screen.fills {
        out.push(fill_draw(fill));
    }
    for image in &screen.images {
        let name = image.name.as_deref().unwrap_or("");
        // The loyalty row's own icon, drawn only alongside the row's own
        // text - see the module doc.
        if name == "LoyaltyImg" && model.loyalty.is_none() {
            continue;
        }
        if name == "MedalImg" && !model.shows_no_medal_glyph() {
            continue;
        }
        if name == "loyaltybg" {
            if model.loyalty.is_some()
                && let Some(placed) = sprites(&image.src)
            {
                out.push(image_draw(image, placed));
            }
            continue;
        }
        if name == "loyaltybar" {
            if let (Some(loyalty), Some(placed)) = (&model.loyalty, sprites(&image.src)) {
                // `loyaltybar`'s fill is `total * 0.00124` **pixels**, not a
                // fraction: `EndRaceRewards_Update` (`0x088dd2b8`) writes it
                // into the widget's width (`+0x9c`) and its U extent
                // (`+0xc4`) alike, between `FUN_088a6a9c` and `FUN_088a68ec`
                // (the texel-space denormalise and renormalise pair), and
                // `FEScreen_SetStatBar` (`0x088ea780`) does the same with
                // `fullWidth * value / max`. `Loyalty_AccumulateTotal`'s
                // `100000` ceiling lands on `124`, the authored width, so the
                // bar only fills at the cap; a total of 90 is `0.11` pixels
                // wide and only `loyaltybg` shows (the reference frame).
                // The total is capped at `100000`, well inside `f32`'s
                // exact-integer range, so this cast loses nothing.
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "total is capped at 100_000, exact in f32"
                )]
                let width = loyalty.total as f32 * LOYALTY_BAR_PIXELS_PER_POINT;
                out.push(loyalty_bar_draw(image, placed, width));
            }
            continue;
        }
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        out.push(image_draw(image, placed));
    }
    for text in &screen.texts {
        let name = text.name.as_deref().unwrap_or("");
        let content =
            match name {
                "RewardLine1" => Some(medal_award_text(model.medal, strings)),
                "RewardLine2" => model.loyalty.as_ref().map(|loyalty| {
                    format!("{} {}", loyalty.team_name, strings.get_or_id("ER_LOY"))
                }),
                "RewardLoyaltyActive" => model
                    .loyalty
                    .as_ref()
                    .map(|loyalty| format!("{} {}", loyalty.award, strings.get_or_id("ER_POINTS"))),
                "loyaltynum" => model.loyalty.as_ref().map(|loyalty| {
                    format!("{} {}", strings.get_or_id("ER_TOT_LOY"), loyalty.total)
                }),
                // `BigPos` (a finishing-position figure this model carries no
                // place for) - drawn nothing, per the module doc.
                "BigPos" => None,
                _ => text.string.clone(),
            };
        let Some(content) = content else { continue };
        out.push(text_draw(text, &content, layout));
    }
    layers.body = out;
    layers
}

/// `EndRace Menu`'s draw list: the option list, and the just-driven run's
/// own best lap. See the module doc on [`super::EndRaceMenu`] for why no
/// existing-ghost row draws.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts a menu page or a picker takes"
)]
pub fn endrace_menu_draw_list(
    model: &EndRaceMenu,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Layers {
    let mut layers = Layers {
        backdrop: frame.backdrops(
            skin.space(),
            skin.background(),
            backdrop.map(Picture::draw),
            race_behind,
        ),
        ..Layers::default()
    };
    let screen = &layout.screen;
    let mut out = Vec::new();
    for fill in &screen.fills {
        out.push(fill_draw(fill));
    }
    for image in &screen.images {
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        out.push(image_draw(image, placed));
    }
    if let Some(menu) = &screen.menu {
        let pitch = font_line_height(&menu.font) * face_scale(layout, &menu.font);
        let color = argb_to_rgba(menu.color);
        let align = match menu.align.to_ascii_lowercase().as_str() {
            "right" => Align::Right,
            "centre" | "center" => Align::Centre,
            _ => Align::Left,
        };
        for (index, option) in model.options().iter().enumerate() {
            let y = menu.y + index as f32 * pitch;
            out.push(Draw::Text {
                x: menu.x,
                y,
                scale: menu.scale * face_scale(layout, &menu.font),
                color: if index == model.index() {
                    MENU_SELECTED
                } else {
                    color
                },
                border: None,
                align,
                text: strings.get_or_id(option.idstring()).to_string(),
                wrap_width: None,
            });
        }
    }
    for text in &screen.texts {
        let name = text.name.as_deref().unwrap_or("");
        let content = match name {
            "GhostTime2" => model
                .new_best_lap_ticks
                .map(|ticks| format_ticks(u64::from(ticks))),
            // No on-disk ghost system yet - see the module doc.
            "GhostLine1" | "GhostTime1" => None,
            "GhostLine2" if model.new_best_lap_ticks.is_none() => None,
            _ => text.string.clone(),
        };
        let Some(content) = content else { continue };
        out.push(text_draw(text, &content, layout));
    }
    layers.body = out;
    layers
}

/// `Line1`'s own resolved text, or `None` for [`Headline::Unresolved`] - see
/// [`super::Headline`]'s own doc. `pub(super)` rather than private: HD's own
/// `Line1` authors no idstring at all (a runtime-filled placeholder), and
/// [`super::hd::hd_results_draw_list`] reuses this same idstring table as a
/// chosen substitution - see that module's own doc.
pub(super) fn headline_text(headline: Headline, strings: &StringTable) -> Option<String> {
    let id = match headline {
        Headline::TimeTrial => "ER_TT_COM",
        Headline::SpeedLap => "ER_SL_COM",
        Headline::NoPosition => "ER_SHIP_DES",
        Headline::Position(place) => ordinal_idstring(place)?,
        Headline::Unresolved => return None,
    };
    Some(strings.get_or_id(id).to_string())
}

/// `ER_1STP`..`ER_8THP`, 1-based - `None` outside that range, which cannot
/// happen for an 8-craft field but is not this function's job to assert.
pub(super) fn ordinal_idstring(place: u8) -> Option<&'static str> {
    Some(match place {
        1 => "ER_1STP",
        2 => "ER_2NDP",
        3 => "ER_3RDP",
        4 => "ER_4THP",
        5 => "ER_5THP",
        6 => "ER_6THP",
        7 => "ER_7THP",
        8 => "ER_8THP",
        _ => return None,
    })
}

/// `RewardLine1`'s own resolved text: `ER_GMA`/`ER_SMA`/`ER_BMA`/`ER_NMA`.
pub(super) fn medal_award_text(
    medal: Option<oag_tables::race_campaign::Medal>,
    strings: &StringTable,
) -> String {
    use oag_tables::race_campaign::Medal;
    let id = match medal {
        Some(Medal::Gold) => "ER_GMA",
        Some(Medal::Silver) => "ER_SMA",
        Some(Medal::Bronze) => "ER_BMA",
        None => "ER_NMA",
    };
    strings.get_or_id(id).to_string()
}

/// A `lap{n}.{c}` cell's own content - the header row, a real lap, the
/// totals row, or nothing (column 2, and any row past the totals one).
fn lap_cell_text(
    name: &str,
    model: &Results,
    strings: &StringTable,
    totals: bool,
) -> Option<String> {
    let (n, c) = lap_slot(name)?;
    if n == 0 {
        return match c {
            0 => Some(strings.get_or_id("RC_LAP").to_string()),
            1 => Some(strings.get_or_id("PRO_TIME").to_string()),
            _ => None,
        };
    }
    if n <= model.laps.len() {
        let split = model.laps[n - 1];
        return match c {
            0 => Some(split.lap.to_string()),
            1 => Some(format_ticks(u64::from(split.ticks))),
            // The `boostimg` column: the pads entered on this lap.
            2 => split.boosts.map(|boosts| boosts.to_string()),
            _ => None,
        };
    }
    if totals && n == model.laps.len() + 1 {
        return match c {
            0 => Some(strings.get_or_id("PRO_STATS_TOT").to_string()),
            1 => Some(format_ticks(model.total_ticks)),
            // `+0x1148`, the sum `Race_BuildEndRaceResult` accumulates over the laps
            // it reports - drawn only when every lap has a count to add up.
            2 => model
                .laps
                .iter()
                .map(|split| split.boosts)
                .sum::<Option<u32>>()
                .map(|total| total.to_string()),
            _ => None,
        };
    }
    None
}

/// `BigTopText`'s own text on Pulse's Tournament results screen -
/// `EndRaceResults_OnEnter`'s `case 4: case 0x10:` block: `ER_END_TOUR` on
/// the last leg, `"%s %d/%d"` of `ER_RES` and the leg counter otherwise.
fn tournament_big_top_text(model: &TournamentResults, strings: &StringTable) -> String {
    if model.last_leg {
        strings.get_or_id("ER_END_TOUR").to_string()
    } else {
        format!(
            "{} {}/{}",
            strings.get_or_id("ER_RES"),
            model.leg_number,
            model.leg_count
        )
    }
}

/// A `lap{n}.{c}` cell's own content on Pulse's Tournament standings table -
/// the header row (`PRO_POS`/`ER_TEAM`/`ER_POINTS`), or `rows[n - 1]`'s own
/// position/name/points. `rows` is already in the table's own display
/// order - [`TournamentResults::current`] - so this draws row `n` as-is,
/// the same "this function does not sort" reading
/// `EndRaceResults_PopulateTournamentTable`'s own doc gives.
fn tournament_lap_cell_text(
    name: &str,
    rows: &[TournamentRow],
    strings: &StringTable,
) -> Option<String> {
    let (n, c) = lap_slot(name)?;
    if n == 0 {
        return match c {
            0 => Some(strings.get_or_id("PRO_POS").to_string()),
            1 => Some(strings.get_or_id("ER_TEAM").to_string()),
            2 => Some(strings.get_or_id("ER_POINTS").to_string()),
            _ => None,
        };
    }
    let row = rows.get(n - 1)?;
    match c {
        0 => Some(n.to_string()),
        1 => row
            .team_name
            .as_deref()
            .map(|id| strings.get_or_id(id).to_string()),
        2 => Some(row.points.to_string()),
        _ => None,
    }
}

/// Ticks (this project's own fixed 60 Hz timestep - see
/// [ADR-0007](../../../../docs/architecture/adr/0007-fixed-timestep-vs-original.md))
/// to `M.SS.CC`, period-separated - the format the disc's own frames show
/// (`docs/ui/endrace-screens.md`), **not** the colon
/// `crate::campaign::draw::format_centiseconds` draws, which
/// `docs/ui/campaign-screens.md`'s own "Target times are formatted with
/// periods" finding already flags as wrong on that screen. Fixing that one
/// is `campaign`'s own lane; this is a second, correct formatter rather
/// than a shared one - see this module's own doc for why the small helpers
/// here duplicate rather than share.
pub(super) fn format_ticks(ticks: u64) -> String {
    let centiseconds = ticks.saturating_mul(100) / 60;
    let minutes = centiseconds / 6000;
    let seconds = (centiseconds / 100) % 60;
    let centis = centiseconds % 100;
    format!("{minutes}.{seconds:02}.{centis:02}")
}

pub(super) fn fill_draw(fill: &Fill) -> Draw {
    fill.draw()
}

pub(super) fn image_draw(image: &Image, placed: Placed) -> Draw {
    sprite_draw(image, placed, image.x, image.y, image.color)
}

/// Pixels of `loyaltybar` per loyalty point, `EndRaceRewards_Update`'s
/// `0.00124` literal: `100000` points (the ceiling) is the authored
/// `124`-pixel width, which is the only reason it looks like a fraction.
const LOYALTY_BAR_PIXELS_PER_POINT: f32 = 0.00124;

/// `loyaltybar`'s own draw: `width` pixels on screen *and* `width` texels of
/// the sheet, because the original writes the one number into both the
/// widget's width and its U extent. Cropping, not squashing - a clone with
/// both overridden reaches the same `sprite_draw` every other image on this
/// screen does, rather than a second copy of its UV logic for one widget.
fn loyalty_bar_draw(image: &Image, placed: Placed, width: f32) -> Draw {
    let mut cropped = image.clone();
    cropped.width = Some(width.max(0.0));
    cropped.texture_width = Some(width.max(0.0));
    sprite_draw(&cropped, placed, image.x, image.y, image.color)
}

fn sprite_draw(image: &Image, placed: Placed, x: f32, y: f32, argb: u32) -> Draw {
    let width = image.width.unwrap_or(placed.width as f32);
    let height = image.height.unwrap_or(placed.height as f32);
    let sampled = [
        image.texture_width.unwrap_or(placed.width as f32),
        image.texture_height.unwrap_or(placed.height as f32),
    ];
    let color = argb_to_rgba(argb);
    if sampled[0] > placed.width as f32 + 0.5 || sampled[1] > placed.height as f32 + 0.5 {
        return Draw::TiledSprite {
            rect: [x, y, width, height],
            uv: [
                placed.x as f32,
                placed.y as f32,
                placed.width as f32,
                placed.height as f32,
            ],
            repeat: [
                sampled[0] / placed.width.max(1) as f32,
                sampled[1] / placed.height.max(1) as f32,
            ],
            color,
        };
    }
    Draw::Sprite {
        rect: [x, y, width, height],
        uv: [
            placed.x as f32 + image.u.unwrap_or(0.0),
            placed.y as f32 + image.v.unwrap_or(0.0),
            sampled[0],
            sampled[1],
        ],
        color,
    }
}

pub(super) fn text_draw(text: &Text, content: &str, layout: &Layout) -> Draw {
    Draw::Text {
        x: text.x,
        y: text.y,
        scale: text.scale * face_scale(layout, &text.font),
        color: argb_to_rgba(text.color),
        border: None,
        align: match text.align.to_ascii_lowercase().as_str() {
            "right" => Align::Right,
            "centre" | "center" => Align::Centre,
            _ => Align::Left,
        },
        text: content.to_string(),
        wrap_width: text.wrap_width,
    }
}
