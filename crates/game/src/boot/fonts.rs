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
use oag_ui::language::load::{role_border, role_font};

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
    preferred: Option<&Language>,
    report: &mut Vec<String>,
) -> oag_ui::font::Atlas {
    let role = oag_ui::language::roles::DEFAULT;
    let Some(name) = role_font(languages, preferred, role) else {
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
            let atlas = oag_ui::font::Atlas::from_font(&font)
                .with_border_extend(role_border(languages, preferred, role));
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
    preferred: Option<&Language>,
    skin: &oag_title::MenuSkin,
    report: &mut Vec<String>,
) -> Option<oag_ui::font::Atlas> {
    let role = skin.menu_font?;
    let name = role_font(languages, preferred, role)?;
    match archives.read_font(&name) {
        Ok(font) => {
            report.push(format!(
                "menu font {name} (role {role:?}): line height {}",
                font.line_height
            ));
            Some(
                oag_ui::font::Atlas::from_font(&font)
                    .with_border_extend(role_border(languages, preferred, role)),
            )
        }
        Err(why) => {
            report.push(format!(
                "menu font {name} (role {role:?}) unavailable ({why}); menus draw in the default face"
            ));
            None
        }
    }
}

/// The menu, title and button-glyph faces in one call, each read at
/// `texel_scale` (see `oag_display::space::Space::font_texel_scale`).
///
/// Three loads that were three statements in `boot::load`, each repeating the
/// same `.map(|atlas| atlas.with_texel_scale(..))`; the scale is applied here
/// once so a fourth face cannot forget it.
pub(super) fn load_role_fonts(
    archives: &mut oag_assets::Archives,
    languages: &[Language],
    preferred: Option<&Language>,
    skin: &oag_title::MenuSkin,
    texel_scale: f32,
    prompts: &oag_title::prompts::Prompts,
    report: &mut Vec<String>,
) -> RoleFonts {
    let scaled = |atlas: Option<oag_ui::font::Atlas>| {
        atlas.map(|a| a.with_prompts(prompts).with_texel_scale(texel_scale))
    };
    (
        scaled(load_menu_font(archives, languages, preferred, skin, report)),
        scaled(load_title_font(
            archives, languages, preferred, skin, report,
        )),
        scaled(load_buttons_font(archives, languages, preferred, report)),
    )
}

/// What [`load_role_fonts`] returns: menu, title, buttons.
pub(super) type RoleFonts = (
    Option<oag_ui::font::Atlas>,
    Option<oag_ui::font::Atlas>,
    Option<oag_ui::font::Atlas>,
);

/// Reads the face the screen title draws in, when this title names one.
///
/// The mirror of [`load_menu_font`], one widget over: the role comes from
/// [`oag_title::MenuSkin::title_font`] rather than `menu_font`, and the file
/// it resolves to still comes from the language plugin's own `<Font>` slots.
/// `None` for a title whose chrome names no role of its own - Pulse, on both
/// its PSP pressings and the PS2 port, deliberately, see `title_font`'s own
/// doc - a plugin with no such slot, or an unreadable `.fnt`; the title then
/// draws in whichever face the frame is already bound to, which is this
/// build's original behaviour and not a fallback invented here.
pub(super) fn load_title_font(
    archives: &mut oag_assets::Archives,
    languages: &[Language],
    preferred: Option<&Language>,
    skin: &oag_title::MenuSkin,
    report: &mut Vec<String>,
) -> Option<oag_ui::font::Atlas> {
    let role = skin.title_font?;
    let name = role_font(languages, preferred, role)?;
    match archives.read_font(&name) {
        Ok(font) => {
            report.push(format!(
                "title font {name} (role {role:?}): line height {}",
                font.line_height
            ));
            Some(
                oag_ui::font::Atlas::from_font(&font)
                    .with_border_extend(role_border(languages, preferred, role)),
            )
        }
        Err(why) => {
            report.push(format!(
                "title font {name} (role {role:?}) unavailable ({why}); the title draws in the frame's own face"
            ));
            None
        }
    }
}

/// The atlas and role name [`crate::render::Renderer::set_face_atlas`]
/// should load into the menu stage's secondary slot, given this title's
/// [`oag_title::MenuSkin`] and its own `Default`-role atlas and
/// already-resolved `Title`-role one.
///
/// **A title whose chrome names a `Title` role (Wipeout HD, Pure) keeps
/// exactly what it always loaded there** - `title_font` unchanged,
/// `Some("Title")` (or whichever string the skin names). **Every other
/// title - Pulse on both PSP pressings and the PS2 port, 2048, Omega - gets
/// its own `Default`-role atlas in the slot instead.** On Pulse that atlas
/// is the one Pulse face with real lowercase glyph art: `Pulse_20.fnt` (the
/// `menu` role primary this never touches) and `Pulse_14.fnt`
/// (`Small`/`Title`) both give every lowercase codepoint the *identical*
/// `(u0, v0, width, height)` box as its uppercase twin - measured off both
/// pressings, not a code fallback - while `pulse_text.fnt` (`Default`) does
/// not. See `docs/ui/menus-original.md`'s "Two faces, not one swapped for
/// the other" section.
///
/// **A no-op everywhere but Pulse today.** 2048 and Omega both use Wipeout
/// HD's own `Title`-role chrome (`oag_hd::frontend::MENU_SKIN` carried
/// forward), so they never reach the `None` arm at all; Pure now joins them
/// (`oag_pure::frontend::MENU_SKIN::title_font` is `Some("Title")`, 2026-09-25),
/// and its `Title` role happens to resolve to the same `.fnt` its `Default`
/// role already does, so the slot's *contents* do not change for it either -
/// only Pulse's `None` arm still swaps in a genuinely different atlas.
/// Pure's own campaign-free menus never construct a `Draw::FacedText { role:
/// "Default", .. }`, so relabelling its slot `"Title"` collides with
/// nothing. Loading the atlas anyway costs one clone of an already-decoded
/// `Atlas` and is not gated on a title, on purpose - a future title's own
/// `Default`-labelled draw finds the slot already filled rather than needing
/// this function taught about it.
///
/// The `Title`/`Default` pair is mutually exclusive on every title measured
/// so far (`oag_pulse::frontend::MENU_SKIN::title_font` is the only `None`
/// left; `oag_hd`'s and `oag_pure`'s are both `Some("Title")`, and 2048/Omega
/// carry HD's) - which is what lets one slot serve either without a title
/// ever needing both in the same frame. A title that grows a second,
/// genuinely simultaneous role will need a second slot, not a change here.
#[must_use]
pub fn face_atlas_slot(
    skin: &oag_title::MenuSkin,
    font: &oag_ui::font::Atlas,
    title_font: Option<oag_ui::font::Atlas>,
) -> (Option<oag_ui::font::Atlas>, Option<&'static str>) {
    match (skin.title_font, skin.body_font) {
        // A title that draws its screen title in one role and everything
        // else in another (Pulse's PSP pressings) keeps the body face here
        // and sends the title to [`third_atlas_slot`].
        (Some(_), Some(body)) => (Some(font.clone()), Some(body)),
        (Some(role), None) => (title_font, Some(role)),
        (None, _) => (Some(font.clone()), Some(oag_ui::language::roles::DEFAULT)),
    }
}

/// What the renderer's third glyph slot carries: the screen title's face on a
/// title whose body face has the second slot ([`oag_title::MenuSkin::body_font`]),
/// the PlayStation button glyphs on every other.
///
/// The two never compete: only HD and Omega declare a `Buttons` role, and
/// neither names a `body_font`. One slot, one role at a time, rather than a
/// fourth texture in the shader.
#[must_use]
pub fn third_atlas_slot(
    skin: &oag_title::MenuSkin,
    title_font: Option<oag_ui::font::Atlas>,
    buttons_font: Option<oag_ui::font::Atlas>,
) -> (Option<oag_ui::font::Atlas>, &'static str) {
    match (skin.title_font, skin.body_font) {
        (Some(role), Some(_)) => (title_font, role),
        _ => (buttons_font, oag_ui::language::roles::BUTTONS),
    }
}

/// Loads both glyph slots into `renderer` for `skin`: the one place the live
/// session, a language switch and a headless capture agree on which role
/// goes where.
pub fn install_faces(
    renderer: &mut crate::render::Renderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    skin: &oag_title::MenuSkin,
    font: &oag_ui::font::Atlas,
    title_font: Option<oag_ui::font::Atlas>,
    buttons_font: Option<oag_ui::font::Atlas>,
) {
    let (face, face_role) = face_atlas_slot(skin, font, title_font.clone());
    renderer.set_face_atlas(device, queue, face, face_role);
    let (third, third_role) = third_atlas_slot(skin, title_font, buttons_font);
    renderer.set_buttons_atlas(device, queue, third, third_role);
}

/// Reads the PlayStation button-glyph face
/// ([`oag_ui::language::roles::BUTTONS`]), when this source's language
/// plugins name one.
///
/// Mirrors [`load_menu_font`]/[`load_title_font`]'s shape one role over -
/// `None` whenever any link in the chain is missing (no plugin fills the
/// `Buttons` slot, or the named `.fnt` does not read), the same honest
/// absence the caller then draws nothing for
/// (`oag_ui_screens::campaign::footer::face_role`'s own doc names why a wrong
/// fallback would be worse). **Not gated by [`oag_title::MenuSkin`]** the
/// way [`load_menu_font`]/[`load_title_font`] are: unlike `Title`/`menu`,
/// nothing about which screen wants this role varies by title package, and
/// a title that never authors a `font="buttons"` widget simply never emits
/// a `Draw::FacedText` asking for it, so loading the face whenever the disc
/// names one costs nothing extra to gate.
pub(super) fn load_buttons_font(
    archives: &mut oag_assets::Archives,
    languages: &[Language],
    preferred: Option<&Language>,
    report: &mut Vec<String>,
) -> Option<oag_ui::font::Atlas> {
    let role = oag_ui::language::roles::BUTTONS;
    let name = role_font(languages, preferred, role)?;
    match archives.read_font(&name) {
        Ok(font) => {
            report.push(format!(
                "buttons font {name} (role {role:?}): {}x{} atlas, {} glyphs",
                font.width,
                font.height,
                font.glyphs.len()
            ));
            Some(
                oag_ui::font::Atlas::from_font(&font)
                    .with_border_extend(role_border(languages, preferred, role)),
            )
        }
        Err(why) => {
            report.push(format!(
                "buttons font {name} (role {role:?}) unavailable ({why}); \
                 button glyphs draw as nothing rather than a wrong codepoint"
            ));
            None
        }
    }
}

/// Every font role the chosen language declares, with its face's line height
/// as a ratio of the `Default` role's - what [`oag_ui::frontend::Frontend::set_face_scales`]
/// takes.
///
/// Read off each `.fnt`'s own header, so the number is the disc's; a face
/// that will not read is reported and left at `1.0`. The `Default` role
/// itself is the denominator and comes out as `1.0` by construction.
pub(super) fn face_scales(
    archives: &mut oag_assets::Archives,
    language: Option<&Language>,
    default_line_height: f32,
    report: &mut Vec<String>,
) -> Vec<(String, f32)> {
    let Some(language) = language else {
        return Vec::new();
    };
    if default_line_height <= 0.0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (role, name) in &language.fonts {
        if role.eq_ignore_ascii_case(oag_ui::language::roles::DEFAULT) {
            continue;
        }
        match archives.read_font(name) {
            Ok(font) => {
                let scale = font.line_height as f32 / default_line_height;
                report.push(format!(
                    "font role {role:?} is {name}, line height {} - {scale:.3} of Default",
                    font.line_height
                ));
                out.push((role.clone(), scale));
            }
            Err(why) => report.push(format!(
                "font role {role:?} is {name}, which did not read ({why}); drawn at Default's size"
            )),
        }
    }
    out
}
