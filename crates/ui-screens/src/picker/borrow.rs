//! `Team SelectionSplit` on a pressing that authors none, **borrowed from the PS2
//! pressing's design**.
//!
//! Only the PS2's `Selection_Definition.xml` authors the shared two-person ship page (see
//! [`super::split`]). The maintainer's rule (2026-10-13) is that anything for Pulse is the
//! same on both pressings, so the PSP builds the page from its own sprites with the PS2's
//! layout numbers rescaled from the PS2's 640x448 space into its own 480x272 grid, and
//! reads it back through the very function that reads the PS2's, [`Layout::read_split`].
//! **That placement is borrowed, not measured on the PSP**: no PSP capture shows it, so it
//! carries no confidence score, and the loader's report line says so.
//!
//! The numbers are [`PS2_SETS`] and the fixed shape of one set, [`set_xml`]: each of the two
//! players' sets is the single-player screen's own widgets laid out tighter (stat rows 16
//! apart where the single screen's are 25). `crates/ui-screens/tests/split_selection_ground_truth.rs`
//! builds the screen in the PS2's own grid and compares it, widget by widget, with the file.
//!
//! **Chosen, not measured:** the PSP's faces are as tall in 272 rows as the PS2's in 448, so
//! the stat rows' text is drawn at the grid ratio (`FIT`) to keep the PS2's room for it.

use std::collections::HashMap;

use oag_ui::language::StringTable;
use oag_ui::screen::Screens;

use super::{FaceScales, Layout};

/// The grid the PS2 pressing authors in.
pub const PS2_GRID: [f32; 2] = [640.0, 448.0];

/// Where one player's set sits in [`PS2_GRID`]: the arrows' and counter's `Item OffsetY`
/// and their own `y`s, and the stat block's `Item OffsetY`.
#[derive(Debug, Clone, Copy)]
pub struct Set {
    pub arrows_item: f32,
    pub up: f32,
    pub down: f32,
    pub counter: f32,
    pub stats_item: f32,
}

/// `Team SelectionSplit`'s two sets: `Player 1` first (`Item OffsetY -15`, stats 79) and
/// `Player 2` (181, 274).
pub const PS2_SETS: [Set; 2] = [
    Set {
        arrows_item: -15.0,
        up: 69.0,
        down: 209.0,
        counter: 206.0,
        stats_item: 79.0,
    },
    Set {
        arrows_item: 181.0,
        up: 49.0,
        down: 189.0,
        counter: 186.0,
        stats_item: 274.0,
    },
];

/// The stat block's `Item OffsetX`.
const STATS_X: f32 = 387.0;

const RULE_L: &str =
    r#"Color1="0x00ffffff" Color2="0x00ffffff" Color3="0x3fffffff" Color4="0x3fffffff""#;
const RULE_R: &str =
    r#"Color1="0x3fffffff" Color2="0x3fffffff" Color3="0x00ffffff" Color4="0x00ffffff""#;
const WHITE_L: &str =
    r#"Color1="0x00ffffff" Color2="0x00ffffff" Color3="0xffffffff" Color4="0xffffffff""#;
const WHITE_R: &str =
    r#"Color1="0xffffffff" Color2="0xffffffff" Color3="0x00ffffff" Color4="0x00ffffff""#;
const PANEL: &str =
    r#"Color1="0x2f000000" Color2="0x2f000000" Color3="0x2f000000" Color4="0x2f000000""#;
const ASSETS: &str = r#"src="Data\FE\Images\pulse_assets.mip""#;

/// One stat row's two fading rules, nested at `OffsetY="1"` under the row.
fn rules(x: impl Fn(f32) -> f32, y: impl Fn(f32) -> f32, height: f32) -> String {
    format!(
        r#"<Item OffsetY="{one}"><Image><Values width="{w}" height="{h}" {RULE_L}></Values></Image><Image><Values x="{w}" width="{w}" height="{h}" {RULE_R}></Values></Image></Item>"#,
        one = y(1.0),
        w = x(113.0),
        h = y(height),
    )
}

/// The scale the stat rows' text is drawn at on a grid smaller than the PS2's.
fn fit(k: [f32; 2], to: [f32; 2]) -> f32 {
    if to[1] < PS2_GRID[1] { k[1] } else { 1.0 }
}

/// One player's widgets in the XML the PS2 file writes them in, every position and size
/// scaled by `k` (`x` by `k[0]`, `y` by `k[1]`).
fn set_xml(player: usize, set: &Set, k: [f32; 2], text_scale: f32) -> String {
    let n = player;
    let (x, y) = (|v: f32| v * k[0], |v: f32| v * k[1]);
    let arrow = |name: &str, at: f32, v: u32| {
        format!(
            r#"<Image name="{name}{n}"><Values x="{}" y="{}" u="242" v="{v}" width="{}" height="{}" txtrwidth="17" txtrheight="14" color="0xffffffff" {ASSETS}></Values></Image>"#,
            x(156.0),
            y(at),
            x(23.0),
            y(23.0),
        )
    };
    let mut xml = format!(
        r#"<Item OffsetX="0" OffsetY="{}">
{}
{}
<Text name="honey{n}"><Values string="1/1" x="{}" y="{}" color="0xffffffff"></Values></Text>
</Item>
<Item OffsetX="{}" OffsetY="{}">
"#,
        y(set.arrows_item),
        arrow("up arrow", set.up, 71),
        arrow("down arrow", set.down, 94),
        x(180.0),
        y(set.counter),
        x(STATS_X),
        y(set.stats_item),
    );
    let stat = |row: f32, bar: &str, value: &str, label: &str| {
        format!(
            r#"<Item OffsetY="{row}">
<Image><Values x="{bx}" y="{by}" width="{bw}" height="{bh}" Color="0x7fffffff" U="437" V="1" TxtrWidth="58" TxtrHeight="10" {ASSETS}></Values></Image>
<Image name="{bar} Bar{n}"><Values x="{bx}" y="{by}" width="{bw}" height="{bh}" Color="0xffffffff" U="437" V="1" TxtrWidth="58" TxtrHeight="10" {ASSETS}></Values></Image>
<Text name="{value}{n}"><Values align="right" string="10" font="default" scale="{text_scale}" x="{vx}" color="0xffffffff"></Values></Text>
<Text><Values idstring="{label}" font="default" scale="{text_scale}" color="0xff34ACC2"></Values></Text>
{rules}</Item>
"#,
            row = y(row),
            bx = x(87.0),
            by = y(2.0),
            bw = x(77.0),
            bh = y(16.0),
            vx = x(193.0),
            rules = rules(x, y, 16.0),
        )
    };
    xml.push_str(&stat(0.0, "Speed", "Speed", "RC_SPEED"));
    xml.push_str(&stat(16.0, "Thrust", "Thrust", "RC_THRUST"));
    xml.push_str(&stat(32.0, "Handling", "Handling", "RC_HANDLING"));
    xml.push_str(&stat(48.0, "Shield", "Shield", "RC_SHIELD"));
    xml.push_str(&format!(
        r#"<Item OffsetY="{}">
<Text><Values idstring="ER_LOY" font="default" scale="{text_scale}" color="0xff34ACC2"></Values></Text>
<Text name="Loyalty{n}"><Values align="right" string="100000" font="default" scale="{text_scale}" x="{}" color="0xffffffff"></Values></Text>
{}</Item>
<Item OffsetY="{}">
<Image><Values x="0" y="{}" width="{}" height="{}" Color="0x7fffffff" U="332" V="72" TxtrWidth="148" TxtrHeight="10" {ASSETS}></Values></Image>
<Image name="Loyalty Bar{n}"><Values x="0" y="{}" width="{}" height="{}" Color="0xffffffff" U="332" V="72" TxtrWidth="148" TxtrHeight="10" {ASSETS}></Values></Image>
{}</Item>
"#,
        y(64.0),
        x(193.0),
        rules(x, y, 16.0),
        y(80.0),
        y(2.0),
        x(197.0),
        y(16.0),
        y(2.0),
        x(197.0),
        y(16.0),
        rules(x, y, 18.0),
    ));
    xml.push_str(&format!(
        r#"<Image name="Infohexgrid"><Values y="{top}" width="{pw}" height="{ph}" U="0" V="0" TxtrWidth="340" TxtrHeight="88" Color="0x7fffffff" src="Data\FE\Images\hex_bg.mip"></Values></Image>
<Item OffsetX="{lx}" OffsetY="{top}">
<Text name="skin{n}"><Values string="Alternative" x="{sx}" y="{sy}" color="FEGlobals->InfoHexBGColor">
</Values><Image name="skin left arrow{n}"><Values x="{lax}" y="{ay}" u="288" v="75" width="{aw}" height="{ah}" txtrwidth="12" txtrheight="12" color="0xffffffff" {ASSETS}></Values></Image><Image name="skin right arrow{n}"><Values x="{rax}" y="{ay}" u="312" v="75" width="{aw}" height="{ah}" txtrwidth="12" txtrheight="12" color="0xffffffff" {ASSETS}></Values></Image></Text>
<Text name="skin fixed{n}"><Values string="Zone Model" x="0" y="{fy}" color="FEGlobals->InfoHexBGColor"></Values></Text>
</Item>
<Image name="Infogradient"><Values y="{top}" width="{pw}" height="{ph}" {PANEL}></Values></Image>
<Item OffsetY="{line}"><Image><Values width="{w}" height="{t}" {WHITE_L}></Values></Image><Image><Values x="{w}" width="{w}" height="{t}" {WHITE_R}></Values></Image></Item>
</Item>
"#,
        top = y(-70.0),
        pw = x(227.0),
        ph = y(72.0),
        lx = x(19.0),
        sx = x(20.0),
        sy = y(26.0),
        lax = x(-2.0),
        rax = x(180.0),
        ay = y(31.0),
        aw = x(16.0),
        ah = y(16.0),
        fy = y(43.0),
        line = y(-16.0),
        w = x(113.0),
        t = y(2.0),
    ));
    xml
}

/// `Team SelectionSplit` in `to`'s grid, as the PS2 file would write it there.
#[must_use]
pub fn team_selection_split_xml(to: [f32; 2]) -> String {
    let k = [to[0] / PS2_GRID[0], to[1] / PS2_GRID[1]];
    let text_scale = fit(k, to);
    let mut xml = String::from(
        r#"<Screen name="Top"><Screen type="TeamSelection" name="Team SelectionSplit" NavigationButtons="2">
<Text><Values idstring="RC_SHIPSEL" font="Title" x="0" y="0"></Values></Text>
<LeftLayer transition="0.5">
"#,
    );
    for (player, set) in PS2_SETS.iter().enumerate() {
        xml.push_str(&set_xml(player, set, k, text_scale));
    }
    xml.push_str(&format!(
        r#"<Item OffsetX="{}" OffsetY="{}"><Item OffsetY="{}"><Text name="Suggest"><Values idstring="ER_FOR_SHIP" font="default" x="{}" color="0xffffffff"></Values></Text></Item></Item>
</LeftLayer>
</Screen></Screen>"#,
        STATS_X * k[0],
        74.0 * k[1],
        242.0 * k[1],
        19.0 * k[0],
    ));
    xml
}

/// The two halves of `Team SelectionSplit` built from [`PS2_SETS`] for the grid `grid`
/// (a pressing's own, 480x272 for the PSP), read back as the PS2's are. `globals` are the
/// front end's `FEGlobals`, which the colours of the skin row name.
#[must_use]
pub fn read_borrowed(
    globals: &HashMap<String, String>,
    strings: &StringTable,
    faces: FaceScales,
    grid: [f32; 2],
) -> Option<[Layout; 2]> {
    let globals: Vec<(&str, &str)> = globals
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let k = [grid[0] / PS2_GRID[0], grid[1] / PS2_GRID[1]];
    let fit = fit(k, grid);
    Layout::read_split(
        &Screens::from_xml_with_fallback_globals(&team_selection_split_xml(grid), &globals),
        strings,
        faces,
        grid,
    )
    .map(|halves| {
        halves.map(|half| Layout {
            name_scale: fit,
            ..half
        })
    })
}

#[cfg(test)]
mod tests;
