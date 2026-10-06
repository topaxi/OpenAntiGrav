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
/// Layouts no mode here reaches - HD's Detonator, Duel and MPTag - stay in
/// their own title crate waiting for the mode that uses them.
#[must_use]
pub const fn hud_layout(title: &'static oag_title::Title, mode: Mode) -> &'static str {
    match mode {
        Mode::TimeTrial => title.hud.time_trial,
        Mode::SpeedLap => title.hud.speed_lap,
        Mode::Zone => title.hud.zone,
        // Measured, not assumed: `docs/formats/race-setup.md`'s own reading
        // of `docs/ui/hud.md`'s five-layout census finds no
        // `Tournament_HUD.xml` at all, and cites `Arcade_HUD.xml` as "the
        // single-race and tournament layout" - see `Mode::Tournament`'s own
        // doc comment. `Head2Head` shares it too - no `Head2Head_HUD.xml`
        // exists either, and `Hud_BindWidgets` reaches the disc's own
        // `HeadToHeadBar` widget through this same layout by branching on
        // the live game mode rather than loading a different file - see
        // `docs/ghidra/functions/psp-pulse-usa/head2head.md`.
        Mode::SingleRace | Mode::Tournament | Mode::Head2Head => title.hud.arcade,
        Mode::Eliminator => title.hud.elimination,
    }
}

/// Reads the HUD's layout, atlas, fonts and strings.
///
/// Every piece degrades on its own and says so. The report matters more here than
/// it looks: a HUD drawn in the 5x7 fallback font looks like a rendering bug, and
/// a silent fallback would send someone looking in the shader.
///
/// `preferred_language` is `race::Options::language` - the player's saved
/// choice - passed straight through rather than read here: `None` when
/// nothing has been chosen yet, resolved the same way everywhere else a
/// chosen language is resolved, through [`oag_ui::language::load::chosen_language`].
pub(super) fn load_hud(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    mode: Mode,
    language_plugins: &[&str],
    preferred_language: Option<&str>,
    report: &mut Vec<String>,
) -> oag_hud::Assets {
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
    let composed = oag_hud::compose(entry, |path| archives.read_name(path).ok());
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

    // The HUD's captions are `idstring` keys - `IG_HUD_LAP` on Pulse,
    // `HUD_Lap` on Pure, whatever the layout's own XML names - and without a
    // table `StringTable::get_or_id` falls back to the key itself, which put
    // `IG_HUD_LAP` on screen where `LAP` belongs. `preferred_language` is the
    // player's own saved choice, threaded down from `race::Options::language`
    // by every caller of [`load`] (`load::load`) - `None` only when nothing
    // has been chosen yet, in which case this takes the chain's own default
    // exactly as the front-end picker's own preselection does. **This used to
    // be `None` unconditionally here**, which is a bug this parameter closes:
    // a player who picked German still got whichever language a title's own
    // plugin order puts first - French on the PSP EU pressing - and it read
    // as correct only for whoever happened to test with that one.
    //
    // **Before the fonts now**, because the plugins parsed here are also what
    // name the two faces below - the same reordering `boot::load_shell` needed.
    let languages = oag_ui::language::load::load_languages(
        archives,
        language_plugins,
        title.front_end.and_then(|front_end| front_end.disc_strings),
        report,
    );
    let chosen = oag_ui::language::load::chosen_language(&languages, preferred_language);
    let strings =
        oag_ui::language::load::load_strings(archives, &languages, preferred_language, report);

    // The role names are this *title's* own, through `oag_title::HudArt` -
    // not the shared `oag_ui::language::roles::HUD`/`HUD_SMALL` literals
    // every source used to be asked for regardless of what its own plugins
    // actually name. That literal ask is what drew 2048's HUD in the 5x7
    // fallback: none of its plugins fill `"HUD"`, they fill `"2048HUD"`
    // instead, and two of its seventeen plugins (`korean`,
    // `traditionalchinese`) carry a *leftover* `"HUD"`/`"HUDSmall"` role
    // pointing at files 2048 does not ship - which the old literal ask could
    // pick up ahead of the plugin the player actually chose. See
    // `oag_title::HudArt::hud_font_role`.
    let font = hud_font(
        archives,
        &languages,
        chosen,
        title.hud_art.hud_font_role,
        report,
    );
    // `None` is a real gap on 2048's own plugins - see
    // `oag_title::HudArt::hud_small_font_role` - and the caption face falls
    // back to the value face's own atlas rather than to 5x7, chosen rather
    // than measured: a Vita3K race frame shows two on-screen sizes, but the
    // layout's own per-widget `scale` is what draws the difference, not a
    // second `.fnt` file this pass located.
    let small_font = match title.hud_art.hud_small_font_role {
        Some(role) => hud_font(archives, &languages, chosen, role, report),
        None => {
            // Named as "reuse", not "draw in the {role} face": `font` above
            // may itself already be the 5x7 fallback if the value role
            // failed to resolve or decode, and this line must not claim a
            // face loaded when it did not.
            report.push(
                "HUD: this title names no caption font role; captions reuse the value face"
                    .to_string(),
            );
            font.clone()
        }
    };

    // The `Default` face, only for a layout with a widget that names it (on
    // Pulse, the Eliminator's kill column), so a layout without one costs no
    // second font decode and no report line about a role it never asked for.
    let default_font = if layout.as_ref().is_some_and(|layout| {
        layout
            .labels
            .iter()
            .any(|label| label.font == oag_hud::Font::Default)
    }) {
        hud_font(
            archives,
            &languages,
            chosen,
            oag_ui::language::roles::DEFAULT,
            report,
        )
    } else {
        font.clone()
    };

    oag_hud::Assets {
        layout,
        sheet,
        font,
        small_font,
        default_font,
        strings,
        // The grid the layout's numbers are in, off the mounted source rather
        // than defaulted: `Space::default()` is the PSP's 480x272, and an HD
        // layout read in it lands every widget four times oversized and a
        // quarter of the way into the picture. Same call `boot::load_shell`
        // makes for the front end.
        space: oag_display::space::Space::of(archives.layout.platform),
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
/// [`oag_hud::sprite_draw`] looks up. Which entry that reference resolves to
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
    layout: Option<&oag_hud::Layout>,
    entry: &str,
    report: &mut Vec<String>,
) -> oag_hud::sprite::Sheet {
    let references = layout.map(oag_hud::Layout::textures).unwrap_or_default();
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
        let extra = vex_model_art(archives, title, layout, report);
        if extra.is_empty() {
            return oag_hud::sprite::Sheet::default();
        }
        let mut notes = Vec::new();
        let sheet = oag_hud::sprite::Sheet::build_with(&[], extra, &mut notes);
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
    let sheet = oag_hud::sprite::Sheet::build_with(
        &blobs,
        vex_model_art(archives, title, layout, report),
        &mut notes,
    );
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

/// The lock-on reticle's and Pure's weapon icons' art, decoded out of the
/// `<Mode3D>` models that carry them.
///
/// **These are the HUD widgets whose picture is not in a texture file.**
/// `Arcade_HUD.xml` authors nine sight widgets - `missile_sight_1` … `_4` and
/// `missile_sight_inner`, then `leachbeam_sight_1` … `_4` - over **three**
/// `.vex` models, each a single flat quad with its texture embedded; Pure adds
/// ten weapon-icon widgets plus their shared backdrop grid, eleven more
/// models, one apiece (`oag_title::HudArt::pickup_icon_models` and
/// `pickup_icon_backdrop_model`). So the art is reached by building the model
/// and taking the texture it unpacked, which is what
/// [`oag_hud::sprite::Sheet::build_with`] exists for. See
/// `docs/ghidra/functions/psp-pulse-usa/lock-sight.md` for the sights' law and
/// `docs/gameplay/pickups.md` for the icons' colour finding.
///
/// Keyed by the model's own `Src`, so [`crate::Race`]'s draw looks it up
/// the way a sprite looks up its atlas.
///
/// **Each entry's quad extent is read off its own vertices**, not carried as a
/// per-model constant the way the sights' `SIGHT_SIZE` is - see
/// [`oag_hud::sprite::Placed::quad_extent`]'s own doc for why the two do not (yet)
/// share a reading.
///
/// **A model that will not read costs its own widget and nothing else.** It is
/// reported and skipped; the draw then finds no entry and puts nothing on
/// screen, which is this project's rule for an asset it cannot play rather than
/// a stand-in that reads as plausible.
fn vex_model_art(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    layout: Option<&oag_hud::Layout>,
    report: &mut Vec<String>,
) -> Vec<oag_hud::sprite::DecodedImage> {
    let Some(layout) = layout else {
        return Vec::new();
    };

    let icon_names: Vec<&str> = title
        .hud_art
        .pickup_icon_models
        .into_iter()
        .flatten()
        .flatten()
        .chain(title.hud_art.pickup_icon_backdrop_model)
        .collect();

    let mut wanted: Vec<&str> = Vec::new();
    for model in &layout.models {
        let is_wanted = oag_race::sight::is_sight_widget(&model.name)
            || icon_names.contains(&model.name.as_str());
        if !is_wanted || model.src.is_empty() {
            continue;
        }
        // Several widgets can share one model (four sight brackets do); the
        // sheet wants it once.
        if !wanted.contains(&model.src.as_str()) {
            wanted.push(model.src.as_str());
        }
    }

    let mut out = Vec::new();
    for src in wanted {
        let blob = match archives.read_name(src) {
            Ok(blob) => blob,
            Err(why) => {
                report.push(format!("HUD model {src} unavailable ({why})"));
                continue;
            }
        };
        let model = match oag_mesh::mesh::build(src, &blob) {
            Ok(model) => model,
            Err(why) => {
                report.push(format!("HUD model {src} did not build ({why})"));
                continue;
            }
        };
        // One texture apiece, and the widget is what it draws. A model that
        // embeds none is reported rather than drawn untextured, which for a
        // white-on-nothing bracket would be an invisible quad.
        let Some(texture) = model.textures.iter().flatten().next() else {
            report.push(format!("HUD model {src} embeds no texture"));
            continue;
        };
        // The sheet is composed on the CPU, so this needs texels rather than a
        // binding. Every HUD model on both PSP titles is a `.vex`, whose
        // textures are always decoded RGBA8; only Wipeout HD's `.gtf` keeps
        // its blocks, and HD draws no sight or icon this way. Reported rather
        // than unwrapped, so the day that stops being true it says so instead
        // of panicking.
        let Some(rgba) = texture.rgba() else {
            report.push(format!(
                "HUD model {src}: its texture is block-compressed, which this sheet cannot compose"
            ));
            continue;
        };
        let blend = quad_blend(src, &model, report);
        report.push(format!(
            "HUD model {src}: {}x{}, quad {:?}, blend {blend:?}",
            texture.width,
            texture.height,
            quad_extent(&model)
        ));
        out.push(oag_hud::sprite::DecodedImage {
            src: src.to_string(),
            width: texture.width,
            height: texture.height,
            rgba: rgba.to_vec(),
            quad_extent: quad_extent(&model),
            blend,
        });
    }
    out
}

/// The blend equation a `<Mode3D>` model's own batch declares, for the sheet
/// entry its texture becomes.
///
/// **Read off the file, not tabulated per model.** Every one of these widgets
/// is a single flat quad, so "the model's blend" is well defined: whatever its
/// one transparent batch put in `pass_mask`. All three of Pulse's sight models
/// declare `0x120e`, whose `0x0200` bit is
/// [`oag_vex::vex::BlendClass::Additive`] - and their textures are named
/// `gunsight_ADD.tga`, `gunsightdot_ADD.tga` and `LeachBeamSight_ADD_nomip.tga`
/// with alpha pinned at 250/255, so the shape lives in the *colour* channels
/// over a black field and an ordinary alpha blend can only draw it on a black
/// tile. Reading it here rather than naming the three models is what makes
/// Pure's ten icon models right the day a Pure disc is present to measure.
///
/// `None` when the model's batches are all opaque, which draws exactly as
/// before. A model whose batches **disagree** is reported and treated as
/// opaque rather than having one of them picked for it: none of the widgets
/// this walks is such a model, and guessing which batch speaks for a quad that
/// is not a quad is the invention this project's asset rule forbids. See
/// [`oag_hud::sprite::Placed::blend`].
fn quad_blend(
    src: &str,
    model: &oag_mesh::mesh::Model,
    report: &mut Vec<String>,
) -> Option<oag_vex::vex::BlendClass> {
    let mut declared: Vec<oag_vex::vex::BlendClass> = Vec::new();
    for draw in model
        .draws
        .iter()
        .chain(&model.alpha_tested_draws)
        .chain(&model.transparent_draws)
    {
        if let Some(blend) = draw.blend
            && !declared.contains(&blend)
        {
            declared.push(blend);
        }
    }
    match declared.as_slice() {
        [] => None,
        [one] => Some(*one),
        several => {
            report.push(format!(
                "HUD model {src}: its batches declare {several:?}, which a single \
                 sheet quad cannot honour; drawn with the pipeline's own blend"
            ));
            None
        }
    }
}

/// A model's own flat quad, read off its vertex positions rather than
/// hand-measured: the widest span on `x` and on `y` across every vertex the
/// model carries.
///
/// `None` for a model with fewer than two vertices - not a quad at all - which
/// none of the sight or icon models are, but a reader elsewhere handing this a
/// track or a ship should not get a bogus `[0.0, 0.0]` back.
fn quad_extent(model: &oag_mesh::mesh::Model) -> Option<[f32; 2]> {
    let mut min = [f32::MAX, f32::MAX];
    let mut max = [f32::MIN, f32::MIN];
    for vertex in &model.vertices {
        for axis in 0..2 {
            min[axis] = min[axis].min(vertex.position[axis]);
            max[axis] = max[axis].max(vertex.position[axis]);
        }
    }
    if model.vertices.len() < 2 {
        return None;
    }
    Some([max[0] - min[0], max[1] - min[1]])
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
/// the filename off a `<Font>` slot rather than naming one, through the same
/// [`oag_ui::language::load::role_font`], and both go through
/// [`oag_assets::Archives::read_font`], so a PS2 source finds the glyph atlas
/// the disc keeps in the entry after the `.fnt` rather than falling back to 5x7.
/// Kept separate only so the report line says which font is being talked about.
///
/// `preferred` is the chosen language - the same one [`load_hud`] resolved
/// `strings` from - asked for `role` first; `role_font`'s own fallback across
/// every plugin only fires when the chosen one names nothing for it. **This
/// used to scan every plugin in source order with no preference at all**,
/// which is the same bug `load_hud`'s old `None` was: a HUD asking for the
/// caption face got whichever plugin happened to be first, not the one the
/// player picked.
///
/// A source whose plugins fill in no such slot draws in 5x7 and says which role
/// went unanswered - see `oag_ui::language::roles` for what each disc fills in.
///
/// Both HUD fonts are pre-outlined on both Pulse pressings - six distinct greys,
/// alpha covering glyph *plus* border - so `Atlas::from_font`'s body/outline
/// split applies unchanged there; see `docs/formats/fnt.md`. Whether Pure's own
/// `HUDFont.fnt` is outlined the same way has not been measured.
pub(super) fn hud_font(
    archives: &mut oag_assets::Archives,
    languages: &[oag_ui::language::Language],
    preferred: Option<&oag_ui::language::Language>,
    role: &str,
    report: &mut Vec<String>,
) -> oag_ui::font::Atlas {
    let Some(name) = oag_ui::language::load::role_font(languages, preferred, role) else {
        report.push(format!(
            "no language plugin names a {role:?} font on this source; drawing with 5x7"
        ));
        return oag_ui::font::Atlas::build();
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
            oag_ui::font::Atlas::from_font(&font)
        }
        Err(why) => {
            report.push(format!(
                "HUD font {name} (role {role:?}) unavailable ({why}); drawing with 5x7"
            ));
            oag_ui::font::Atlas::build()
        }
    }
}
