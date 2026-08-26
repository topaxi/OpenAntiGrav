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
    Draw, Font, Label, Layout, Precision, Readout, Sprite, VertAlign, colour, format_lap_time,
    is_screen_positioned,
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
/// arguments: whether *this layout* is drawing a place at all, which decides who
/// owns the top-right anchor. See [`place_owns_the_anchor`]. `speed_unit` is the
/// second, and [`SPEED_UNIT_WIDGET`] is why.
pub(super) fn text_for(
    label: &Label,
    readout: &Readout,
    strings: &crate::language::StringTable,
    place_shown: bool,
    speed_unit: bool,
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
        // A **percentage**, not the raw pool: the reference frame reads `100%` on
        // an undamaged craft, and `<Misc shield/>` is a few hundred units.
        "ShieldBarText" => Some(format!("{:.0}%", readout.shield_fraction() * 100.0)),
        "Lap" => (readout.lap > 0).then(|| readout.lap.to_string()),
        "Lap Outof" => (readout.laps > 0).then(|| readout.laps.to_string()),
        "Position" => (readout.place > 0).then(|| readout.place.to_string()),
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
        // The total time and its caption yield the top-right anchor to the place.
        // See [`place_owns_the_anchor`].
        "TotalTime" => {
            (!place_shown).then(|| format_lap_time(readout.race_ticks, Precision::Tenths))
        }
        "TotalTimeTxt" => (!place_shown).then(|| caption(label, strings)).flatten(),
        // The one caption gated on its own value, because the widget beside it is a
        // *different group's* caption rather than blank space: a `POS` with nothing
        // under it, three pixels from a `TOTAL` with a time under it, reads as a
        // rendering fault. The reachable case is a single race on a track with no
        // authored `Start Position`, which grids one craft and so has no place.
        "PositionTxt" => place_shown.then(|| caption(label, strings)).flatten(),
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

        // Separators, authored as literals. Drawn only when both sides have a
        // value: a lone `/` between two blanks reads as a rendering fault, and
        // with lap counting unrecovered that is the common case rather than a
        // corner one.
        "LapOf" => (readout.lap > 0 && readout.laps > 0).then(|| literal("/")),
        "PositionOf" => (readout.place > 0 && readout.ships > 0).then(|| literal("/")),

        // Conditional.
        "WrongWay" => readout.wrong_way.then(|| {
            label.idstring.as_deref().map_or_else(
                || "WRONG WAY".to_string(),
                |id| strings.get_or_id(id).to_string(),
            )
        }),

        // Static labels: anything with an `idstring` that this function has not
        // already claimed. These are the `IG_HUD_*` captions beside the values.
        _ => caption(label, strings),
    }
}

/// A widget's `idstring` resolved through the language's table, or `None` when it
/// carries no id and so has no caption to draw.
pub(super) fn caption(label: &Label, strings: &crate::language::StringTable) -> Option<String> {
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
pub(super) fn place_owns_the_anchor(layout: &Layout, readout: &Readout) -> bool {
    readout.place > 0 && layout.label("Position").is_some()
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

/// The backdrop the held pickup's icon sits on.
///
/// Authored `Centred="true"` at `x=240` - the middle of the PSP's 480.
pub(super) const PICKUP_BACKGROUND: &str = "PickupBackground";

/// The layout's sprite name for one weapon's icon.
///
/// **A name rule, not an id table, and that is a finding rather than a
/// convenience.** `docs/ui/hud.md` recorded these as "14 `*Icon` widgets" whose
/// "icon ids are numeric", with no id-to-weapon mapping known. Read off the
/// disc, `Arcade_HUD.xml` authors **thirteen** and names each after its weapon's
/// own `type` string - `TurboIcon`, `ShieldIcon`, `RocketIcon` and so on - which
/// is exactly `weapons::Weapon::ALL`. So the lookup needs nothing recovered:
/// the layout and the weapon table agree on the spelling, misspellings
/// (`LeachBeam`, `Repulser`) included.
///
/// The numeric ids are still real - `0x0883b3b8` forces one - and are simply not
/// needed to draw the right icon.
#[must_use]
pub fn pickup_icon_name(weapon: oag_formats::weapons::Weapon) -> String {
    format!("{}Icon", weapon.as_type())
}

/// The sprites a held pickup adds to the frame, in paint order.
///
/// Empty when nothing is held. The backdrop is retinted on a title whose
/// `art.pickup_backdrop_colour` names a constant, and drawn as authored
/// otherwise; see [`PICKUP_BACKDROP_COLOUR`] for the measurement behind Pulse's
/// answer. It is skipped entirely on a title that already draws it as part of
/// `art.always_on`, so HD's backdrop is one quad whether or not a pickup is
/// held rather than two stacked on the same pixels.
pub(super) fn pickup_sprites(
    layout: &Layout,
    weapon: oag_formats::weapons::Weapon,
    art: &oag_title::HudArt,
) -> Vec<Sprite> {
    let icon = pickup_icon_name(weapon);
    let backdrop = (!art.always_on.contains(&PICKUP_BACKGROUND)).then_some(PICKUP_BACKGROUND);
    // Backdrop first: the icon sits on it, and paint order here is the layout's
    // own back-to-front convention.
    backdrop
        .into_iter()
        .chain([icon.as_str()])
        .filter_map(|name| layout.sprite(name))
        .map(|sprite| {
            let mut sprite = sprite.clone();
            // Only when this title asks for the substitution *and* the layout
            // defines the constant it names. A source that does not gets the
            // authored colour and, on Pulse, the unreadable picture - which is
            // the honest failure: this build does not know what colour the
            // backdrop is, and inventing one for a layout that never offered it
            // would be a second guess on top of the first.
            if sprite.name == PICKUP_BACKGROUND
                && let Some(raw) = art
                    .pickup_backdrop_colour
                    .and_then(|key| layout.constants.get(key))
            {
                sprite.color = colour(&layout.constants, Some(raw));
            }
            sprite
        })
        .collect()
}

/// Everything the HUD needs that does not change from frame to frame.
///
/// Bundled rather than passed as four arguments because the caller assembles it
/// once at race load and holds it for the whole race.
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    /// The parsed layout for this race's mode.
    pub layout: &'a Layout,
    /// The language's string table, for `idstring` captions.
    pub strings: &'a crate::language::StringTable,
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
    /// What a widget with no `BorderColor` outlines its glyphs in.
    ///
    /// 57 of the 84 HUD-font widgets are in that position, and the original draws
    /// an outline on them regardless - see [`BORDER_CONSTANT`].
    pub default_border: [f32; 4],
}

/// One frame's worth of HUD, split by which pass draws it.
///
/// Three lists rather than one because a [`crate::render::Renderer`] binds
/// exactly one font atlas, so the two text fonts cannot share a pass. Draw them
/// in field order: sprites, then `HUD` text, then `HUDSmall` text.
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
}

impl Frame {
    /// Whether there is nothing at all to draw.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sprites.is_empty() && self.hud_text.is_empty() && self.small_text.is_empty()
    }

    /// How many quads' worth of work this frame is, for a log line.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sprites.len() + self.hud_text.len() + self.small_text.len()
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
        let drawn = match bar_fraction(&sprite.name, readout) {
            // A bar at zero is not drawn at all: a zero-width quad is a
            // degenerate triangle pair, and asking the rasteriser to do nothing
            // is worse than not asking.
            Some(fraction) if fraction <= 0.0 => None,
            Some(fraction) => sprite_draw(&crop_horizontally(sprite, fraction), cx.sheet),
            None => sprite_draw(sprite, cx.sheet),
        };
        frame.sprites.extend(drawn);
    }

    // The lock-on reticle, over the bars and under the text - it is the only
    // HUD sprite anchored on the *world* rather than on the screen, so it moves
    // across everything else and must not be under it.
    if let Some(sight) = &readout.sight {
        frame.sprites.extend(sight_draws(cx, sight));
    }

    // The pickup, after the always-on sprites and before the text. Conditional
    // rather than allow-listed, because *which* icon is live changes with what
    // the craft is holding - see `pickup_sprites`.
    if let Some(weapon) = readout.pickup {
        for sprite in pickup_sprites(cx.layout, weapon, cx.art) {
            frame.sprites.extend(sprite_draw(&sprite, cx.sheet));
        }
    }

    // Decided once for the frame rather than per widget: it is a fact about the
    // layout and the readout together, and two widgets have to agree on it.
    let place_shown = place_owns_the_anchor(cx.layout, readout);
    // Likewise a fact about the layout rather than about one widget.
    let speed_unit = cx.layout.label(SPEED_UNIT_WIDGET).is_none();

    for label in &cx.layout.labels {
        if !is_screen_positioned(&label.name) {
            continue;
        }
        let Some(text) = text_for(label, readout, cx.strings, place_shown, speed_unit) else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        let small = label.font == Font::Small;
        let line_height = if small {
            cx.small_line_height
        } else {
            cx.hud_line_height
        };
        let draw = Draw::Text {
            x: label.x,
            y: top_edge(label, line_height),
            scale: label.scale,
            color: label.color,
            // The layout's own `BorderColor`, which is what makes the HUD fonts'
            // baked outline visible as an outline rather than as more glyph. The
            // 57 widgets that name none still get one - the original draws it -
            // from the layout's `HudBGColour`. See `crate::font::Atlas::luma`.
            border: Some(label.border.unwrap_or(cx.default_border)),
            align: label.align,
            text,
            wrap_width: None,
        };
        if small {
            frame.small_text.push(draw);
        } else {
            frame.hud_text.push(draw);
        }
    }

    frame
}

/// Draws a sprite widget at its authored geometry, out of **its own** texture.
///
/// It exists so that no caller has to remember that `rect` is the destination
/// and `uv` the source - getting those the wrong way round produces a picture
/// rather than an error - and, since 2026-08-25, so that no caller has to
/// remember that a layout may name more than one texture.
///
/// `None` when `sheet` does not hold the texture this sprite names, which is
/// the honest outcome and not a guess: the alternative is offsetting by `(0, 0)`
/// and sampling whichever texture the packer happened to put first, at
/// coordinates meant for a different one. That is what an HD race did until the
/// sheet learnt to hold six.
#[must_use]
/// The lock-on reticle's five pieces, as sprites.
///
/// **Four instances of one model plus one of another**, which is what the
/// original's nine widgets over three models come to for the Missile. Each is
/// drawn at the size the model's own quad is - `8.0` units, and the
/// `<Mode3D mode="orthographic">` block those widgets live in spans the 480x272
/// screen one unit to a pixel, so eight units is eight pixels. The *box* they
/// sit at the corners of grows and shrinks; the brackets themselves do not, and
/// the original writes only a position and a rotation to each. See
/// `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`.
///
/// Empty whenever the reticle has nothing to show, when the layout authors no
/// sight widgets, or when their model did not decode - the last being this
/// project's rule for an asset it cannot play: draw nothing, and the loader
/// report already said why.
fn sight_draws(cx: &Context<'_>, sight: &crate::race::sight::Sight) -> Vec<Draw> {
    use crate::race::sight;

    // **Visibility alone.** The seeking blink is a *tint* - the original writes
    // a colour on both of its phases and drops the draw on neither - so gating
    // on it here would strobe the reticle off every 0.1 s. See
    // [`crate::race::sight::Sight::tint`].
    if !sight.visible() {
        return Vec::new();
    }
    let alpha = sight.alpha();
    if alpha <= 0.0 {
        return Vec::new();
    }
    let tint = sight.tint();
    let colour = [tint, tint, tint, alpha];

    let model_for = |name: &str| {
        cx.layout
            .models
            .iter()
            .find(|model| model.name == name)
            .and_then(|model| cx.sheet.get(&model.src))
    };

    let mut out = Vec::new();
    let mut place = |placed: crate::sprite::Placed, piece: sight::Piece| {
        let (w, h) = (placed.width as f32, placed.height as f32);
        // The model is a quad centred on its own origin, so the piece's centre
        // is the middle of the rectangle rather than its corner.
        out.push(Draw::RotatedSprite {
            rect: [
                piece.centre[0] - SIGHT_SIZE * 0.5,
                piece.centre[1] - SIGHT_SIZE * 0.5,
                SIGHT_SIZE,
                SIGHT_SIZE,
            ],
            uv: [placed.x as f32, placed.y as f32, w, h],
            color: colour,
            rotation: piece.rotation,
        });
    };

    for (name, piece) in sight::MISSILE_BRACKETS.iter().zip(sight.brackets()) {
        if let Some(placed) = model_for(name) {
            place(placed, piece);
        }
    }
    if let Some(placed) = model_for(sight::MISSILE_INNER) {
        place(placed, sight.inner());
    }
    out
}

/// How big one piece of the reticle is drawn, in screen pixels.
///
/// The model's own quad: `missile_sight_outer.vex` runs `-3.9989 .. 3.9989` on
/// both axes, and its `<Mode3D>` block is orthographic over the 480x272 screen
/// at one unit to the pixel. Rounded to the eight units the exporter plainly
/// meant, rather than carrying the quantisation of a 16-bit position through to
/// a screen rectangle.
const SIGHT_SIZE: f32 = 8.0;

pub fn sprite_draw(sprite: &Sprite, sheet: &crate::sprite::Sheet) -> Option<Draw> {
    let placed = sheet.get(&sprite.src)?;
    Some(Draw::Sprite {
        rect: sprite.rect,
        // The texture is packed into a shared sheet, so its own pixel
        // coordinates are offset by wherever the packer put it. The width and
        // height are *not* offset, and may be negative: 26 of HD's sprites
        // author a negative `TxtrWidth` to mirror a patch, and `ui.wgsl` walks
        // the corner from `uv.xy` by `uv.zw`, so a negative one runs backwards
        // and mirrors. See `crate::sprite::Sheet`.
        uv: [
            placed.x as f32 + sprite.uv[0],
            placed.y as f32 + sprite.uv[1],
            sprite.uv[2],
            sprite.uv[3],
        ],
        color: sprite.color,
    })
}
