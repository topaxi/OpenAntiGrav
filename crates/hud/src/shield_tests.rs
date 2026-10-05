//! What [`super::draw::draw_list`] draws for `ShieldBar`: the threshold, the
//! post-hit flash and the authored-colour default `Hud_UpdateEnergyBar`
//! recovers - see
//! `docs/ghidra/functions/psp-pulse-usa/shield.md#hud_updateenergybar-0x0881c638-tints-the-bar-from-a-20-threshold-not-a-gradient`.
//!
//! Its own file rather than more of `hud/tests.rs`, which is at the
//! 1,000-line ceiling `scripts/check-file-size.py` enforces - the same reason
//! [`super::zone_tests`] and [`super::reticle_tests`] are.
//!
//! `crate::race::telemetry`'s own `shield_flash_tests` covers the flash
//! timer's own arming and expiry; this covers only what colour a given
//! [`Readout`] produces, which is the half of the rule this handover thread
//! was scoped to wire in.

use super::*;

/// `ShieldBar`'s authored colour in `Arcade_HUD.xml`, `FEConst->HudColour3`
/// read as a literal so the test needs no constant table - a light cyan.
const AUTHORED_ARGB: u32 = 0xFF0D_DFDD;

fn strings() -> oag_ui::language::StringTable {
    oag_ui::language::StringTable::default()
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
            blend: None,
        },
    )])
}

fn context<'a>(
    layout: &'a Layout,
    strings: &'a oag_ui::language::StringTable,
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
        default_line_height: 10.0,
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

/// Every colour `draw_list` puts on the named sprite's rect this frame, in
/// paint order - plural because absorbing draws two: the `+0xf4` flash layer
/// under the bar itself, then the bar. See [`drawn_colour`] for the
/// single-sprite case every test above this one uses.
fn drawn_colours(readout: &Readout, name: &str) -> Vec<[f32; 4]> {
    let layout = layout();
    let strings = strings();
    let sheet = sheet();
    let cx = context(&layout, &strings, &sheet);
    draw_list(&cx, readout)
        .sprites
        .into_iter()
        .filter_map(|s| match s {
            Draw::Sprite { rect, color, .. } if rect[1] == layout_rect_y(&cx, name) => Some(color),
            _ => None,
        })
        .collect()
}

/// Absorbing suppresses forced red even under the 20% floor: `iVar1`
/// (identified as [`Readout::shield_absorbing`]) bypasses the whole
/// forced-red branch in the original, not merely this build's earlier
/// approximation of it. `shield_blink_phase` defaults to `0.0`, an "on"
/// blink phase (see the phase tests below), so the bar's own alpha is
/// untouched and only the colour is at stake here.
#[test]
fn absorbing_suppresses_forced_red_below_the_floor() {
    let mut readout = readout_with(5.0, 100.0, false);
    readout.shield_absorbing = true;
    let colours = drawn_colours(&readout, "ShieldBar");
    let bar = colours.last().expect("a 5% absorbing bar still draws");
    assert_eq!(*bar, argb_to_rgba(AUTHORED_ARGB));
}

/// Absorbing still blinks even though it is never forced red - the original
/// enters the same `hud+0x1dc` accumulator either way. At an "off" phase the
/// bar's own alpha drops to zero.
#[test]
fn absorbing_blinks_even_when_healthy() {
    let mut readout = readout_with(90.0, 100.0, false);
    readout.shield_absorbing = true;
    readout.shield_blink_phase = 0.2; // floor(0.2*8)=1, the "off" half of the cycle.
    let colours = drawn_colours(&readout, "ShieldBar");
    let bar = colours.last().expect("a 90% absorbing bar still draws");
    assert_eq!(bar[3], 0.0, "the off phase zeroes the bar's own alpha");
}

/// The `+0xf4` highlight: a flat white copy of the same crop, painted before
/// (under) the bar's own colour, present only while absorbing.
#[test]
fn absorbing_draws_a_white_flash_under_the_bar() {
    let mut readout = readout_with(90.0, 100.0, false);
    readout.shield_absorbing = true;
    let colours = drawn_colours(&readout, "ShieldBar");
    assert_eq!(colours.len(), 2, "the flash layer plus the bar itself");
    assert_eq!(colours[0], [1.0, 1.0, 1.0, 1.0], "the flash is flat white");
}

/// Outside the absorb window, no second sprite - every test above this one
/// in the file already relies on that being true.
#[test]
fn no_flash_layer_outside_the_absorb_window() {
    let readout = readout_with(90.0, 100.0, false);
    let colours = drawn_colours(&readout, "ShieldBar");
    assert_eq!(colours.len(), 1, "only the bar itself");
}

/// [`Readout::shield_blink_phase_on`]'s own parity, off
/// [`Readout::shield_blink_phase`] alone - `Hud_UpdateEnergyBar`'s
/// `(uint)(t * 8.0) & 1`, read directly off the decompile.
#[test]
fn blink_phase_toggles_every_eighth_of_a_second() {
    let phase_0 = Readout {
        shield_blink_phase: 0.0,
        ..Readout::blank()
    };
    assert!(phase_0.shield_blink_phase_on(), "t=0.0s, phase 0, even");
    let phase_1 = Readout {
        shield_blink_phase: 0.133,
        ..Readout::blank()
    };
    assert!(!phase_1.shield_blink_phase_on(), "t=0.133s, phase 1, odd");
    let phase_2 = Readout {
        shield_blink_phase: 0.25,
        ..Readout::blank()
    };
    assert!(phase_2.shield_blink_phase_on(), "t=0.25s, phase 2, even");
}

/// [`Readout::shield_blinking`] is true while absorbing alone, a full pool
/// and no post-hit flash - the case [`Readout::shield_forced_red`] used to
/// conflate with "never blinks".
#[test]
fn shield_blinking_is_true_while_absorbing_even_when_healthy() {
    let mut readout = readout_with(90.0, 100.0, false);
    readout.shield_absorbing = true;
    assert!(readout.shield_blinking());
}

/// The converse: healthy, no flash, not absorbing - no blink at all.
#[test]
fn shield_blinking_is_false_with_nothing_active() {
    let readout = readout_with(90.0, 100.0, false);
    assert!(!readout.shield_blinking());
}
