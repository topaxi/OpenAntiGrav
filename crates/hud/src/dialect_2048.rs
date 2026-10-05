//! Wipeout 2048's own naming quirks: the widgets it names differently from
//! the shared Pulse/HD dialect [`super::draw`] otherwise assumes, and the
//! `EnergyBg`/`EnergyBar`/`EnergyBarDelay` trio decompiled off this title's
//! own `Hud_UpdateEnergyBar` (`0x811957a2`, `/2048/eboot-vita-2048-eu-v104.elf`),
//! a different address on a different binary from Pulse's own function of
//! the same name.
//!
//! Split out of [`super::draw`] under the 1,000-line rule in
//! `scripts/check-file-size.py`, the same seam [`super::shield_bar`] and
//! [`super::lap_splits`] already used.

use super::{Layout, Readout, Sprite, argb_to_rgba};

/// One widget for the whole `1/3`, where Pulse/HD split lap and lap count
/// into `Lap`/`LapOf`/`Lap Outof`. `Arcade_HUD.xml` authors `Laps` alone,
/// with no `Lap`/`Lap Outof` companions anywhere in the composed layout
/// (`vita_2048_hud_dump` against `data/extracted/vita/PCSF00007`), and
/// `11-load-8.png` (`docs/formats/2048-hud.md`) reads `1/3` as one string
/// under the `LAP` caption. `Laps` names nothing on Pulse/HD, so this needs
/// no title flag to stay out of their way.
pub(super) fn laps_text(readout: &Readout) -> Option<String> {
    (readout.lap > 0 && readout.laps > 0).then(|| format!("{}/{}", readout.lap, readout.laps))
}

/// `Position`'s text, combined (`8/8`) when `combined` is set - 2048's own
/// shape, one widget and no `PositionOf`/`Position Outof` companions
/// (`vita_2048_hud_dump`, same evidence as [`laps_text`]) - and the bare
/// place digit otherwise, Pulse/HD's own split reading. `11-load-8.png`
/// reads `8/8` under the `POS` caption.
pub(super) fn position_text(readout: &Readout, combined: bool) -> Option<String> {
    (readout.place > 0).then(|| match combined {
        true if readout.ships > 0 => format!("{}/{}", readout.place, readout.ships),
        _ => readout.place.to_string(),
    })
}

/// Which vertical-crop fraction a widget's height tracks -
/// [`super::draw::bar_fraction`]'s counterpart for the one dialect that
/// fills bottom-up. `EnergyBar` is the immediate fill, cropped straight to
/// [`Readout::shield_fraction`]; `EnergyBarDelay` is the lagging trail
/// behind it, cropped to [`Readout::energy_bar_delay_fraction`] instead -
/// both confirmed by the same decompile this module's own doc comment
/// names: `Hud_UpdateEnergyBar` applies the identical rect-plus-uv crop
/// twice, once per widget, differing only in which fraction feeds it. See
/// [`super::draw::crop_vertically`] for the crop itself and
/// `crate::race::telemetry::Race::advance_energy_bar_delay` for how the
/// lagging fraction is kept.
pub(super) fn vertical_bar_fraction(name: &str, readout: &Readout) -> Option<f32> {
    match name {
        "EnergyBar" => Some(readout.shield_fraction()),
        "EnergyBarDelay" => Some(readout.energy_bar_delay_fraction),
        _ => None,
    }
}

/// Opaque, unblended light grey - `Hud_UpdateEnergyBar`'s own
/// `0xffa7a5a7`, the colour it writes onto `EnergyBg` every tick the pool is
/// not critical.
const NORMAL_ARGB: u32 = 0xffa7_a5a7;

/// Opaque red - the same function's `0xffff0000`, written instead at or
/// under the critical threshold or during the shared post-hit flash.
const CRITICAL_ARGB: u32 = 0xffff_0000;

/// `EnergyBg`'s runtime tint, or `None` for any other widget.
///
/// **Read straight off the decompile, not recalled from a frame.**
/// `Hud_UpdateEnergyBar` calls `FUN_8109cfae(energyBg, 0xffff0000, 0)` when
/// the shield percentage is at or under `20.0` - literally
/// [`oag_physics::damage::CRITICAL_PERCENT`] - **or** a second timer
/// (`hud+0x260`) is still running from the tick the truncated integer
/// percentage last dropped, and `FUN_8109cfae(energyBg, 0xffa7a5a7, 0)`
/// otherwise. That second timer is structurally
/// [`Readout::shield_flashing`]'s own ~1 s arm-on-drop shape (count up by
/// `dt`, reset past `1.0`), already computed every tick for every title, so
/// this reuses it rather than re-deriving a parallel one from a truncated
/// comparison the size of the difference does not justify. Confidence 85 on
/// the whole function (`docs/ghidra/functions/vita-2048-eu-v104/pickup-icon-uv-table.md`);
/// the fixed-point colour draw call, the two literal ARGB words and the
/// `20.0` threshold are read directly off `eboot.elf` (`0x811957a2`), not
/// inferred.
///
/// **Left out**: a third, separate 8 Hz blink this same branch drives on an
/// unidentified widget (`hud+8`, not `EnergyBg` or `EnergyBar`) - the target
/// of that write was not chased past the offset, so it is not reproduced
/// here rather than guessed at. See `docs/formats/2048-hud.md`.
pub(super) fn energy_bg_tint(sprite: &Sprite, readout: &Readout) -> Option<Sprite> {
    if sprite.name != "EnergyBg" {
        return None;
    }
    let critical = readout.shield_fraction() * 100.0 <= oag_physics::damage::CRITICAL_PERCENT;
    let argb = if critical || readout.shield_flashing {
        CRITICAL_ARGB
    } else {
        NORMAL_ARGB
    };
    Some(Sprite {
        color: argb_to_rgba(argb),
        ..sprite.clone()
    })
}

/// The name of 2048's one pickup-icon widget, rewritten per weapon rather
/// than selected by name. See [`oag_title::HudArt::pickup_icon_uv`].
pub(super) const PICKUP_ICON_2048: &str = "PickupIcon";

/// The backdrop the held pickup's icon sits inside, on 2048's dialect.
///
/// Not `PICKUP_BACKGROUND`: `HUD_pickups.xml`'s `PickupBackground` authors
/// no `<Values>` at all on this title (see `oag_2048::hud::ART`'s own doc
/// comment), and the visible arc round the icon is a different, separately
/// named widget instead.
pub(super) const PICKUP_BG_FRAME_2048: &str = "PickupBgFrame";

/// [`super::draw::pickup_sprites`]'s branch for 2048's dialect: one
/// `PickupIcon` widget, UV rewritten per weapon from `art.pickup_icon_uv`,
/// drawn centred inside [`PICKUP_BG_FRAME_2048`]'s own authored rect.
///
/// **The held state only.** The frames show the icon at a *second*,
/// top-centre position once, with a caption, the instant a pickup is
/// granted (`docs/formats/2048-hud.md`'s "The pickup slot") - that
/// announcement has no state to key off yet (`Readout` carries no
/// time-since-grant), so this draws only the steady held state, which is
/// what a race shows the rest of the time a pickup is carried.
///
/// **The destination rect is measured, not decompiled.** `Hud_UpdatePickupIcon`
/// (`docs/ghidra/functions/vita-2048-eu-v104/pickup-icon-uv-table.md`) never
/// writes `PickupIcon`'s own `x`/`y`, only its UV; the held icon's on-screen
/// position was found by scanning a captured frame's pixels for the icon's own
/// tint and matches "native 85x86 size, centred inside `PickupBgFrame`'s own
/// authored rect" to within 2-3 px on every edge - confidence 80, one frame,
/// one weapon.
pub(super) fn pickup_sprites_uv_rewrite(
    layout: &Layout,
    weapon: oag_tables::weapons::Weapon,
    uv_table: [Option<[u16; 4]>; 14],
) -> Vec<Sprite> {
    let frame = layout.sprite(PICKUP_BG_FRAME_2048);

    let mut sprites = Vec::new();
    // The backdrop draws whenever a weapon is held, whether or not this title
    // has an icon for it - the same "backdrop alone, rather than nothing or a
    // panic" rule `pickup_sprites`' Pulse/HD branch already keeps for a name
    // the layout does not author.
    if let Some(frame) = frame {
        sprites.push(frame.clone());
    }

    if let (Some(uv), Some(icon)) = (uv_table[weapon as usize], layout.sprite(PICKUP_ICON_2048)) {
        let mut icon = icon.clone();
        icon.uv = uv.map(f32::from);
        if let Some(frame) = frame {
            icon.rect[0] = frame.rect[0] + (frame.rect[2] - icon.rect[2]) * 0.5;
            icon.rect[1] = frame.rect[1] + (frame.rect[3] - icon.rect[3]) * 0.5;
        }
        sprites.push(icon);
    }
    sprites
}

/// What one `SpeedBarN` segment is worth: the `0x430c0000` (140.0) that
/// `Hud_BindWidgets` (`0x81192cd0`) stores at `hud+0x1d0` and the HUD update
/// (`0x81195cdc`, dirty bit `1`) multiplies by `N + 1`.
const SPEED_BAR_STEP_KMH: f32 = 140.0;

/// How many `ZoneLightN` dashes the arc has.
const ZONE_LIGHTS: u32 = 10;

/// The widgets 2048 draws off race state rather than always: the lit speed
/// segments, the thrust swoosh, the Pilot Assist icon and the Zone arc's
/// lit dashes. Empty on every other title and outside the layouts that
/// author them.
///
/// - **`SpeedBar0`-`4`** are discrete: segment `i` is up while
///   `speed >= (i + 1) * 140` km/h, `speed` being the player's `|v| * 3.6`
///   (`0x811b1ee2`, `player+0x48`). Read off the disassembly of `0x81195cdc`
///   (`vcmpe`/`bmi`), not a recalled frame.
/// - **`ThrustBar`** is the horizontal crop [`super::draw::crop_horizontally`]
///   already does, to [`Readout::thrust_chase_percent`] of its width.
/// - **`PilotAssist`** is up while [`Readout::pilot_assist`] is. The original
///   also pulses its scale by `1 + 0.1 * sin(phase)`; that is not drawn.
/// - **`ZoneLight0`-`9`** light from the **last** index backwards, one per
///   zone reached: `Hud_UpdateZoneSpeedClassWidget` (`0x81197d6c`) hides
///   light `i` while `i < 9 - X` for a non-zero counter `X`, which lights
///   `X + 1`, and the Zone frames show two lit at zone 2 and five at zone 5
///   (`68-zone-5.png`: the bottom dashes), so `X` is the zone minus one
///   there. The counter itself (`hud+0x73c`, constructor-zeroed by
///   `0x81197b88`) has no reader of its writer found, so this takes
///   `X = zone - 1` and **follows the decompiled branch from there**, which
///   is unmeasured at the edges: zone 0 hides every light (`9 - (-1)` is
///   ten), zone 1 (`X == 0`) lights all ten, and past zone 10 the unsigned
///   `9 - X` wraps and hides them all again. Only zones 2 and 5 are seen.
///
/// The `SpeedPad*` family is not drawn: the original picks it **once at
/// bind** from a ship-definition float, not from touching a pad.
pub(super) fn state_sprites(
    layout: &Layout,
    art: &oag_title::HudArt,
    readout: &Readout,
) -> Vec<Sprite> {
    if art.pickup_icon_uv.is_none() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 0..5u32 {
        let lit = readout.speed_kmh >= (i + 1) as f32 * SPEED_BAR_STEP_KMH;
        if let (true, Some(bar)) = (lit, layout.sprite(&format!("SpeedBar{i}"))) {
            out.push(bar.clone());
        }
    }
    let thrust = (readout.thrust_chase_percent * 0.01).clamp(0.0, 1.0);
    if thrust > 0.0
        && let Some(bar) = layout.sprite("ThrustBar")
    {
        out.push(super::draw::crop_horizontally(bar, thrust));
    }
    if readout.pilot_assist
        && let Some(icon) = layout.sprite("PilotAssist")
    {
        out.push(icon.clone());
    }
    let lit_lights = match readout.zone {
        0 => 0,
        1 => ZONE_LIGHTS,
        zone if zone <= ZONE_LIGHTS => zone,
        _ => 0,
    };
    for i in (ZONE_LIGHTS - lit_lights)..ZONE_LIGHTS {
        if let Some(light) = layout.sprite(&format!("ZoneLight{i}")) {
            out.push(light.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy_bg() -> Sprite {
        Sprite {
            name: "EnergyBg".to_string(),
            rect: [27.0, 372.0, 50.0, 110.0],
            uv: [270.0, 16.0, 52.0, 110.0],
            color: [1.0, 1.0, 1.0, 1.0],
            src: "Data\\XML\\2048_hud\\Texture\\hud_2048.gxt".to_string(),
            rotation: 0.0,
        }
    }

    fn readout(shield: f32, shield_max: f32, shield_flashing: bool) -> Readout {
        Readout {
            shield,
            shield_max,
            shield_flashing,
            ..Readout::blank()
        }
    }

    /// A healthy, untouched pool tints light grey - `Hud_UpdateEnergyBar`'s
    /// own `0xffa7a5a7` - matching a Vita3K frame of the running original at
    /// full shield, not left at the layout's own authored opaque white.
    #[test]
    fn a_healthy_pool_tints_light_grey() {
        let tinted = energy_bg_tint(&energy_bg(), &readout(100.0, 100.0, false)).unwrap();
        assert_eq!(tinted.color, argb_to_rgba(NORMAL_ARGB));
    }

    /// At or under the critical threshold, red - `36-w-15.png`'s red silhouette.
    #[test]
    fn a_critical_pool_tints_red() {
        let tinted = energy_bg_tint(&energy_bg(), &readout(20.0, 100.0, false)).unwrap();
        assert_eq!(tinted.color, argb_to_rgba(CRITICAL_ARGB));
    }

    /// The shared post-hit flash window also forces red, above the floor -
    /// `Hud_UpdateEnergyBar`'s second, independent `hud+0x260` condition.
    #[test]
    fn a_healthy_but_flashing_pool_also_tints_red() {
        let tinted = energy_bg_tint(&energy_bg(), &readout(90.0, 100.0, true)).unwrap();
        assert_eq!(tinted.color, argb_to_rgba(CRITICAL_ARGB));
    }

    /// Any other widget's name is untouched - this is `EnergyBg`'s own tint,
    /// not a blanket shield-critical override.
    #[test]
    fn any_other_widget_is_left_alone() {
        let mut other = energy_bg();
        other.name = "EnergyBar".to_string();
        assert!(energy_bg_tint(&other, &readout(5.0, 100.0, false)).is_none());
    }

    /// `Laps`/`Position` combine only when asked to, and stay `None` with no
    /// place or laps to report - the same "nothing means nothing" rule every
    /// other arm in `draw::text_for` follows.
    #[test]
    fn laps_and_position_combine_and_omit_the_same_as_every_other_arm() {
        let full = Readout {
            lap: 2,
            laps: 3,
            place: 3,
            ships: 8,
            ..Readout::blank()
        };
        assert_eq!(laps_text(&full), Some("2/3".to_string()));
        assert_eq!(position_text(&full, true), Some("3/8".to_string()));
        assert_eq!(position_text(&full, false), Some("3".to_string()));
        assert_eq!(laps_text(&Readout::blank()), None);
        assert_eq!(position_text(&Readout::blank(), true), None);
    }
}
