//! What a frame of the HUD actually draws: which widgets are live, what text
//! they carry, and where each one's pixels come from.
//!
//! Split out of [`super`], which is now the layout and the widget model alone.
//! The division is the one the whole module is built around and is worth having
//! in the file tree: [`super::Layout`] is geometry straight off the disc and
//! decides nothing, and everything here decides - and every decision it makes
//! is a claim about the original that carries its evidence in a doc comment.
//!
//! Nothing here needs a GPU, which is what lets a test ask what a real race's
//! HUD would draw. [`super::Overlay`] is the half that does.

use super::{
    Draw, Font, Label, Layout, Precision, Readout, Sprite, VertAlign, format_lap_time,
    is_screen_positioned,
    lap_splits::{lap_split_row_text, lap_split_sprites},
    pickup::{pickup_model_draws, pickup_sprites},
};

/// The colour a glyph's baked outline takes when a widget names none.
///
/// The layouts declare `HudBGColour` and every widget that *does* carry a
/// `BorderColor` points at it, so using it for the 57 that do not is a
/// data-driven default rather than an invented one. Resolved from the layout;
/// [`FALLBACK_BORDER`] covers a layout that declares no such constant.
///
/// **Why a dark outline at all**: a reference frame of the original shows one on
/// every HUD-font widget, including `CurrentTime` and `BestTime`, which carry no
/// `BorderColor`. An earlier revision drew those with a transparent outline on the
/// grounds that it invented nothing; the frame says the original does draw one.
pub(super) const BORDER_CONSTANT: &str = "HudBGColour";

/// What the outline falls back to when the layout declares no `HudBGColour`.
///
/// Opaque black. Every shipped layout declares the constant, so this is a
/// belt-and-braces default rather than a value in use.
pub(super) const FALLBACK_BORDER: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// Whether a widget is one of the bar fills, whose width tracks a value.
///
/// The two are recognised by name because the layout gives no other signal: a
/// fill and its background are identical in every attribute but colour. See
/// `each_bar_exactly_overlays_its_own_background` in the ground-truth tests.
pub(super) fn bar_fraction(name: &str, readout: &Readout) -> Option<f32> {
    match name {
        "SpeedBar" => Some(readout.speed_fraction()),
        "ShieldBar" => Some(readout.shield_fraction()),
        _ => None,
    }
}

/// Crops a sprite horizontally to `fraction` of its width.
///
/// **The fill model, and it is confidence 80 rather than higher.** A bar and its
/// background share rectangle *and* source rectangle exactly, differing only in
/// colour, which leaves a horizontal crop of the same art as the only way the
/// fill can work - there is no second piece of geometry for it to be. What is not
/// established is that the crop is *linear* in the value, or that it grows from
/// the left. A reference frame at a known speed settles both.
pub(super) fn crop_horizontally(sprite: &Sprite, fraction: f32) -> Sprite {
    let mut out = sprite.clone();
    out.rect[2] = sprite.rect[2] * fraction;
    out.uv[2] = sprite.uv[2] * fraction;
    out
}

/// Crops a sprite vertically to `fraction` of its height, anchored at the
/// bottom - [`crop_horizontally`]'s counterpart for a fill that grows upward
/// rather than rightward. [`super::dialect_2048::vertical_bar_fraction`] is
/// the one caller, for `EnergyBar` and `EnergyBarDelay` - see its own doc
/// comment for which widget gets which fraction.
///
/// **Confidence 80, the same terms `crop_horizontally`'s own doc comment
/// gives.** `rect` and `uv` share one top-left, y-down convention -
/// `crop_horizontally` leaves both `x` origins alone and scales both widths
/// by the same factor, which only works if the two rectangles are unflipped
/// against each other - so shrinking from the top is a shared shift of both
/// `y` origins by the cropped-away height, plus a matching scale of both
/// heights. Not established: that the crop is linear in the value (assumed,
/// on `crop_horizontally`'s own precedent).
pub(super) fn crop_vertically(sprite: &Sprite, fraction: f32) -> Sprite {
    let mut out = sprite.clone();
    let cropped_rect_height = sprite.rect[3] * (1.0 - fraction);
    let cropped_uv_height = sprite.uv[3] * (1.0 - fraction);
    out.rect[1] = sprite.rect[1] + cropped_rect_height;
    out.rect[3] = sprite.rect[3] - cropped_rect_height;
    out.uv[1] = sprite.uv[1] + cropped_uv_height;
    out.uv[3] = sprite.uv[3] - cropped_uv_height;
    out
}

/// The text a widget shows, or `None` when it shows nothing this frame.
///
/// **An allow-list, deliberately.** A widget this function does not name is not
/// drawn, so a layout carrying 35 text widgets - most of them for modes,
/// opponents and progression that do not exist yet - produces only the handful
/// that have a real source. The alternative, drawing every widget and letting the
/// empty ones be invisible, hides the difference between "no value" and "not
/// wired up".
///
/// `place_shown` is the one decision this function cannot make from its own
/// arguments: whether the clock is hidden - the place owning its anchor
/// ([`place_owns_the_anchor`]) or the title's own mode rule
/// ([`super::time_trial_pace::mode_hides_total_time`]). `speed_unit` is the second, and
/// [`SPEED_UNIT_WIDGET`] is why. `classes` is the third: what this title
/// calls each rung of its Zone ladder
/// ([`oag_title::HudArt::zone_speed_classes`], `None` unread). `shield_percent`
/// is the fourth ([`oag_title::HudArt::shield_percent`]). `position_combined`
/// is the fifth, `speed_unit`'s own pattern applied to `Position`: `true`
/// when the layout authors no `Position Outof` companion - 2048's shape, one
/// widget carrying the whole `8/8`.
#[allow(clippy::too_many_arguments)]
pub(super) fn text_for(
    label: &Label,
    readout: &Readout,
    strings: &oag_ui::language::StringTable,
    place_shown: bool,
    speed_unit: bool,
    classes: Option<&oag_title::ZoneSpeedClasses>,
    shield_percent: bool,
    position_combined: bool,
) -> Option<String> {
    // A literal in the XML wins for the widgets that have one: `LapOf`'s "/" is
    // the separator between lap and total, and it is authored rather than
    // computed.
    let literal =
        |fallback: &str| -> String { label.string.clone().unwrap_or_else(|| fallback.to_string()) };

    match label.name.as_str() {
        // Values.
        "SpeedBarText" => Some(match speed_unit {
            true => format!("{:.0} kmh", readout.speed_kmh.max(0.0)),
            false => format!("{:.0}", readout.speed_kmh.max(0.0)),
        }),
        // A **percentage** on the titles that measure one - Pulse's own
        // reference frame reads `100%` on an undamaged craft, and `<Misc
        // shield/>` is a few hundred units. HD's three frames
        // (`talons-matched/{00,01,03}.png`) all read a bare number instead -
        // `100`, `100`, `98` - so [`oag_title::HudArt::shield_percent`] carries
        // which, and HD's executable truncates rather than rounds.
        //
        // `EnergyText` is 2048's own name for the same widget, under the
        // shield silhouette rather than beside a horizontal bar; every
        // Vita3K frame with a shield reads a `%` there too, so no separate
        // arm is needed - only the name differs.
        "ShieldBarText" | "EnergyText" => Some(match shield_percent {
            true => format!("{:.0}%", readout.shield_fraction() * 100.0),
            false => super::runtime::shield_digits(readout),
        }),
        // **Drawn nothing, not always-on.** `HUD_pickups.xml`'s
        // `PickupDamageTxt`/`PickupAbsorbTxt` (idstring `MSC_DAMAGE`/
        // `MSC_ABSORB`) and the numbers beside them, `PickupDamage`/
        // `PickupAbsorb`, are the "recent damage taken" / "recent damage
        // absorbed" readout: `talons-matched/03.png` (mid-race, after a hit)
        // shows `DAMAGE 15` and `Absorb 17` on them, and `00.png`/`01.png`
        // (full shield, no recent hit) show neither word - the pill
        // backgrounds (`PickupDamageBG`/`PickupAbsorbBG`, both already
        // `ALWAYS_ON`) are up in all three frames with nothing drawn on top.
        // Falling through to the catch-all's idstring lookup below drew the
        // captions unconditionally, which is the "on in this state" for "on
        // whenever the HUD is" confusion this module's own allow-list rule
        // exists to prevent. Nothing here tracks a recent hit's damage/absorb
        // total - that is a new `RaceState`/`Standing` field, the same
        // prerequisite the per-lap history needed - so this is `None` outright
        // rather than a half-wired gate, until that exists.
        "PickupDamageTxt" | "PickupAbsorbTxt" | "PickupDamage" | "PickupAbsorb" => None,
        // **Drawn nothing.** `HUD_positions.xml`'s `PositionTxt2` is the
        // second of two widgets sharing `idstring="IG_HUD_POS"` - the first,
        // `PositionTxt`, is this module's own `"PositionTxt"` arm below,
        // gated on `place_shown`. `PositionTxt2` sits inside the `PosTag0`-`7`
        // head-to-head cluster (`VoiceCom0`-`7` beside it, per-opponent tags),
        // which every reference frame's own top-right corner shows only once -
        // no `talons-matched` frame draws a second `POS` - and whose sprites
        // are already excluded from every title's `ALWAYS_ON`. Falling
        // through to the catch-all drew this one anyway, since text is not
        // gated by `ALWAYS_ON` the way sprites are: a second `POS` label with
        // nothing under it, floating beside the real one.
        "PositionTxt2" => None,
        // The Eliminator's kill column - `PosTag0` to `PosTag7`. See
        // [`super::kill_tags`].
        "KillsText" => super::kill_tags::header(readout, label, strings),
        name if name.starts_with("PosTag") => super::kill_tags::text(readout, name, strings),
        "Lap" => (readout.lap > 0).then(|| readout.lap.to_string()),
        "Lap Outof" => (readout.laps > 0).then(|| readout.laps.to_string()),
        // 2048's own name and own shape: one widget for the whole `1/3`
        // rather than Pulse/HD's `Lap`/`LapOf`/`Lap Outof` triple. See
        // [`super::dialect_2048::laps_text`].
        "Laps" => super::dialect_2048::laps_text(readout),
        // `position_combined` tells 2048's one-widget `8/8` apart from
        // Pulse/HD's split `Position`/`PositionOf`/`Position Outof` - see
        // [`super::dialect_2048::position_text`].
        "Position" => super::dialect_2048::position_text(readout, position_combined),
        // Both halves of the place are gated on the *place*, not on the field
        // size. The field size is known from the moment a race is set up, so
        // gating this half on `ships` alone drew the caption and a bare `8` with
        // no number and no separator - which is what a single race showed until
        // 2026-08-11.
        "Position Outof" => {
            (readout.place > 0 && readout.ships > 0).then(|| readout.ships.to_string())
        }
        // Running clocks: tenths. See [`Precision`].
        "CurrentTime" => Some(format_lap_time(readout.lap_ticks, Precision::Tenths)),
        // The total time and its caption yield the top-right anchor to the
        // place only when the two actually coincide - see
        // [`place_owns_the_anchor`], `false` throughout on 2048.
        //
        // **A Time Trial/Speed Lap counts down to a target (a campaign
        // medal, or the record) instead of the plain elapsed clock** -
        // [`Readout::time_trial_pace`], `Hud_UpdateTimeCluster_q`'s law.
        // `0` once [`super::TimeTrialPace::missed`], which is what a missed
        // target shows alongside `TotalTime`'s own reddened colour -
        // see [`time_trial_colour`].
        "TotalTime" => (!place_shown).then(|| match readout.time_trial_pace {
            Some(pace) => format_lap_time(u64::from(pace.remaining_ticks), Precision::Tenths),
            None => format_lap_time(readout.race_ticks, Precision::Tenths),
        }),
        "TotalTimeTxt" => (!place_shown)
            .then(|| match readout.time_trial_pace {
                Some(pace) => Some(super::time_trial_pace::tier_caption(pace.tier, strings)),
                None => caption(label, strings),
            })
            .flatten(),
        // **Its own value, not `place_shown`**, which now answers "does the
        // place win a *shared* anchor" - a question about `TotalTime`, not
        // about whether `POS` itself has anything to show. A `POS` with
        // nothing under it reads as a rendering fault; the reachable empty
        // case is a single race on a track with no authored `Start Position`.
        "PositionTxt" => (readout.place > 0)
            .then(|| caption(label, strings))
            .flatten(),
        // A time that has been set: hundredths. With none set the original shows
        // zeros rather than a dash placeholder - `best 0.00.00` on the reference
        // frame - so an absent best formats as zero rather than as its own string.
        "BestTime" => Some(format_lap_time(
            u64::from(readout.best_lap_ticks.unwrap_or(0)),
            Precision::Hundredths,
        )),

        // Zone mode. The number and the score are the two widgets with a real
        // source; `Zone_Bar_*` is the mode's own graphic and nothing was found
        // that writes it, so it stays off the list. See
        // `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`.
        //
        // Drawn from zone 1 onward: the counter is zero for the first ten
        // seconds of a run, and a HUD reading `ZONE 0` would be reporting a zone
        // the player is not in yet.
        "Zone" => (readout.zone > 0).then(|| readout.zone.to_string()),
        "Score" => (readout.zone > 0).then(|| readout.score.to_string()),

        // HD/Fury's zone ladder: the name of the rung the race is on, beside the
        // current row of [`zone_plus`]'s list.
        //
        // **Off the grade's stage, not off the zone number.** Which zone sits on
        // which rung is a per-title table that is unrecovered on HD, and the
        // maintainer's play rules out the obvious guess of one rung per zone -
        // see `oag_hd::hud::ZONE_SPEED_CLASSES`. So this reads the same index
        // the colour grade shows, which leaves the HUD's class name and the
        // circuit's grade wrong together or right together. Nothing on a title
        // with no ladder, and nothing on rung zero, which has no name.
        "SpeedClass" => classes
            .and_then(|classes| classes.id_for(readout.zone_stage))
            .map(|id| strings.get_or_id(id).to_string()),
        // The rung above it, on the row [`zone_next_row`] puts the bar. Both
        // gates matter: with no next rung there is no name, and with no row
        // there is nowhere honest to put it.
        "NextSpeedClass" => readout
            .zone_next_in
            .and(classes)
            .and_then(|classes| classes.id_for(readout.zone_stage + 1))
            .map(|id| strings.get_or_id(id).to_string()),

        // Separators, authored as literals. Drawn only when both sides have a
        // value: a lone `/` between two blanks reads as a rendering fault, and
        // with lap counting unrecovered that is the common case rather than a
        // corner one.
        "LapOf" => (readout.lap > 0 && readout.laps > 0).then(|| literal("/")),
        "PositionOf" => (readout.place > 0 && readout.ships > 0).then(|| literal("/")),

        // The four message slots - see [`super::messages`].
        name if super::messages::slot_of(name).is_some() => super::messages::slot_of(name)
            .and_then(|slot| readout.messages[slot].as_ref())
            .map(|line| strings.get_or_id(&line.text).to_string()),

        // Conditional.
        "WrongWay" => readout.wrong_way.then(|| {
            label.idstring.as_deref().map_or_else(
                || "WRONG WAY".to_string(),
                |id| strings.get_or_id(id).to_string(),
            )
        }),

        // Static labels: anything with an `idstring` that this function has not
        // already claimed - the `IG_HUD_*` captions beside the values.
        // `ZonePlusN` and the per-lap history are checked as families here
        // rather than as arms apiece. See [`lap_split_row_text`].
        name => lap_split_row_text(name, readout).unwrap_or_else(|| {
            // Gated on the **current** row alone, not the ladder - see [`zone_plus`].
            zone_plus(name)
                .map(|ahead| {
                    (ahead > 0 || readout.zone > 0).then(|| (readout.zone + ahead).to_string())
                })
                .unwrap_or_else(|| caption(label, strings))
        }),
    }
}

/// The bar behind the **next** speed class's name, and the label on it.
///
/// Not part of [`oag_title::HudArt::always_on`], because unlike every widget on
/// that list these two are only up when there *is* a next class and a row to put
/// it on. See [`zone_next_row`].
pub(super) const NEXT_CLASS_WIDGETS: [&str; 2] = ["NextSpeedClassBG", "NextSpeedClass"];

/// Where the **next** speed class's bar and name go, as `[x, y]` for each.
///
/// # The authored position is a placeholder, so this reconstructs one
///
/// `zone_hud.xml` writes `<Image name="NextSpeedClassBG">` and its `<Text>` both
/// at `x="0" y="0"` inside `CurrentZonePanel`'s group - which is the *current*
/// row, underneath the bar already there, with the name to the left of that
/// row's own number. `x=0 y=0` is this dialect's "the runtime writes this"
/// idiom, the same one the lock-on reticle uses at `(-960, 540)`
/// ([`oag_title::hud::Sights`]), so the numbers carry no geometry and something
/// has to supply one.
///
/// **What the original does, from the maintainer's play**: an upcoming speed
/// class is named beside **the zone number it starts at**. So the bar sits on
/// that zone's ladder row and slides down as the bands widen - at zone 1 the
/// next bump is zone 2, one row down, which is what the reference frame
/// (`data/shots/hd_zone_hud_original.png`) shows.
///
/// # The rule: the name sits on its row's own line, inset like the current one
///
/// Two frames of the original, at zone 1 and zone 8, and the second is the one
/// that says it plainly. At zone 8 the ladder reads
///
/// ```text
///  8  SUB-RAPIER
///  9
/// 10
/// 11
/// 12  RAPIER
/// 13
/// ```
///
/// `RAPIER` is **level with its own `12`**, one line, the same font and size as
/// the digit beside it - not offset the way `SpeedClass` is offset from the big
/// `ZonePlus0`. So the vertical answer is the row's own `y`, and the horizontal
/// one is the inset `SpeedClass` already has from `ZonePlus0` (66 layout units,
/// which measures at ~39 screen pixels between the digit and the name on that
/// capture, against 42 predicted).
///
/// The bar behind it takes the same row and the panel's own horizontal inset,
/// keeping its authored size - the art is its own, only the anchor is
/// reconstructed.
///
/// **Confidence 80**, up from 72 when only the zone-1 frame existed: that frame
/// put the row-`n` line within six screen pixels of two competing readings and
/// could not separate them, and this one puts a *four-row* gap between the two,
/// where they differ by enough to see. Everything in the rule is read out of the
/// layout; no constant here was chosen to match a picture.
///
/// `None` when there is no next class, or when its row is past `ZonePlus10` -
/// the bottom of the ladder, where the original has nowhere to draw it either.
pub(super) fn zone_next_row(layout: &Layout, readout: &Readout) -> Option<([f32; 2], [f32; 2])> {
    let rows = readout.zone_next_in?;
    let row = layout.label(&format!("ZonePlus{rows}"))?;
    let head = layout.label(CURRENT_ROW_NUMBER)?;
    let panel = layout.sprite(CURRENT_ROW_PANEL)?;
    let class = layout.label(CURRENT_ROW_CLASS)?;
    // Horizontal only: `y` is the row's own line for both, which is what the
    // zone-8 frame shows. `ZonePlus0` and `ZonePlus<n>` are anchored
    // differently - one `middle`, the others `top` - so their raw `y` values are
    // not the same quantity and subtracting them would be a category error as
    // well as the wrong answer.
    Some((
        [panel.rect[0] + row.x - head.x, row.y],
        [class.x + row.x - head.x, row.y],
    ))
}

/// The current row's own number, which the two widgets above are placed
/// against. See [`zone_next_row`].
pub(super) const CURRENT_ROW_NUMBER: &str = "ZonePlus0";
/// The panel behind the current row.
pub(super) const CURRENT_ROW_PANEL: &str = "CurrentZonePanel";
/// The current row's class name.
pub(super) const CURRENT_ROW_CLASS: &str = "SpeedClass";

/// How far ahead of the current zone a `ZonePlus<N>` widget's row is, or `None`
/// for a widget that is not one./// How far ahead of the current zone a `ZonePlus<N>` widget's row is, or `None`
/// for a widget that is not one.
///
/// # The rule is in the name, and the frame reads it back
///
/// HD's Zone layout authors eleven of these down the left-hand column -
/// `ZonePlus0` at the current row and `ZonePlus1`-`ZonePlus10` below it - and
/// nothing but their placeholder strings says what they hold. Those
/// placeholders are `1` through `10`, one per widget, with `ZonePlus0`'s left
/// empty: `N` at rung `N`, which is `zone + N` for a zone counter still at
/// zero.
///
/// **A Zone frame of the running original reads `1` through `11`**
/// (`data/shots/hd_zone_hud_original.png`), with the current row's `1` beside
/// `SUB-VENOM` - zone 1; a second at zone 8 reads `8` through `18`. Confidence
/// **88**: the name, the placeholders and two frames all say `zone + N`, against
/// no disassembly of what writes them.
///
/// # Only the current row is gated on the zone, and the layout is what says so
///
/// The counter is zero from the grid until the first ten-second step, and
/// `Zone`/`Score` are both off for it - a HUD reading `ZONE 0` would be
/// reporting a zone the player is not in yet. An earlier revision extended that
/// to the whole ladder, which left the column blank for the first ten seconds of
/// every run.
///
/// **The authored placeholders are the zone-0 frame, spelled out.** `ZonePlus0`
/// carries no `string` at all and `ZonePlus1`-`ZonePlus10` carry `1` through
/// `10` - which is `zone + N` evaluated at `zone = 0`, with the current row
/// blank. So the disc itself says the rows ahead are numbered before the first
/// zone and the current one is not, and that is the rule rather than a choice
/// made here. It is also a third, independent confirmation of `zone + N`: the
/// shipped strings only line up that way if the arithmetic is right.
fn zone_plus(name: &str) -> Option<u32> {
    name.strip_prefix("ZonePlus")?.parse().ok()
}

/// A widget's `idstring` resolved through the language's table, or `None` when it
/// carries no id and so has no caption to draw.
pub(super) fn caption(label: &Label, strings: &oag_ui::language::StringTable) -> Option<String> {
    label
        .idstring
        .as_deref()
        .map(|id| strings.get_or_id(id).to_string())
}

/// Whether the place, rather than the total time, owns the top-right anchor.
///
/// # Why the total time disappears when a place appears
///
/// **`Arcade_HUD.xml` authors `TotalTime` and `Position` at exactly the same
/// anchor**, inside the same `<Item OffsetX="445" OffsetY="5">`:
///
/// ```text
/// <Text name="TotalTime"><Values scale="1.0" font="HUD" align="right" vertalign="bottom" x="0" y="30" .../>
/// <Text name="Position"> <Values scale="1.0" font="HUD" align="right" vertalign="bottom" x="0" y="30" .../>
/// ```
///
/// Same `x`, same `y`, same font, same scale, same alignment on both axes - so the
/// place's digit lands on the last digit of the time and at most one of the two can
/// be on screen. The captions overlap too: `TotalTimeTxt` is right-aligned to 445
/// and `PositionTxt` to 460, which is inside a `HUDSmall` "TOTAL".
///
/// **Coincidence is this dialect's way of saying "at most one of these is live"** -
/// the precedent is the thirteen weapon icons, every one authored at `x=240 y=35`,
/// of which [`pickup_sprites`] draws the one being carried. Confidence **95**;
/// reproduce with
///
/// ```sh
/// # Doubled backslashes: `just` runs the recipe through a shell, which eats one layer.
/// just wad cat --expand <image>:PSP_GAME/USRDIR/Data.wad 'Data\\XML\\Arcade_HUD.xml'
/// ```
///
/// **And the place is the one that wins, measured on the original.** Confidence
/// **95**, no longer the 55 this comment carried for its first hours: a single
/// race was driven on the real game (PPSSPP under Xvfb, `psp-drive.py menu
/// --single-race`, `pulse-psp-usa`, 2026-08-11) and its top-right corner reads
///
/// ```text
/// pos
/// 8 / 8
/// ```
///
/// on the grid and again fifty seconds into the lap - the place, the separator and
/// the field size, and **no total time anywhere on the screen**. Which is also
/// what the layout implies: `TimeTrial`, `Zone` and `Elimination` carry the total
/// time and no place, so the clock is not homeless without this anchor and the
/// place would be. The recipe is on `docs/reverse-engineering/ppsspp-debugger.md`.
///
/// The choice is per *layout*, not per readout: `Elimination_HUD.xml` has a
/// `TotalTime` and no `Position`, so a race with a field still shows its clock
/// there - which is why this asks the layout and not only the readout.
///
/// **Checks the coincidence itself, not merely that both widgets exist.**
/// 2048's `Arcade_HUD.xml` authors `TotalTime` at `(145, 70)` and `Position`
/// at `(945, 30)` - nowhere near each other - and a Vita3K frame of the
/// running original (`11-load-8.png`, `docs/formats/2048-hud.md`) shows
/// `TOTAL 0.29.76` and `POS 8/8` **at once**. The presence-only version read
/// this as Pulse's own coincident-anchor rule and hid `TotalTime` on every
/// 2048 race with a field, which no frame shows. `PositionTxt`'s own
/// visibility is [`readout.place`] alone, not this function - see its arm in
/// [`text_for`].
pub(super) fn place_owns_the_anchor(layout: &Layout, readout: &Readout) -> bool {
    let Some(position) = layout.label("Position") else {
        return false;
    };
    let Some(total_time) = layout.label("TotalTime") else {
        return false;
    };
    /// A few authoring pixels of slack for float aggregation across nested
    /// `<Item>` offsets - the two anchors this function looks for are either
    /// authored identically (Pulse/HD) or nowhere near each other (2048), so
    /// this is not a tolerance doing real work, just insurance against
    /// `f32` rounding in the composed sum.
    const ANCHOR_EPSILON: f32 = 0.5;
    readout.place > 0
        && (position.x - total_time.x).abs() < ANCHOR_EPSILON
        && (position.y - total_time.y).abs() < ANCHOR_EPSILON
}

/// The widget that draws the speed's unit when the layout has one of its own.
///
/// **`SpeedBarText`'s ` kmh` is Pulse's, and it is authored rather than
/// invented**: all five of Pulse's layouts write the widget's placeholder as
/// `string="0 kmh"`, unit included, and nothing beside it says `km/h` again.
///
/// HD writes the same placeholder - the XML is inherited - and then authors
/// this second widget at `x` ten pixels to its right, `idstring="RC_KMH"`,
/// three-quarter scale, which the language table renders as `KM/H`. Taking the
/// placeholder at its word there puts `0 kmh KM/H` on screen, the unit twice,
/// which is not a thing a shipped layout does.
///
/// So the value keeps the unit only on a layout that does not spell it out
/// separately. **Confidence 75**: the sibling widget is on the disc and its
/// string is in the language table, but the frame this was checked against
/// crops the speed readout off the right edge, so the recovered text has not
/// been read back off the original. It is the same shape of argument
/// [`place_owns_the_anchor`] makes from two coincident anchors.
pub(super) const SPEED_UNIT_WIDGET: &str = "SpeedBarTextKMH";

/// Everything the HUD needs that does not change from frame to frame.
///
/// Bundled rather than passed as four arguments because the caller assembles it
/// once at race load and holds it for the whole race.
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    /// The parsed layout for this race's mode.
    pub layout: &'a Layout,
    /// The language's string table, for `idstring` captions.
    pub strings: &'a oag_ui::language::StringTable,
    /// Every texture the layout names, packed into one sheet and keyed by the
    /// reference the layout spells. See [`sprite_draw`], which is the only
    /// thing that reads it.
    pub sheet: &'a crate::sprite::Sheet,
    /// How this title's HUD sprites reach the screen. See [`oag_title::HudArt`].
    pub art: &'a oag_title::HudArt,
    /// Line height of the `HUD` font, **as actually loaded**.
    ///
    /// Not the 25 the XML role table implies: if the disc font is missing the
    /// caller falls back to the 5x7 built-in set, whose line height is 8, and
    /// vertical alignment has to follow what is really on screen or every
    /// bottom-aligned value drifts by the difference.
    pub hud_line_height: f32,
    /// Line height of the `HUDSmall` font, as actually loaded.
    pub small_line_height: f32,
    /// Line height of the `Default` font (`pulse_text.fnt`), as actually
    /// loaded. Only the per-craft rows use it - see [`super::kill_tags`].
    pub default_line_height: f32,
    /// What a widget with no `BorderColor` outlines its glyphs in.
    ///
    /// 57 of the 84 HUD-font widgets are in that position, and the original draws
    /// an outline on them regardless - see [`BORDER_CONSTANT`].
    pub default_border: [f32; 4],
}

/// One frame's worth of HUD, split by which pass draws it.
///
/// Four lists rather than one because a [`crate::render::Renderer`] binds
/// exactly one font atlas, so the three text fonts cannot share a pass. Draw them
/// in field order: sprites, then `HUD`, `HUDSmall` and `Default` text.
///
/// Splitting text across two passes means paint order is no longer strictly the
/// layout's document order *between* fonts. That is safe here because no label
/// overlaps another in any shipped layout - asserted by
/// `no_two_live_labels_overlap` rather than assumed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
    /// Bars, backgrounds and icons, in paint order.
    pub sprites: Vec<Draw>,
    /// Text in the `HUD` font: the values.
    pub hud_text: Vec<Draw>,
    /// Text in the `HUDSmall` font: the captions.
    pub small_text: Vec<Draw>,
    /// Text in the `Default` font: the per-craft rows.
    pub default_text: Vec<Draw>,
}

impl Frame {
    /// Whether there is nothing at all to draw.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sprites.is_empty()
            && self.hud_text.is_empty()
            && self.small_text.is_empty()
            && self.default_text.is_empty()
    }

    /// How many quads' worth of work this frame is, for a log line.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sprites.len() + self.hud_text.len() + self.small_text.len() + self.default_text.len()
    }
}

/// Turns a widget's anchored `y` into the top edge the renderer draws text at.
///
/// [`crate::render::Renderer::render`] positions a line by its top edge, and the
/// layout anchors 94 of its 127 text widgets by their bottom or middle instead.
/// Ignoring that puts a bottom-anchored value a whole line below where it belongs:
/// on the lap counter, the difference between sitting on its caption and sitting
/// under the screen edge.
pub(super) fn top_edge(label: &Label, line_height: f32) -> f32 {
    let height = line_height * label.scale;
    match label.vertalign {
        VertAlign::Top => label.y,
        VertAlign::Middle => label.y - height / 2.0,
        VertAlign::Bottom => label.y - height,
    }
}

/// Builds the frame's draw list from a layout and a readout.
///
/// Paint order within each list is the layout's own document order, which is
/// back-to-front - the same rule [`crate::render::Renderer`] applies to every
/// other draw list in the game.
///
/// # An always-on name draws one widget, not every widget with that name
///
/// **HD's `speedlap_hud.xml` authors `BestLapImage` twice** - at `x=70` and
/// `x=355`, same `y`, same size, same source rectangle - where the arcade and
/// time-trial layouts author it once, at `x=70`. The second copy sits in the
/// same fragment as that layout's `GhostParent`/`GhostText`/`GhostTime` group:
/// it is the row a loaded ghost's lap time goes in, and this build loads no
/// ghost.
///
/// Matching by name alone therefore drew both, which put an empty rounded bar
/// beside the `BEST` row. A frame of the running original (`just rpcs3-race`,
/// speed lap, 2026-08-24) has **one** bar there and nothing in the space the
/// second occupies, so the first is the one that is up unconditionally.
///
/// The rule is *first occurrence*, in the layout's own document order, and it
/// is a no-op everywhere else: `BestLapImage` in HD's default-skin speed lap is
/// the only name any composed layout across the three titles authors twice
/// among its own always-on set - the two retro skins' speed-lap layouts author
/// it once - asserted by
/// `an_always_on_name_is_authored_once_bar_the_one_that_is_not` in
/// `crates/game/tests/hd_hud_ground_truth.rs`.
///
/// **Confidence 70.** The frame is one state of one mode, and what the original
/// keys the ghost row on is unread; if the row turns out to be drawn and merely
/// too faint to see at that exposure, this rule is what to revisit.
#[must_use]
pub fn draw_list(cx: &Context<'_>, readout: &Readout) -> Frame {
    // The kill column is Pulse's alone - see [`oag_title::HudArt::kill_column`] -
    // and so are the message lines, [`oag_title::HudArt::message_slots`].
    let ungated;
    let readout = if (cx.art.kill_column
        || (readout.kill_tags.is_empty() && readout.kill_target == 0))
        && (cx.art.message_slots || readout.messages.iter().all(Option::is_none))
    {
        readout
    } else {
        ungated = Readout {
            kill_tags: if cx.art.kill_column {
                readout.kill_tags.clone()
            } else {
                Vec::new()
            },
            kill_target: if cx.art.kill_column {
                readout.kill_target
            } else {
                0
            },
            messages: if cx.art.message_slots {
                readout.messages.clone()
            } else {
                Default::default()
            },
            ..readout.clone()
        };
        &ungated
    };
    let mut frame = Frame::default();

    // An always-on name draws **once**, even where the layout authors it twice.
    // See `ONE_WIDGET_PER_NAME`.
    let mut drawn_once: Vec<&str> = Vec::new();
    for sprite in &cx.layout.sprites {
        if !cx.art.always_on.contains(&sprite.name.as_str())
            || drawn_once.contains(&sprite.name.as_str())
        {
            continue;
        }
        drawn_once.push(sprite.name.as_str());
        // `EnergyBg`'s critical-threshold tint, 2048 only - see
        // `dialect_2048::energy_bg_tint`. Tried after HD's own runtime
        // mechanism so a title with both never has one silently mask the
        // other; only one of the two ever answers `Some` for a given name.
        let tinted = super::runtime::tinted(cx, readout, sprite)
            .or_else(|| super::dialect_2048::energy_bg_tint(sprite, readout));
        let fill = super::runtime::shield_fill(cx, readout, sprite);
        let sprite = tinted.as_ref().unwrap_or(sprite);
        // `ShieldBar`'s absorb flash, painted *under* the bar - see
        // `shield_bar::tint`. `None` outside the absorb window.
        let mut flash = None;
        let drawn = match bar_fraction(&sprite.name, readout) {
            // A bar at zero is not drawn at all: a zero-width quad is a
            // degenerate triangle pair, and asking the rasteriser to do nothing
            // is worse than not asking.
            Some(fraction) if fraction <= 0.0 => None,
            Some(fraction) => {
                let mut cropped = crop_horizontally(sprite, fraction);
                // `ShieldBar` alone: confirmed against the digits this
                // function formats, `"%"` and not `"kmh"`. See
                // `shield_bar::tint` for the three rules it applies.
                if sprite.name == "ShieldBar" {
                    flash = super::shield_bar::tint(&mut cropped, readout, cx.sheet);
                }
                sprite_draw(&cropped, cx.sheet)
            }
            // `EnergyBar`/`EnergyBarDelay`, 2048 only - see
            // `dialect_2048::vertical_bar_fraction`. Both crop without
            // tinting: the frames show `EnergyBar` live as white rather than
            // its authored green-at-half-alpha, and nothing decompiled here
            // writes a colour onto either widget, so that gap stays open -
            // see `oag_2048::hud::ALWAYS_ON`'s doc comment.
            None => match super::dialect_2048::vertical_bar_fraction(&sprite.name, readout) {
                Some(fraction) if fraction <= 0.0 => None,
                Some(fraction) => sprite_draw(&crop_vertically(sprite, fraction), cx.sheet),
                None => sprite_draw(sprite, cx.sheet),
            },
        };
        frame.sprites.extend(flash);
        frame.sprites.extend(drawn);
        frame.sprites.extend(fill);
    }
    frame
        .sprites
        .extend(super::runtime::arc_sprites(cx, readout));

    // The lock-on reticle, over the bars and under the text - it is the only
    // HUD sprite anchored on the *world* rather than on the screen, so it moves
    // across everything else and must not be under it.
    if let Some(sight) = &readout.sight {
        frame
            .sprites
            .extend(super::sight_draw::sight_draws(cx, sight));
    }

    // 2048's state-gated fills, over their dim backgrounds - see
    // `dialect_2048::state_sprites`.
    for sprite in super::dialect_2048::state_sprites(cx.layout, cx.art, readout) {
        frame.sprites.extend(sprite_draw(&sprite, cx.sheet));
    }

    // The pickup, after the always-on sprites and before the text. Conditional
    // rather than allow-listed, because *which* icon is live changes with what
    // the craft is holding - see `pickup_sprites`.
    if let Some(weapon) = readout.pickup {
        for sprite in pickup_sprites(cx.layout, weapon, cx.art) {
            frame.sprites.extend(sprite_draw(&sprite, cx.sheet));
        }
        frame.sprites.extend(pickup_model_draws(cx, weapon));
    }

    // Zone's next-class bar, which is conditional twice over and so cannot be
    // allow-listed: there has to *be* a next class, and it has to have a row on
    // screen. See `zone_next_row`, which is also what moves it.
    let next_row = zone_next_row(cx.layout, readout);
    if let Some((bar, _)) = next_row
        && let Some(sprite) = cx.layout.sprite(NEXT_CLASS_WIDGETS[0])
    {
        // Its own art and its own size; only the anchor is reconstructed.
        let mut moved = sprite.clone();
        moved.rect[0] = bar[0];
        moved.rect[1] = bar[1];
        frame.sprites.extend(sprite_draw(&moved, cx.sheet));
    }

    frame.sprites.extend(lap_split_sprites(cx, readout));

    // Decided once for the frame rather than per widget: it is a fact about the
    // layout, the readout and the title together, and two widgets have to agree.
    let place_shown = place_owns_the_anchor(cx.layout, readout)
        || super::time_trial_pace::mode_hides_total_time(cx.art, readout);
    // Likewise a fact about the layout rather than about one widget.
    let speed_unit = cx.layout.label(SPEED_UNIT_WIDGET).is_none();
    // `speed_unit`'s own pattern: `Position` carries the whole `8/8` only
    // where the layout authors no separate `Position Outof` to put the field
    // size in - see `text_for`'s own doc comment.
    let position_combined = cx.layout.label("Position Outof").is_none();

    // Head2Head's own gap readout replaces the place widgets wholesale.
    let head_to_head = super::head_to_head::draws(cx, readout, &mut frame);

    for label in &cx.layout.labels {
        if !is_screen_positioned(&label.name)
            || (head_to_head && super::head_to_head::owns(&label.name))
        {
            continue;
        }
        let Some(text) = text_for(
            label,
            readout,
            cx.strings,
            place_shown,
            speed_unit,
            cx.art.zone_speed_classes,
            cx.art.shield_percent,
            position_combined,
        ) else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        // The kill column's rows are drawn at the size the row's own craft
        // gets: the player's larger. Everything downstream reads `label`.
        let scaled;
        let label = match super::kill_tags::scale(readout, &label.name) {
            Some(factor) => {
                scaled = Label {
                    scale: label.scale * factor,
                    ..label.clone()
                };
                &scaled
            }
            None => label,
        };
        let line_height = match label.font {
            Font::Small => cx.small_line_height,
            Font::Default => cx.default_line_height,
            Font::Hud => cx.hud_line_height,
        };
        // The next class's name is the one widget in any layout whose authored
        // position this build overrides, because the layout authors it as a
        // placeholder - `zone_next_row` carries the whole reasoning.
        let placed = next_row.filter(|_| label.name == NEXT_CLASS_WIDGETS[1]);
        let draw = Draw::Text {
            x: placed.map_or(label.x, |(_, text)| text[0]),
            // The row's `y` is a **top** edge - every `ZonePlus1`-`ZonePlus10`
            // is `vertalign="top"` - and this label rides that line directly,
            // so it takes it unconverted rather than through `top_edge`.
            y: placed.map_or_else(|| top_edge(label, line_height), |(_, text)| text[1]),
            scale: label.scale,
            color: super::messages::colour(readout, &label.name)
                .or_else(|| super::time_trial_pace::time_trial_colour(readout, &label.name))
                .or_else(|| super::runtime::colour(cx, readout, &label.name))
                .unwrap_or(label.color),
            // The layout's own `BorderColor`, which is what makes the HUD fonts'
            // baked outline visible as an outline rather than as more glyph. The
            // 57 widgets that name none still get one - the original draws it -
            // from the layout's `HudBGColour`. See `oag_ui::font::Atlas::luma`.
            border: Some(super::messages::fade_border(
                readout,
                &label.name,
                label.border.unwrap_or(cx.default_border),
            )),
            align: label.align,
            text,
            wrap_width: None,
        };
        match label.font {
            Font::Small => frame.small_text.push(draw),
            Font::Default => frame.default_text.push(draw),
            Font::Hud => frame.hud_text.push(draw),
        }
    }

    frame
}

/// One layout sprite as a draw, or `None` for a sprite whose texture the sheet
/// does not hold.
///
/// A widget carrying a `RotationTheta` comes back as
/// [`Draw::RotatedSprite`] rather than [`Draw::Sprite`]. Both spin about the
/// same point - the rectangle's centre - so a `Centred="true"` widget, which
/// [`super::Layout`] has already converted to a top-left corner, still turns
/// about the position the layout authored. See [`Sprite::rotation`].
pub fn sprite_draw(sprite: &Sprite, sheet: &crate::sprite::Sheet) -> Option<Draw> {
    let placed = sheet.get(&sprite.src)?;
    // The texture is packed into a shared sheet, so its own pixel coordinates
    // are offset by wherever the packer put it. The width and height are *not*
    // offset, and may be negative: 26 of HD's sprites author a negative
    // `TxtrWidth` to mirror a patch, and `ui.wgsl` walks the corner from
    // `uv.xy` by `uv.zw`, so a negative one runs backwards and mirrors. See
    // `crate::sprite::Sheet`.
    let uv = [
        placed.x as f32 + sprite.uv[0],
        placed.y as f32 + sprite.uv[1],
        sprite.uv[2],
        sprite.uv[3],
    ];
    if sprite.rotation == 0.0 {
        return Some(Draw::Sprite {
            rect: sprite.rect,
            uv,
            color: sprite.color,
        });
    }
    Some(Draw::RotatedSprite {
        rect: sprite.rect,
        uv,
        color: sprite.color,
        rotation: sprite.rotation,
    })
}
