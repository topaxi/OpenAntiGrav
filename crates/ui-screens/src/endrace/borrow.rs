//! `EndRace Results Split` and `EndRace Rewards Split` on a pressing that authors neither,
//! **borrowed from the PS2 pressing's design**.
//!
//! Only the PS2's `EndRace_Definition.xml` authors the two-person screens. The maintainer's
//! rule (2026-10-13) is that anything for Pulse is the same on both pressings: the PSP
//! builds them from its own assets with the PS2's layout numbers rescaled from the PS2's
//! 640x448 space into its own 480x272 grid. **That placement is borrowed, not measured on
//! the PSP**: no PSP capture shows it, so it carries no confidence score, and the loader
//! says so in its report line.
//!
//! - **`Results Split`** is generated from [`PS2_TABLE`] (the numbers the PS2 file authors
//!   per person), the PSP's own sprites and its own `Line1`/`ContinueButton` text.
//! - **`Rewards Split`** is the PSP's own single `EndRace Rewards` panel, stamped twice at
//!   the PS2's offsets: the PS2's single panel is the PSP's scaled by the same ratio (its
//!   `BGgradient` at 91 against the PSP's 55, `MedalImg` 43x53 against 32x32), so nothing
//!   about a panel is transcribed.
//!
//! The numbers are re-derivable: both functions take the grid they express their layout
//! in, and `crates/game/tests/endrace_split_ground_truth.rs` builds them in the PS2's own
//! space and compares every widget with the real file.

use oag_ui::screen::{Screen, Screens};

use super::Layout;

/// The grid the PS2 pressing authors in: PAL's 640x448.
pub const PS2_GRID: [f32; 2] = [640.0, 448.0];

/// What the PS2's `EndRace Results Split` authors for one person's lap table, in
/// [`PS2_GRID`]. Read off `Data\Plugins\PI001\GUI\EndRace_Definition.xml` in `WADS2.WAD`
/// (`<Item name="Player_0" OffsetY="-96">`, `Player_1` at `110`).
#[derive(Debug, Clone, Copy)]
pub struct Table {
    /// Each person's `<Item OffsetY>`.
    pub item_y: [f32; 2],
    /// `Line1_N`: the centred headline.
    pub headline: (f32, f32),
    /// `zonetopline`'s `y`, and the rule's thickness.
    pub top_rule: (f32, f32),
    /// `tablebg1`'s `OffsetY` and the distance to the next row's.
    pub first_row: f32,
    pub row_pitch: f32,
    /// Row 1's fill is `[0, first_height)`, a later row's `[row_top, row_top + row_height)`.
    pub first_height: f32,
    pub row_top: f32,
    pub row_height: f32,
    /// The `y` of the two fading rules under a row.
    pub row_rule: f32,
    /// `topbarcenter`: `y` and height.
    pub bar: (f32, f32),
    /// The three columns' `x`.
    pub columns: [f32; 3],
    /// The header row's cell `y`, the first lap row's, and the step between them.
    pub header_cell: f32,
    pub first_cell: f32,
    pub cell_pitch: f32,
    /// `boostimg`: `x`, `y`, width, height.
    pub boost: [f32; 4],
    /// `tablehighlight`'s height.
    pub highlight_height: f32,
}

/// The PS2 file's numbers.
pub const PS2_TABLE: Table = Table {
    item_y: [-96.0, 110.0],
    headline: (320.0, 106.0),
    top_rule: (150.0, 2.0),
    first_row: 152.0,
    row_pitch: 25.0,
    first_height: 28.0,
    row_top: 2.0,
    row_height: 23.0,
    row_rule: 25.0,
    bar: (124.0, 41.0),
    columns: [187.0, 293.0, 427.0],
    header_cell: 131.0,
    first_cell: 154.0,
    cell_pitch: 25.0,
    boost: [427.0, 127.0, 16.0, 20.0],
    highlight_height: 25.0,
};

/// `EndRace Rewards Split`'s panel offsets in [`PS2_GRID`] (`PlayerRW_0`, `PlayerRW_1`) and
/// the `y` of the `ER_REWARD` header each carries inside its panel.
pub const PS2_PANEL_Y: [f32; 2] = [-52.0, 176.0];
pub const PS2_PANEL_HEADER_Y: f32 = 64.0;

/// The widgets the file gives no per-person style for: the PSP's own `Line1` carries
/// them (`font="title"`, white, a `0x3f000000` border), and the cells inherit the screen's.
const HEADLINE_STYLE: &str =
    r#"align="centre" scale="1.0" font="title" color="0xffffffff" bordercolor="0x3f000000""#;

const GRADIENT_DOWN: &str =
    r#"Color1="0x9f000000" Color2="0x9f000000" Color3="0x9f000000" Color4="0x9f000000""#;
const RULE_LEFT: &str =
    r#"Color1="0x00ffffff" Color2="0x00ffffff" Color3="0xffffffff" Color4="0xffffffff""#;
const RULE_RIGHT: &str =
    r#"Color1="0xffffffff" Color2="0xffffffff" Color3="0x00ffffff" Color4="0x00ffffff""#;

/// The scale a borrowed headline is drawn at: the PSP's faces are as tall in its 272 rows as
/// the PS2's are in 448, so a text sized for the PS2's room crowds the PSP's. **Chosen, not
/// measured**: 0.85, with the text lifted to end just above the rule it sits over.
const FIT_SCALE: f32 = 0.85;

/// One person's `Item`, in the XML the PS2 file writes it in, with every position and
/// size scaled by `k` (`x` by `k[0]`, `y` by `k[1]`). `fit` lays the headline out for a
/// grid smaller than the PS2's, see [`FIT_SCALE`].
fn person_xml(table: &Table, person: usize, k: [f32; 2], fit: bool) -> String {
    let (x, y) = (|v: f32| v * k[0], |v: f32| v * k[1]);
    let n = person;
    let (headline_y, style) = if fit {
        let height = super::draw::font_line_height("title") * FIT_SCALE;
        (
            y(table.bar.0) - height - 1.0,
            HEADLINE_STYLE.replace("scale=\"1.0\"", &format!("scale=\"{FIT_SCALE}\"")),
        )
    } else {
        (y(table.headline.1), HEADLINE_STYLE.to_string())
    };
    let half = 320.0;
    let mut xml = format!(
        r#"<Item OffsetX="0" OffsetY="{item}" name="Player_{n}">
<Text name="Line1_{n}"><Values string="race complete!" x="{hx}" y="{headline_y}" {style}></Values></Text>
<Text name="table_{n}">
<Image name="zonetopline"><Values x="0" y="{ty}" width="{hw}" height="{th}" {RULE_LEFT}></Values>
<Image><Values x="{hw}" y="{ty}" width="{hw}" height="{th}" {RULE_RIGHT}></Values></Image></Image>
"#,
        item = y(table.item_y[n]),
        hx = x(table.headline.0),
        ty = y(table.top_rule.0),
        th = y(table.top_rule.1),
        hw = x(half),
    );
    for row in 1..=8usize {
        let offset = y(table.first_row + table.row_pitch * (row - 1) as f32);
        let (top, height) = if row == 1 {
            (0.0, table.first_height)
        } else {
            (table.row_top, table.row_height)
        };
        let (top, height, rule) = (y(top), y(height), y(table.row_rule));
        xml.push_str(&format!(
            r#"<Image name="tablebg{row}_{n}" OffsetY="{offset}">
<Values x="0" y="{top}" width="{w}" height="{height}" {GRADIENT_DOWN}></Values>
<Image><Values x="0" y="{top}" width="{w}" height="{height}" U="0" V="0" TxtrWidth="960" TxtrHeight="43" Color="0x3fffffff" src="Data\FE\Images\hex_bg.mip"></Values></Image>
<Image><Values x="0" y="{rule}" width="{hw}" height="{th}" {RULE_LEFT}></Values></Image>
<Image><Values x="{hw}" y="{rule}" width="{hw}" height="{th}" {RULE_RIGHT}></Values></Image>
</Image>
"#,
            w = x(640.0),
            hw = x(half),
            th = y(table.top_rule.1),
        ));
    }
    xml.push_str(&format!(
        r#"<Image name="topbarcenter_{n}"><Values x="0" y="{by}" width="{w}" height="{bh}" U="340" V="0" TxtrWidth="8" TxtrHeight="24" Color="0xffffffff" src="Data\FE\Images\pulse_assets.mip"></Values></Image>
"#,
        by = y(table.bar.0),
        bh = y(table.bar.1),
        w = x(640.0),
    ));
    for row in 0..=8usize {
        let cell_y = if row == 0 {
            table.header_cell
        } else {
            table.first_cell + table.cell_pitch * (row - 1) as f32
        };
        for (column, column_x) in table.columns.iter().enumerate() {
            xml.push_str(&format!(
                "<Text name=\"lap{row}.{column}_{n}\"><Values x=\"{}\" y=\"{}\"></Values></Text>\n",
                x(*column_x),
                y(cell_y),
            ));
        }
    }
    xml.push_str(&format!(
        r#"<Image name="boostimg_{n}"><Values x="{}" y="{}" width="{}" height="{}" U="480" V="32" TxtrWidth="12" TxtrHeight="12" Color="0xffffffff" src="Data\FE\Images\pulse_assets.mip"></Values></Image>
<Image name="tablehighlight_{n}"><Values x="0" y="{}" width="{}" height="{}" RealGlow="128" Color="0x30009fff"></Values></Image>
</Text></Item>
"#,
        x(table.boost[0]),
        y(table.boost[1]),
        x(table.boost[2]),
        y(table.boost[3]),
        y(33.0),
        x(640.0),
        y(table.highlight_height),
    ));
    xml
}

/// `EndRace Results Split` in `to`'s grid, built from [`PS2_TABLE`] and `single`'s own
/// `ContinueButton` text. Pass the PSP's `EndRace Results` and its 480x272 grid for the
/// borrowed screen; the ground-truth test passes [`PS2_GRID`] to compare it with the file.
#[must_use]
pub fn results_split(single: &Layout, to: [f32; 2]) -> Option<Layout> {
    let k = [to[0] / PS2_GRID[0], to[1] / PS2_GRID[1]];
    let mut xml =
        String::from("<Screen type=\"EndRace Results\" name=\"EndRace Results Split\">\n");
    for person in 0..2 {
        xml.push_str(&person_xml(&PS2_TABLE, person, k, to[1] < PS2_GRID[1]));
    }
    xml.push_str(
        r#"<Redirect name="EndRaceRewardsRedirect"><Values backward="none"></Values><Default goto="EndRace Rewards Split"></Default></Redirect>
<Redirect name="EndRaceMenuRedirect"><Values backward="none"></Values><Default goto="Race End Save"></Default></Redirect>
</Screen>"#,
    );
    let mut screen = Screens::from_xml(&xml)
        .by_name("EndRace Results Split")?
        .clone();
    // The screen's own backdrop (the first colour-only `Image`, the whole screen) is the
    // PSP's: it is the one widget the two-person screen shares with the one-person's.
    if let Some(backdrop) = single.screen.fills.iter().find(|fill| fill.name.is_none()) {
        screen.fills.insert(0, backdrop.clone());
    }
    screen.texts.extend(
        single
            .screen
            .texts
            .iter()
            .filter(|text| {
                matches!(
                    text.name.as_deref(),
                    Some("ContinueButton" | "ControlTextConfirm" | "EndRaceCountDown")
                )
            })
            .cloned(),
    );
    Some(Layout {
        screen,
        scale: single.scale,
        faces: single.faces,
    })
}

/// A widget's name with the panel's `_n` suffix.
fn suffixed(name: &Option<String>, panel: usize) -> Option<String> {
    name.as_ref().map(|name| format!("{name}_{panel}"))
}

/// `EndRace Rewards Split` from `single`, the pressing's own `EndRace Rewards`: its panel
/// (everything but the screen backdrop, the header and the confirm prompt) stamped once
/// per person at [`PS2_PANEL_Y`] scaled by `k_y`, with a header per panel at
/// [`PS2_PANEL_HEADER_Y`]. The `Mode3D` trophies are left out: a split has no medal to
/// show one for.
#[must_use]
pub fn rewards_split(single: &Layout, to: [f32; 2]) -> Option<Layout> {
    let k = to[1] / PS2_GRID[1];
    let source = &single.screen;
    let mut screen = Screen {
        name: "EndRace Rewards Split".to_string(),
        kind: source.kind.clone(),
        path: "EndRace Rewards Split".to_string(),
        redirects: source.redirects.clone(),
        ..Screen::default()
    };
    // The screen backdrop comes first and belongs to no panel.
    let backdrop = source.fills.iter().position(|fill| fill.name.is_none())?;
    screen.fills.push(source.fills[backdrop].clone());
    let panel_top = source
        .fills
        .iter()
        .find(|fill| fill.name.as_deref() == Some("BGgradient"))?
        .y;
    let header = source
        .texts
        .iter()
        .find(|text| text.name.is_none() && text.idstring.as_deref() == Some("ER_REWARD"))?;
    for panel in 0..2 {
        let shift = PS2_PANEL_Y[panel] * k;
        for fill in source.fills.iter().skip(backdrop + 1) {
            let mut fill = fill.clone();
            fill.name = suffixed(&fill.name, panel);
            fill.y += shift;
            screen.fills.push(fill);
        }
        for image in &source.images {
            let mut image = image.clone();
            image.name = suffixed(&image.name, panel);
            image.y += shift;
            screen.images.push(image);
        }
        for text in &source.texts {
            let name = text.name.as_deref();
            if name.is_none() && text.idstring.as_deref() == Some("ER_REWARD") {
                let mut text = header.clone();
                text.y = PS2_PANEL_HEADER_Y * k + shift;
                if to[1] < PS2_GRID[1] {
                    // Lifted to end just above the panel's top rule, see `FIT_SCALE`.
                    text.scale *= FIT_SCALE;
                    text.y = panel_top + shift
                        - super::draw::font_line_height(&text.font) * text.scale
                        - 1.0;
                }
                screen.texts.push(text);
            } else if matches!(
                name,
                Some(
                    "BigPos" | "RewardLine1" | "RewardLine2" | "RewardLoyaltyActive" | "loyaltynum"
                )
            ) {
                let mut text = text.clone();
                text.name = suffixed(&text.name, panel);
                text.y += shift;
                screen.texts.push(text);
            }
        }
    }
    screen.texts.extend(
        source
            .texts
            .iter()
            .filter(|text| {
                matches!(
                    text.name.as_deref(),
                    Some("ContinueButton" | "ControlTextConfirm" | "EndRaceCountDown")
                )
            })
            .cloned(),
    );
    Some(Layout {
        screen,
        scale: single.scale,
        faces: single.faces,
    })
}
