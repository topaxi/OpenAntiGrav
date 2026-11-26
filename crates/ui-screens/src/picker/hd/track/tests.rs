//! `oag_ui_screens::picker::hd::track` against a hand-written miniature of HD's
//! `Track_Selection_Definition.xml`. `selection_screens_ground_truth.rs`
//! reads the real one.

use super::*;
use crate::picker::pointer::{What, targets};
use crate::picker::{Entry, Event, Kind};
use oag_gameplay::input::{Button, Input};

const XML: &str = r#"<Variable global="Grey" name="Grey"><Values String="0xff969696"></Values></Variable>
<Screen name="TrackSelectionTopLevel">
  <Model name="TrackModel"><Values OriginX="1308" OriginY="440" z="-180.0"></Values></Model>
  <Item OffsetX="160" OffsetY="170">
    <MiniText><Values y="-30" idstring="RB_CHOOSE_CIRCUIT" color="FEGlobals->Grey"></Values></MiniText>
    <Item OffsetY="0">
      <Bracket><Values corner="true" X="0" Y="0" Width="680" Height="242"></Values></Bracket>
      <Image name="Emblem"><Values x="110" y="8" width="192" height="192"></Values></Image>
      <Text name="TrackName"><Values idstring="RB_TRACK_SEL" x="340" y="200"></Values></Text>
      <Movie name="FlyByMovie"><Values X="500" Y="190" Width="260" height="170" src="Data\FE\Images\Loading_Icon.bik" repeat="true"></Values></Movie>
    </Item>
    <Bracket><Values corner="true" X="0" Y="282" Width="680" Height="230"></Values></Bracket>
    <TrackHexSelection name="TrackHexSelector" OffsetX="50" OffsetY="330"><Values columns="9" rows="2"></Values></TrackHexSelection>
  </Item>
  <Bracket><Values corner="true" middle="true" X="856" Y="170" Width="900" Height="565"></Values></Bracket>
  <Screen type="TrackSelection" name="Track Creation">
    <Text><Values idstring="RB_TRACK_SEL" font="Title" x="0" y="0"></Values></Text>
    <Image name="RecordParent" OffsetX="160" OffsetY="744">
      <Item OffsetY="25">
        <Block name="Record.1.1"><Values string="16" width="367" y="50"></Values></Block>
      </Item>
    </Image>
  </Screen>
  <Screen type="TournamentSelection" name="Tournament C">
    <Text name="TrackNumber0"><Values string="1"></Values></Text>
  </Screen>
</Screen>"#;

fn strings() -> StringTable {
    StringTable::from_xml(
        r#"<Screen name="Top">
        <Entry ID="RB_TRACK_SEL" String="TRACK SELECT"/>
        <Entry ID="RB_CHOOSE_CIRCUIT" String="CHOOSE CIRCUIT"/>
        <Entry ID="FE_PERSONAL" String="PERSONAL"/>
        <Entry ID="FE_FRIENDS" String="FRIENDS"/>
        <Entry ID="FE_GLOBAL" String="GLOBAL"/>
        </Screen>"#,
    )
}

fn layout() -> Layout {
    let screens = Screens::from_xml_with_fallback_globals(XML, &[("HD_Blue", "0xffac0717")]);
    read(
        XML,
        &screens,
        &strings(),
        FaceScales::default(),
        [1920.0, 1080.0],
    )
    .expect("both screens are there")
}

fn circuit(id: &str, reversed: bool) -> Entry {
    Entry {
        id: id.to_string(),
        label: id.to_uppercase(),
        details: Details::Track {
            info: ["4.4KM".into(), "13.2KM".into(), "-".into()],
            emblem: Some(emblem_src(&format!(r"Data\Environments\{id}"))),
            icon: Some(grid_icon_src(&format!(r"Data\Environments\{id}"))),
            reversed,
            model_pose: None,
        },
    }
}

#[test]
fn the_child_screen_carries_the_parents_widgets_and_none_of_tournament_cs() {
    let layout = layout();
    assert_eq!(layout.title, "TRACK SELECT");
    assert_eq!(layout.screen.name, SCREEN);
    assert!(layout.hd.is_none() && layout.is_hd());
    assert!(
        layout
            .screen
            .texts
            .iter()
            .all(|text| text.name.as_deref() != Some("TrackNumber0")),
        "the sibling Tournament C screen is not merged in"
    );
    let extra = layout.hd_track.as_deref().expect("an HD track layout");
    assert_eq!(extra.common.labels.len(), 1);
    assert_eq!(extra.common.labels[0].y, 140.0);
    assert_eq!(extra.emblem, Some([270.0, 178.0, 192.0, 192.0]));
    assert_eq!(extra.model.map(|m| m.origin), Some([1308.0, 440.0]));
    // HD authors no `x`/`y` and no `orthoScale`: the placement is `z` alone.
    let model = extra.model.expect("the TrackModel widget");
    assert_eq!((model.offset, model.ortho_scale), ([0.0; 2], [1.0; 3]));
    let grid = extra.hex_grid.expect("the hex selector");
    assert_eq!(
        (grid.origin, grid.columns, grid.rows),
        ([210.0, 500.0], 9, 2)
    );
    assert_eq!(extra.row_labels, ["PERSONAL", "FRIENDS", "GLOBAL"]);
    assert_eq!(layout.panel, [160.0, 170.0, 680.0, 242.0]);
    assert_eq!(layout.preview, [856.0, 170.0, 900.0, 565.0]);
}

#[test]
fn a_file_missing_either_screen_reads_as_none() {
    let xml = r#"<Screen name="TrackSelectionTopLevel"></Screen>"#;
    let screens = Screens::from_xml_with_fallback_globals(xml, &[("HD_Blue", "0xffac0717")]);
    assert!(
        read(
            xml,
            &screens,
            &strings(),
            FaceScales::default(),
            [1920.0, 1080.0]
        )
        .is_none()
    );
}

#[test]
fn the_emblem_is_the_environments_own_fe_texture() {
    assert_eq!(
        emblem_src(r"Data\Environments\01_Vineta_K"),
        r"Data\Environments\01_Vineta_K\FE\TrackSelectEmblem_Fury.gtf"
    );
}

#[test]
fn the_fly_by_clip_is_the_environments_own_preview() {
    assert_eq!(
        fly_by_src(r"Data\Environments\01_Vineta_K", r"FE\preview.bik"),
        r"Data\Environments\01_Vineta_K\FE\preview.bik"
    );
}

#[test]
fn the_fly_by_panel_is_where_the_widget_says_and_not_offset_by_its_item() {
    let layout = layout();
    let extra = layout.hd_track.as_deref().expect("hd");
    // The frame's picture sits at the widget's own X/Y: (500,190), where its `Item`'s
    // (160,170) would put it at (660,360).
    assert_eq!(extra.fly_by, Some([500.0, 190.0, 260.0, 170.0]));
    let picker = Picker::new(Kind::Track, vec![circuit("a", false)], Some("a"), None);
    let draws = body(&picker, &layout, extra, &Frame::default(), &sheet);
    let video = draws.iter().find_map(|draw| match draw {
        Draw::Video { rect, source, .. } => Some((*rect, *source)),
        _ => None,
    });
    assert_eq!(
        video,
        Some(([500.0, 190.0, 260.0, 170.0], Video::FlyBy)),
        "dropping the wiring leaves the panel with no video draw"
    );
}

#[test]
fn lengths_print_as_kilometres_and_a_lapless_race_has_no_distance() {
    assert_eq!(length_rows(4400.0, Some(3)), ["4.4KM", "13.2KM"]);
    assert_eq!(length_rows(4400.0, None), ["4.4KM", ""]);
}

fn press(picker: &mut Picker, button: Button) -> Vec<Event> {
    let mut input = Input::default();
    input.begin_frame(0);
    input.begin_frame(1 << button as u32);
    picker.update(&mut input)
}

fn grid() -> Picker {
    let entries = ["a", "b", "c"]
        .iter()
        .map(|id| circuit(id, false))
        .chain(["a", "b", "c"].iter().map(|id| circuit(id, true)))
        .collect();
    Picker::new(Kind::Track, entries, Some("a"), None).with_rows(3)
}

/// Lets the grid's input cooldown run out: more than 0.2 s of ticks.
fn rest(picker: &mut Picker) {
    for _ in 0..13 {
        picker.tick(1.0 / 60.0);
    }
}

#[test]
fn right_wraps_along_the_row_and_up_down_switch_direction() {
    let mut picker = grid();
    for expected in [1, 2, 0] {
        rest(&mut picker);
        assert_eq!(press(&mut picker, Button::Right), vec![Event::Moved]);
        assert_eq!(
            picker.index(),
            expected,
            "twelve presses in the original, three here"
        );
    }
    rest(&mut picker);
    assert_eq!(press(&mut picker, Button::Left), vec![Event::Moved]);
    assert_eq!(picker.index(), 2, "left wraps the other way");
    rest(&mut picker);
    assert_eq!(press(&mut picker, Button::Down), vec![Event::Moved]);
    assert_eq!(picker.index(), 5, "same circuit, the reverse row");
    rest(&mut picker);
    assert_eq!(press(&mut picker, Button::Right), vec![Event::Moved]);
    assert_eq!(picker.index(), 3, "still on the reverse row");
    rest(&mut picker);
    assert_eq!(press(&mut picker, Button::Up), vec![Event::Moved]);
    assert_eq!(picker.index(), 0);
}

/// `TrackHexSelection_Update` takes a direction only while its input counter is at or above 0.0 and
/// sets it to -0.2 when it does: a second direction inside 0.2 s of a taken one is dropped, not
/// deferred, and any of the four directions starts the cooldown (12 frames, counted in whole frames).
/// This fails if the gate is dropped.
#[test]
fn a_direction_inside_the_cooldown_of_the_last_one_taken_is_dropped() {
    let mut picker = grid();
    assert_eq!(press(&mut picker, Button::Right), vec![Event::Moved]);
    assert!(
        press(&mut picker, Button::Right).is_empty(),
        "the same frame"
    );
    for _ in 0..11 {
        picker.tick(1.0 / 60.0);
    }
    assert!(
        press(&mut picker, Button::Right).is_empty(),
        "11 frames in: still cooling"
    );
    assert!(press(&mut picker, Button::Down).is_empty(), "the rows too");
    assert_eq!(picker.index(), 1);
    for _ in 0..2 {
        picker.tick(1.0 / 60.0);
    }
    assert_eq!(press(&mut picker, Button::Down), vec![Event::Moved]);
    assert_eq!(
        picker.index(),
        4,
        "a vertical step is taken after it and cools too"
    );
    assert!(press(&mut picker, Button::Right).is_empty());
}

#[test]
fn a_single_row_has_nothing_for_up_and_down_to_do() {
    let entries = ["a", "b"].iter().map(|id| circuit(id, false)).collect();
    let mut picker = Picker::new(Kind::Track, entries, None, None).with_rows(2);
    assert!(press(&mut picker, Button::Down).is_empty());
    assert_eq!(press(&mut picker, Button::Right), vec![Event::Moved]);
    assert_eq!(picker.index(), 1);
}

#[test]
fn pointer_halves_step_and_the_model_frame_confirms() {
    let layout = layout();
    let picker = grid();
    let skin = oag_ui::menu::Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    let found = targets(&picker, &layout, &skin, &|_| None);
    let at = |x, y| crate::picker::pointer::hit(&found, (x, y)).map(|t| t.what);
    assert_eq!(at(200.0, 300.0), Some(What::Previous));
    assert_eq!(at(800.0, 300.0), Some(What::Next));
    assert_eq!(at(1200.0, 400.0), Some(What::Confirm));
    // The grid draws, so its frame's halves are not targets: a hexagon is.
    // Circuit `a` is the centre column (origin 210 + 4 * 72.2), its reverse
    // twin the cell below.
    let (cx, top, bottom) = (210.0 + 4.0 * 72.2, 500.0, 500.0 + 82.8);
    assert_eq!(
        at(cx, top),
        Some(What::Cell {
            entry: 0,
            variant: 0
        })
    );
    assert_eq!(
        at(cx, bottom),
        Some(What::Cell {
            entry: 3,
            variant: 0
        })
    );
    // The next column is the next circuit, `b`, on its half-row shift.
    assert_eq!(
        at(cx + 72.2, top + 41.4),
        Some(What::Cell {
            entry: 1,
            variant: 0
        })
    );
    // A click on the other direction's hexagon moves the cursor there, and
    // a second click on it confirms.
    let mut picker = grid();
    let pointer = oag_ui::pointer::Pointer {
        at: Some((cx, bottom)),
        moved: true,
        clicked: true,
        ..oag_ui::pointer::Pointer::default()
    };
    assert_eq!(picker.pointer(&pointer, &found), vec![Event::Moved]);
    assert_eq!(picker.index(), 3);
    let found = targets(&picker, &layout, &skin, &|_| None);
    assert_eq!(picker.pointer(&pointer, &found), vec![Event::Confirmed]);
    // Without the grid the frame's halves switch the direction row.
    let mut bare = layout.clone();
    bare.hd_track.as_mut().expect("hd").hex_grid = None;
    let found = targets(&grid(), &bare, &skin, &|_| None);
    let at = |x, y| crate::picker::pointer::hit(&found, (x, y)).map(|t| t.what);
    assert_eq!(at(400.0, 500.0), Some(What::PreviousVariant));
    assert_eq!(at(400.0, 640.0), Some(What::NextVariant));
    // A single row has no such target.
    let single = Picker::new(Kind::Track, vec![circuit("a", false)], None, None).with_rows(1);
    let found = targets(&single, &bare, &skin, &|_| None);
    assert!(found.iter().all(|t| t.what != What::NextVariant));
}

/// The direction sheet sits apart from the hexagon art, so a draw's `uv` names it.
fn sheet(name: &str) -> Option<oag_ui::frontend::Placed> {
    let direction = name == DIRECTION_SRC;
    (direction
        || name.contains("Hexagon")
        || name.to_ascii_lowercase().contains("trackselectemblem_bw"))
    .then_some(oag_ui::frontend::Placed {
        x: if direction { 512 } else { 0 },
        y: 0,
        width: 128,
        height: 64,
        quad_extent: None,
        quad_centre: None,
        blend: None,
        scale: [1.0, 1.0],
    })
}

/// The direction glyph in `draws`, as its rect and its window on the strip.
fn glyph(draws: &[Draw]) -> Option<([f32; 4], [f32; 4])> {
    draws.iter().find_map(|draw| match draw {
        Draw::Sprite { rect, uv, .. } if rect[2] == 40.0 && rect[3] == 10.0 => {
            Some((*rect, [uv[0] - 512.0, uv[1], uv[2], uv[3]]))
        }
        _ => None,
    })
}

fn grid_draws(picker: &Picker) -> Vec<Draw> {
    let layout = layout();
    let extra = layout.hd_track.as_deref().expect("hd");
    let mut out = Vec::new();
    draw_grid(
        extra.hex_grid.as_ref().expect("grid"),
        true,
        picker,
        &sheet,
        &mut out,
    );
    out
}

#[test]
fn the_grid_puts_a_circuit_in_each_hexagon_and_red_on_the_chosen_column() {
    let picker = grid();
    let draws = grid_draws(&picker);
    // 9 columns x 2 rows: a hexagon and an icon each, flat on the chosen column and shaded across
    // on the other eight, with the ring's one quad and the direction glyph on top.
    let sprites = draws
        .iter()
        .filter(|d| matches!(d, Draw::Sprite { .. }))
        .count();
    let shaded = draws
        .iter()
        .filter(|d| matches!(d, Draw::GradientSprite { .. }))
        .count();
    assert_eq!((sprites, shaded), (2 + 2 + 1 + 1, 16 + 16));
    let red = draws
        .iter()
        .filter(|d| matches!(d, Draw::Sprite { color, .. } if *color == oag_ui::screen::argb_to_rgba(0xffac_0717)))
        .count();
    assert_eq!(red, 2, "both chosen-column hexagons, once each");
    // Each icon is the widget's 50 unit quad: the test's 128x64 art fits it as 50 by 25.
    let icons = draws
        .iter()
        .filter(|d| {
            matches!(d, Draw::Sprite { rect, .. } | Draw::GradientSprite { rect, .. } if rect[2..] == [50.0, 25.0])
        })
        .count();
    assert_eq!(icons, 18);
    // Circuits other than the chosen one are black, fading from 100/255 at the chosen column's
    // edges to 18/255 at the grid's outer edges (nine columns, so four columns of ramp).
    let outer = draws.iter().find_map(|d| match d {
        Draw::GradientSprite {
            rect, left, right, ..
        } if rect[2..] == [50.0, 25.0] => Some((*left, *right)),
        _ => None,
    });
    let (left, right) = outer.expect("a shaded icon");
    assert_eq!(left[..3], [0.0; 3]);
    assert!((left[3] * 255.0 - 18.0).abs() < 1e-3, "{left:?}");
    assert!(
        (right[3] * 255.0 - (18.0 + 82.0 / 4.0)).abs() < 1e-3,
        "{right:?}"
    );
}

/// The `>>>` is a 40x10 quad 24 units under the ring, walking one frame of its strip a quarter
/// second, hidden for the 0.2 s a direction cannot be taken, and turned half round on the reverse
/// row; the other style's frames are the strip's last four.
#[test]
fn the_direction_glyph_walks_under_the_ring_and_hides_while_a_step_cools() {
    let mut picker = grid();
    let ring = |draws: &[Draw]| {
        draws.iter().find_map(|d| match d {
            Draw::Sprite { rect, .. } if rect[2] == 92.0 && rect[3] == 80.0 => Some(*rect),
            _ => None,
        })
    };
    let draws = grid_draws(&picker);
    let (rect, uv) = glyph(&draws).expect("the glyph at the page's opening");
    let rings: Vec<[f32; 4]> = draws
        .iter()
        .filter_map(|d| match d {
            Draw::Sprite { rect, .. } if rect[2] == 92.0 && rect[3] == 80.0 => Some(*rect),
            _ => None,
        })
        .collect();
    let on = rings.last().copied().or(ring(&draws)).expect("the ring");
    assert!((rect[0] + 20.0 - (on[0] + 46.0)).abs() < 0.01, "centred");
    assert!(
        (rect[1] + 5.0 - (on[1] + 40.0 + 24.0)).abs() < 0.01,
        "{rect:?} {on:?}"
    );
    assert_eq!(uv, [0.0, 0.0, 32.0, 8.0]);

    picker.tick(0.3);
    assert_eq!(glyph(&grid_draws(&picker)).expect("shown").1[1], 8.0);
    assert_eq!(
        direction_frame(&picker, false),
        Some(5.0),
        "the other style"
    );
    picker.tick(0.75);
    assert_eq!(
        glyph(&grid_draws(&picker)).expect("shown").1[1],
        0.0,
        "a second a cycle"
    );

    press(&mut picker, Button::Down);
    assert!(glyph(&grid_draws(&picker)).is_none(), "cooling");
    picker.tick(0.19);
    assert!(glyph(&grid_draws(&picker)).is_none(), "still cooling");
    picker.tick(0.02);
    let (reverse, uv) = glyph(&grid_draws(&picker)).expect("back after 0.2 s");
    assert_eq!(uv, [32.0, 8.0, -32.0, -8.0], "turned half round");
    assert!(reverse[1] > rect[1], "under the reverse row's ring");
}

#[test]
fn the_ring_follows_the_direction_row_and_dropping_the_grid_draws_nothing() {
    let mut picker = grid();
    let ring_y = |draws: &[Draw]| {
        draws.iter().rev().find_map(|d| match d {
            Draw::Sprite { rect, .. } if rect[2] == 92.0 && rect[3] == 80.0 => Some(rect[1]),
            _ => None,
        })
    };
    let forward = ring_y(&grid_draws(&picker)).expect("ring on the forward row");
    press(&mut picker, Button::Down);
    let reverse = ring_y(&grid_draws(&picker)).expect("ring on the reverse row");
    assert!((reverse - forward - 83.0).abs() < 0.01);
    let layout = layout();
    let extra = layout.hd_track.as_deref().expect("hd");
    let mut bare = extra.clone();
    bare.hex_grid = None;
    let with = body(&picker, &layout, extra, &Frame::default(), &sheet);
    let without = body(&picker, &layout, &bare, &Frame::default(), &sheet);
    assert!(with.len() > without.len());
}

#[test]
fn a_horizontal_drag_pans_the_circuits_along_the_row_and_wraps() {
    let layout = layout();
    let skin = oag_ui::menu::Skin::new(
        oag_pulse::FRONT_END.menu.unwrap(),
        oag_display::space::Space::PSP,
        22.0,
    );
    let drag = |dx: f32| oag_ui::pointer::Pointer {
        drag: (dx / crate::picker::pointer::DRAG_GAIN, 0.0),
        ..oag_ui::pointer::Pointer::default()
    };
    let pitch = crate::picker::hd::hex::PITCH[0];
    let mut picker = grid();
    let found = targets(&picker, &layout, &skin, &|_| None);
    assert!(picker.pointer(&drag(pitch * 0.5), &found).is_empty());
    assert_eq!(picker.index(), 0);
    // Nearer the next column than its own: the previous circuit, which
    // wraps to the row's last.
    assert_eq!(
        picker.pointer(&drag(pitch * 0.2), &found),
        vec![Event::Moved]
    );
    assert_eq!(picker.index(), 2);
    assert_eq!(
        picker.pointer(&drag(-pitch * 1.2), &found),
        vec![Event::Moved]
    );
    assert_eq!(picker.index(), 0);
    // The reverse row keeps its row while panning.
    let mut reverse =
        Picker::new(Kind::Track, grid().entries().to_vec(), Some("b"), None).with_rows(3);
    assert_eq!(reverse.index(), 1);
    let found = targets(&reverse, &layout, &skin, &|_| None);
    assert_eq!(
        reverse.pointer(&drag(-pitch * 1.5), &found),
        vec![Event::Moved]
    );
    assert_eq!(reverse.index(), 2);
    // A vertical drag does nothing, and a drag never confirms.
    let vertical = oag_ui::pointer::Pointer {
        drag: (0.0, 80.0),
        ..oag_ui::pointer::Pointer::default()
    };
    assert!(reverse.pointer(&vertical, &found).is_empty());
    assert_eq!(reverse.index(), 2);
    // The cells' hit rects follow the finger.
    let mut panning = grid();
    let found = targets(&panning, &layout, &skin, &|_| None);
    panning.pointer(&drag(pitch * 0.5), &found);
    let moved = targets(&panning, &layout, &skin, &|_| None);
    let centre = 210.0 + 4.0 * pitch;
    let near = |t: &[crate::picker::pointer::Target], x: f32| {
        t.iter()
            .filter(|t| {
                t.what
                    == (What::Cell {
                        entry: 0,
                        variant: 0,
                    })
            })
            .map(|t| t.rect[0] + t.rect[2] * 0.5)
            .min_by(|a, b| (a - x).abs().total_cmp(&(b - x).abs()))
            .unwrap()
    };
    assert!((near(&found, centre) - centre).abs() < 1e-2);
    assert!((near(&moved, centre + pitch * 0.5) - centre - pitch * 0.5).abs() < 1e-2);
}

/// The circuit's `Emblem` authors `vwipe` with `delay="0.6"`: nothing for 36 frames after the page
/// opens, then the slab and the ramp, whole after 0.5 s more
/// (`docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`, "Page entry"). This fails if the delay
/// or the entry clock is dropped.
#[test]
fn the_emblem_waits_its_delay_and_then_wipes() {
    let xml = XML.replace(
        r#"<Image name="Emblem"><Values x="110""#,
        r#"<Image name="Emblem"><Values transitiontype="vwipe" delay="0.6" EnableTransition="0.5" x="110""#,
    );
    let screens = Screens::from_xml_with_fallback_globals(&xml, &[]);
    let layout = read(
        &xml,
        &screens,
        &strings(),
        FaceScales::default(),
        [1920.0, 1080.0],
    )
    .expect("both screens are there");
    let extra = layout.hd_track.as_deref().expect("hd");
    let emblem_height = |picker: &Picker| {
        let emblem_sheet = |name: &str| {
            name.to_ascii_lowercase()
                .contains("trackselectemblem_fury")
                .then(|| sheet("trackselectemblem_bw").expect("a placement"))
        };
        let draws = body(picker, &layout, extra, &Frame::default(), &emblem_sheet);
        let found: Vec<f32> = draws
            .iter()
            .filter_map(|draw| match draw {
                Draw::Sprite { rect, .. } if rect[0] == 270.0 && rect[2] == 192.0 => Some(rect[3]),
                _ => None,
            })
            .collect();
        assert!(found.len() <= 1, "{found:?}");
        found.first().copied()
    };
    let mut picker = grid();
    assert_eq!(emblem_height(&picker), None, "frame 0");
    for _ in 0..35 {
        picker.tick(1.0 / 60.0);
    }
    assert_eq!(
        emblem_height(&picker),
        None,
        "frame 35, the last of the delay"
    );
    picker.tick(1.0 / 60.0);
    let first = emblem_height(&picker).expect("frame 36 is the first ramp frame");
    assert!((first - 19.2).abs() < 1e-3, "{first}: a slab a tenth tall");
    for _ in 0..30 {
        picker.tick(1.0 / 60.0);
    }
    assert_eq!(
        emblem_height(&picker),
        Some(192.0),
        "whole when the ramp is out"
    );
}

fn wiped_layout() -> Layout {
    let xml = XML.replace(
        r#"<Image name="Emblem"><Values x="110""#,
        r#"<Image name="Emblem"><Values transitiontype="vwipe" delay="0.6" EnableTransition="0.5" x="110""#,
    );
    let screens = Screens::from_xml_with_fallback_globals(&xml, &[]);
    read(
        &xml,
        &screens,
        &strings(),
        FaceScales::default(),
        [1920.0, 1080.0],
    )
    .expect("both screens are there")
}

fn drawn_emblem_height(picker: &Picker, layout: &Layout) -> Option<f32> {
    let extra = layout.hd_track.as_deref().expect("hd");
    let emblem_sheet = |name: &str| {
        name.to_ascii_lowercase()
            .contains("trackselectemblem_fury")
            .then(|| sheet("trackselectemblem_bw").expect("a placement"))
    };
    body(picker, layout, extra, &Frame::default(), &emblem_sheet)
        .iter()
        .find_map(|draw| match draw {
            Draw::Sprite { rect, .. } if rect[0] == 270.0 && rect[2] == 192.0 => Some(rect[3]),
            _ => None,
        })
}

/// `TrackSelectionScreen_SelectCircuit` restarts the `Emblem` with `Widget_ResetTransition`, which
/// leaves its countdown alone: a circuit step after the page has settled replays the wipe at once
/// (a slab on the press, whole 0.5 s on), and one inside the opening delay keeps waiting for it.
/// This fails if the step is not passed to the emblem.
#[test]
fn a_circuit_step_replays_the_emblems_wipe_without_the_delay() {
    let layout = wiped_layout();
    let mut picker = grid();
    picker.tick(2.0);
    assert_eq!(drawn_emblem_height(&picker, &layout), Some(192.0));
    press(&mut picker, Button::Right);
    let slab = drawn_emblem_height(&picker, &layout).expect("no delay after a step");
    assert!((slab - 19.2).abs() < 1e-3, "{slab}: a slab a tenth tall");
    picker.tick(0.5);
    assert_eq!(drawn_emblem_height(&picker, &layout), Some(192.0));

    let mut early = grid();
    early.tick(0.2);
    press(&mut early, Button::Right);
    assert_eq!(
        drawn_emblem_height(&early, &layout),
        None,
        "the opening delay is still running"
    );
}

/// The corner marks the page has out, as the top-left corners of the 16 by 16 sprites, with the
/// three brackets' `delay`s authored as the disc does (0.2, 0.3, 1.0 s).
fn corner_marks(picker: &Picker) -> Vec<[f32; 2]> {
    let mut delays = ["0.2", "0.3", "1.0"].into_iter();
    let xml = XML
        .split("<Bracket>")
        .enumerate()
        .map(|(index, part)| match index {
            0 => part.to_string(),
            _ => format!(
                "<Bracket delay=\"{}\">{part}",
                delays.next().expect("three brackets")
            ),
        })
        .collect::<String>();
    let screens = Screens::from_xml_with_fallback_globals(&xml, &[]);
    let layout = read(
        &xml,
        &screens,
        &strings(),
        FaceScales::default(),
        [1920.0, 1080.0],
    )
    .expect("both screens are there");
    let extra = layout.hd_track.as_deref().expect("hd");
    let corners = |name: &str| {
        name.ends_with("corner2.gtf")
            .then_some(oag_ui::frontend::Placed {
                x: 0,
                y: 0,
                width: 16,
                height: 16,
                quad_extent: None,
                quad_centre: None,
                blend: None,
                scale: [1.0, 1.0],
            })
    };
    body(picker, &layout, extra, &Frame::default(), &corners)
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, .. } if rect[2] == 16.0 && rect[3] == 16.0 => {
                Some([rect[0], rect[1]])
            }
            _ => None,
        })
        .collect()
}

/// The three `Bracket`s count their own `delay` from the page's entry (0.2, 0.3 and 1.0 s), each
/// opening out over 0.1 s, and a circuit step re-arms none of them (it restarts the `Emblem`
/// alone). This fails if the entry clock is not passed to the brackets.
#[test]
fn the_brackets_wait_their_delays_from_the_pages_entry() {
    let mut picker = grid();
    assert!(corner_marks(&picker).is_empty(), "frame 0");
    picker.tick(0.19);
    assert!(corner_marks(&picker).is_empty(), "inside the 0.2 s delay");
    picker.tick(0.05);
    assert_eq!(corner_marks(&picker).len(), 4, "the circuit panel opens");
    picker.tick(0.1);
    assert_eq!(corner_marks(&picker).len(), 8, "the hex selector (0.3 s)");
    picker.tick(0.8);
    assert_eq!(corner_marks(&picker).len(), 12, "the model frame (1.0 s)");
    press(&mut picker, Button::Right);
    assert_eq!(
        corner_marks(&picker).len(),
        12,
        "a circuit step re-arms no bracket"
    );
}

const DT: f32 = 1.0 / 60.0;

fn ring_rect(draws: &[Draw]) -> Option<([f32; 4], [f32; 4])> {
    draws.iter().rev().find_map(|d| match d {
        Draw::Sprite { rect, color, .. } if rect[2] == 92.0 && rect[3] == 80.0 => {
            Some((*rect, *color))
        }
        _ => None,
    })
}

/// `TrackHexSelection_Update` steps the lattice with `HexSelection_Step` as Team Selection does
/// (`ship-select-ring.md`, "Track Select") and runs the widget's update in the same frame: the
/// first frame is a sixth of a pitch in (RAM: timer 0.0833), the columns slide left linearly, and
/// the fifth frame is the snap with the next circuit in the centre. A stopped grab's lattice stood
/// 11.3 px left where RAM said the fourth frame, three frames of picture lag. This fails if the
/// grid is drawn without the picker's slide.
#[test]
fn a_circuit_step_slides_the_lattice_a_pitch_and_snaps_on_the_fifth_frame() {
    let first_x = |picker: &Picker| match grid_draws(picker)
        .iter()
        .find(|d| matches!(d, Draw::Sprite { .. }))
    {
        Some(Draw::Sprite { rect, .. }) => rect[0],
        _ => panic!("the grid draws a hexagon"),
    };
    let mut picker = grid();
    picker.tick(DT);
    let rest = first_x(&picker);
    press(&mut picker, Button::Right);
    picker.tick(DT);
    assert!(
        (first_x(&picker) - (rest - 12.0)).abs() < 0.01,
        "frame 0 is a sixth of a pitch: {}",
        first_x(&picker) - rest
    );
    for _ in 0..3 {
        picker.tick(DT);
    }
    assert!(
        (first_x(&picker) - (rest - 48.0)).abs() < 0.01,
        "frame 3 is four sixths: {}",
        first_x(&picker) - rest
    );
    picker.tick(DT);
    assert!((first_x(&picker) - (rest - 60.0)).abs() < 0.01, "frame 4");
    picker.tick(DT);
    assert_eq!(first_x(&picker), rest, "frame 5: back in the slots");
    assert_eq!(
        picker.slide().shown(),
        1,
        "with the next circuit in the centre"
    );
}

/// Each step flips which columns sit a half row low, so the lattice follows the circuit's index
/// (RAM: `+0xb4` was 0, 1, 0, 1 at circuits 0 to 3), and the ring, fixed on the centre column,
/// sits half a row lower on an odd one.
#[test]
fn the_lattice_and_the_ring_follow_the_circuits_index() {
    let ring_y = |picker: &Picker| ring_rect(&grid_draws(picker)).expect("the ring").0[1];
    let even = grid();
    let odd = Picker::new(Kind::Track, grid().entries().to_vec(), Some("b"), None).with_rows(3);
    assert!((ring_y(&odd) - ring_y(&even) - hex::HALF_ROW).abs() < 0.01);
}

/// Moving to the other direction's row writes the ring's target and leaves it where it was: it
/// closes half the distance on every update (`HexSelection_Update`), so it sits half a row off
/// after the press's own tick and a quarter after the next.
#[test]
fn the_ring_eases_to_the_other_row_by_halves() {
    let ring_y = |picker: &Picker| ring_rect(&grid_draws(picker)).expect("the ring").0[1];
    let mut picker = grid();
    picker.tick(DT);
    let top = ring_y(&picker);
    press(&mut picker, Button::Down);
    picker.tick(DT);
    assert!(
        (ring_y(&picker) - (top + 41.5)).abs() < 0.01,
        "half of 83 left after one update: {}",
        ring_y(&picker) - top
    );
    picker.tick(DT);
    assert!((ring_y(&picker) - (top + 62.25)).abs() < 0.01);
    for _ in 0..20 {
        picker.tick(DT);
    }
    assert!((ring_y(&picker) - (top + 83.0)).abs() < 0.01, "arrived");
}

/// The ring's pulse is the widget's own counter, which a pair script pins to the phase it read
/// out of RAM: white to `HD_Blue` over a third of a second. This fails if the grid draws the
/// screen's clock, or the pin is dropped.
#[test]
fn the_rings_pulse_follows_its_own_clock() {
    let blue = 0xffac_0717;
    let colour = |picker: &Picker| ring_rect(&grid_draws(picker)).expect("the ring").1;
    let mut picker = grid();
    picker.tick(5.0);
    assert_eq!(
        colour(&picker),
        oag_ui::screen::argb_to_rgba(hex::pulse_argb(blue, hex::pulse_phase(5.0)))
    );
    let unpinned = colour(&picker);
    picker.set_ring_seconds(Some(0.195));
    assert_eq!(
        colour(&picker),
        oag_ui::screen::argb_to_rgba(hex::pulse_argb(blue, 0.585))
    );
    assert_ne!(colour(&picker), unpinned);
}

/// A list (no rows) draws no leading slide and has no cooldown: `with_rows(0)` keeps both off.
#[test]
fn a_list_without_rows_neither_leads_nor_cools() {
    let entries = ["a", "b", "c"]
        .iter()
        .map(|id| circuit(id, false))
        .collect();
    let mut picker = Picker::new(Kind::Track, entries, None, None).with_rows(0);
    assert_eq!(press(&mut picker, Button::Right), vec![Event::Moved]);
    assert_eq!(press(&mut picker, Button::Right), vec![Event::Moved]);
    picker.tick(1.0 / 60.0);
    assert!(
        picker.slide().progress() < 1e-4,
        "no step frame tick on a list"
    );
}
