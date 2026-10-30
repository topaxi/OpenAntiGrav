//! The held pickup's icon: which sprite, on which backdrop, in which colour.
//!
//! Split out of [`super::draw`] under `scripts/check-file-size.py`'s 1,000-line
//! ratchet; nothing about it changed.

use super::draw::Context;
use super::pickup_fade::{Look, scale_word};
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

/// What the held pickup draws, once a mod's own pickup (`Weapon::Extension`) is resolved
/// to its declared look. A weapon of the title's draws its own icon and its own backdrop row.
pub(super) struct Shown<'a> {
    /// The weapon whose icon sprite, model or UV row is drawn.
    pub icon: Option<oag_tables::weapons::Weapon>,
    /// A mod's own icon, by its key in the sheet.
    pub mod_icon: Option<&'a str>,
    /// Whether the backdrop is drawn at all.
    pub backdrop: bool,
}

/// `held` resolved: the title's weapons as they are, a mod's pickup through the look the mod
/// declared. A mod pickup with no look draws nothing: no title ships an icon for it.
pub(super) fn shown<'a>(
    held: oag_tables::weapons::Weapon,
    ext: &'a super::ext_draws::Outputs,
) -> Shown<'a> {
    use oag_tables::pickup_look::Icon;
    use oag_tables::weapons::Weapon;
    if held != Weapon::Extension {
        return Shown {
            icon: Some(held),
            mod_icon: None,
            backdrop: true,
        };
    }
    match ext.pickup_look.as_ref() {
        Some(look) => Shown {
            icon: match &look.icon {
                Icon::Title(weapon) => Some(*weapon),
                Icon::Mod(_) => None,
            },
            mod_icon: match &look.icon {
                Icon::Mod(src) => Some(src.as_str()),
                Icon::Title(_) => None,
            },
            backdrop: true,
        },
        None => Shown {
            icon: None,
            mod_icon: None,
            backdrop: false,
        },
    }
}

/// The backdrop's `0xAARRGGBB` word for `held`: the title's own row for the weapon, or for a
/// mod pickup the row its look names or the colour it carries. `None` on a title with no
/// backdrop colours, or a weapon it has no row for.
pub(super) fn backdrop_word(
    art: &oag_title::HudArt,
    held: oag_tables::weapons::Weapon,
    ext: &super::ext_draws::Outputs,
) -> Option<u32> {
    use oag_tables::pickup_look::Colour;
    use oag_tables::weapons::Weapon;
    let table = art.pickup_colours?;
    let row = if held == Weapon::Extension {
        match ext.pickup_look.as_ref()?.colour {
            Colour::Title(weapon) => weapon,
            Colour::Rgba([r, g, b, a]) => {
                return Some(u32::from_be_bytes([a, r, g, b]));
            }
        }
    } else {
        held
    };
    table.get(row as usize).copied().flatten()
}

/// The fade's look this frame on a title that plays one ([`oag_title::HudArt::pickup_fade`]),
/// or `None` to draw the held weapon as authored.
pub(super) fn look(art: &oag_title::HudArt, readout: &super::Readout) -> Option<Look> {
    art.pickup_fade?;
    art.pickup_colours?;
    readout.pickup_fade.look(readout.pickup)
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
    held: oag_tables::weapons::Weapon,
    ext: &super::ext_draws::Outputs,
    art: &oag_title::HudArt,
    announcing: bool,
    look: Option<Look>,
) -> Vec<Sprite> {
    let shown = shown(held, ext);
    if let Some(uv_table) = art.pickup_icon_uv {
        return super::dialect_2048::pickup_sprites_uv_rewrite(
            layout,
            shown.icon,
            shown.backdrop,
            uv_table,
            announcing,
        );
    }
    let icon = shown.icon.map(pickup_icon_name);
    let backdrop = (shown.backdrop && !art.always_on.contains(&PICKUP_BACKGROUND))
        .then_some(PICKUP_BACKGROUND);
    // Backdrop first: the icon sits on it, and paint order here is the layout's
    // own back-to-front convention.
    backdrop
        .into_iter()
        .chain(icon.as_deref())
        .filter_map(|name| layout.sprite(name))
        .map(|sprite| {
            let mut sprite = sprite.clone();
            if sprite.name == PICKUP_BACKGROUND {
                // `pickup_colours` is positional - see its own doc comment -
                // indexed the same way `Weapon::ALL` declares its thirteen.
                if let Some(argb) = backdrop_word(art, held, ext) {
                    sprite.color = argb_to_rgba(match look {
                        Some(look) => scale_word(argb, look.backdrop),
                        None => argb,
                    });
                    if let Some(look) = look
                        && art.pickup_fade.is_some_and(|fade| fade.size_pulse)
                    {
                        pulse_size(&mut sprite, look.size);
                    }
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
            if let Some(look) = look
                && sprite.name != PICKUP_BACKGROUND
            {
                for channel in &mut sprite.color {
                    *channel = (*channel * look.icon).min(1.0);
                }
            }
            sprite
        })
        .collect()
}

/// The backdrop folded to `width / f` by `height * f` about its own centre
/// (`Hud_UpdatePickupWidgets`' `+0x9c`/`+0xa0` pair). The anchor is `Centred="true"`, so the
/// centre is the fixed point; the original's frames show the shape hanging from the top of the
/// box, which is a **chosen** reading of the anchor, not a measured one.
fn pulse_size(sprite: &mut Sprite, f: f32) {
    let [x, y, w, h] = sprite.rect;
    let (cx, cy) = (x + w * 0.5, y + h * 0.5);
    let (w, h) = (w / f, h * f);
    sprite.rect = [cx - w * 0.5, cy - h * 0.5, w, h];
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
    held: oag_tables::weapons::Weapon,
    ext: &super::ext_draws::Outputs,
) -> Vec<Draw> {
    let shown = shown(held, ext);
    let Some(models) = cx.art.pickup_icon_models else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if shown.backdrop
        && let Some(name) = cx.art.pickup_icon_backdrop_model
    {
        out.extend(model_draw(cx, name, [1.0, 1.0, 1.0, 1.0]));
    }
    let icon = shown
        .icon
        .and_then(|weapon| models.get(weapon as usize).copied().flatten());
    if let Some(name) = icon {
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

/// A mod's own pickup icon, centred in the place the title draws a pickup's icon and fitted
/// inside it with its own aspect, faded as the title's own icon is. **Chosen, not measured**:
/// the place is the layout's `TurboIcon` sprite (2048: inside `PickupBgFrame`), since every
/// title's weapon icons share one slot. A title that draws its icons as models (Pure) draws
/// nothing for a mod's own PNG.
pub(super) fn mod_icon_draw(cx: &Context<'_>, src: &str, look: Option<Look>) -> Option<Draw> {
    let placed = cx.sheet.get(src)?;
    let slot = if cx.art.pickup_icon_uv.is_some() {
        cx.layout
            .sprite(super::dialect_2048::PICKUP_BG_FRAME_2048)?
    } else {
        cx.layout
            .sprite(&pickup_icon_name(oag_tables::weapons::Weapon::Turbo))?
    }
    .rect;
    let fit = (slot[2] / placed.width as f32).min(slot[3] / placed.height as f32);
    let (w, h) = (placed.width as f32 * fit, placed.height as f32 * fit);
    let fade = look.map_or(1.0, |look| look.icon);
    Some(Draw::Sprite {
        rect: [
            slot[0] + (slot[2] - w) * 0.5,
            slot[1] + (slot[3] - h) * 0.5,
            w,
            h,
        ],
        uv: [
            placed.x as f32,
            placed.y as f32,
            placed.width as f32,
            placed.height as f32,
        ],
        color: [fade.min(1.0); 4],
    })
}
