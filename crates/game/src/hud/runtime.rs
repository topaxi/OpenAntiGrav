//! What a title's per-tick HUD update writes over its layout:
//! [`oag_title::hud::RuntimeHud`]'s rules, applied to one frame.
//!
//! HD is the one title with this read, off its executable
//! (`docs/ghidra/functions/ps3-hdfury-eu/hud-readouts.md`), and every rule
//! below is the one read there unless its own comment says "chosen":
//!
//! - **The shield readout.** `DamageBar` is cropped from the top to the shield
//!   fraction, bottom edge fixed, and drawn opaque in the readout's colour;
//!   `ShieldBarText` takes the same colour; `DamageBarBg` is forced white,
//!   and flashes the warning colour on a flash's "on" phase. A flash runs at
//!   or under the critical percentage, or through the post-hit second.
//! - **The two arcs.** Segments are shown or hidden, never tinted: the
//!   yellow is baked into the atlas.

use oag_title::hud::{RuntimeHud, SegmentArc, ShieldReadout};
use oag_ui::screen::argb_to_rgba;

use super::draw::{Context, crop_vertically, sprite_draw};
use super::{Draw, Readout, Sprite};

/// Opaque alpha, as the high byte of an ARGB word.
const OPAQUE: u32 = 0xFF00_0000;

/// The shield percentage the way the original computes it: `shield * 100 /
/// max`, floored at zero, **not** capped at 100.
fn percent(readout: &Readout) -> f32 {
    if readout.shield_max <= 0.0 {
        return 0.0;
    }
    (readout.shield * 100.0 / readout.shield_max).max(0.0)
}

/// `ShieldBarText`'s digits: the percentage **truncated**, not rounded - the
/// original converts with `fctiwz` - and no `%` sign.
pub(super) fn shield_digits(readout: &Readout) -> String {
    // Truncation toward zero is `as`'s own rule for a non-negative float.
    format!("{}", percent(readout) as u32)
}

/// Whether the readout is flashing this frame: at or under the critical
/// percentage, through the post-hit window, or absorbing.
///
/// The post-hit window is [`Readout::shield_flashing_whole`], HD's own: armed
/// by a drop of the **truncated whole** percentage, which the running
/// original shows (60.9 to 60.2 arms nothing, 60.2 to 59.9 does; hud-readouts.md).
/// The third condition is `0x000cf490`, "within a second of `ship+0x6a80`",
/// and `ship+0x6a80` is stamped as the pickup absorb's feedback starts, so it
/// is [`Readout::shield_absorbing`]. The other half of the original's test,
/// `ship+0x6958` (a craft with a live beam of a weapon type 4), is not wired.
fn flashing(shield: &ShieldReadout, readout: &Readout) -> bool {
    readout.shield_absorbing
        || readout.shield_flashing_whole
        || percent(readout) <= shield.critical_percent as f32
}

/// Whether a flash is in its "on" phase: `floor(t * phases_per_second)` even,
/// `t` being the original's own accumulator ([`Readout::shield_blink_phase_whole`]).
///
/// That accumulator runs only while flashing, **keeps its value between
/// flashes** and wraps once it passes one second, so where in its cycle a
/// flash starts is the original's too: a flash that begins at a stored
/// 0.9833 starts on an "off" phase (`floor(7.9)` is odd), measured on the
/// running original.
fn phase_on(shield: &ShieldReadout, readout: &Readout) -> bool {
    let phase = (readout.shield_blink_phase_whole * shield.phases_per_second as f32).floor() as i64;
    phase.rem_euclid(2) == 0
}

/// The readout's colour this frame, `0xRRGGBB`.
///
/// White in HD's Eliminator: the original's mode test sends modes 8 and 20
/// (`SPElimination`, `MPElimination`) - and 13, 14 and 21, none of which
/// this build races - to white, and every other mode to [`ShieldReadout::rgb`].
fn rgb(shield: &ShieldReadout, readout: &Readout) -> u32 {
    if readout.mode == oag_race::Mode::Eliminator {
        0x00FF_FFFF
    } else {
        shield.rgb
    }
}

/// The colour `name` is drawn in this frame when the runtime overrides what
/// its layout authors, or `None` to keep the layout's.
///
/// `ShieldBarText` is opaque in the readout's colour, and fully transparent
/// on a flash's "off" phase; `DamageBarBg` is white, or the warning colour
/// on a flash's "on" phase - **except while absorbing**, when the background
/// stays white and the fill blinks with the number instead (`shield_fill`).
pub(super) fn colour(cx: &Context<'_>, readout: &Readout, name: &str) -> Option<[f32; 4]> {
    let shield = &cx.art.runtime?.shield;
    let blinking = flashing(shield, readout);
    let on = !blinking || phase_on(shield, readout);
    if name == shield.text {
        let alpha = if on { OPAQUE } else { 0 };
        return Some(argb_to_rgba(alpha | rgb(shield, readout)));
    }
    if name == shield.background {
        let argb = if blinking && on && !readout.shield_absorbing {
            OPAQUE | shield.warning_rgb
        } else {
            0xFFFF_FFFF
        };
        return Some(argb_to_rgba(argb));
    }
    None
}

/// `sprite` in the colour the runtime draws it in, or `None` to draw it as
/// authored. See [`colour`].
pub(super) fn tinted(cx: &Context<'_>, readout: &Readout, sprite: &Sprite) -> Option<Sprite> {
    colour(cx, readout, &sprite.name).map(|color| Sprite {
        color,
        ..sprite.clone()
    })
}

/// The fill drawn over `background`, cropped and coloured, when `background`
/// is this title's shield background.
///
/// **Which of the layout's `DamageBar`s is the fill is an inference.**
/// `arcade_hud.xml` composes two widgets by that name - the hexagon's own,
/// and an unrelated pickup-bar piece - and the original's name lookup
/// (`0x0067e6d0`) was not read far enough to say which it binds. The one
/// taken here is the one sharing its background's texture and source
/// rectangle, which is the hexagon's; three frames of the running original
/// show that hexagon filled blue, which a bound pickup piece would have left
/// in its authored red.
pub(super) fn shield_fill(
    cx: &Context<'_>,
    readout: &Readout,
    background: &Sprite,
) -> Option<Draw> {
    let shield = &cx.art.runtime?.shield;
    if background.name != shield.background {
        return None;
    }
    let fill = cx.layout.sprites.iter().find(|sprite| {
        sprite.name == shield.fill && sprite.src == background.src && sprite.uv == background.uv
    })?;
    let fraction = percent(readout) * 0.01;
    if fraction <= 0.0 {
        return None;
    }
    let mut cropped = crop_vertically(fill, fraction);
    // The fill is solid through a plain flash and blinks with the number
    // only while absorbing (`0x00086dd8` onward, the third row of the table).
    let alpha = if readout.shield_absorbing && !phase_on(shield, readout) {
        0
    } else {
        OPAQUE
    };
    cropped.color = argb_to_rgba(alpha | rgb(shield, readout));
    sprite_draw(&cropped, cx.sheet)
}

/// The lap and place arcs' segments that are up this frame.
pub(super) fn arc_sprites(cx: &Context<'_>, readout: &Readout) -> Vec<Draw> {
    let Some(runtime) = cx.art.runtime else {
        return Vec::new();
    };
    let mut draws = Vec::new();
    for k in lit_segments(runtime, readout) {
        if let Some(sprite) = cx.layout.sprite(&k) {
            draws.extend(sprite_draw(sprite, cx.sheet));
        }
    }
    draws
}

/// The names of the arc segments up this frame, lap arc first.
fn lit_segments(runtime: &RuntimeHud, readout: &Readout) -> Vec<String> {
    let mut names = Vec::new();
    // No lap count, no lap arc - the original hides every segment when the
    // race has fewer than one lap.
    if readout.laps >= 1 {
        let remaining = i64::from(readout.laps) - i64::from(readout.lap);
        names.extend(segments(&runtime.lap_arc, |k| k >= remaining));
    }
    // No place, no place arc. **Chosen, not read**: the original never meets
    // a place of zero, which its own rule would read as every segment up;
    // zero here means "no place to report" and the rest of the HUD already
    // omits the place group for it.
    if readout.place >= 1 {
        let spare = i64::from(runtime.place_arc.segments) - i64::from(readout.place);
        names.extend(segments(&runtime.place_arc, |k| k <= spare));
    }
    names
}

/// `<prefix>k` for every `k` in the arc that `up` accepts.
fn segments(arc: &SegmentArc, up: impl Fn(i64) -> bool) -> Vec<String> {
    (0..arc.segments)
        .filter(|&k| up(i64::from(k)))
        .map(|k| format!("{}{k}", arc.prefix))
        .collect()
}

#[cfg(test)]
mod tests;
