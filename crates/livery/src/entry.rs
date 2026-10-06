//! The archive entries a ship is built from: the hull, the boost plume, the
//! shield shell and a PS2 model's texture set.
//!
//! Moved here from `race::assets` so this crate does not reach back into the
//! race module; `race` now imports these from `oag_livery::entry`.

use oag_mesh::mesh;
use oag_pulse::race::ships;
use oag_race::Mode;

/// The archive entry name of a team's `.vex` model, under the ship directory
/// this title keeps its roster in.
///
/// `paths` is [`oag_title::RaceDefaults::ships`]; see
/// [`oag_title::RaceDefaults::ship_dir`] for the third-title disagreement its
/// directory exists for.
///
/// Assembled the way the loader assembles it, with backslashes, which is what the
/// name hash needs.
///
/// **A [`Mode::Zone`] run flies a different hull, and where that hull lives is a
/// title fact** - [`oag_title::ZoneCraft`], measured on all three titles. Pulse
/// keeps it inside the player's own team directory as `Zone.vex`; Pure and HD
/// give Zone a ship directory of its own, so the player's team stops reaching
/// the hull at all.
///
/// Pulse's is the branch with a recovered selector rather than a name probe:
/// `Ship_LoadModel` (`0x08843258`) switches on the same
/// `DAT_08ab07e3 == 0 && DAT_08b31048 == 6` expression already established as
/// the Zone selector (see `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`),
/// and only that case builds the `%s\Zone.vex` path. Every team's `Zone.vex`
/// decodes to the same 1213 vertices / 1149 triangles / 8 meshes, so the hull
/// itself is shared there - only the livery painted on it still varies by team.
/// On the other two there is no per-team Zone hull to share.
///
/// **`hull_variant` is Pulse's Normal/Concept axis, and only applies here.**
/// `oag_title::race::HullVariant::stem` names a file inside the *same*
/// directory `Ship.vex` already resolves in, so a Zone run ignores it: Zone's
/// own hull selection is a title fact of its own
/// ([`oag_title::ZoneCraft`]), unaffected by which model variant the RACE
/// page has picked, and every Pulse team's `Zone.vex` is one shared hull
/// regardless. `None` keeps today's behaviour - [`ships::HULL`] - exactly.
#[must_use]
pub fn ship_entry_name(
    paths: oag_title::race::ShipPaths,
    team: &str,
    mode: Mode,
    hull_variant: Option<&str>,
) -> String {
    if mode != Mode::Zone {
        return ships::entry_name_in(paths.dir, team, hull_variant.unwrap_or(ships::HULL));
    }
    ships::entry_name_in(
        paths.zone.root(paths.dir),
        paths.zone.directory(team),
        paths.zone.hull().unwrap_or(ships::HULL),
    )
}

/// The boost plume that goes with [`ship_entry_name`]'s hull, or `None` when
/// this title's own package names no standalone boost model at all.
///
/// **`None` is a title fact, not a lookup failure.** [`oag_title::race::ShipPaths::boost`]
/// is `Some` only where a title's own executable composes a per-team boost
/// path - Pulse - and `None` everywhere else, Pure's measured absent
/// (`docs/ghidra/functions/psp-pure-usa/ship-models.md`) included. A caller
/// that gets `None` back should say "this title has none", never "not
/// found" - see `livery::plume`, the one caller today.
///
/// **Zone mode has its own plume file and this used to load the wrong one.**
/// `ship_entry_name` switches the hull between `Ship.vex` and `Zone.vex` on
/// mode; the plume load hardcoded `shipboost.vex`, so a Zone race drew a
/// `Zone.vex` hull with the `Ship.vex` plume. Every team ships a `Zoneboost.vex`
/// beside its `Zone.vex`, so the pairing exists in the data and we simply were
/// not using it.
///
/// Cosmetic today rather than visibly broken - Feisar's `Zoneboost.vex` decodes
/// geometrically identical to its `shipboost.vex` - but "identical on the one
/// team that was checked" is not a reason to keep loading the wrong file, and
/// the caller's missing-entry path already handles a set that does not carry
/// one.
///
/// **The Zone fallback reads `paths.boost` now, not a hardcoded Pulse
/// constant.** [`oag_title::race::ZoneCraft::boost`] returns `None` for
/// [`oag_title::race::ZoneCraft::OwnShip`]/[`oag_title::race::ZoneCraft::PlayerShip`]
/// meaning "whatever a non-Zone race would have used" - which is this title's
/// own [`oag_title::race::ShipPaths::boost`], not necessarily Pulse's
/// `shipboost`. A title with a dedicated Zone ship directory and no ordinary
/// boost model either - Pure - now correctly resolves to `None` in Zone mode
/// too, instead of composing a `shipboost.vex` path under a ship directory
/// that never carried one.
#[must_use]
pub fn boost_entry_name(
    paths: oag_title::race::ShipPaths,
    team: &str,
    mode: Mode,
) -> Option<String> {
    if mode != Mode::Zone {
        return Some(ships::entry_name_in(paths.dir, team, paths.boost?));
    }
    let stem = paths.zone.boost().or(paths.boost)?;
    Some(ships::entry_name_in(
        paths.zone.root(paths.dir),
        paths.zone.directory(team),
        stem,
    ))
}

/// The shield shells a craft can draw, best first.
///
/// **Two names rather than one, because the two discs disagree and neither is a
/// guess.** Pulse authors a per-team `Data\Ships\<Team>\shipshield.vex` and its
/// `ShipShield_Construct` (`0x0885db38`) assembles exactly that; Pure ships no
/// per-team shell at all and carries only `Data\Weapons\shield.vex`, which is
/// byte-identical in tree and texture. Both were checked by hashing the names
/// against each disc's own `Data.wad` directory. So the first entry is the
/// recovered Pulse path and the second is what a source without it falls back
/// to.
///
/// **The fallback is a title gap, not a rendering choice**, and the caller says
/// so in its report: Pure's own loader has not been read, so using the shared
/// model there is this project's reading of which model Pure must mean, not a
/// recovered one. Recording it that way is what keeps the pressure on to read
/// Pure's executable rather than letting a working picture close the question.
///
/// **The prefix is a per-build fact.** The PSP build assembles `shipshield`
/// from the `FE_TeamModel` config key; the PS2 build passes the literal
/// `extra`, so its shell is `extrashield.vex` - a different model with a
/// different texture (`pulse_shield_extra_ADD`, not `shipshield.vex`'s
/// `grid_GLOW` lattice). See [`ships::PS2_SHIELD`].
///
/// **`team_model` is the player's own hull stem, and it reaches every craft's
/// shell.** `ShipShield_Construct` reads the `FE_TeamModel` registry value
/// itself - the same value `Ship_LoadModel` builds the player's `%s\%s.vex`
/// hull from, `extra` for the Concept model - and formats it into `%s\%sshield.vex`
/// with the *constructed craft's* team directory, so a Concept player raises
/// `extrashield.vex` on the PSP, and so does every opponent beside them. `None`
/// (or the baseline `Ship`) is the literal default, `shipshield`. See
/// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`, "The PSP's shell
/// follows the Concept model".
///
/// **No mode switch**, unlike [`boost_entry_name`]: no `Zoneshield.vex` exists
/// on either disc, and the original's own format string takes its prefix from a
/// config key rather than from the game mode. See
/// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
#[must_use]
pub fn shield_entry_names(
    ship_dir: &str,
    team: &str,
    platform: oag_assets::Platform,
    team_model: Option<&str>,
) -> [String; 2] {
    let stem = if platform == oag_assets::Platform::Ps2 {
        ships::PS2_SHIELD.to_string()
    } else {
        match team_model {
            Some(model) if !model.eq_ignore_ascii_case(ships::HULL) => format!("{model}shield"),
            _ => ships::SHIELD.to_string(),
        }
    };
    [
        ships::entry_name_in(ship_dir, team, &stem),
        oag_pulse::race::SHARED_SHIELD.to_string(),
    ]
}

/// Reads and decodes a model's external PS2 texture set, from the archive
/// entry directly before it.
///
/// **Only attempted when the model's own embedded textures are all
/// missing** - a PSP model already has them and this never runs for one; a
/// PS2 model's texture block is empty by design and this is what replaces
/// it. That gate matters beyond efficiency: the "entry before this one" rule
/// is a directory-position heuristic, not a name or a checked format tag, so
/// it must never have the chance to overwrite a model that already decoded
/// correctly on its own.
///
/// See [`oag_assets::Archives::read_preceding`] for the rule itself and the
/// evidence behind it.
pub fn ps2_texture_set(
    archives: &mut oag_assets::Archives,
    entry_name: &str,
) -> Option<mesh::Ps2TextureSet> {
    let blob = archives.read_preceding(entry_name).ok()?;
    mesh::Ps2TextureSet::parse(&blob).ok()
}
