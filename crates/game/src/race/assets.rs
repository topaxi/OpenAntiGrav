//! The small readers `load` is built out of - a particle effect, an exhaust
//! texture - and the two notes it puts in the load report when an asset is
//! absent or arrives untextured.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// The exhaust sprite's texture, named as a literal string in the executable.
///
/// At `0x08a84c80`, loaded by `Texture_LoadEngineFlare`. Being a literal means the
/// WAD lookup is an exact `wad::hash_name` hit rather than a mined candidate, which
/// is unusual for this project and worth the note.
pub const FLARE_TEXTURE: &str = r"Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip";

/// The trail ribbon's texture, also a literal in the executable.
///
/// At `0x08a889e4`, loaded by `Texture_LoadEngineNoise` into three slots that
/// `Trail_DrawRibbon` indexes per layer. Note the directory case differs from
/// [`FLARE_TEXTURE`]'s - `engineFlare` here, `EngineFlare` there - which does not
/// matter, since the WAD hash is case-insensitive.
///
/// The PS2 build loads this same literal, from `0x002be7d0` in `SCES_547.48`,
/// into the same three slots - and finds it under `.pct` rather than `.mip`.
/// [`oag_pulse::ps2_texture_name`] is where that rewrite lives; nothing here
/// needs to know which disc it is reading.
pub const NOISE_TEXTURE: &str = r"Data\Tex\engineFlare\Engine_noise.mip";

/// Reads one `Data\Psys\<name>.POB` out of the archive set and parses it
/// into a playable effect.
///
/// The note it returns goes in the loader report next to the other assets',
/// because "the sparks did not appear" and "the file was not found" are
/// otherwise indistinguishable from a screenshot - the failure mode the
/// `--give` flag exists to remove for weapons.
pub(super) fn particle_effect(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> std::result::Result<(psys::Effect, String), String> {
    let path = sparks::effect_path(name);
    let blob = archives
        .read_name(&path)
        .map_err(|e| format!("{path}: not in the archive set ({e})"))?;
    // The one thing about a `.pob` that is a property of the release rather
    // than of the file - see `psys::ColourScale`. Read off the source
    // because it cannot be read off the bytes.
    let scale = match archives.layout.platform {
        oag_assets::Platform::Ps2 => psys::ColourScale::Half,
        _ => psys::ColourScale::Full,
    };
    let effect = psys::Effect::parse(&blob, scale)
        .map_err(|e| format!("{path}: {} bytes, does not parse ({e})", blob.len()))?;
    let note = format!(
        "{name}: {} emitter(s) - {}",
        effect.emitters.len(),
        effect
            .emitters
            .iter()
            .map(|spec| spec.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok((effect, note))
}

/// Decodes an exhaust texture out of the archive set, in whichever of the two
/// formats this disc stores it.
///
/// Returns the pixels and a line for the load report, or the reason it could not -
/// **as text, not as `None`**. A missing entry and a blob that does not parse are
/// different problems with different fixes (name mining versus the decoder), and a
/// silent fallback hides which one happened.
///
/// [`oag_pulse::read_image`] rather than `read_name`, for the same reason the
/// HUD atlas uses it: the PS2 keeps these two under the declared name with its
/// own extension. The PSP path is unchanged - the declared name answers first
/// and the rewrite is never reached.
pub(super) fn exhaust_texture(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> std::result::Result<(FlareTexture, String), String> {
    let blob = oag_pulse::read_image(archives, name)
        .map_err(|e| format!("{name}: not in the archive set ({e})"))?;
    decode_either(name, &blob)
}

/// Decodes whichever of the two texture formats the blob is, with the pixels a
/// [`FlareTexture`] wants.
///
/// The PSP's `.mip` is tried first and **its** error is the one reported, the
/// same rule `crate::sprite::Image::decode` follows: a `.mip` that will not
/// parse is far and away the commoner failure, and reporting the PS2 parser's
/// complaint about a PSP blob sends the reader after the wrong decoder.
///
/// The note names the format, because "64x64" alone does not distinguish a PSP
/// disc from a PS2 one in the load report, and the two reach this point by
/// different names.
fn decode_either(name: &str, blob: &[u8]) -> std::result::Result<(FlareTexture, String), String> {
    match oag_formats::texture::Texture::parse(blob) {
        Ok(texture) => Ok((
            FlareTexture {
                width: u32::from(texture.width),
                height: u32::from(texture.height),
                rgba: texture.to_rgba(),
            },
            format!(
                "{name}: {}x{} .mip, {} mip level(s)",
                texture.width, texture.height, texture.mip_levels
            ),
        )),
        Err(psp) => match oag_formats::ps2_texture::parse(blob) {
            Ok(texture) => Ok((
                FlareTexture {
                    width: u32::from(texture.width),
                    height: u32::from(texture.height),
                    rgba: texture.to_rgba(),
                },
                format!(
                    "{name}: {}x{} .pct, {} bpp",
                    texture.width, texture.height, texture.bits_per_pixel
                ),
            )),
            Err(_) => Err(format!(
                "{name}: {} bytes, does not parse ({psp})",
                blob.len()
            )),
        },
    }
}

/// Why a class produced nothing: the file authors none, or nobody has recovered
/// its id for this format version.
///
/// **Two very different findings that look identical in a draw list**, and
/// collapsing them is how a decoding gap gets filed as authored content. A Pure
/// track really does author no `Speedup Pad` under version 6's `0x3bd` - but it
/// authors none under *any* id this project knows, because version 4's numbering
/// for that class has never been recovered, and saying "the track authors no
/// pads" would close a question that is still open. See
/// `docs/formats/pure-status.md`.
pub(super) fn unrecovered_or_absent(
    classes: oag_formats::vex::classes::Classes,
    id: Option<u32>,
    what: &str,
    consequence: &str,
) -> String {
    match id {
        Some(_) => format!("the track authors no {what} geometry; {consequence}"),
        None => format!(
            "no {what} class id is recovered for .vex version {}, so this track was \
             never asked for any; {consequence}",
            classes.version
        ),
    }
}

#[must_use]
pub(super) fn untextured_note(model: &Model) -> Option<String> {
    let slots = model.textures.len();
    let decoded = model.textures.iter().filter(|t| t.is_some()).count();
    if slots == 0 || decoded == slots {
        return None;
    }
    Some(format!(
        "{}: {decoded} of {slots} texture slot(s) decoded, drawing the rest \
         untextured. PS2 models keep their textures in separate archive entries, \
         found by directory position; a handful of circuits' texture sets are \
         short 1-2 slots on disc rather than unresolved by this lookup - see \
         docs/formats/ps2-texture.md",
        model.label
    ))
}

/// The archive entry name of a team's `.vex` model.
///
/// Assembled the way the loader assembles it, with backslashes, which is what the
/// name hash needs.
///
/// A [`Mode::Zone`] run loads `Zone.vex` instead of `Ship.vex`. That is not a
/// livery swap of convenience: `Ship_LoadModel` (`0x08843258`) switches on the
/// same `DAT_08ab07e3 == 0 && DAT_08b31048 == 6` expression already established
/// as the Zone selector (see `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`),
/// and only that case builds the `%s\Zone.vex` path. Every team's `Zone.vex`
/// decodes to the same 1213 vertices / 1149 triangles / 8 meshes, so the hull
/// itself is shared - only the livery painted on it still varies by team.
#[must_use]
pub fn ship_entry_name(team: &str, mode: Mode) -> String {
    let model = if mode == Mode::Zone {
        ships::ZONE_HULL
    } else {
        ships::HULL
    };
    ships::entry_name(team, model)
}

/// The boost plume that goes with [`ship_entry_name`]'s hull.
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
#[must_use]
pub fn boost_entry_name(team: &str, mode: Mode) -> String {
    let model = if mode == Mode::Zone {
        ships::ZONE_BOOST
    } else {
        ships::BOOST
    };
    ships::entry_name(team, model)
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
/// **No mode switch**, unlike [`boost_entry_name`]: no `Zoneshield.vex` exists
/// on either disc, and the original's own format string takes its prefix from a
/// config key rather than from the game mode. See
/// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`.
#[must_use]
pub fn shield_entry_names(team: &str) -> [String; 2] {
    [
        ships::entry_name(team, ships::SHIELD),
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
pub(crate) fn ps2_texture_set(
    archives: &mut oag_assets::Archives,
    entry_name: &str,
) -> Option<Vec<Option<mesh::ModelTexture>>> {
    let blob = archives.read_preceding(entry_name).ok()?;
    mesh::ps2_texture_set(&blob).ok()
}

/// The error a circuit that is in none of the source's archives produces.
///
/// **A Zone race gets a different sentence, and the reason is that it has a
/// different commonest cause.** Any other mode reaching this point named a
/// circuit that is not on the disc, and saying so is the whole story. A Zone
/// race on a title whose Zone circuits are the race ones with a prefixed file
/// reaches it for a second reason that looks identical from the archive's side:
/// the circuit is there, and it simply has no Zone variant beside it. Sixteen of
/// Pulse's twenty-four `PI_Track` entries carry one and eight do not, exactly as
/// each declares with `availableInZone` - see
/// [`crate::catalogue::Track::available_in_zone`].
///
/// Collapsing the two sends a reader after a name-mining problem that is not
/// there, which is the failure [`unrecovered_or_absent`] exists to prevent one
/// layer down.
pub(super) fn zone_circuit_miss(
    track: &str,
    mode: Mode,
    title: &oag_title::Title,
    archives: &oag_assets::Archives,
) -> anyhow::Error {
    let layout = archives.layout.describe();
    if mode != Mode::Zone {
        return anyhow::anyhow!("{track} is in none of this source's archives ({layout})");
    }
    match title.race.zone {
        oag_title::ZoneCircuit::Prefixed(prefix) => anyhow::anyhow!(
            "{track} is in none of this source's archives ({layout}). A zone race \
             flies the circuit's own zone environment, named with the `{prefix}` \
             prefix, and not every circuit has one - on {} the plugin definition \
             marks the ones that do with `availableInZone`. Pick one of those, or \
             race another mode here",
            title.name
        ),
        oag_title::ZoneCircuit::Separate(default) => anyhow::anyhow!(
            "{track} is in none of this source's archives ({layout}). {} keeps its \
             zone circuits apart from its race ones, so a race circuit's name will \
             not do: {default} is the one this title opens by default",
            title.name
        ),
    }
}
