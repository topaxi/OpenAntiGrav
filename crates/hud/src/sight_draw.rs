//! The lock-on reticle, as sprites: which widget set is up, and where each
//! piece of it goes.
//!
//! Split out of [`super::draw`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change, along the
//! seam [`super::lap_splits`] already used. The reticle is the one HUD element
//! with two whole dialects behind it - four rotated `<Mode3D>` models on the
//! PSP titles against concentric `<Image>` sprites on HD - so it is the natural
//! second thing to lift out.
//!
//! The law it draws is `oag_race::sight`; this is only the pixels. See
//! `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`.

use super::Draw;
use super::draw::Context;

/// The lock-on reticle, as sprites, in whichever dialect this title authors.
///
/// **Two dialects over one law.** Both PSP titles instance a corner-bracket
/// `<Mode3D>` model four times at the corners of a box and rotate each a quarter
/// turn on; Wipeout HD draws concentric `<Image>` sprites at one centre and
/// rotates nothing. Where the centre goes and when the lock is taken is the same
/// recovered law either way - see `oag_race::sight` and
/// `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`. Which shape a title
/// authors is [`oag_title::hud::Sights`].
///
/// Empty whenever the reticle has nothing to show, when the layout authors none
/// of the named widgets, or when their art did not decode - the last being this
/// project's rule for an asset it cannot play: draw nothing, and the loader
/// report already said why.
pub(super) fn sight_draws(cx: &Context<'_>, sight: &oag_race::sight::Sight) -> Vec<Draw> {
    // **Visibility alone.** The seeking blink is a *tint* - the original writes
    // a colour on both of its phases and drops the draw on neither - so gating
    // on it here would strobe the reticle off every 0.1 s. See
    // [`oag_race::sight::Sight::tint`].
    if !sight.visible() {
        return Vec::new();
    }
    let alpha = sight.alpha();
    if alpha <= 0.0 {
        return Vec::new();
    }

    match cx.art.sights {
        oag_title::hud::Sights::Unread => Vec::new(),
        oag_title::hud::Sights::Brackets {
            brackets,
            inner,
            leach,
        } => {
            // **Off the reticle's own held weapon, never off its target.** The
            // reticle belongs to what is in the player's pickup slot; a craft
            // ahead is the same craft whichever weapon found it. See
            // `oag_race::sight::Held`.
            //
            // The LeachBeam's set is four arrowheads and **no inner** - only the
            // Missile's nine-widget bind gets a closed box. A title that authors
            // no LeachBeam sights draws nothing for one, which is this project's
            // answer for art it does not have rather than lending it the
            // Missile's.
            let (names, inner) = match sight.held() {
                oag_race::sight::Held::Missile => (Some(*brackets), Some(*inner)),
                oag_race::sight::Held::LeachBeam => (*leach, None),
            };
            match names {
                Some(names) => bracket_draws(cx, sight, names, inner, alpha),
                None => Vec::new(),
            }
        }
        // **`MissileSight*` is the Missile's own set, and the LeachBeam wears
        // its own** - `leach`, on the same terms `Brackets::leach` already
        // uses. `None` there is Pure's and Pulse-shaped titles' answer for a
        // dialect that authors no such set; HD and 2048 both author one (see
        // their own `ART::sights` doc comments) and reach this arm for real:
        // HD's weapon table *does* author a `<Weapon type="LeachBeam">` with a
        // lock window - pinned by `only_pulse_and_hd_author_a_leachbeam_block`
        // in `crates/game/tests/lock_sight_ground_truth.rs` - so
        // `oag_race::sight::Held::LeachBeam` is reachable on this title and
        // would otherwise put the wrong weapon's reticle on screen.
        //
        // **The LeachBeam's own three outer rings reveal one at a time**, per
        // `docs/ghidra/functions/ps3-hdfury-eu/hud-sight.md`'s
        // `Hud_UpdateLeachBeamSight` reading - see [`leach_reveal_draws`]. There
        // is no `locked` counterpart to add either way: neither title's layout
        // authors a `LeachBeamSight*LockedOn*` widget.
        oag_title::hud::Sights::Concentric {
            leach,
            seeking: _,
            locked: _,
        } if sight.held() == oag_race::sight::Held::LeachBeam => match leach {
            Some(names) => leach_reveal_draws(cx, sight, *names, alpha),
            None => Vec::new(),
        },
        oag_title::hud::Sights::Concentric {
            seeking, locked, ..
        } => {
            let names = seeking
                .iter()
                .chain(locked.iter().take_while(|_| sight.locked()));
            concentric_draws(cx, sight, names, alpha)
        }
    }
}

/// The PSP dialect: four rotated brackets around a box, plus an optional inner.
///
/// Each is drawn at the size the model's own quad is - `8.0` units, and the
/// `<Mode3D mode="orthographic">` block those widgets live in spans the 480x272
/// screen one unit to a pixel, so eight units is eight pixels. The *box* they
/// sit at the corners of grows and shrinks; the brackets themselves do not, and
/// the original writes only a position and a rotation to each.
///
/// `inner` is `None` for the LeachBeam, whose four arrowheads have no closed box
/// at their centre - `HudSight_Bind` (`0x0881b604`) binds one inner across the
/// whole nine and it belongs to the Missile.
fn bracket_draws(
    cx: &Context<'_>,
    sight: &oag_race::sight::Sight,
    brackets: [&'static str; 4],
    inner: Option<&'static str>,
    alpha: f32,
) -> Vec<Draw> {
    let [r, g, b] = sight.tint();
    let colour = [r, g, b, alpha];
    let mut out = Vec::new();
    let mut place = |name: &str, piece: oag_race::sight::Piece| {
        let Some(placed) = model_art(cx, name) else {
            return;
        };
        out.push(Draw::BlendedSprite {
            // The model is a quad centred on its own origin, so the piece's
            // centre is the middle of the rectangle rather than its corner.
            rect: [
                piece.centre[0] - SIGHT_SIZE * 0.5,
                piece.centre[1] - SIGHT_SIZE * 0.5,
                SIGHT_SIZE,
                SIGHT_SIZE,
            ],
            uv: [
                placed.x as f32,
                placed.y as f32,
                placed.width as f32,
                placed.height as f32,
            ],
            color: colour,
            rotation: piece.rotation,
            // The model's own declared class, never a choice made here. All
            // three sight models declare `Additive`; see
            // `crate::sprite::Placed::blend`.
            blend: placed.blend,
        });
    };

    for (name, piece) in brackets.iter().zip(sight.brackets()) {
        place(name, piece);
    }
    if let Some(inner) = inner {
        place(inner, sight.inner());
    }
    out
}

/// The HD dialect: concentric sprites at the reticle's centre.
///
/// **The layout carries the geometry.** Each widget's authored rectangle is a
/// placeholder whose *size* is real - 128, 108, 80 and 64 pixels square - and
/// whose position the runtime overwrites, exactly as the PSP models' is. So this
/// keeps the authored size and its authored colour, and moves only the centre.
///
/// Nothing rotates: HD authors rings rather than corners, and there is no fourth
/// copy of one asset for a rotation to differentiate.
fn concentric_draws<'a>(
    cx: &Context<'_>,
    sight: &oag_race::sight::Sight,
    names: impl Iterator<Item = &'a &'static str>,
    alpha: f32,
) -> Vec<Draw> {
    let brightness = sight.brightness();
    let [cx_px, cy_px] = sight.centre();
    let mut out = Vec::new();
    for name in names {
        let Some(sprite) = cx.layout.sprites.iter().find(|s| s.name == **name) else {
            continue;
        };
        let Some(placed) = cx.sheet.get(&sprite.src) else {
            continue;
        };
        let (w, h) = (sprite.rect[2], sprite.rect[3]);
        out.push(Draw::Sprite {
            rect: [cx_px - w * 0.5, cy_px - h * 0.5, w, h],
            uv: [
                placed.x as f32 + sprite.uv[0],
                placed.y as f32 + sprite.uv[1],
                sprite.uv[2],
                sprite.uv[3],
            ],
            // The layout's own colour, dimmed by the reticle's fade and its
            // blink. HD authors these individually - a red outer, a green inner
            // - so overwriting them with white would throw away real data.
            color: [
                sprite.color[0] * brightness,
                sprite.color[1] * brightness,
                sprite.color[2] * brightness,
                sprite.color[3] * alpha,
            ],
        });
    }
    out
}

/// HD/2048's own LeachBeam reticle: the backdrop plus one of its three outer
/// rings, revealed in turn as the hold progresses rather than all three
/// together.
///
/// **Recovered mechanism, ported at reduced confidence.**
/// `Hud_UpdateLeachBeamSight` (`docs/ghidra/functions/ps3-hdfury-eu/hud-sight.md`)
/// shows Outer, Middle and Inner each own one exclusive quarter of the hold
/// window - `names` is `[BG, Outer, Middle, Inner]`, the order
/// `oag_title::hud::Sights::Concentric::leach` already carries. HD's own
/// quarter marks are absolute seconds into *its* `0.5` s hold; this engine
/// scales the same quarters onto [`oag_race::sight::Sight::hold_progress`]
/// instead of adopting HD's own hold time - see that constant's own doc for
/// why. Past three-quarters of the hold, or once the lock completes, only the
/// backdrop is shown: the original's own table stops there, and what (if
/// anything) replaces the rings on a full lock is not read, so this draws
/// nothing invented for that span rather than guessing.
fn leach_reveal_draws(
    cx: &Context<'_>,
    sight: &oag_race::sight::Sight,
    names: [&'static str; 4],
    alpha: f32,
) -> Vec<Draw> {
    let progress = sight.hold_progress();
    let ring = if progress <= 0.25 {
        Some(names[1])
    } else if progress <= 0.5 {
        Some(names[2])
    } else if progress <= 0.75 {
        Some(names[3])
    } else {
        None
    };
    let shown = [Some(names[0]), ring];
    concentric_draws(cx, sight, shown.iter().flatten(), alpha)
}

/// Where a `<Mode3D><Model>` widget's art sits in the sheet, by widget name.
fn model_art(cx: &Context<'_>, name: &str) -> Option<crate::sprite::Placed> {
    cx.layout
        .models
        .iter()
        .find(|model| model.name == name)
        .and_then(|model| cx.sheet.get(&model.src))
}

/// How big one piece of the reticle is drawn, in screen pixels.
///
/// **Pulse's** model's own quad: its `missile_sight_outer.vex` runs
/// `-3.9989 .. 3.9989` on both axes, and its `<Mode3D>` block is orthographic
/// over the 480x272 screen at one unit to the pixel. Rounded to the eight units
/// the exporter plainly meant, rather than carrying the quantisation of a
/// 16-bit position through to a screen rectangle.
///
/// **It is not every title's number, and this constant flattens that.**
/// Wipeout Pure authors the same widgets at twelve - its
/// `missile_sight_inner.vex` measures `[11.999471, 11.999471]` where Pulse's
/// measures `[7.9978027, 7.9978027]`, both read off the vertices into
/// [`crate::sprite::Placed::quad_extent`] at load. So Pure's reticle draws two
/// thirds the size the disc authors. Deliberately left rather than swapped
/// blind: `model_draw` already takes the per-model extent and this is the one
/// draw site that does not, but which of the two the *original* uses for a
/// bracket has not been read, and a capture on Pure is what settles it. See
/// `docs/ui/hud.md`.
const SIGHT_SIZE: f32 = 8.0;
