//! The grid's geometry is the file's own: the PSP's, the PS2's, and `Results Split`'s two
//! tables beside the one-person table it keeps.

use super::*;
use oag_ui::frontend::Placed;
use oag_ui::language::StringTable;
use oag_ui::screen::Screens;

/// One authored table group in the shape both files use: `zonetopline`, eight `tablebg`
/// wrappers (a fill, a tile and two rules each) at `first + pitch * (row - 1)`, the
/// header bar, a cell per row and the highlight. Row 1's fill is `[0, height1)`, later
/// rows' start `own` below the wrapper, as the files author them.
struct Table {
    suffix: &'static str,
    item_y: f32,
    first: f32,
    pitch: f32,
    own: f32,
    rule: f32,
}

impl Table {
    fn xml(&self) -> String {
        let s = self.suffix;
        let mut xml = format!(
            r#"<Item OffsetX="0" OffsetY="{}"><Text name="table{s}">
<Image name="zonetopline"><Values x="0" y="{}" width="320" height="2" Color1="0x00ffffff" Color2="0x00ffffff" Color3="0xffffffff" Color4="0xffffffff"></Values>
<Image><Values x="320" y="{}" width="320" height="2" Color1="0xffffffff" Color2="0xffffffff" Color3="0x00ffffff" Color4="0x00ffffff"></Values></Image></Image>
"#,
            self.item_y,
            self.first - 2.0,
            self.first - 2.0
        );
        for row in 1..=8 {
            let offset = self.first + self.pitch * (row - 1) as f32;
            let inner = if row == 1 { 0.0 } else { self.own };
            let rule = self.rule;
            xml.push_str(&format!(
                r#"<Image name="tablebg{row}{s}" OffsetY="{offset}">
<Values x="0" y="{inner}" width="640" height="30" Color1="0x9f000000" Color2="0x9f000000" Color3="0x9f000000" Color4="0x9f000000"></Values>
<Image><Values x="0" y="{inner}" width="640" height="30" U="0" V="0" TxtrWidth="960" TxtrHeight="43" Color="0x3fffffff" src="Data\FE\Images\hex_bg.mip"></Values></Image>
<Image><Values x="0" y="{rule}" width="320" height="2" Color1="0x00ffffff" Color2="0x00ffffff" Color3="0xffffffff" Color4="0xffffffff"></Values></Image>
<Image><Values x="320" y="{rule}" width="320" height="2" Color1="0xffffffff" Color2="0xffffffff" Color3="0x00ffffff" Color4="0x00ffffff"></Values></Image>
</Image>
"#
            ));
        }
        xml.push_str(&format!(
            r#"<Image name="topbarcenter{s}"><Values x="0" y="{}" width="640" height="41" src="Data\FE\Images\pulse_assets.mip"></Values></Image>
<Text name="lap0.0{s}"><Values x="187" y="{}"></Values></Text>
<Text name="lap1.0{s}"><Values x="187" y="{}"></Values></Text>
<Image name="tablehighlight{s}"><Values x="0" y="33" width="640" height="25" RealGlow="128" Color="0x30009fff"></Values></Image>
</Text></Item>
"#,
            self.first - 28.0,
            self.first - 26.0,
            self.first + 2.0
        ));
        xml
    }
}

/// The PSP's own numbers: pitch 20 from 92, tiles one pixel in, rules at `+20`.
const PSP: Table = Table {
    suffix: "",
    item_y: 0.0,
    first: 92.0,
    pitch: 20.0,
    own: 1.0,
    rule: 20.0,
};

/// The PS2's `EndRace Results`: pitch 32.941 from 152 under `<Item OffsetY="-10">`.
const PS2: Table = Table {
    suffix: "",
    item_y: -10.0,
    first: 152.0,
    pitch: 32.941,
    own: 2.0,
    rule: 33.0,
};

fn split(item_y: f32, suffix: &'static str) -> Table {
    Table {
        suffix,
        item_y,
        first: 152.0,
        pitch: 25.0,
        own: 2.0,
        rule: 25.0,
    }
}

fn layout(tables: &[&Table]) -> Layout {
    let xml = format!(
        "<Screen name=\"EndRace Results\">\n{}</Screen>",
        tables.iter().map(|table| table.xml()).collect::<String>()
    );
    Layout::read(
        &Screens::from_xml(&xml),
        "EndRace Results",
        &StringTable::from_xml("<Strings></Strings>"),
        crate::picker::FaceScales::default(),
        [640.0, 448.0],
    )
    .expect("the fixture is a screen")
}

fn placed() -> Option<Placed> {
    Some(Placed {
        x: 0,
        y: 0,
        width: 32,
        height: 32,
        quad_extent: None,
        quad_centre: None,
        blend: None,
    })
}

fn shown(rows: usize, highlight: Option<usize>) -> Shown {
    Shown {
        table: true,
        rows,
        header_bar: true,
        boost_icon: false,
        highlight,
        also_highlight: 0,
    }
}

fn draw(layout: &Layout, shows: &[(Suffix, Shown)]) -> Layers {
    table_layers(
        layout,
        &Skin::new(
            oag_pulse::FRONT_END.menu.unwrap(),
            oag_display::space::Space::PSP,
            22.0,
        ),
        &Frame::default(),
        None,
        false,
        &|_| placed(),
        shows,
        &|text| text.string.clone().or_else(|| Some(String::new())),
    )
}

/// The `y` of every drawn rectangle, sorted.
fn ys(layers: &Layers) -> Vec<f32> {
    let mut ys: Vec<f32> = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::GradientFill { rect, .. }
            | Draw::Sprite { rect, .. }
            | Draw::TiledSprite { rect, .. }
            | Draw::Fill { rect, .. } => Some(rect[1]),
            _ => None,
        })
        .collect();
    ys.sort_by(f32::total_cmp);
    ys
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// The PSP's highlight is the literal the executable steps, `0x5d + 0x14 * (row - 1)`.
#[test]
fn the_psp_highlight_steps_the_executables_literal() {
    let grid = Grid::read(&layout(&[&PSP]));
    for row in 1..=8 {
        let y = grid.highlight_y(None, row).unwrap();
        assert!(close(y, 93.0 + 20.0 * (row - 1) as f32), "row {row}: {y}");
    }
}

/// The PS2's rows start at the file's `152 - 10` and step its own 32.94: the highlight
/// is on the row it is asked for, not the PSP's row at `93 + 20 (row - 1)`.
#[test]
fn the_ps2_rows_are_where_its_file_puts_them() {
    let grid = Grid::read(&layout(&[&PS2]));
    assert!(close(grid.tables[0].tops[0], 142.0), "{:?}", grid.tables);
    assert!(close(grid.tables[0].tops[1], 142.0 + 32.941 + 2.0));
    for row in 2..=8 {
        let y = grid.highlight_y(None, row).unwrap();
        let top = 142.0 + 32.941 * (row - 1) as f32 + 2.0;
        assert!(close(y, top), "row {row}: {y} against {top}");
    }
    assert!(close(grid.highlight_y(None, 1).unwrap(), 144.0));
}

/// A fill that drew every row's backing at the PSP's `y` piled the PS2's up: each of the
/// eight must sit at its own top.
#[test]
fn a_ps2_table_draws_each_rows_backing_at_its_own_top() {
    let layers = draw(&layout(&[&PS2]), &[(None, shown(8, None))]);
    let fills: Vec<f32> = layers
        .body
        .iter()
        .filter_map(|draw| match draw {
            Draw::GradientFill { rect, .. } if rect[2] > 600.0 => Some(rect[1]),
            _ => None,
        })
        .collect();
    assert_eq!(fills.len(), 8, "{fills:?}");
    for (index, y) in fills.iter().enumerate() {
        let top = 142.0 + 32.941 * index as f32 + if index == 0 { 0.0 } else { 2.0 };
        assert!(close(*y, top), "row {}: {y} against {top}", index + 1);
    }
}

/// Four widgets per row plus the top rule's two: a row past `rows` takes its tile and
/// rules with it, on the PS2's bands as on the PSP's.
#[test]
fn a_row_past_the_shown_ones_takes_its_tile_and_rules_with_it() {
    for table in [&PSP, &PS2] {
        let layout = layout(&[table]);
        let all = ys(&draw(&layout, &[(None, shown(8, None))])).len();
        let three = ys(&draw(&layout, &[(None, shown(3, None))])).len();
        assert_eq!(all - three, 5 * 4, "five rows of four widgets");
        let none = draw(
            &layout,
            &[(
                None,
                Shown {
                    table: false,
                    ..shown(8, None)
                },
            )],
        );
        assert!(ys(&none).is_empty(), "a hidden table draws no widget");
    }
}

/// `Results Split` keeps the one-person table beside its two, and their rows overlap in
/// `y`: showing only table 1 must draw only table 1's widgets, tile and rules included.
#[test]
fn a_split_screen_draws_only_the_tables_it_is_told_to() {
    let single = Table { suffix: "", ..PS2 };
    let (p0, p1) = (split(-96.0, "_0"), split(110.0, "_1"));
    let layout = layout(&[&single, &p0, &p1]);
    let grid = Grid::read(&layout);
    assert_eq!(
        grid.tables.iter().map(|t| t.suffix).collect::<Vec<_>>(),
        [None, Some(0), Some(1)]
    );
    let both = draw(
        &layout,
        &[(Some(0), shown(2, Some(2))), (Some(1), shown(2, Some(1)))],
    );
    let one = draw(&layout, &[(Some(1), shown(2, Some(1)))]);
    // Per table: top rule 2 + 2 rows of 4 + header bar + highlight.
    assert_eq!(ys(&both).len(), 2 * (2 + 8 + 1 + 1));
    assert_eq!(ys(&one).len(), 2 + 8 + 1 + 1);
    let first_row_1 = 110.0 + 152.0;
    assert!(
        ys(&one).iter().all(|y| *y >= first_row_1 - 30.0),
        "table 1 only: {:?}",
        ys(&one)
    );
    // Person 1's highlight is row 1 of its own table, person 0's is row 2 of its own.
    let highlights = |layers: &Layers| -> Vec<f32> {
        layers
            .body
            .iter()
            .filter_map(|draw| match draw {
                Draw::Fill { rect, .. } => Some(rect[1]),
                _ => None,
            })
            .collect()
    };
    let h = highlights(&both);
    assert!(close(h[0], -96.0 + 177.0 + 2.0), "person 0's row 2: {h:?}");
    assert!(close(h[1], 110.0 + 152.0 + 2.0), "person 1's row 1: {h:?}");
}

#[test]
fn lap_cells_read_their_table_and_slot() {
    let cell = |name| lap_cell(name).map(|c| (c.table, c.row, c.column));
    assert_eq!(cell("lap3.1"), Some((None, 3, 1)));
    assert_eq!(cell("lap0.2"), Some((None, 0, 2)));
    assert_eq!(cell("lap3.1_1"), Some((Some(1), 3, 1)));
    assert_eq!(cell("Line1_0"), None);
    assert_eq!(cell("Line1"), None);
}

/// `tablehighlight` authors `RealGlow="128"`, and `Image`'s draw stamps that value as written
/// over the quad (`FUN_088a7a5c`): one stamp on each highlighted row's rectangle, none without a
/// highlight.
#[test]
fn the_highlight_stamps_its_authored_real_glow_on_the_row_it_moved_to() {
    let single = Table { suffix: "", ..PS2 };
    let (p0, p1) = (split(-96.0, "_0"), split(110.0, "_1"));
    let layout = layout(&[&single, &p0, &p1]);
    let stamps = |layers: &Layers| -> Vec<([f32; 4], u8)> {
        layers
            .body
            .iter()
            .filter_map(|draw| match draw {
                Draw::GlowStamp { rect, value } => Some((*rect, *value)),
                _ => None,
            })
            .collect()
    };
    let fills = |layers: &Layers| -> Vec<[f32; 4]> {
        layers
            .body
            .iter()
            .filter_map(|draw| match draw {
                Draw::Fill { rect, color } if color[3] < 0.2 => Some(*rect),
                _ => None,
            })
            .collect()
    };
    let both = draw(
        &layout,
        &[(Some(0), shown(2, Some(2))), (Some(1), shown(2, Some(1)))],
    );
    let stamped = stamps(&both);
    assert_eq!(stamped.len(), 2, "{stamped:?}");
    assert!(stamped.iter().all(|(_, value)| *value == 128));
    assert_eq!(
        stamped.iter().map(|(rect, _)| *rect).collect::<Vec<_>>(),
        fills(&both),
        "each stamp covers the translucent highlight it follows"
    );
    assert!(stamps(&draw(&layout, &[(Some(1), shown(2, None))])).is_empty());
}
