//! The held pickup's icon: which sprite, on which backdrop, in which colour.
//!
//! Split out of [`super::draw`] under `scripts/check-file-size.py`'s 1,000-line
//! ratchet; nothing about it changed.

use super::draw::Context;
use super::{Draw, Layout, Sprite, argb_to_rgba, colour};

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
pub fn pickup_icon_name(weapon: oag_tables::weapons::Weapon) -> String {
    format!("{}Icon", weapon.as_type())
}

/// The sprites a held pickup adds to the frame, in paint order.
///
/// Empty when nothing is held. The backdrop takes the held weapon's own colour
/// when `art.pickup_colours` has measured one - see `oag_pulse::hud::
/// PICKUP_COLOURS` for what that measurement is and is not - and otherwise
/// falls back to a title whose `art.pickup_backdrop_colour` names a constant,
/// or is drawn as authored if neither answers. It is skipped entirely on a
/// title that already draws it as part of `art.always_on`, so HD's backdrop is
/// one quad whether or not a pickup is held rather than two stacked on the
/// same pixels.
pub(super) fn pickup_sprites(
    layout: &Layout,
    weapon: oag_tables::weapons::Weapon,
    art: &oag_title::HudArt,
) -> Vec<Sprite> {
    if let Some(uv_table) = art.pickup_icon_uv {
        return super::dialect_2048::pickup_sprites_uv_rewrite(layout, weapon, uv_table);
    }
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
            if sprite.name == PICKUP_BACKGROUND {
                // `pickup_colours` is positional - see its own doc comment -
                // indexed the same way `Weapon::ALL` declares its thirteen.
                if let Some(argb) = art
                    .pickup_colours
                    .and_then(|colours| colours[weapon as usize])
                {
                    sprite.color = argb_to_rgba(argb);
                } else if let Some(raw) = art
                    .pickup_backdrop_colour
                    .and_then(|key| layout.constants.get(key))
                {
                    // Only when this title asks for the substitution *and* the
                    // layout defines the constant it names. A source that does
                    // not gets the authored colour and, on Pulse, the
                    // unreadable picture - which is the honest failure: this
                    // build does not know what colour the backdrop is, and
                    // inventing one for a layout that never offered it would
                    // be a second guess on top of the first.
                    sprite.color = colour(&layout.constants, Some(raw));
                }
            }
            sprite
        })
        .collect()
}

/// [`pickup_sprites`]'s counterpart for Pure's dialect: the held pickup's
/// icon as a `<Mode3D><Model>` draw, plus its backdrop grid, rather than
/// `<Image>` sprites.
///
/// Empty whenever `cx.art.pickup_icon_models` is `None` (every title but
/// Pure, so far), whenever the held weapon's slot in it is `None` - Pure
/// authors no icon for `Cannon`, `LeachBeam`, `Repulser` or `Shuriken`, and a
/// Pure race can hand out the last of those (`oag_weapons::pickup::
/// IMPLEMENTED`), so this is a live path and not a dead branch - or whenever
/// the model failed to build or its art did not reach the sheet, which
/// [`crate::race::hud::vex_model_art`] reports and this draws nothing for
/// rather than guessing a placeholder size.
///
/// **Draws the backdrop grid first**, the same paint order [`pickup_sprites`]
/// puts the icon over `PICKUP_BACKGROUND` in, and for the same reason: the
/// icon sits on it.
pub(super) fn pickup_model_draws(
    cx: &Context<'_>,
    weapon: oag_tables::weapons::Weapon,
) -> Vec<Draw> {
    let Some(models) = cx.art.pickup_icon_models else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(name) = cx.art.pickup_icon_backdrop_model {
        out.extend(model_draw(cx, name, [1.0, 1.0, 1.0, 1.0]));
    }
    if let Some(name) = models[weapon as usize] {
        // The model's own authored `colour` - see `Model::colour`'s doc for
        // why this is not routed through `pickup_colours`' substitution at
        // all: there is nothing to substitute, the XML already carries the
        // final answer. White is the fallback for a name this table gives
        // that the layout does not actually author with a `colour=`, which
        // none of Pure's ten currently are.
        let tint = cx
            .layout
            .models
            .iter()
            .find(|model| model.name == name)
            .and_then(|model| model.colour)
            .unwrap_or([1.0, 1.0, 1.0, 1.0]);
        out.extend(model_draw(cx, name, tint));
    }
    out
}

/// One `<Mode3D><Model>` widget as a flat, tinted quad - the icon and grid
/// widgets' shared shape, neither of which rotates the way the sight brackets
/// do.
///
/// `None` when the widget is not in the layout, its model did not resolve
/// into the sheet, or the sheet holds no [`crate::sprite::Placed::quad_extent`]
/// for it - the last being why [`crate::race::hud::vex_model_art`] computes
/// one for every model it decodes rather than leaving it for the sight
/// brackets' hand-measured `SIGHT_SIZE` to answer for widgets it was never
/// measured against.
fn model_draw(cx: &Context<'_>, name: &str, color: [f32; 4]) -> Option<Draw> {
    let model = cx.layout.models.iter().find(|model| model.name == name)?;
    let placed = cx.sheet.get(&model.src)?;
    let [w, h] = placed.quad_extent?;
    let [x, y, _z] = model.position;
    Some(Draw::BlendedSprite {
        // Centred on its own authored position, the same convention the sight
        // brackets' quads use - see `bracket_draws`.
        rect: [x - w * 0.5, y - h * 0.5, w, h],
        uv: [
            placed.x as f32,
            placed.y as f32,
            placed.width as f32,
            placed.height as f32,
        ],
        color,
        // Neither the icon nor its grid turns - the sight brackets are the only
        // `<Mode3D>` widget that does.
        rotation: 0.0,
        // The model's own declared class. **Unmeasured here**: no Pure disc is
        // present in this worktree, so what Pure's eleven icon models declare
        // is whatever their own `pass_mask` says, taken as read. A model whose
        // batches are opaque carries `None` and draws exactly as it did before
        // this variant existed. See `crate::sprite::Placed::blend`.
        blend: placed.blend,
    })
}
