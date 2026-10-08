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
    screens
        .globals
        .get("HD_BG")
        .and_then(|value| oag_ui::screen::parse_argb(value))
        .is_some_and(|argb| {
            let [r, g, b] = [argb >> 16 & 0xff, argb >> 8 & 0xff, argb & 0xff];
            r + g + b < 3 * 128
        })
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
    bottom_up: &[&str],
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

    if srcs.is_empty() {
        return oag_hud::sprite::Sheet::default();
    }

    let mut blobs: Vec<(String, Vec<u8>)> = Vec::new();
    let mut reversed: Vec<oag_hud::sprite::DecodedImage> = Vec::new();
    for src in &srcs {
        match read_front_end_first(archives, src) {
            Ok(blob) => blobs.push((src.clone(), blob)),
            // Wipeout: Omega Collection's front-end XML still spells its
            // images `.gtf`, the way `vita_texture_name` above already
            // documents for 2048's `.gxt`, but the shipped file is Sony's
            // PS4 `.gnf` container (`docs/formats/gnf.md`) - checked and,
            // when it decodes, pushed under `src`'s own `.gtf`-spelled key
            // so every downstream lookup keeps working unchanged; when it
            // does not, a report line names the real file and the exact
            // reason rather than the `.gtf` spelling that was never going
            // to resolve, what `CLAUDE.md`'s "draw nothing and say so" asks
            // for.
            Err(e) => match gnf_sibling(archives, src) {
                Some(Ok(blob)) if is_bottom_up(src, bottom_up) => {
                    match bottom_up_rows(src, &blob) {
                        Some(image) => reversed.push(image),
                        None => report.push(format!("image {src}: .gnf rows would not reverse")),
                    }
                }
                Some(Ok(blob)) => blobs.push((src.clone(), blob)),
                Some(Err(reason)) => report.push(reason),
                None => report.push(format!("image {src}: {e}")),
            },
        }
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
    if let Some(hash) = super::images::hash_spec(name) {
        return archives.read_hash(hash);
    }
    if oag_ui_screens::picker::hd::hex::is_thumb(name)
        && let Some((_, blob)) = archives.read_every_name(name).pop()
    {
        return Ok(blob);
    }
    if let Some(fe) = archives.fe.as_mut()
        && let Ok(blob) = fe.read_entry(name)
    {
        return Ok(blob);
    }
    match oag_pulse::read_image(archives, name) {
        Ok(blob) => Ok(blob),
        // **Wipeout 2048's front-end XML spells every texture `.gtf` and its
        // package holds every one as `.gxt`** - `Data\FE\Images\StudioLogo.gtf`
        // in `Intro_Definition.xml` is `data/FE/Images/StudioLogo.gxt` in
        // `data.psarc`, and so on for all of them (`2048-frontend.md`). The
        // XML is Wipeout HD's, retargeted, and the extension was never
        // re-authored; the Vita loader evidently maps it. Tried only after
        // the name as written fails, so a source that does ship a `.gtf`
        // still gets the file it named. The same rewrite HD's own HUD
        // loader makes in the other direction (`oag_hd::hud`).
        Err(error) => match vita_texture_name(name) {
            Some(gxt) => archives.read_name(&gxt).or(Err(error)),
            None => Err(error),
        },
    }
}

/// `name` with a `.gtf` extension respelled `.gxt`, or `None` for any other
/// extension. Case-insensitive on the extension, as the archive is on the
/// whole path.
fn vita_texture_name(name: &str) -> Option<String> {
    let stem = name
        .strip_suffix(".gtf")
        .or_else(|| name.strip_suffix(".GTF"))?;
    Some(format!("{stem}.gxt"))
}

/// If `name`'s `.gtf` spelling failed to resolve but a `.gnf` sibling is
/// present in the served archives, decodes it (`docs/formats/gnf.md`) and
/// returns `Some(Ok(bytes))` - still the raw `.gnf` bytes, not pixels, so
/// [`oag_hud::sprite::Image::decode`]'s own `.gnf` branch is what actually
/// draws them, the same one `Sheet::build` already runs every other image
/// through - or `Some(Err(reason))` naming the file, its size and the exact
/// reason it draws nothing instead (a genuinely tiled surface this project
/// has no formula for, or [`oag_texture::gnf::Error::CorruptBlocks`] - the
/// PSARC-level missing-content population `docs/formats/psarc.md`'s "Block
/// data location" section documents, landing on this specific file).
/// `None` when there is no `.gnf` sibling either, so the caller falls back
/// to the ordinary "not found" message.
pub(crate) fn gnf_sibling(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> Option<Result<Vec<u8>, String>> {
    let stem = name
        .strip_suffix(".gtf")
        .or_else(|| name.strip_suffix(".GTF"))?;
    let gnf = format!("{stem}.gnf");
    let bytes = archives.read_name(&gnf).ok()?;
    let refuse = |detail: String| {
        format!(
            "image {name}: found as {gnf} ({} bytes) - PS4 GNF container, {detail}, drawing nothing",
            bytes.len()
        )
    };
    let texture = match oag_texture::gnf::Texture::parse(&bytes) {
        Ok(t) => t,
        Err(e) => return Some(Err(refuse(e.to_string()))),
    };
    match texture.decode(&bytes) {
        Ok(_) => Some(Ok(bytes)),
        Err(e) => Some(Err(refuse(e.to_string()))),
    }
}

/// A `.gnf` whose rows are in HD's `.gtf` order, decoded and reversed into the
/// top-down order a sheet holds - see
/// [`oag_title::FrontEnd::bottom_up_gnf`] for which images and why.
fn bottom_up_rows(src: &str, blob: &[u8]) -> Option<oag_hud::sprite::DecodedImage> {
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
        blend: None,
    })
}

/// Whether `src`'s file stem, lower-cased, is one of `stems`.
fn is_bottom_up(src: &str, stems: &[&str]) -> bool {
    let leaf = src.rsplit(['\\', '/']).next().unwrap_or(src);
    let stem = leaf.rsplit_once('.').map_or(leaf, |(stem, _)| stem);
    stems.iter().any(|known| known.eq_ignore_ascii_case(stem))
}
