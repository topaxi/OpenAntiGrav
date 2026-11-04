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
    if !matches!(held, Weapon::Extension(_)) {
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
    let row = if matches!(held, Weapon::Extension(_)) {
        match ext.pickup_look.as_ref()?.colour {
            Colour::Title(weapon) => weapon,
            Colour::Rgba([r, g, b, a]) => {
                return Some(u32::from_be_bytes([a, r, g, b]));
            }
        }
    } else {
        held
    };
    table.get(row.index()).copied().flatten()
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
        .and_then(|weapon| models.get(weapon.index()).copied().flatten());
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

/// Where one `<Mode3D><Model>` widget's flat quad sits on the layout's screen: `[x, y, w, h]`.
fn model_rect(cx: &IconFrame<'_>, name: &str) -> Option<[f32; 4]> {
    let model = cx.layout.models.iter().find(|model| model.name == name)?;
    let placed = cx.sheet.get(&model.src)?;
    let [w, h] = placed.quad_extent?;
    let [x, y, _z] = model.position;
    // Orthographic `y` counts up from the bottom of the screen, as the gauge's
    // models do (`bar_models`): (240, 250) is the top middle, where the
    // original draws the pickup box.
    let y = oag_display::space::SCREEN.1 - y;
    // A flat-rasterised model (Pure's icons) says where its canvas sits from
    // the model's origin; a plain quad is centred on it. Centred on its own
    // authored position, the same convention the sight brackets' quads use.
    let [dx, dy] = placed.quad_centre.unwrap_or([0.0, 0.0]);
    Some([x + dx - w * 0.5, y + dy - h * 0.5, w, h])
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
    Some(Draw::BlendedSprite {
        rect: model_rect(&cx.frame(), name)?,
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

/// The opaque box of the title's own Turbo icon, in layout pixels: its size and its middle.
/// Read off the sheet's pixels (alpha above 32 inside the icon's own region), so it is what the
/// title really draws, margins and all: the glyph a mod's icon stands in for. `None` where
/// the layout, the sheet or the pixels do not give one (the icon is a model, the region is
/// empty).
fn reference_glyph(cx: &IconFrame<'_>) -> Option<([f32; 2], [f32; 2])> {
    let turbo = oag_tables::weapons::Weapon::Turbo;
    let sprite = if let Some(table) = cx.art.pickup_icon_uv {
        let uv = table.get(turbo.index()).copied().flatten()?;
        let mut icon = cx
            .layout
            .sprite(super::dialect_2048::PICKUP_ICON_2048)?
            .clone();
        icon.uv = uv.map(f32::from);
        if let Some(frame) = cx.layout.sprite(super::dialect_2048::PICKUP_BG_FRAME_2048) {
            icon.rect[0] = frame.rect[0] + (frame.rect[2] - icon.rect[2]) * 0.5;
            icon.rect[1] = frame.rect[1] + (frame.rect[3] - icon.rect[3]) * 0.5;
        }
        icon
    } else {
        cx.layout.sprite(&pickup_icon_name(turbo))?.clone()
    };
    let placed = cx.sheet.get(&sprite.src)?;
    let [u, v, mut uw, mut vh] = sprite.uv;
    if uw == 0.0 && vh == 0.0 {
        (uw, vh) = (placed.width as f32, placed.height as f32);
    }
    if uw <= 0.0 || vh <= 0.0 || u < 0.0 || v < 0.0 {
        return None;
    }
    let (x0, y0) = (placed.x + u as u32, placed.y + v as u32);
    let (x1, y1) = (x0 + uw as u32, y0 + vh as u32);
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (u32::MAX, u32::MAX, 0, 0);
    for y in y0..y1.min(cx.sheet.height) {
        for x in x0..x1.min(cx.sheet.width) {
            let alpha = cx
                .sheet
                .rgba
                .get(((y * cx.sheet.width + x) * 4 + 3) as usize)?;
            if *alpha > 32 {
                (min_x, min_y) = (min_x.min(x), min_y.min(y));
                (max_x, max_y) = (max_x.max(x), max_y.max(y));
            }
        }
    }
    if min_x > max_x {
        return None;
    }
    let (sx, sy) = (sprite.rect[2] / uw, sprite.rect[3] / vh);
    let extent = [
        (max_x - min_x + 1) as f32 * sx,
        (max_y - min_y + 1) as f32 * sy,
    ];
    let centre = [
        sprite.rect[0] + ((min_x + max_x + 1) as f32 * 0.5 - x0 as f32) * sx,
        sprite.rect[1] + ((min_y + max_y + 1) as f32 * 0.5 - y0 as f32) * sy,
    ];
    Some((extent, centre))
}

/// What a mod icon's slot is measured from: the layout, the sheet the title's own glyph is
/// read off and the title's art table. Everything [`mod_icon_draw`] needs of a
/// [`Context`], so a loader can size an icon before there is a context to draw it in.
#[derive(Debug, Clone, Copy)]
pub struct IconFrame<'a> {
    /// The mode's parsed layout.
    pub layout: &'a crate::Layout,
    /// The sheet the title's Turbo glyph and backdrop model are read from.
    pub sheet: &'a crate::sprite::Sheet,
    /// How the title's HUD sprites reach the screen.
    pub art: &'a oag_title::HudArt,
}

impl Context<'_> {
    fn frame(&self) -> IconFrame<'_> {
        IconFrame {
            layout: self.layout,
            sheet: self.sheet,
            art: self.art,
        }
    }
}

/// Where a mod's icon goes: the pickup's frame, the box the art aims at, its middle, and
/// whether the art fills that box (a title whose Turbo glyph is readable) or is only
/// shrunk to the frame (Pure, which has no glyph to measure).
struct Slot {
    frame: [f32; 4],
    aim: [f32; 2],
    centre: [f32; 2],
    fill: bool,
}

impl Slot {
    /// The scale that takes art of `art` layout units to the slot.
    fn fit(&self, art: [f32; 2]) -> f32 {
        let to_aim = (self.aim[0] / art[0]).min(self.aim[1] / art[1]);
        let to_frame = (self.frame[2] / art[0]).min(self.frame[3] / art[1]);
        if self.fill { to_aim } else { to_aim.min(1.0) }.min(to_frame)
    }
}

fn slot(cx: &IconFrame<'_>) -> Option<Slot> {
    let frame = if cx.art.pickup_icon_models.is_some() {
        // Pure draws its icons as models: the frame is the striped backdrop's quad.
        model_rect(cx, cx.art.pickup_icon_backdrop_model?)?
    } else {
        if cx.art.pickup_icon_uv.is_some() {
            cx.layout.sprite(super::dialect_2048::PICKUP_BG_FRAME_2048)
        } else {
            cx.layout.sprite(PICKUP_BACKGROUND)
        }
        .or_else(|| {
            cx.layout
                .sprite(&pickup_icon_name(oag_tables::weapons::Weapon::Turbo))
        })?
        .rect
    };
    let (aim, centre, fill) = match reference_glyph(cx) {
        Some((extent, centre)) => (extent, centre, true),
        None => (
            [frame[2], frame[3]],
            [frame[0] + frame[2] * 0.5, frame[1] + frame[3] * 0.5],
            false,
        ),
    };
    // A title whose backdrop frame is always up (HD, Omega) has the icon in the middle of
    // it; the real glyph there sits a little below, and only its size is taken.
    let centre = if fill && cx.art.always_on.contains(&PICKUP_BACKGROUND) {
        [frame[0] + frame[2] * 0.5, frame[1] + frame[3] * 0.5]
    } else {
        centre
    };
    Some(Slot {
        frame,
        aim,
        centre,
        fill,
    })
}

/// The size, in layout units, a vector icon whose drawing is `svg` layout units across is
/// drawn at in the pickup's slot, so a loader can rasterise it at exactly that many pixels
/// ([`crate::sprite::Sheet::texel_density`]). `None` where the slot cannot be measured.
#[must_use]
pub fn mod_icon_size(cx: IconFrame<'_>, svg: [f32; 2]) -> Option<[f32; 2]> {
    if svg[0] <= 0.0 || svg[1] <= 0.0 {
        return None;
    }
    let fit = slot(&cx)?.fit(svg);
    Some([svg[0] * fit, svg[1] * fit])
}

/// A mod's own pickup icon: its art (the opaque box of the PNG, [`crate::sprite::Placed::quad_extent`]
/// and [`crate::sprite::Placed::quad_centre`] where the loader measured it, else the whole
/// image) scaled, aspect kept, to fill the box the title's own Turbo glyph fills and centred
/// on it ([`reference_glyph`]), so a mod's icon is as big as a real one and sits where it
/// sits (HD and Omega: the middle of the always-up frame instead); where the title's glyph cannot be read, the art is drawn at its own size on the
/// pickup's backdrop frame. Either way it is shrunk to the frame if it would not fit it.
/// Faded as the title's own icon is. **Chosen, not measured**: the art's margin never
/// counts, so a PNG authored on the title's own texture cell and one cropped to its glyph
/// draw alike. The frame is the layout's `PickupBackground` (2048: `PickupBgFrame`), else the
/// `TurboIcon` sprite. A title that draws its icons as models (Pure) takes the backdrop
/// model's quad as the frame and draws the PNG on it at its own size (the art is the glyph alone).
pub(super) fn mod_icon_draw(cx: &Context<'_>, src: &str, look: Option<Look>) -> Option<Draw> {
    let placed = cx.sheet.get(src)?;
    let slot = slot(&cx.frame())?;
    // Texels per layout unit: 1 for a PNG, the raster's own for a vector icon, whose
    // quad is then laid on whole screen pixels so each texel lands on one.
    let rasterised = cx.sheet.texel_density(src);
    let density = rasterised.unwrap_or(1.0);
    let canvas = [
        placed.width as f32 / density,
        placed.height as f32 / density,
    ];
    let art = placed
        .quad_extent
        .map_or(canvas, |e| e.map(|v| v / density));
    let offset = placed
        .quad_centre
        .map_or([0.0, 0.0], |o| o.map(|v| v / density));
    let fit = slot.fit(art);
    let snap = |v: f32| {
        if rasterised.is_some() {
            (v * density).round() / density
        } else {
            v
        }
    };
    let (w, h) = (snap(canvas[0] * fit), snap(canvas[1] * fit));
    let fade = look.map_or(1.0, |look| look.icon);
    Some(Draw::Sprite {
        rect: [
            snap(slot.centre[0] - offset[0] * fit - w * 0.5),
            snap(slot.centre[1] - offset[1] * fit - h * 0.5),
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

#[cfg(test)]
pub(crate) fn mod_icon_draw_for_test(cx: &Context<'_>, src: &str) -> Option<Draw> {
    mod_icon_draw(cx, src, None)
}
