//! Reading a mode's HUD off the disc: which layout it authors, the widgets in
//! it, and the font role each one draws in.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// Which layout a mode draws its HUD from.
///
/// Time trial and speed lap share one: `TimeTrial_HUD.xml` carries both, which
/// is why `docs/ui/hud.md` counts five layouts for six modes and why the disc has
/// no `SpeedLap_HUD.xml`. Zone has its own.
///
/// `Arcade_HUD.xml` is the single race's, and it is the only shipped layout that
/// carries pickup widgets at all: `PickupBackground`, `SubWeapon` and one
/// `<Type>Icon` per weapon. `TimeTrial_HUD.xml` and `Zone_HUD.xml` carry none,
/// which is the disc agreeing from the presentation side with what
/// [`Mode::weapons_enabled`] reads out of the code. See
/// `docs/gameplay/pickups.md`.
///
/// The one layout this does not reach - `Elimination_HUD.xml` - is in
/// [`crate::hud::layouts`] waiting for the mode that uses it.
#[must_use]
pub const fn hud_layout(mode: Mode) -> &'static str {
    match mode {
        Mode::TimeTrial | Mode::SpeedLap => crate::hud::layouts::TIME_TRIAL,
        Mode::Zone => crate::hud::layouts::ZONE,
        Mode::SingleRace => crate::hud::layouts::ARCADE,
    }
}

/// Reads the HUD's layout, atlas, fonts and strings.
///
/// Every piece degrades on its own and says so. The report matters more here than
/// it looks: a HUD drawn in the 5x7 fallback font looks like a rendering bug, and
/// a silent fallback would send someone looking in the shader.
pub(super) fn load_hud(
    archives: &mut oag_assets::Archives,
    mode: Mode,
    language_plugins: &[&str],
    report: &mut Vec<String>,
) -> crate::hud::Assets {
    let entry = hud_layout(mode);
    let layout = match archives
        .read_name(entry)
        .map_err(|e| e.to_string())
        // `text`, not `expand`: the PS2 ships these five layouts as plain
        // `<?xml` where the PSP shortens them, and reaching for `expand`
        // refused the PS2's outright and took the whole HUD with it.
        .and_then(|blob| oag_formats::fexml::text(&blob).map_err(|e| e.to_string()))
    {
        Ok(xml) => {
            let layout = crate::hud::Layout::from_xml(&xml);
            report.push(format!(
                "HUD {entry}: {} sprite(s), {} fill(s), {} label(s), {} model(s)",
                layout.sprites.len(),
                layout.fills.len(),
                layout.labels.len(),
                layout.models.len()
            ));
            for note in &layout.skipped {
                report.push(format!("HUD: skipped {note}"));
            }
            Some(layout)
        }
        Err(why) => {
            report.push(format!("HUD {entry} unavailable ({why}); no HUD"));
            None
        }
    };

    // One texture in the sheet, which is what `Sheet::build` is for. Going
    // through the sheet rather than binding the atlas directly means the HUD
    // shares the renderer every other screen uses, `Draw::Sprite` and all.
    let mut sheet = crate::sprite::Sheet::default();
    // **The layout names its own texture**; this used to name it instead. See
    // `crate::hud::Layout::atlas` for why that mattered - Pure's HUDs name none
    // at all, so a constant sent a Pure race looking for a Pulse file.
    match layout.as_ref().and_then(crate::hud::Layout::atlas) {
        None => report.push(format!(
            "HUD {entry} names no texture; its sprites are drawn from <Model> \
             geometry rather than an atlas"
        )),
        // `read_image`, not `read_name`: the PS2 keeps its atlas under an entry
        // its own XML's name does not hash to (see `oag_pulse::PS2_IMAGES`). That
        // substitution table is keyed on Pulse-PSP names and is inert on any other
        // source - a name it does not hold falls through to the original error -
        // so it stays here rather than becoming a per-title lookup.
        Some(atlas) => match oag_pulse::read_image(archives, atlas) {
            Ok(blob) => {
                let mut notes = Vec::new();
                let built = crate::sprite::Sheet::build(&[(atlas.to_string(), blob)], &mut notes);
                report.extend(notes);
                if built.get(atlas).is_some() {
                    report.push(format!(
                        "HUD atlas {atlas}: {}x{} sheet",
                        built.width, built.height
                    ));
                    sheet = built;
                } else {
                    report.push(format!("HUD atlas {atlas} did not decode"));
                }
            }
            Err(why) => report.push(format!("HUD atlas {atlas} unavailable ({why})")),
        },
    }

    // The HUD's captions are `idstring` keys - `IG_HUD_LAP`, `IG_HUD_BEST` - and
    // without a table `StringTable::get_or_id` falls back to the key itself, which
    // put `ig_hud_lap` on screen where `LAP` belongs. The preferred language is the
    // player's saved one; a race reached through `--race` has no settings to read,
    // so this takes the chain's default rather than threading one through.
    //
    // **Before the fonts now**, because the plugins parsed here are also what
    // name the two faces below - the same reordering `boot::load_shell` needed.
    let languages = crate::boot::load_languages(archives, language_plugins, report);
    let strings = crate::boot::load_strings(archives, &languages, None, report);

    // One role each, and no fallback to a second: both titles fill in both of
    // these slots, so a chain of alternatives would be an unexercised guess about
    // a disc nobody has. A source filling in neither draws in 5x7 and says so,
    // which is the honest answer to a question its data has not been asked.
    let font = hud_font(archives, &languages, roles::HUD, report);
    let small_font = hud_font(archives, &languages, roles::HUD_SMALL, report);

    crate::hud::Assets {
        layout,
        sheet,
        font,
        small_font,
        strings,
    }
}

/// Reads the `.fnt` this source's language plugins fill `role` in with.
///
/// The same shape as `crate::boot::load_font`, which reads the front end's
/// `Default` face, and now the same read *and* the same resolution: both take
/// the filename off a `<Font>` slot rather than naming one, and both go through
/// [`oag_assets::Archives::read_font`], so a PS2 source finds the glyph atlas
/// the disc keeps in the entry after the `.fnt` rather than falling back to 5x7.
/// Kept separate only so the report line says which font is being talked about.
///
/// A source whose plugins fill in no such slot draws in 5x7 and says which role
/// went unanswered - see [`crate::language::roles`] for what each disc fills in.
///
/// Both HUD fonts are pre-outlined on both Pulse pressings - six distinct greys,
/// alpha covering glyph *plus* border - so `Atlas::from_font`'s body/outline
/// split applies unchanged there; see `docs/formats/fnt.md`. Whether Pure's own
/// `HUDFont.fnt` is outlined the same way has not been measured.
pub(super) fn hud_font(
    archives: &mut oag_assets::Archives,
    languages: &[crate::language::Language],
    role: &str,
    report: &mut Vec<String>,
) -> crate::font::Atlas {
    let Some(name) = languages
        .iter()
        .find_map(|language| language.font(role))
        .map(str::to_string)
    else {
        report.push(format!(
            "no language plugin names a {role:?} font on this source; drawing with 5x7"
        ));
        return crate::font::Atlas::build();
    };
    match archives.read_font(&name).map_err(|e| e.to_string()) {
        Ok(font) => {
            report.push(format!(
                "HUD font {name} (role {role:?}): {}x{} atlas, {} glyph(s), line height {}",
                font.width,
                font.height,
                font.glyphs.len(),
                font.line_height
            ));
            crate::font::Atlas::from_font(&font)
        }
        Err(why) => {
            report.push(format!(
                "HUD font {name} (role {role:?}) unavailable ({why}); drawing with 5x7"
            ));
            crate::font::Atlas::build()
        }
    }
}
