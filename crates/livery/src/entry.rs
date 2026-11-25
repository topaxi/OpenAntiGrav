//! The archive entries a ship is built from: the hull, the boost plume, the
//! shield shell and a PS2 model's texture set.
//!
//! Moved here from `race::assets` so this crate does not reach back into the
//! race module; `race` now imports these from `oag_livery::entry`.

use oag_mesh::mesh;
use oag_pulse::race::ships;
use oag_race::Mode;

/// The craft of its own a race in `mode` flies: Zone's, or Detonator's where the title ships
/// one ([`oag_title::race::ShipPaths::own_craft`]). `None` for a mode that flies the team's
/// ordinary hull.
#[must_use]
pub fn own_craft(
    paths: oag_title::race::ShipPaths,
    mode: Mode,
) -> Option<oag_title::race::ZoneCraft> {
    paths.own_craft(mode == Mode::Zone, mode == Mode::Detonator)
}

/// What a report calls the own craft of `mode`'s race: `"Detonator"`, else `"Zone"`.
#[must_use]
pub fn craft_label(mode: Mode) -> &'static str {
    if mode == Mode::Detonator {
        "Detonator"
    } else {
        "Zone"
    }
}

/// The craft whose directory holds the engine light: Zone's for every mode on HD's Zone mask,
/// Detonator included (`Ship_LoadEngineLightData` reads `Data\Ships\Zone\EngineLightData.xml`
/// for modes 6, 13, 14 and 21; `docs/ghidra/functions/ps3-hdfury-eu/detonator-hull.md`, "Engine
/// lights: Zone's", 80). `None` for every other mode.
#[must_use]
pub fn light_craft(
    paths: oag_title::race::ShipPaths,
    mode: Mode,
) -> Option<oag_title::race::ZoneCraft> {
    mode.on_zone_mask().then_some(paths.zone)
}

/// The ship tree and directory a craft's *other* model files (the engine
/// flare) are read from: the same place `oag_title::race::ShipPaths::hull_entry_name` puts the hull.
///
/// A Zone race on a title whose Zone craft has a directory of its own (HD:
/// `/data/ships/zone`) flies that hull, so its flare's `EF_Main` origin and
/// shapes are that directory's `engineflare.vex`, authored about the Zone hull,
/// not the player's team file authored about the team's. A Detonator race does
/// the same with its own directory ([`own_craft`]). Any other mode, and
/// a title whose Zone keeps the player's team, answers `(paths.dir, team)`.
#[must_use]
pub fn model_directory(paths: oag_title::race::ShipPaths, team: &str, mode: Mode) -> (&str, &str) {
    directory_of(paths, team, own_craft(paths, mode))
}

fn directory_of(
    paths: oag_title::race::ShipPaths,
    team: &str,
    craft: Option<oag_title::race::ZoneCraft>,
) -> (&str, &str) {
    match craft {
        Some(craft) => (craft.root(paths.dir), craft.directory(team)),
        None => (paths.dir, team),
    }
}

/// The per-team shield shell's stem, which the Zone directory probe asks for.
pub const SHIELD_STEM: &str = ships::SHIELD;

/// `dir\team\stem.vex`, the entry a model stem names under a ship directory.
#[must_use]
pub fn stem_entry_name(dir: &str, team: &str, stem: &str) -> String {
    oag_title::race::entry_name_in(dir, team, stem)
}

/// The directory of `craft` ([`own_craft`] or [`light_craft`]), unless it lacks `file`: then
/// the team's own, and a line in `report` says so. `file` maps a `(directory, team)` pair
/// to the entry name the caller is about to read.
///
/// A Zone craft's engine light, shield shell and absorb shell are authored about
/// its hull like the flare is, so they come from the same place; a title whose
/// craft directory ships none of one keeps the team's, never nothing.
pub fn model_directory_with<'a>(
    archives: &mut oag_assets::Archives,
    paths: oag_title::race::ShipPaths,
    team: &'a str,
    craft: Option<oag_title::race::ZoneCraft>,
    file: impl Fn(&str, &str) -> String,
    report: &mut Vec<String>,
) -> (&'a str, &'a str) {
    let (dir, owner) = directory_of(paths, team, craft);
    if craft.is_some() && (dir, owner) != (paths.dir, team) {
        let own = file(dir, owner);
        if archives.read_name(&own).is_err() {
            report.push(format!(
                "{own}: the craft's directory ships none - the team's own file is read instead"
            ));
            return (paths.dir, team);
        }
    }
    (dir, owner)
}

/// The boost plume that goes with `oag_title::race::ShipPaths::hull_entry_name`'s hull, or `None` when
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
/// `oag_title::race::ShipPaths::hull_entry_name` switches the hull between `Ship.vex` and `Zone.vex` on
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
    let Some(craft) = own_craft(paths, mode) else {
        return Some(oag_title::race::entry_name_in(
            paths.dir,
            team,
            paths.boost?,
        ));
    };
    let stem = craft.boost().or(paths.boost)?;
    Some(oag_title::race::entry_name_in(
        craft.root(paths.dir),
        craft.directory(team),
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
        oag_title::race::entry_name_in(ship_dir, team, &stem),
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

/// A PS3 `.vex` as a model: its scene, the `.rcsmodel` beside it, and every material and `.gtf`
/// read through `archives`, the way a race reads them. The `.gtf` requests come back in
/// texture-slot order, which is how a draw names its texture; `report` gets what the build and
/// the texture reads say.
///
/// `build_scene` and not `build`: the viewer and the race both take the scene path, and a flyer
/// is all first-pass (every mesh node addresses a chunk), so the two give the same model; the
/// scene path is the one the rest of this project's HD ground truth is pinned against.
///
/// # Errors
///
/// The `.vex` names no `.rcsmodel`, the sibling is not in the archives, or the scene will not
/// decode.
pub fn ps3_scene_model(
    archives: &mut oag_assets::Archives,
    entry: &str,
    blob: &[u8],
    livery: Option<crate::LiveryFolder<'_>>,
    report: &mut Vec<String>,
) -> anyhow::Result<(mesh::Model, Vec<String>)> {
    let mut textures = Vec::new();
    let model = scene_model_requesting(
        archives,
        entry,
        blob,
        livery,
        report,
        Reading::default(),
        &mut |path| {
            if oag_texture::decode::Container::of_name(path)
                == Some(oag_texture::decode::Container::Gtf)
            {
                textures.push(path.to_string());
            }
        },
    )?;
    Ok((model, textures))
}

/// How [`scene_model_requesting`] builds.
#[derive(Clone, Copy, Debug)]
pub struct Reading {
    /// The water law a 2048 or Omega circuit is built under: it decides which of a sea's
    /// textures are asked for (a race builds under the player's, `Original` by default).
    pub water: oag_display::display::Water,
    /// Whether a texture file's bytes are handed to the build. Without them the build still
    /// asks for each file it would use, and decodes none: the model is untextured and
    /// only the requests are of use.
    pub pictures: bool,
}

impl Default for Reading {
    /// What [`ps3_scene_model`] reads: pictures, and no water.
    fn default() -> Self {
        Self {
            water: oag_display::display::Water::Off,
            pictures: true,
        }
    }
}

/// [`ps3_scene_model`]'s build, handing `requested` every texture path the build asks the
/// archives for (any container: `.gtf`, `.gxt`, `.gnf`), in request order and repeats included.
/// What a race's circuit draws is exactly these files, so a caller that wants a circuit's
/// pictures reads them from here.
///
/// The `.rcsmodel` is the first of the plain and the Omega `.final` spellings the archives hold
/// (`mesh::rcs::siblings::Siblings::probe`); 2048's and Omega's container needs no `.vex`
/// (`oag_mesh::mesh::rcs::psp2`).
///
/// # Errors
///
/// As [`ps3_scene_model`].
pub fn scene_model_requesting(
    archives: &mut oag_assets::Archives,
    entry: &str,
    blob: &[u8],
    livery: Option<crate::LiveryFolder<'_>>,
    report: &mut Vec<String>,
    reading: Reading,
    requested: &mut dyn FnMut(&str),
) -> anyhow::Result<mesh::Model> {
    use anyhow::Context as _;
    let sets = mesh::rcs::siblings::Siblings::probe(entry, false);
    anyhow::ensure!(!sets.is_empty(), "{entry} names no .rcsmodel");
    let mut first_error = None;
    let mut found = None;
    for set in &sets {
        match archives.read_name(&set.model) {
            Ok(geometry) => {
                found = Some(geometry);
                break;
            }
            Err(error) => {
                first_error.get_or_insert((set.model.clone(), error));
            }
        }
    }
    let Some(geometry) = found else {
        let (sibling, error) = first_error.expect("at least one spelling was tried");
        return Err(error).with_context(|| format!("{entry}: reading its {sibling}"));
    };
    let mut reads = crate::zone_livery::Reads::new(livery);
    let mut read = |archives: &mut oag_assets::Archives, path: &str| {
        if oag_texture::decode::Container::of_name(path).is_some() {
            requested(path);
            if !reading.pictures {
                return None;
            }
        }
        reads.read(archives, path)
    };
    // 2048's and Omega's container is a different file under the same extension and needs no
    // `.vex` to be read (`oag_mesh::mesh::rcs::psp2`); a hull's flap hinges are the `.vex`'s.
    let mut model = if mesh::rcs::psp2::is_psp2(&geometry) {
        let (mut model, built) = mesh::rcs::psp2::build_circuit(
            entry,
            &geometry,
            None,
            &mut |path| read(archives, path),
            reading.water,
            mesh::rcs::psp2::circuit_pads::HidePads::None,
        )
        .with_context(|| format!("decoding the model {entry}"))?;
        // A hull's flaps; no other model names an `Airbrake`.
        model.seat_flaps_on_vex(blob);
        report.push(format!("{entry}: {}", built.describe()));
        model
    } else {
        let (model, built) = mesh::rcs::build_scene(
            entry,
            blob,
            &geometry,
            &mut |path| read(archives, path),
            // A front-end or HUD model draws into a gamma target, whose shaders decode with the
            // plain power.
            Default::default(),
        )
        .with_context(|| format!("decoding the model {entry}"))?;
        report.push(format!("{entry}: {}", built.describe()));
        model
    };
    report.extend(reads.describe(entry));
    model.keep_nearest();
    Ok(model)
}
