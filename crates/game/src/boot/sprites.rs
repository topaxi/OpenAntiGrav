//! The front end's sprite sheet: every image the screens name, plus the ones
//! the executable draws on its own, and the menu blocks' art read off it.
//!
//! Its own file rather than a stretch of `boot.rs`, for the reason
//! `provenance.rs` and `images.rs` already give: that file is baselined by
//! `scripts/check-file-size.py` and may shrink but not grow, and the menu
//! blocks arrived as a net addition. The sprite loading moved with them
//! because [`block_art`] reads what [`load`] produced and the two are one
//! subject.

use super::Screens;

/// The menu blocks' decoded art, for a title that has blocks.
///
/// The nine-patch is required - no frame, no boxes, and the report says so
/// rather than the menu quietly going back to bare text. The two marks are
/// each optional on their own: a strip with no underline mark, or rows with
/// no arrows, is a missing sprite and not a missing idiom.
///
/// **The style is the served page's colour.** `FrontEnd_IsFuryStyle` reads a
/// byte whose writer is unread (`menu-blocks.md`), so what stands in for it
/// is the one thing the two styles' archives disagree on that this build
/// already resolves: `DATA00` clears its front end to black and `DATA06` to
/// white, and every other `HD_*` global follows the same split. A black page
/// is the Fury style. Reported, so a menu drawn in the wrong style is a line
/// in the boot log.
pub(super) fn block_art(
    blocks: Option<oag_title::MenuBlocks>,
    sprites: &oag_hud::sprite::Sheet,
    screens: &Screens,
    report: &mut Vec<String>,
) -> Option<oag_ui::menu::block::BlockArt> {
    let blocks = blocks?;
    let Some(frame) = sprites.get(blocks.frame_texture) else {
        report.push(format!(
            "menu blocks: {} did not decode, so entries draw with no box",
            blocks.frame_texture
        ));
        return None;
    };
    let (u, v) = oag_ui::menu::block::FILL_SWATCH_UV;
    let (su, sv) = oag_ui::menu::block::SOLID_SWATCH_UV;
    let (Some(fill_alpha), Some(solid_alpha)) = (
        sprites.alpha_at(frame, u, v),
        sprites.alpha_at(frame, su, sv),
    ) else {
        report.push(format!(
            "menu blocks: {}'s fill swatch is outside the sheet, so entries draw with no box",
            blocks.frame_texture
        ));
        return None;
    };
    let fury = fury_style(screens);
    let cursor = sprites.get(blocks.cursor_texture);
    let arrow = sprites.get(blocks.arrow_texture);
    report.push(format!(
        "menu blocks: {} style, fill swatch alpha {fill_alpha:.3} then {solid_alpha:.3}, underline {}, arrows {}",
        if fury { "Fury" } else { "HD" },
        if cursor.is_some() {
            "decoded"
        } else {
            "missing"
        },
        if arrow.is_some() {
            "decoded"
        } else {
            "missing"
        },
    ));
    Some(oag_ui::menu::block::BlockArt {
        frame,
        fill_alpha,
        solid_alpha,
        cursor,
        arrow,
        fury,
    })
}

/// Whether the served front end is the Fury style - see [`block_art`] for the
/// rule and why it stands in for `FrontEnd_IsFuryStyle`. Read by the menu
/// backdrop's loader too, since it is the same byte both widgets check.
pub(super) fn fury_style(screens: &Screens) -> bool {
    screens.global_is_dark("HD_BG")
}

/// Decodes every image the screens name.
///
/// The names come from the screens rather than from a list here, so a screen
/// that gains an `Image` gains its texture without this function changing.
///
/// **Every archive the source has is searched, `FE.wad` first, and both parts of
/// that matter.** `pulse_logo.mip` is in `FE.wad` *and* `Data.wad` at the same
/// size, which makes `FE.wad` look sufficient; `gameshare_backdrop.mip` is in
/// `Data.wad` only, which proves it is not. The order is deliberate and is
/// therefore written here rather than taken from
/// [`oag_assets::Archives::read_name`], which searches the *bulk* archive
/// first because that is the right default for a race: same size is not same
/// bytes, and a front-end image should come off the front end's own archive.
///
/// `extra` is for the images a screen never names because the executable
/// draws them on its own - the menu blocks' nine-patch and its two marks on
/// HD. They go through the same lookup and the same report line, so one of
/// them missing from the served archives reads exactly like a screen's own
/// image missing.
pub(super) fn load(
    archives: &mut oag_assets::Archives,
    sources: &[&Screens],
    extra: &[&str],
    front_end: &oag_title::FrontEnd,
    report: &mut Vec<String>,
) -> oag_hud::sprite::Sheet {
    let mut srcs: Vec<String> = Vec::new();
    for screen in sources.iter().flat_map(|screens| &screens.screens) {
        // A `TouchButton`'s icon is an image the screen names the same way
        // an `Image` does - 2048's grids are drawn off them.
        let icons = screen
            .touch_buttons
            .iter()
            .filter_map(|button| button.src.as_ref());
        for src in screen.images.iter().map(|image| &image.src).chain(icons) {
            if !srcs.contains(src) {
                srcs.push(src.clone());
            }
        }
    }
    for src in extra {
        if !srcs.iter().any(|known| known == src) {
            srcs.push((*src).to_string());
        }
    }

    let bottom_up = front_end.bottom_up_gnf;
    let lattice_atlas = front_end.touch.as_ref().map(|touch| touch.canvas_texture);
    if srcs.is_empty() && lattice_atlas.is_none() {
        return oag_hud::sprite::Sheet::default();
    }

    let mut blobs: Vec<(String, Vec<u8>)> = Vec::new();
    let mut reversed: Vec<oag_hud::sprite::DecodedImage> = Vec::new();
    for src in &srcs {
        match read_front_end_texture(archives, src) {
            Ok((held, blob)) if is_gnf(&held) && is_bottom_up(src, bottom_up) => {
                match gnf_refusal(&held, src, &blob) {
                    Some(reason) => report.push(reason),
                    None => match bottom_up_rows(src, &blob) {
                        Some(image) => reversed.push(image),
                        None => report.push(format!("image {src}: .gnf rows would not reverse")),
                    },
                }
            }
            Ok((held, blob)) => match gnf_refusal(&held, src, &blob) {
                Some(reason) => report.push(reason),
                None => blobs.push((src.clone(), blob)),
            },
            Err(e) => report.push(format!("image {src}: {e}")),
        }
    }

    if let Some(atlas) = lattice_atlas {
        reversed.extend(lattice_tile(archives, atlas, report));
    }

    let sheet = oag_hud::sprite::Sheet::build_with(&blobs, reversed, report);
    report.push(format!(
        "{} of {} front-end image(s) decoded into a {}x{} sheet",
        sheet.len(),
        srcs.len(),
        sheet.width,
        sheet.height
    ));
    sheet
}

/// Reads a front-end asset, preferring the companion archive over the bulk one.
///
/// The mirror image of [`oag_assets::Archives::read_name`]'s order, for the
/// callers that want a front-end asset specifically. See [`load`] for the
/// two entries that decide it.
///
/// A name neither archive has falls through to
/// [`oag_assets::Archives::read_image`], which knows the handful of
/// images the PS2 keeps under an entry its own XML's name does not hash to -
/// `pulse_logo.mip` among them.
///
/// **Checked before any of that**: a `hash:`-prefixed `name` is a
/// `fallback_images` entry, not a path, and belongs nowhere near
/// `oag_pulse::read_image`'s PS2 `.pct` name rewrite - that rule turns a real
/// PSP path into a PS2 one, which a hash spec is not.
/// [`oag_assets::Archives::read_hash`] already searches every mounted
/// archive, so this both short-circuits and replaces the FE-then-Data order
/// below, which a raw hash has no use for.
pub(crate) fn read_front_end_first(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> oag_assets::Result<Vec<u8>> {
    read_front_end_texture(archives, name).map(|(_, blob)| blob)
}

/// [`read_front_end_first`], and the name the entry was held under.
///
/// A name that is none of the above is read as a texture
/// ([`oag_assets::Archives::read_texture`]): as written, then in this
/// platform's own spelling. **Wipeout 2048's and Omega's front-end XML spells
/// every texture `.gtf`** - HD's, retargeted, the extension never re-authored -
/// and their packages hold the same files as `.gxt` and `.gnf`
/// (`docs/formats/2048-frontend.md`, `docs/formats/gnf.md`). The resolved name
/// is what says which container a blob is.
pub(crate) fn read_front_end_texture(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> oag_assets::Result<(String, Vec<u8>)> {
    if let Some(hash) = super::images::hash_spec(name) {
        return archives
            .read_hash(hash)
            .map(|blob| (name.to_string(), blob));
    }
    if oag_ui_screens::picker::hd::hex::is_thumb(name)
        && let Some((_, blob)) = archives.read_every_name(name).pop()
    {
        return Ok((name.to_string(), blob));
    }
    if let Some(fe) = archives.fe.as_mut()
        && let Ok(blob) = fe.read_entry(name)
    {
        return Ok((name.to_string(), blob));
    }
    archives.read_texture(name)
}

/// Whether the resolved name `held` is a PS4 `.gnf`.
pub(crate) fn is_gnf(held: &str) -> bool {
    oag_texture::decode::Container::of_name(held) == Some(oag_texture::decode::Container::Gnf)
}

/// Why a `.gnf` read as `held` for `name` draws nothing, or `None` when it
/// decodes (`docs/formats/gnf.md`) or is not a `.gnf` at all.
///
/// The reason names the file, its size and the exact refusal - a genuinely
/// tiled surface this project has no formula for, or
/// [`oag_texture::gnf::Error::CorruptBlocks`] (the PSARC-level missing-content
/// population `docs/formats/psarc.md`'s "Block data location" section
/// documents, landing on this specific file) - so a report says "draw nothing"
/// about the real file rather than about a `.gtf` spelling that was never
/// going to resolve. The blob itself stays raw so
/// [`oag_hud::sprite::Image::decode`]'s own `.gnf` branch is what draws it.
pub(crate) fn gnf_refusal(held: &str, name: &str, bytes: &[u8]) -> Option<String> {
    if !is_gnf(held) {
        return None;
    }
    let refuse = |detail: String| {
        format!(
            "image {name}: found as {held} ({} bytes) - PS4 GNF container, {detail}, drawing nothing",
            bytes.len()
        )
    };
    let texture = match oag_texture::gnf::Texture::parse(bytes) {
        Ok(t) => t,
        Err(e) => return Some(refuse(e.to_string())),
    };
    texture.decode(bytes).err().map(|e| refuse(e.to_string()))
}

/// A `.gnf` whose rows are in HD's `.gtf` order, decoded and reversed into the
/// top-down order a sheet holds - see
/// [`oag_title::FrontEnd::bottom_up_gnf`] for which images and why.
pub(crate) fn bottom_up_rows(src: &str, blob: &[u8]) -> Option<oag_hud::sprite::DecodedImage> {
    let texture = oag_texture::gnf::Texture::parse(blob).ok()?;
    let pixels = texture.decode(blob).ok()?;
    let width = usize::try_from(texture.width).ok()?.max(1);
    let rgba = pixels
        .chunks_exact(width)
        .rev()
        .flat_map(|row| row.iter().flatten().copied())
        .collect();
    Some(oag_hud::sprite::DecodedImage {
        src: src.to_string(),
        width: texture.width,
        height: texture.height,
        rgba,
        quad_extent: None,
        quad_centre: None,
        blend: None,
    })
}

/// Whether `src`'s file stem, lower-cased, is one of `stems`.
pub(crate) fn is_bottom_up(src: &str, stems: &[&str]) -> bool {
    let leaf = src.rsplit(['\\', '/']).next().unwrap_or(src);
    let stem = leaf.rsplit_once('.').map_or(leaf, |(stem, _)| stem);
    stems.iter().any(|known| known.eq_ignore_ascii_case(stem))
}

/// 2048's backdrop lattice tile, rendered off its atlas (`canvasTexture.gxt`,
/// a 2048 x 2048 sheet that never goes into the sprite sheet whole). `None`
/// with a report line when the atlas is missing or does not decode: the page
/// then draws without the lattice rather than with a stand-in.
fn lattice_tile(
    archives: &mut oag_assets::Archives,
    atlas: &str,
    report: &mut Vec<String>,
) -> Option<oag_hud::sprite::DecodedImage> {
    use oag_ui::frontend::canvas_lattice as lattice;
    let blob = match read_front_end_first(archives, atlas) {
        Ok(blob) => blob,
        Err(e) => {
            report.push(format!("lattice atlas {atlas}: {e}"));
            return None;
        }
    };
    let decoded = oag_texture::gxt::Gxt::parse(&blob).ok().and_then(|gxt| {
        let texture = gxt.only()?;
        let rgba = texture.to_rgba(&blob).ok()?;
        Some((texture.width, texture.height, rgba))
    });
    let Some((width, height, rgba)) = decoded else {
        report.push(format!("lattice atlas {atlas}: did not decode as a .gxt"));
        return None;
    };
    let rgba: Vec<u8> = rgba.into_iter().flatten().collect();
    let Some(tile) = lattice::bake(&rgba, u32::from(width), u32::from(height)) else {
        report.push(format!(
            "lattice atlas {atlas}: {width}x{height} is too small"
        ));
        return None;
    };
    report.push(format!(
        "lattice picture baked off {atlas} ({width}x{height}) at {}x{}",
        lattice::VIEWPORT[0],
        lattice::VIEWPORT[1]
    ));
    Some(oag_hud::sprite::DecodedImage {
        src: lattice::TILE.to_string(),
        width: lattice::VIEWPORT[0],
        height: lattice::VIEWPORT[1],
        rgba: tile,
        quad_extent: None,
        quad_centre: None,
        blend: None,
    })
}
