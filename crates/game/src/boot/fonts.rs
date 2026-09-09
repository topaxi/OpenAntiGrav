//! Which face a source's front end draws in, and what happens when it has none.
//!
//! Split out of `boot.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. What holds
//! the three together is one question asked twice - **the language plugin's
//! `<Font>` slots name every face, and a role that resolves to nothing is a
//! finding rather than a cue to reach for a constant.**
//!
//! Pure is the case that makes it more than a principle, and it is a **historical**
//! one now: `load_font` used to name `Data\FE\Fonts\pulse_text.fnt` outright, which
//! is absent from Pure's `Data.wad` and drew its whole front end in the built-in 5x7
//! glyphs. Since the role is resolved through the plugin's own `<Font>` slots
//! (below), a Pure boot now loads `Data\FE\Fonts\FX300ANG.fnt` for the same
//! `Default` role - confirmed by `--dry-run`'s own report line
//! (`font Data\FE\Fonts\FX300ANG.fnt (role "Default"): 256x256 atlas, 201 glyphs,
//! line height 15`) and by `font_roles_ground_truth::every_disc_names_a_body_face_and_it_loads`.
//! A role that still resolves to nothing on some future source is reported rather
//! than papered over - a silent fallback would make a rendering bug
//! indistinguishable from a loading one.
//!
//! # HD was in that state too, and it was this module's own reading that was wrong
//!
//! This comment used to say HD "ships **four `.fnt` files and all four are
//! Asian**" and had no Latin face for either role. It ships 33, and `DATA02`
//! alone holds 17 of them - `helv`, `helvb`, `pulsehud`, `small`,
//! `ps_buttons`, `arialbd`, `ariblk`, `russianhud` and the Asian ones together.
//! The four-file count was `DATA04`, `DATA05` and `DATA06`, which really do
//! carry exactly `chinese`, `chinesemx`, `korean` and `koreanbold` and nothing
//! else: a survey that was right about the archives it looked at and wrong
//! about the disc, which is the same shape as the `.gitignore` trap
//! [`CLAUDE.md`](../../../../CLAUDE.md) already warns about - a scoped result
//! that reads exactly like a whole-disc one.
//!
//! What actually kept HD in 5x7 was that
//! [`oag_texture::fnt`] read the header little-endian, so all 33 failed the
//! magic check with `not a .fnt`. The report said so accurately the whole time;
//! nothing here had to change for the fonts to appear, only the parser.

use super::*;

/// Reads the front end's own font, falling back to the built-in glyphs.
///
/// A missing or undecodable font is not fatal: the menu still draws, in the 5x7
/// approximation, and the report says which one is on screen. That matters more
/// than it sounds - the two look very different, and a silent fallback would
/// make a rendering bug indistinguishable from a loading one. The PS2 spent a
/// while in exactly that state: its `.fnt` decodes as far as the metrics and
/// then stops, because the glyph sheet is a separate archive entry.
///
/// Which is why this goes through [`oag_assets::Archives::read_font`]
/// rather than [`read_front_end_first`] plus a parse: locating the atlas is the
/// archive's problem, not the front end's. The two archives hold byte-identical
/// copies of all five fonts on the PSP, checked, so reading the bulk one costs
/// nothing there.
/// **Which file that is comes from the disc**, through the same `<Font>` slots
/// [`load_menu_font`] already resolved its own role through. This used to name
/// `Data\FE\Fonts\pulse_text.fnt` outright, which is what Pulse's own `PI008`
/// resolves `Default` to - correct there, and the reason a Pure boot drew its
/// entire front end in 5x7: Pure resolves the same role to `FX300ANG.fnt` and
/// shares no font filename with Pulse at all. Asking the plugin is not a new
/// mechanism for a second title, it is the mechanism that was already there
/// being asked one role earlier. See [`oag_ui::language::roles`].
pub(super) fn load_font(
    archives: &mut oag_assets::Archives,
    languages: &[Language],
    report: &mut Vec<String>,
) -> oag_ui::font::Atlas {
    let role = oag_ui::language::roles::DEFAULT;
    let Some(name) = role_font(languages, role) else {
        // Reported rather than fallen back to a remembered filename: a source
        // whose plugins name no body face is a finding about that source, and a
        // constant here would hide it behind another title's answer.
        report.push(format!(
            "no language plugin names a {role:?} font on this source; drawing with 5x7"
        ));
        return oag_ui::font::Atlas::build();
    };
    match archives.read_font(&name).map_err(|e| e.to_string()) {
        Ok(font) => {
            let atlas = oag_ui::font::Atlas::from_font(&font);
            report.push(format!(
                "font {name} (role {role:?}): {}x{} atlas, {} glyphs, line height {}",
                font.width,
                font.height,
                font.glyphs.len(),
                font.line_height
            ));
            atlas
        }
        Err(why) => {
            report.push(format!("font {name} unavailable ({why}); drawing with 5x7"));
            oag_ui::font::Atlas::build()
        }
    }
}

/// The `.fnt` a role resolves to on this source, from its language plugins.
///
/// Any language will do: the plugins differ in which glyphs a face carries, not
/// in which file a role names. `None` for a role no plugin on this source fills
/// in - which is an ordinary answer rather than a failure, because the two discs
/// fill in different subsets: Pure declares four slots to Pulse's eight.
pub(super) fn role_font(languages: &[Language], role: &str) -> Option<String> {
    languages
        .iter()
        .find_map(|language| language.font(role))
        .map(str::to_string)
}

/// Reads the face this title draws menu *rows* in, when it names one.
///
/// Separate from [`load_font`] rather than replacing it: the default face is
/// what the language picker, the loading tips and the HUD are drawn with, and
/// the picker's layout is measured against it at confidence 95. Only the menus
/// move to the bigger face.
///
/// The role comes from the title's own `MenuSkin` and the file it resolves to
/// comes from the language plugin's `<Font>` slots, so neither the role list nor
/// the filenames are hard-coded here. `None` whenever any link in that chain is
/// missing - a title whose menus name no role of their own, a plugin with no
/// such slot, an unreadable `.fnt` - and the menus then draw in the default
/// face, which is Pure's measured case as much as it is an absence of evidence.
/// See [`oag_title::MenuSkin::menu_font`].
pub(super) fn load_menu_font(
    archives: &mut oag_assets::Archives,
    languages: &[Language],
    skin: &oag_title::MenuSkin,
    report: &mut Vec<String>,
) -> Option<oag_ui::font::Atlas> {
    let role = skin.menu_font?;
    let name = role_font(languages, role)?;
    match archives.read_font(&name) {
        Ok(font) => {
            report.push(format!(
                "menu font {name} (role {role:?}): line height {}",
                font.line_height
            ));
            Some(oag_ui::font::Atlas::from_font(&font))
        }
        Err(why) => {
            report.push(format!(
                "menu font {name} (role {role:?}) unavailable ({why}); menus draw in the default face"
            ));
            None
        }
    }
}
