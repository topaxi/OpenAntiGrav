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

    let sheet = load_atlases(archives, title, layout.as_ref(), entry, report);

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
        art: title.hud_art,
    }
}

/// Every texture this layout's sprites name, decoded into one sheet.
///
/// **Every, not the first**, and that is the whole of the difference between an
/// HD HUD that draws and one that draws the right patch of the wrong picture.
/// Pulse's and Pure's nine layouts name at most one texture each - measured,
/// and still pinned by `the_layouts_name_at_most_one_texture` - so on those this
/// builds the same one-image sheet it always did. HD's eighteen name twelve
/// between them, up to six in one layout: 45 of the arcade HUD's 138 sprites
/// name something other than `HUD_Components.gtf`, and until 2026-08-25 all 45
/// were offset by `HUD_Components`'s placement and sampled it at coordinates
/// meant for a different texture.
///
/// Keyed by the reference the layout spells, which is what
/// [`crate::hud::sprite_draw`] looks up. Which entry that reference resolves to
/// is this function's business: the declared name first, then the title's own
/// rewrite of it - `oag_title::HudArt::texture_extension`, which is `None` on
/// both PSP titles and `.gtf` on HD, where a `src` names the exporter's input
/// rather than the shipped file.
///
/// A texture that will not resolve or will not decode costs its own sprites and
/// nothing else: they are reported here and skipped at draw time rather than
/// drawn out of a neighbour's pixels.
fn load_atlases(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    layout: Option<&crate::hud::Layout>,
    entry: &str,
    report: &mut Vec<String>,
) -> crate::sprite::Sheet {
    let references = layout.map(crate::hud::Layout::textures).unwrap_or_default();
    // **A layout that names no `.mip` may still have sight art**, and taking the
    // early return before [`sight_art`] is what made Pure's reticle draw
    // nothing: all four of its HUD layouts are `<Model>` geometry with no atlas
    // at all, so this branch is the *only* one they ever take. The report line
    // stays - it is still true and still worth saying - but the sheet is built
    // either way.
    if references.is_empty() {
        report.push(format!(
            "HUD {entry} names no texture; its sprites are drawn from <Model> \
             geometry rather than an atlas"
        ));
        let extra = sight_art(archives, layout, report);
        if extra.is_empty() {
            return crate::sprite::Sheet::default();
        }
        let mut notes = Vec::new();
        let sheet = crate::sprite::Sheet::build_with(&[], extra, &mut notes);
        report.extend(notes);
        return sheet;
    }

    let mut blobs: Vec<(String, Vec<u8>)> = Vec::new();
    for reference in &references {
        match read_hud_texture(archives, title, reference) {
            Ok(blob) => blobs.push(((*reference).to_string(), blob)),
            Err(why) => report.push(format!("HUD atlas {reference} unavailable ({why})")),
        }
    }

    let mut notes = Vec::new();
    let sheet =
        crate::sprite::Sheet::build_with(&blobs, sight_art(archives, layout, report), &mut notes);
    report.extend(notes);
    report.push(format!(
        "HUD {entry}: {} of {} texture(s) in a {}x{} sheet",
        sheet.len(),
        references.len(),
        sheet.width,
        sheet.height
    ));
    for reference in &references {
        if sheet.get(reference).is_none() {
            report.push(format!(
                "HUD atlas {reference} did not decode; its sprites draw nothing"
            ));
        }
    }
    sheet
}

/// The lock-on reticle's art, decoded out of the `<Mode3D>` models that carry it.
///
/// **The sights are the one HUD widget whose picture is not in a texture file.**
/// `Arcade_HUD.xml` authors nine of them - `missile_sight_1` … `_4` and
/// `missile_sight_inner`, then `leachbeam_sight_1` … `_4` - over **three**
/// `.vex` models, each a single 8-unit quad with its texture embedded. So the
/// art is reached by building the model and taking the texture it unpacked,
/// which is what [`crate::sprite::Sheet::build_with`] exists for. See
/// `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`.
///
/// Keyed by the model's own `Src`, so [`crate::race::Race`]'s draw looks it up
/// the way a sprite looks up its atlas.
///
/// **A model that will not read costs its own reticle and nothing else.** It is
/// reported and skipped; the draw then finds no entry and puts nothing on
/// screen, which is this project's rule for an asset it cannot play rather than
/// a stand-in that reads as plausible.
fn sight_art(
    archives: &mut oag_assets::Archives,
    layout: Option<&crate::hud::Layout>,
    report: &mut Vec<String>,
) -> Vec<(String, u32, u32, Vec<u8>)> {
    let Some(layout) = layout else {
        return Vec::new();
    };

    let mut wanted: Vec<&str> = Vec::new();
    for model in &layout.models {
        if !crate::race::sight::is_sight_widget(&model.name) || model.src.is_empty() {
            continue;
        }
        // Four widgets share one model; the sheet wants it once.
        if !wanted.contains(&model.src.as_str()) {
            wanted.push(model.src.as_str());
        }
    }

    let mut out = Vec::new();
    for src in wanted {
        let blob = match archives.read_name(src) {
            Ok(blob) => blob,
            Err(why) => {
                report.push(format!("sight model {src} unavailable ({why})"));
                continue;
            }
        };
        let model = match oag_render::mesh::build(src, &blob) {
            Ok(model) => model,
            Err(why) => {
                report.push(format!("sight model {src} did not build ({why})"));
                continue;
            }
        };
        // One texture apiece, and the reticle is what it draws. A model that
        // embeds none is reported rather than drawn untextured, which for a
        // white-on-nothing bracket would be an invisible quad.
        let Some(texture) = model.textures.iter().flatten().next() else {
            report.push(format!("sight model {src} embeds no texture"));
            continue;
        };
        report.push(format!(
            "sight model {src}: {}x{}",
            texture.width, texture.height
        ));
        out.push((
            src.to_string(),
            texture.width,
            texture.height,
            texture.rgba.clone(),
        ));
    }
    out
}

/// Reads one HUD texture, by the name the layout declares and then by the name
/// this title rewrites it to.
///
/// Two rewrites stack here and they answer different questions.
/// [`oag_pulse::read_image`] answers "which *pressing* keeps this where" - the
/// PS2 build's `.pct` - and every source goes through it.
/// `HudArt::texture_extension` answers "does this title's XML name the shipped
/// file at all", which is HD's `.gtf`. The declared name is tried first in both,
/// so a title needing neither takes exactly the path it took before.
///
/// # Errors
///
/// The error naming the entry the layout asked for, never the rewritten one: a
/// message about `hdhud.gtf` sends a reader looking for a name their XML does
/// not contain.
fn read_hud_texture(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    reference: &str,
) -> oag_assets::Result<Vec<u8>> {
    let declared = oag_pulse::read_image(archives, reference);
    let Some(extension) = title.hud_art.texture_extension else {
        return declared;
    };
    match declared {
        Ok(blob) => Ok(blob),
        Err(original) => {
            let rewritten = oag_title::hud::replace_extension(reference, extension);
            if rewritten == reference {
                return Err(original);
            }
            oag_pulse::read_image(archives, &rewritten).map_err(|_| original)
        }
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
