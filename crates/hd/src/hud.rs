//! The in-race HUD's entries: eighteen root layouts, three skins, twelve
//! textures.
//!
//! Entry names in the [ADR-0022] sense, the same division [`oag_pulse::hud`]
//! draws: *which* files HD ships and what each is. Everything that **reads**
//! them - the `<LoadXML>` splice, the widget model, the offset composition - is
//! `oag_game::hud` and stays there, because it is one dialect across three
//! titles.
//!
//! Measured on `hdfury-ps3-eu-dec.iso` (`BCES-00664`), 2026-08-17. Written up
//! with its evidence in [`hd-hud.md`].
//!
//! # HD's HUD is not one file per mode
//!
//! Pulse ships five self-contained layouts. HD ships a **shell** per mode that
//! pulls in a dozen fragments by `<LoadXML SrcRel=...>`, so a mode's HUD is 5 to
//! 17 files rather than one, and reading only the root gets two empty
//! rectangles. `oag_game::hud::compose` is what assembles one - this crate
//! cannot link to it, being the wrong side of the dependency arrow.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [`hd-hud.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/hd-hud.md
//! [`oag_pulse::hud`]: https://github.com/topaxi/OpenAntiGrav/blob/main/crates/pulse/src/hud.rs

/// The three HUD skins, as directory prefixes under `/data/xml/`.
///
/// `wo3` and `2097` are Wipeout 3 and Wipeout 2097 - the retro HUD skins the
/// game offers as an option - with the bare directory as the default. The same
/// three-way split appears in the fonts: `docs/formats/hd-frontend.md` records
/// `skin.xml` declaring `HUD`, `wo3HUD` and `2097HUD` font slots.
///
/// **How a skin is chosen is not read here.** The executable's race-manager
/// constructors pass whole paths as literals, so the choice is made by which
/// call runs rather than by composing a directory at runtime - see
/// `docs/ghidra/functions/ps3-hdfury-eu/race-hud.md`.
pub mod skins {
    /// No prefix: the default, modern HUD.
    pub const DEFAULT: &str = "";
    /// Wipeout 3's skin.
    pub const WO3: &str = "wo3_hud/";
    /// Wipeout 2097's skin.
    pub const RETRO_2097: &str = "2097_hud/";
}

/// The eighteen root layouts, by mode and skin.
///
/// **Read off the manifests rather than off the executable**, and the two do not
/// agree: `.rodata` carries 31 `*_HUD.xml` paths, which include the split-screen
/// and vertical-split variants this list leaves out. What is here is every root
/// that ships as `/data/xml/[<skin>/]<mode>_hud.xml`.
///
/// Two absences are the shape of the thing rather than gaps in the reading:
/// **Detonator, Duel and MPTag have no skinned variant at all**, and Duel's root
/// is one directory further down.
pub mod layouts {
    /// Single race and tournament.
    pub const ARCADE: &str = "/data/xml/arcade_hud.xml";
    /// Eliminator.
    pub const ELIMINATION: &str = "/data/xml/elimination_hud.xml";
    /// Time trial.
    pub const TIME_TRIAL: &str = "/data/xml/timetrial_hud.xml";
    /// Speed lap.
    pub const SPEED_LAP: &str = "/data/xml/speedlap_hud.xml";
    /// Zone and Zone Battle.
    pub const ZONE: &str = "/data/xml/zone_hud.xml";
    /// Detonator, the Fury mode. **No skinned variant.**
    pub const DETONATOR: &str = "/data/xml/detonator_hud.xml";
    /// Multiplayer tag. **No skinned variant.**
    pub const MP_TAG: &str = "/data/xml/mptag_hud.xml";
    /// Duel, the Fury head-to-head mode. **No skinned variant**, and the only
    /// root that is not directly under `/data/xml/`.
    pub const DUEL: &str = "/data/xml/duel_hud/duel_hud.xml";
}

/// Every root layout the disc ships, default skin first.
///
/// Eighteen: the eight in [`layouts`], plus five each for [`skins::WO3`] and
/// [`skins::RETRO_2097`]. `crates/game/tests/hd_hud_ground_truth.rs` re-derives
/// this from the manifests rather than trusting it.
///
/// **Single-screen only.** The disc also ships a split-screen family -
/// `splitscreen_hud/`, `splitscreenzone_hud/`, `duel_hud/duel_splitscreen_hud/`
/// and a `vert_` variant of most of it - which is not enumerated here because
/// nothing in this project draws two viewports. The one that would slip past a
/// looser rule is `/data/xml/splitscreenzone_hud/zone_hud.xml`: the same
/// basename as [`layouts::ZONE`], one directory across.
pub const ROOTS: &[&str] = &[
    layouts::ARCADE,
    layouts::ELIMINATION,
    layouts::TIME_TRIAL,
    layouts::SPEED_LAP,
    layouts::ZONE,
    layouts::DETONATOR,
    layouts::MP_TAG,
    layouts::DUEL,
    "/data/xml/wo3_hud/arcade_hud.xml",
    "/data/xml/wo3_hud/elimination_hud.xml",
    "/data/xml/wo3_hud/timetrial_hud.xml",
    "/data/xml/wo3_hud/speedlap_hud.xml",
    "/data/xml/wo3_hud/zone_hud.xml",
    "/data/xml/2097_hud/arcade_hud.xml",
    "/data/xml/2097_hud/elimination_hud.xml",
    "/data/xml/2097_hud/timetrial_hud.xml",
    "/data/xml/2097_hud/speedlap_hud.xml",
    "/data/xml/2097_hud/zone_hud.xml",
];

/// The archive entry a layout's `src=` reference names.
///
/// # The layouts name source art, and the disc ships the conversion
///
/// A HUD sprite's `src` is the **exporter's input**, not the shipped file:
/// across the eighteen composed layouts the twelve distinct references spell
/// themselves `.gtf` ten times, `.mip` once and `.tga` once, and the two odd
/// ones are the two with the most uses between them. `Data\HUD\Textures\
/// hdHUD.mip` has **148** references and there is **no `.mip` anywhere on this
/// disc**; `Data\HUD\Textures\detonator_hud2.tga` has 44 and the disc's only
/// `.tga` is a smoke ramp in `data/ribboneffects`. Both resolve when the
/// extension is replaced with `.gtf`: `/data/hud/textures/hdhud.gtf` is in
/// `DATA02` and `DATA03`, `/data/hud/textures/detonator_hud2.gtf` in `DATA00`.
///
/// So the rule is **replace the extension**, and with it all twelve references
/// resolve to a shipped entry rather than ten of twelve. Read as literal names
/// they are dangling references and 192 of the HUD's 1,029 sprites have no
/// texture - which is exactly how it looked before this was measured.
///
/// # Confidence 85
///
/// Twelve of twelve resolve, and the two that need the rule need it for 192
/// sprites, so this is not a coincidence of one file. It is 85 rather than
/// higher because **no code path in the executable has been read** for it: the
/// rule is inferred from the file set agreeing with it, and an engine that
/// instead tries the literal name, fails, and falls back would be
/// indistinguishable from here.
///
/// A second spelling exists on this disc and is **not** what the HUD uses:
/// `/data/environments/02_track/hd_textures/and_thinsteps.tga.gtf` *appends*
/// rather than replaces. Nothing in the HUD reaches a name of that shape.
///
/// Separators and case are left alone - [`oag_assets::psarc::Archive::read_path`]
/// folds both.
#[must_use]
pub fn texture_entry(reference: &str) -> String {
    let cut = reference.rfind('.');
    let stem = match cut {
        // Only a *file* extension: a dot in a directory name is not one.
        Some(at) if !reference[at..].contains(['/', '\\']) => &reference[..at],
        _ => reference,
    };
    format!("{stem}.gtf")
}

/// The twelve textures the eighteen composed layouts sample, as the layouts
/// spell them.
///
/// Kept as the **reference** spelling rather than the resolved one, because the
/// reference is what is authored and [`texture_entry`] is the claim about it -
/// storing the resolved name would hide the rule inside a constant. Counts are
/// sprites across all eighteen layouts, so a texture shared by three skins is
/// counted three times.
///
/// | Uses | Reference |
/// | ---: | --- |
/// | 338 | `HUD_Components.gtf` |
/// | 148 | `hdHUD.mip` |
/// | 105 | `fury_hud.gtf` |
/// | 104 | `wo3_hud.gtf` |
/// | 97 | `wo2097_hud.gtf` |
/// | 63 | `HUD_Components_01.gtf` |
/// | 56 | `voiceCom.gtf` |
/// | 44 | `detonator_hud2.tga` |
/// | 36 | `missile_reticule.gtf` |
/// | 24 | `HUD_Components_02.gtf` |
/// | 12 | `nitro_hud.gtf` |
/// | 2 | `ZoneDamage.gtf` |
///
/// **One of them is not a HUD texture at all**: `Data\FE\Images\voiceCom.gtf` is
/// a front-end image, reached from the position-list fragment for the
/// voice-chat indicator. So a HUD's texture set is not confined to
/// `data/hud/textures/`, which a loader that assumed a directory would get
/// wrong for 56 sprites.
pub const TEXTURES: &[&str] = &[
    r"Data\HUD\Textures\HUD_Components.gtf",
    r"Data\HUD\Textures\HUD_Components_01.gtf",
    r"Data\HUD\Textures\HUD_Components_02.gtf",
    r"Data\HUD\Textures\ZoneDamage.gtf",
    r"Data\HUD\Textures\detonator_hud2.tga",
    r"Data\HUD\Textures\fury_hud.gtf",
    r"Data\HUD\Textures\hdHUD.mip",
    r"Data\HUD\Textures\missile_reticule.gtf",
    r"Data\HUD\Textures\nitro_hud.gtf",
    r"Data\FE\Images\voiceCom.gtf",
    r"Data\XML\wo3_hud\texture\wo3_hud.gtf",
    r"Data\XML\2097_hud\texture\wo2097_hud.gtf",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_source_art_references_resolve_to_a_gtf() {
        assert_eq!(
            texture_entry(r"Data\HUD\Textures\hdHUD.mip"),
            r"Data\HUD\Textures\hdHUD.gtf"
        );
        assert_eq!(
            texture_entry(r"Data\HUD\Textures\detonator_hud2.tga"),
            r"Data\HUD\Textures\detonator_hud2.gtf"
        );
    }

    #[test]
    fn a_reference_that_is_already_a_gtf_is_unchanged() {
        for reference in TEXTURES.iter().filter(|r| r.ends_with(".gtf")) {
            assert_eq!(&texture_entry(reference), reference);
        }
    }

    /// The **append** spelling exists elsewhere on this disc, so replacing must
    /// not turn one of those back into its source name.
    #[test]
    fn the_appended_spelling_survives_the_rule() {
        assert_eq!(
            texture_entry("/data/environments/02_track/hd_textures/and_thinsteps.tga.gtf"),
            "/data/environments/02_track/hd_textures/and_thinsteps.tga.gtf"
        );
    }

    /// A dot in a directory name is not a file extension, and appending `.gtf`
    /// to the whole path is the only right answer for an extensionless name.
    #[test]
    fn a_dot_in_a_directory_is_not_an_extension() {
        assert_eq!(texture_entry(r"Data\v1.2\atlas"), r"Data\v1.2\atlas.gtf");
        assert_eq!(texture_entry("data/v1.2/atlas"), "data/v1.2/atlas.gtf");
    }

    #[test]
    fn every_root_is_named_once() {
        let mut seen: Vec<&str> = ROOTS.to_vec();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), before);
        assert_eq!(before, 18);
    }
}
