//! Wipeout HD/Fury's own `Team Selection` - the ship screen between `Cell
//! Selection` and the race, and the same name Pulse's is authored under, read
//! off a different file in a different dialect.
//!
//! # Where it is, and that it is live
//!
//! `Data\Plugins\Frontend\Gui\Team_Selection_Definition.xml`, which only
//! `DATA06` carries. `DATA00`'s own `skin.xml` - the live front-end root, see
//! `docs/formats/hd-frontend.md` - includes it by `SrcRel` and does **not**
//! include `DATA02`/`DATA03`/`DATA05`'s older `Selection_Definition.xml`,
//! whose own `Team Selection` is therefore dead data on this pressing. Both
//! copies of `CellMode_Definition.xml` redirect `Cell Selection` to `Team
//! Selection` unconditionally (`Cell Mode Redirect Team`; the direct
//! `Launch Game` redirect is commented out beside a note that the ship
//! screen is now always shown). See `docs/ui/campaign-screens.md`'s "Wipeout
//! HD/Fury: `Team Selection`" section for the RPCS3 walks that confirm it.
//!
//! # The nested screen
//!
//! Every widget sits on `Team Selection Top Level`; the `Team Selection`
//! screen nested inside it holds only its title and its redirects, one per
//! player count. [`oag_ui::screen::Screens::collect`] flattens the two into
//! separate entries, so [`read`] merges the parent's widgets under the
//! child's name.
//!
//! # What the generic parser does not reach
//!
//! `MiniText`, `Bracket`, `Model` and `HexSelection` have no arm in
//! [`oag_ui::screen::Screens::collect`] - a generic `MiniText` arm was tried
//! once and reverted because it would start drawing widgets on every title's
//! screens - so [`read`] walks this one file's own tree for them
//! ([`TeamScreen`]), with every enclosing `Item`'s `OffsetX`/`OffsetY` folded
//! in the same way the generic parser folds them.
//!
//! # What this draws, and what it does not
//!
//! Drawn, off the disc: the `NAVIGATE TEAM` honeycomb ([`hex`]), the title,
//! the four `MiniText` headings, the `LogoOutline` frame, the selected team's own `Data\Ships\<team>\FE\Logo.gtf`
//! in the `Logo` widget's rect, the four `STATISTICS` blocks with their
//! value labels and down-nobbles, and the `LOYALTY` block's box and label.
//!
//! Not drawn, and said so:
//!
//! - **The 3-D ship** is the team's race hull (`ship.vex` plus
//!   `ship.rcsmodel`; HD ships no `ship_FE.vex`), drawn by `oag_game` at a
//!   fixed pose in this screen's `SHIP MODEL` frame. The pose is chosen, not
//!   measured: the widget's own `ShipModel` values ([`ShipModel`]) are read
//!   and not yet composed.
//! - **The `Bracket` corner marks**, the `Padlock` and the `Unlockcondition`
//!   text (this build keeps no unlock state), and the loyalty value, its
//!   nobbles. `LiveryString` is authored empty and filled by code this
//!   build has not read; the honeycomb's cursor row shows the chosen model
//!   instead, so nothing stands in for it.

use oag_ui::frontend::{Align, Draw, Placed};
use oag_ui::language::StringTable;
use oag_ui::menu::Frame;
use oag_ui::screen::{BlockWidget, Node, Screen, Screens, Text, argb_to_rgba, parse, parse_argb};

use super::{Details, FaceScales, Layout, Picker};

pub mod hex;
pub mod track;

/// The screen every widget is authored on.
pub const TOP_LEVEL: &str = "Team Selection Top Level";
/// The single-player screen nested inside [`TOP_LEVEL`] - the one `Cell
/// Selection` redirects to.
pub const SCREEN: &str = "Team Selection";

/// A team model's four front-end ratings, in **tenths** of the authored
/// `0..=10` scale - `thrust="8.5"` is `85`. HD's own screen prints each as
/// exactly this number, three digits wide: Feisar's `normal` reads `070`,
/// `080`, `100`, `080` on an RPCS3 frame of this screen, its `concept1`
/// `080`, `085`, `080` for speed, thrust and shield.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub speed: u8,
    pub thrust: u8,
    pub handling: u8,
    pub shield: u8,
}

impl Stats {
    /// In the order the screen's `Slide_0`..`Slide_3` blocks are authored:
    /// `RC_SPEED`, `RC_THRUST`, `RC_HANDLING`, `RC_SHIELD`.
    #[must_use]
    pub fn values(self) -> [u8; 4] {
        [self.speed, self.thrust, self.handling, self.shield]
    }
}

/// A `<MiniText>` heading, resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct MiniText {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub color: u32,
}

/// A `<Bracket>`: a framed region, drawn by the widget class as corner marks
/// this build does not reproduce. Kept for its rect, which places the
/// preview and the pointer targets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bracket {
    pub rect: [f32; 4],
    /// `middle="true"`: marks at each edge's midpoint too. Only the `SHIP
    /// MODEL` frame authors it.
    pub middle: bool,
}

/// The `<Model name="ShipModel">` widget's own values - where and how the
/// original poses the selected craft. Read and carried; nothing draws it
/// yet (see the module doc).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShipModel {
    pub origin: [f32; 2],
    pub z: f32,
    pub rot_x: f32,
    pub rot_y: f32,
}

/// Everything [`oag_ui::screen::Screens::collect`] does not reach on this
/// screen - see the module doc.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TeamScreen {
    pub labels: Vec<MiniText>,
    pub brackets: Vec<Bracket>,
    pub ship_model: Option<ShipModel>,
    /// The `Logo` image's rect. It authors no `src`: the original assigns
    /// the selected team's own `FE\Logo.gtf` at runtime, which is the only
    /// per-team texture of that name on the disc (twelve, one per team).
    pub logo: Option<[f32; 4]>,
    /// `AlwaysSolidColor` per `Block` name - the darker remainder of a stat
    /// bar, `0xff646464` on all five.
    pub solid: Vec<(String, u32)>,
    /// The `NAVIGATE TEAM` honeycomb - see [`hex`].
    pub hex: Option<hex::HexGrid>,
}

/// Reads HD's `Team Selection` off its own definition file: `xml` is the
/// expanded text, `screens` the same file parsed with the front end's
/// globals (for every `FEGlobals->` colour).
///
/// `None` when either screen is missing - a source that authors none, which
/// leaves the caller with no picker, as [`Layout::read`] does.
#[must_use]
pub fn read(
    xml: &str,
    screens: &Screens,
    strings: &StringTable,
    faces: FaceScales,
    grid: [f32; 2],
) -> Option<Layout> {
    let parent = screens.by_name(TOP_LEVEL)?;
    let child = screens.by_name(SCREEN)?;
    let mut screen: Screen = parent.clone();
    screen.name = child.name.clone();
    screen.kind.clone_from(&child.kind);
    screen.path.clone_from(&child.path);
    screen.texts.extend(child.texts.iter().cloned());
    screen.redirects.clone_from(&child.redirects);
    for text in &mut screen.texts {
        if let Some(id) = text.idstring.as_deref()
            && let Some(resolved) = strings.get(id)
        {
            text.string = Some(resolved.to_string());
        }
    }
    // The child's own `Title`-face text (`RC_SHIPSEL`), not the parent's:
    // `netLobbyStatus` is authored in the same face, empty, and first.
    let title = child
        .texts
        .iter()
        .find(|text| text.font.eq_ignore_ascii_case("Title"))
        .and_then(|text| text.idstring.as_deref())
        .map_or_else(
            || SCREEN.to_string(),
            |id| strings.get_or_id(id).to_string(),
        );

    let root = parse(xml);
    let top = find_screen(&root, TOP_LEVEL)?;
    let mut extra = TeamScreen::default();
    walk(top, (0.0, 0.0), screens, strings, &mut extra);

    // The first bracket frames `CHOOSE TEAM` - the logo - and is where a
    // click steps the team; the one authoring `middle` frames `SHIP MODEL`
    // and is where the preview would go. Both are the disc's own rects.
    let panel = extra.brackets.first().map_or([0.0; 4], |b| b.rect);
    let preview = extra
        .brackets
        .iter()
        .find(|b| b.middle)
        .map_or(panel, |b| b.rect);
    Some(Layout {
        title,
        screen,
        preview,
        panel,
        faces,
        scale: [grid[0] / super::PSP_GRID[0], grid[1] / super::PSP_GRID[1]],
        hd: Some(Box::new(extra)),
        hd_track: None,
    })
}

fn find_screen<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    if node.name.eq_ignore_ascii_case("Screen") && node.attr("name") == Some(name) {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| find_screen(child, name))
}

/// One node's children, with `offset` the sum of every enclosing element's
/// own `OffsetX`/`OffsetY`. A nested `<Screen>` is another screen's and is
/// not entered.
fn walk(
    node: &Node,
    offset: (f32, f32),
    screens: &Screens,
    strings: &StringTable,
    out: &mut TeamScreen,
) {
    let number = |node: &Node, attr: &str| screens.number(node.value(attr));
    let color = |node: &Node, attr: &str| {
        screens
            .resolve(node.value(attr).unwrap_or_default())
            .and_then(parse_argb)
    };
    for child in &node.children {
        let name = child.name.to_ascii_lowercase();
        if name == "screen" || name == "values" {
            continue;
        }
        let inner = (
            offset.0 + number(child, "OffsetX").unwrap_or(0.0),
            offset.1 + number(child, "OffsetY").unwrap_or(0.0),
        );
        let x = inner.0 + number(child, "x").unwrap_or(0.0);
        let y = inner.1 + number(child, "y").unwrap_or(0.0);
        match name.as_str() {
            "minitext" => {
                if let Some(id) = child.value("idstring") {
                    out.labels.push(MiniText {
                        x,
                        y,
                        text: strings.get_or_id(id).to_string(),
                        color: color(child, "color").unwrap_or(0xffff_ffff),
                    });
                }
            }
            "bracket" => out.brackets.push(Bracket {
                rect: [
                    x,
                    y,
                    number(child, "Width").unwrap_or(0.0),
                    number(child, "Height").unwrap_or(0.0),
                ],
                middle: child.flag("middle").unwrap_or(false),
            }),
            "model" if child.attr("name") == Some("ShipModel") => {
                out.ship_model = Some(ShipModel {
                    origin: [
                        number(child, "OriginX").unwrap_or(0.0),
                        number(child, "OriginY").unwrap_or(0.0),
                    ],
                    z: number(child, "z").unwrap_or(0.0),
                    rot_x: number(child, "RotX").unwrap_or(0.0),
                    rot_y: number(child, "RotY").unwrap_or(0.0),
                });
            }
            "hexselection" => {
                let count = |attr: &str| number(child, attr).map_or(0, |n| n as usize);
                let colour = |attr: &str| {
                    child
                        .value(attr)
                        .and_then(|raw| screens.resolve(raw))
                        .and_then(parse_argb)
                };
                out.hex = Some(hex::HexGrid {
                    origin: [x, y],
                    columns: count("columns"),
                    rows: count("rows"),
                    hex: colour("HexCol").unwrap_or(0x6480_8080),
                    hex_fade: colour("HexColFade").unwrap_or(0x3280_8080),
                    selected_column: colour("SelectedColumnCol").unwrap_or(0x64ff_0000),
                    selected_lock: colour("SelectedLockCol").unwrap_or(0xff80_8080),
                    selected_lock_fade: colour("SelectedLockColFade").unwrap_or(0xff24_2424),
                });
            }
            "image" if child.attr("name") == Some("Logo") => {
                out.logo = Some([
                    x,
                    y,
                    number(child, "width").unwrap_or(0.0),
                    number(child, "height").unwrap_or(0.0),
                ]);
            }
            "block" => {
                if let (Some(block), Some(solid)) =
                    (child.attr("name"), color(child, "AlwaysSolidColor"))
                {
                    out.solid.push((block.to_string(), solid));
                }
            }
            _ => {}
        }
        walk(child, inner, screens, strings, out);
    }
}

/// How large a `MiniText` heading draws against the menu face, and how far
/// its text sits right of its square bullet - **read off the RPCS3 frame**
/// (`screen-Team-Selection.png`, `racebox` walk): the headings stand about
/// 0.45 of a `default`-face line tall, the bullet a square about 14 units a
/// side at the widget's own `x`, the text starting 20 units right of it.
/// The widget class that draws them is unread.
const MINI_SCALE: f32 = 0.45;
const MINI_BULLET: f32 = 14.0;
const MINI_TEXT_INSET: f32 = 20.0;

/// The block names the four ratings draw on, in [`Stats::values`] order,
/// and the loyalty block beside them.
const STAT_BLOCKS: [&str; 4] = ["Slide_0", "Slide_1", "Slide_2", "Slide_3"];
const LOYALTY_BLOCK: &str = "Slide_4";

/// The body of the screen - see the module doc for what is and is not here.
pub(super) fn body(
    picker: &Picker,
    layout: &Layout,
    extra: &TeamScreen,
    frame: &Frame,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Vec<Draw> {
    let mut out = Vec::new();
    let screen = &layout.screen;
    for fill in &screen.fills {
        out.push(super::fill_draw(fill));
    }
    for image in &screen.images {
        let name = image.name.as_deref().unwrap_or("");
        // Placed per stat below, or never drawn - see the module doc.
        if name.starts_with("Nobble_") || name.starts_with("UnlockNobble") || name == "Padlock" {
            continue;
        }
        if let Some(placed) = sprites(&image.src) {
            out.push(sprite(image, placed, None));
        }
    }
    let Some(entry) = picker.selected() else {
        return out;
    };
    if let Some(rect) = extra.logo
        && let Some(placed) = sprites(&logo_src(&entry.id))
    {
        out.push(Draw::Sprite {
            rect,
            uv: [
                placed.x as f32,
                placed.y as f32,
                placed.width as f32,
                placed.height as f32,
            ],
            color: [1.0, 1.0, 1.0, 1.0],
        });
    }
    draw_labels(&extra.labels, &mut out);
    if let Some(grid) = &extra.hex {
        hex::draw(grid, picker, sprites, &mut out);
    }

    let stats = selected_stats(picker);
    for (index, name) in STAT_BLOCKS.iter().enumerate() {
        let Some(block) = find_block(screen, name) else {
            continue;
        };
        let value = stats.map(|stats| stats.values()[index]);
        stat_bar(block, value, layout, extra, frame, sprites, &mut out);
    }
    if let Some(block) = find_block(screen, LOYALTY_BLOCK) {
        stat_bar(block, None, layout, extra, frame, sprites, &mut out);
    }
    out
}

/// The `MiniText` headings: a square bullet and the text beside it.
fn draw_labels(labels: &[MiniText], out: &mut Vec<Draw>) {
    for label in labels {
        let color = argb_to_rgba(label.color);
        out.push(Draw::Fill {
            rect: [label.x, label.y, MINI_BULLET, MINI_BULLET],
            color,
        });
        out.push(Draw::Text {
            x: label.x + MINI_TEXT_INSET,
            y: label.y,
            scale: MINI_SCALE,
            color,
            border: None,
            align: Align::Left,
            text: label.text.clone(),
            wrap_width: None,
        });
    }
}

/// The selected livery's own ratings, where the entry carries them.
fn selected_stats(picker: &Picker) -> Option<Stats> {
    match picker.selected().map(|entry| &entry.details) {
        Some(Details::Ship { stats, .. }) => {
            let variant = picker
                .variant()
                .and_then(|chosen| picker.variants().iter().position(|v| v == chosen))
                .unwrap_or(0);
            stats.get(variant).copied().flatten()
        }
        _ => None,
    }
}

/// `Data\Ships\<team>\FE\Logo.gtf` - the texture the `Logo` widget shows for
/// `team`. Named by no widget; see [`TeamScreen::logo`]. `pub` so the boot
/// can put all twelve on the sheet.
#[must_use]
pub fn logo_src(team: &str) -> String {
    format!(r"Data\Ships\{team}\FE\Logo.gtf")
}

/// One statistics block: the box, its label, and - where `value` is known -
/// the bar split at `value / 100` with the down-nobble and three-digit value
/// above the split.
///
/// **Measured off the settled RPCS3 frame** (`racebox` walk, Feisar
/// `normal`): the split sits where the nobble does, at `0.69`, `0.79` and
/// `1.0` of the block for `070`, `080` and `100`. Left of it the inside is
/// `102/255` (`HD_Grey`, opaque); right of it `65/255` over the black page,
/// which is the block's own `AlwaysSolidColor` (`100/255`) through
/// `Block_Render`'s translucent two-pass fill - `100 x 0.676 = 67.6` (see
/// `oag_ui::menu::block`'s module doc for the `0.676`), as `102` is `HD_Grey`'s
/// `150` through the same fill. The light part's own top band steps down
/// before the split, the landing a shaped block draws. So this draws the
/// block's inside in those two colours, split there
/// (`oag_ui::menu::block::draw_split_fill`), and the whole block's border
/// over both. **That composition is chosen**: every piece is
/// `Block_Render`'s own, how the widget combines them is unread. Where there is no value (the
/// loyalty block) the block draws in its own colour, as any block does.
fn stat_bar(
    block: &BlockWidget,
    value: Option<u8>,
    layout: &Layout,
    extra: &TeamScreen,
    frame: &Frame,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    out: &mut Vec<Draw>,
) {
    let name = block.name.as_deref().unwrap_or("");
    let solid = extra
        .solid
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, solid)| *solid);
    let split = value.map(|value| block.x + block.width * (f32::from(value) / 100.0).min(1.0));
    if let Some(art) = frame.blocks {
        let whole = oag_ui::menu::block::Block {
            x: block.x,
            y: block.y,
            width: block.width,
            height: block.height,
            color: argb_to_rgba(block.color),
            landing: block.landing,
        };
        match (split, solid) {
            (Some(split), Some(solid)) => {
                oag_ui::menu::block::draw_split_fill(&whole, split, argb_to_rgba(solid), &art, out);
                oag_ui::menu::block::draw_border(&whole, &art, (1.0, 1.0), out);
            }
            _ => oag_ui::menu::block::draw(&whole, &art, (1.0, 1.0), out),
        }
    }
    if let Some(label) = find_text(&layout.screen, name) {
        let content = label.string.clone().unwrap_or_default();
        let mut positioned = label.clone();
        positioned.x = block.x + 40.0;
        positioned.y = block.y + 3.0;
        positioned.scale = block.text_scale;
        out.push(text_draw(&positioned, &content, layout));
    }
    let (Some(value), Some(split)) = (value, split) else {
        return;
    };
    let index = name.trim_start_matches("Slide_");
    if let Some(nobble) = layout
        .screen
        .images
        .iter()
        .find(|image| image.name.as_deref() == Some(&format!("Nobble_{index}")))
        && let Some(placed) = sprites(&nobble.src)
    {
        let width = nobble.width.unwrap_or(placed.width as f32);
        let mut moved = nobble.clone();
        moved.x = split - width;
        out.push(sprite(&moved, placed, None));
    }
    if let Some(text) = find_text(&layout.screen, &format!("Label_{index}")) {
        let mut moved = text.clone();
        let nobble_width = 16.0;
        moved.x = split - nobble_width - 2.0;
        out.push(text_draw(&moved, &format!("{value:03}"), layout));
    }
}

fn find_block<'a>(screen: &'a Screen, name: &str) -> Option<&'a BlockWidget> {
    screen
        .blocks
        .iter()
        .find(|block| block.name.as_deref() == Some(name))
}

fn find_text<'a>(screen: &'a Screen, name: &str) -> Option<&'a Text> {
    screen
        .texts
        .iter()
        .find(|text| text.name.as_deref() == Some(name))
}

fn sprite(image: &oag_ui::screen::Image, placed: Placed, rect: Option<[f32; 4]>) -> Draw {
    let width = image.width.unwrap_or(placed.width as f32);
    let height = image.height.unwrap_or(placed.height as f32);
    Draw::Sprite {
        rect: rect.unwrap_or([image.x, image.y, width, height]),
        uv: super::image_uv(image, placed),
        color: argb_to_rgba(image.color),
    }
}

fn text_draw(text: &Text, content: &str, layout: &Layout) -> Draw {
    Draw::Text {
        x: text.x,
        y: text.y,
        scale: text.scale * layout.face_scale(&text.font),
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

#[cfg(test)]
mod tests;
