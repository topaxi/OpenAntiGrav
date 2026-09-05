//! What [`super::draw::draw_list`] draws for `ShieldBar`: the threshold, the
//! post-hit flash and the authored-colour default `Hud_UpdateEnergyBar`
//! recovers - see
//! `docs/ghidra/functions/psp-pulse-usa/shield.md#hud_updateenergybar-0x0881c638-tints-the-bar-from-a-20-threshold-not-a-gradient`.
//!
//! Its own file rather than more of `hud/tests.rs`, which is at the
//! 1,000-line ceiling `scripts/check-file-size.py` enforces - the same reason
//! [`super::zone_tests`] and [`super::reticle_tests`] are.
//!
//! [`super::race::telemetry::shield_flash_tests`] covers the flash timer's own
//! arming and expiry; this covers only what colour a given [`Readout`]
//! produces, which is the half of the rule this handover thread was scoped to
//! wire in.

use super::*;

/// `ShieldBar`'s authored colour in `Arcade_HUD.xml`, `FEConst->HudColour3`
/// read as a literal so the test needs no constant table - a light cyan.
const AUTHORED_ARGB: u32 = 0xFF0D_DFDD;

fn strings() -> crate::language::StringTable {
    crate::language::StringTable::default()
}

/// One `ShieldBar` sprite over its own tiny atlas, and a `SpeedBar` beside it
/// so a test can confirm the tint is not bleeding into the other bar that
/// shares this same crop-and-draw path.
fn layout() -> Layout {
    Layout::from_xml(&format!(
        r#"<Screen>
             <Image name="ShieldBar">
               <Values x="0" y="0" width="100" height="10" U="0" V="0"
                       TxtrWidth="100" TxtrHeight="10"
                       Color="0x{AUTHORED_ARGB:08X}"
                       Src="Data\HUD\Textures\PulseHUD.mip"/>
             </Image>
             <Image name="SpeedBar">
               <Values x="0" y="20" width="100" height="10" U="0" V="20"
                       TxtrWidth="100" TxtrHeight="10"
                       Color="0xFF7DEFC0"
                       Src="Data\HUD\Textures\PulseHUD.mip"/>
             </Image>
           </Screen>"#
    ))
}

fn sheet() -> crate::sprite::Sheet {
    crate::sprite::Sheet::placed_at(&[(
        r"Data\HUD\Textures\PulseHUD.mip",
        crate::sprite::Placed {
            x: 0,
            y: 0,
            width: 128,
            height: 128,
            quad_extent: None,
        },
    )])
}

fn context<'a>(
    layout: &'a Layout,
    strings: &'a crate::language::StringTable,
    sheet: &'a crate::sprite::Sheet,
) -> Context<'a> {
    Context {
        default_border: layout.default_border(),
        layout,
        strings,
        sheet,
        art: oag_pulse::hud::ART,
        hud_line_height: 25.0,
        small_line_height: 10.0,
    }
}

/// A [`Readout`] with only the shield fields set, everything else blank.
fn readout_with(shield: f32, shield_max: f32, flashing: bool) -> Readout {
    Readout {
        shield,
        shield_max,
        shield_flashing: flashing,
        ..Readout::blank()
    }
}

/// The colour `draw_list` puts on the named sprite this frame, or `None` if it
/// drew nothing at all.
fn drawn_colour(readout: &Readout, name: &str) -> Option<[f32; 4]> {
    let layout = layout();
    let strings = strings();
    let sheet = sheet();
    let cx = context(&layout, &strings, &sheet);
    draw_list(&cx, readout)
        .sprites
        .into_iter()
        .find_map(|s| match s {
            Draw::Sprite { rect, color, .. } if rect[1] == layout_rect_y(&cx, name) => Some(color),
            _ => None,
        })
}

/// `ShieldBar` and `SpeedBar` differ only in `y`, which is what
/// [`drawn_colour`] tells them apart by - `Draw::Sprite` carries no name.
fn layout_rect_y(cx: &Context<'_>, name: &str) -> f32 {
    cx.layout.sprite(name).expect("test layout names it").rect[1]
}

/// Well above the 20% floor and outside the flash window: the widget's own
/// authored colour, untouched.
#[test]
fn authored_colour_when_neither_condition_holds() {
    let readout = readout_with(50.0, 100.0, false);
    let color = drawn_colour(&readout, "ShieldBar").expect("a half-full bar draws");
    assert_eq!(color, argb_to_rgba(AUTHORED_ARGB));
}

/// At and under 20%, forced solid red - `15%` rather than `0%`, since
/// `draw_list` drops a bar at `fraction <= 0.0` entirely and a zero-width
/// quad would prove nothing about colour.
#[test]
fn forced_red_at_or_under_twenty_percent() {
    for shield in [15.0, 20.0] {
        let readout = readout_with(shield, 100.0, false);
        let color = drawn_colour(&readout, "ShieldBar").unwrap_or_else(|| {
            panic!("a {shield}% bar draws");
        });
        assert_eq!(color, [1.0, 0.0, 0.0, 1.0], "shield={shield}");
    }
}

/// One tenth of a percent over the floor, not flashing: still the authored
/// colour. The disc's own test is `<=`, and this is the boundary it excludes.
#[test]
fn authored_colour_just_above_the_floor() {
    let readout = readout_with(20.1, 100.0, false);
    let color = drawn_colour(&readout, "ShieldBar").expect("a 20.1% bar draws");
    assert_eq!(color, argb_to_rgba(AUTHORED_ARGB));
}

/// The post-hit flash forces red regardless of the percentage - a pool still
/// at 90% flashes solid red the instant it drops, per `shield.md`'s reading of
/// `Hud_UpdateEnergyBar`.
#[test]
fn flash_forces_red_above_the_floor() {
    let readout = readout_with(90.0, 100.0, true);
    let color = drawn_colour(&readout, "ShieldBar").expect("a 90% bar draws");
    assert_eq!(color, [1.0, 0.0, 0.0, 1.0]);
}

/// The tint is `ShieldBar`'s alone. `SpeedBar` shares the same crop-and-draw
/// path and must not pick up the flash meant for the other bar.
#[test]
fn the_flash_does_not_bleed_into_the_speed_bar() {
    let mut readout = readout_with(90.0, 100.0, true);
    readout.speed_kmh = 300.0;
    readout.speed_full_kmh = 600.0;
    let color = drawn_colour(&readout, "SpeedBar").expect("a half-full speed bar draws");
    assert_eq!(color, argb_to_rgba(0xFF7D_EFC0));
}
