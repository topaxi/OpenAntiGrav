//! Reading a mode's HUD off the disc: which layout it authors, the widgets in
//! it, and the font role each one draws in.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// Which layout a mode draws its HUD from, on the title that is open.
///
/// **The names are the title's** ([`oag_title::HudLayouts`]) and the mapping is
/// this function's, for the reason `oag_pulse::race` states about the Zone
/// hull: `Mode` is the engine's type and a title package must not grow one.
///
/// Until 2026-08-18 this returned `oag_pulse`'s names for every title, and HD
/// worked only because PSARC path normalisation folds
/// `Data\XML\Arcade_HUD.xml` onto the `/data/xml/arcade_hud.xml` its manifest
/// stores - so HD's own tables had no reader at all. Finding S2 of that day's
/// review. The row where the luck ran out is **speed lap**: neither PSP disc
/// ships a `SpeedLap_HUD.xml`, so both draw the time trial's, while HD ships a
/// separate one that nothing here could ask for.
///
/// The single race's layout is the only shipped one that carries pickup widgets
/// at all: `PickupBackground`, `SubWeapon` and one `<Type>Icon` per weapon. The
/// time trial's and Zone's carry none, which is the disc agreeing from the
/// presentation side with what [`Mode::weapons_enabled`] reads out of the code.
/// See `docs/gameplay/pickups.md`.
///
/// Layouts no mode here reaches - Eliminator's on every title, and HD's
/// Detonator, Duel and MPTag - stay in their own title crate waiting for the
/// mode that uses them.
#[must_use]
pub const fn hud_layout(title: &'static oag_title::Title, mode: Mode) -> &'static str {
    match mode {
        Mode::TimeTrial => title.hud.time_trial,
        Mode::SpeedLap => title.hud.speed_lap,
        Mode::Zone => title.hud.zone,
        Mode::SingleRace => title.hud.arcade,
    }
}

/// Reads the HUD's layout, atlas, fonts and strings.
///
/// Every piece degrades on its own and says so. The report matters more here than
/// it looks: a HUD drawn in the 5x7 fallback font looks like a rendering bug, and
/// a silent fallback would send someone looking in the shader.
pub(super) fn load_hud(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    mode: Mode,
    language_plugins: &[&str],
    report: &mut Vec<String>,
) -> crate::hud::Assets {
    let entry = hud_layout(title, mode);
    // **Through `compose`, on every title.** A Pulse or Pure layout includes
    // nothing and composes to itself, so this is the same read it always was
    // for them; HD's roots are *shells* that pull in up to sixteen fragments by
    // `<LoadXML SrcRel=>`, and reading only the root got what an HD race
    // actually drew before finding S2 was fixed - one fill, no sprites, no
    // labels. `oag_hd::hud`'s own docs called that out ("reading only the root
    // gets two empty rectangles") while nothing here composed.
    //
    // `fexml::text` inside `compose` decides shortened-versus-plain from each
    // blob's own first bytes, which is what lets one call serve both dialects:
    // the PSP shortens its layouts, the PS2 and HD ship plain `<?xml`, and
    // reaching for `expand` refused the PS2's outright and took the whole HUD
    // with it.
    let composed = crate::hud::compose(entry, |path| archives.read_name(path).ok());
    let layout = match composed {
        Some(composed) => {
            let layout = composed.layout;
            report.push(format!(
                "HUD {entry}: {} sprite(s), {} fill(s), {} label(s), {} model(s){}",
                layout.sprites.len(),
                layout.fills.len(),
                layout.labels.len(),
                layout.models.len(),
                match composed.files.len() {
                    1 => String::new(),
                    n => format!(" composed from {n} file(s)"),
                }
            ));
            for missing in &composed.missing {
                report.push(format!("HUD: include {missing} is not in this source"));
            }
            for note in &layout.skipped {
                report.push(format!("HUD: skipped {note}"));
            }
            Some(layout)
        }
        None => {
            report.push(format!("HUD {entry} unavailable; no HUD"));
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
        // `read_image`, not `read_name`: the PS2 keeps its atlas under the
        // declared name with the extension rewritten to `.pct` (see
        // `oag_pulse::ps2_texture_name`). The declared name is tried first, so
        // any other source takes exactly the path it took before, and a name
        // under neither spelling still fails naming the one that was asked for.
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
        // The grid the layout's numbers are in, off the mounted source rather
        // than defaulted: `Space::default()` is the PSP's 480x272, and an HD
        // layout read in it lands every widget four times oversized and a
        // quarter of the way into the picture. Same call `boot::load_shell`
        // makes for the front end.
        space: crate::frontend::Space::of(archives.layout.platform),
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
