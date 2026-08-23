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
///
/// **The loader reaches it through [`oag_title::flare::Flare::Sprite`] now**,
/// not by name from here: Wipeout HD carries no such entry and authors a model
/// instead. This stays as the PSP and PS2 literal that
/// `crates/game/tests/ps2_source_ground_truth.rs` asserts both discs decode,
/// on the same footing as [`NOISE_TEXTURE`] beside it.
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
/// The ribbon's texture and blend, from wherever this title keeps them.
///
/// **Two sources, because the titles disagree about what kind of thing the
/// answer is** - see [`oag_title::exhaust::Exhaust`]. A title that *names* a
/// texture gets the existing lookup and the ribbon's recovered PSP blend; a
/// title that *authors* the ribbon hands back its material's own noise texture
/// and its own factor pair, both read off the disc.
///
/// **The first texture is located and deliberately not wired, and on HD's real
/// ribbon that is now the expensive half.** Its material names two, and the
/// `bluered` pair the executable points at spells out what each is for:
/// `hd_enginetrail_blue_alphaistrail.gtf` and
/// `hd_enginetrail_red_alphaisnoise.gtf`. So the second slot carries the noise
/// **in its alpha**, which is the rule this loader already followed - and the
/// first carries *the ribbon's own shape mask*, which this renderer's
/// one-texture shader has no slot for. That is a stronger reason to want a
/// two-texture ribbon than "a colour map is left over" was. Putting either in
/// the wrong slot would be the plausible-looking substitution `CLAUDE.md`
/// names, so the noise goes in the noise slot and the report says what is left
/// out.
pub(super) fn trail_texture(
    archives: &mut oag_assets::Archives,
    title: &oag_title::Title,
    report: &mut Vec<String>,
) -> (
    Option<FlareTexture>,
    Option<wgpu::BlendState>,
    Option<FlareTexture>,
) {
    match title.exhaust {
        oag_title::exhaust::Exhaust::Unread => {
            report.push(format!(
                "{}: no exhaust ribbon texture is located for this title - the trail \
                 falls back to a procedural glow",
                title.name
            ));
            (None, None, None)
        }
        oag_title::exhaust::Exhaust::Named(name) => match exhaust_texture(archives, name) {
            Ok((texture, note)) => {
                report.push(note);
                (Some(texture), None, None)
            }
            Err(why) => {
                report.push(format!("{why} - the trail falls back to a procedural glow"));
                (None, None, None)
            }
        },
        oag_title::exhaust::Exhaust::Authored(model) => match authored_ribbon(archives, model) {
            Ok((texture, blend, shape, notes)) => {
                report.extend(notes);
                (Some(texture), Some(blend), shape)
            }
            Err(why) => {
                report.push(format!("{why} - the trail falls back to a procedural glow"));
                (None, None, None)
            }
        },
    }
}

/// The `Authored` half: a `.rcsmodel` whose first material names everything.
fn authored_ribbon(
    archives: &mut oag_assets::Archives,
    model: &str,
) -> std::result::Result<
    (
        FlareTexture,
        wgpu::BlendState,
        Option<FlareTexture>,
        Vec<String>,
    ),
    String,
> {
    let blob = archives
        .read_name(model)
        .map_err(|e| format!("{model}: not in the archive set ({e})"))?;
    let parsed = oag_formats::rcsmodel::Model::parse(&blob)
        .map_err(|e| format!("{model}: {} bytes, does not parse ({e})", blob.len()))?;
    let material = parsed
        .materials
        .first()
        .ok_or_else(|| format!("{model}: parses, but names no material"))?;
    // The *second* texture is the noise map on every ribbon in this family -
    // `hd_enginetrail_noise`, `hd_waketrail_clouds`, `smoke_trails_frame2`.
    // A material with only one is a shape this has not seen, so it says so
    // rather than falling back to the colour map in a noise slot.
    let noise = material
        .second_texture
        .as_deref()
        .ok_or_else(|| format!("{model}: its material names no second texture"))?;
    let pixels = archives
        .read_name(noise)
        .map_err(|e| format!("{noise}: named by {model}'s material but not in the set ({e})"))?;
    let gtf = oag_formats::gtf::Gtf::parse(&pixels)
        .map_err(|e| format!("{noise}: {} bytes, does not parse ({e})", pixels.len()))?;
    let texture = gtf
        .only()
        .ok_or_else(|| format!("{noise}: parses, but is not a single texture"))?;
    let rgba = texture
        .to_rgba(&pixels)
        .map_err(|e| format!("{noise}: does not decode ({e})"))?;
    let (width, height) = texture.level_size(0);
    // Through the recovered mapping rather than a second reading of the same
    // bytes: `Material::blend` names the factors and `rcs::blend_state` turns
    // that pair into a pipeline state, and both already draw every HD surface.
    let authored = material.blend();
    let oag_formats::rcsmodel::Blend::Factors { src, dst } = authored else {
        return Err(format!(
            "{model}: its material's blend is {authored:?} rather than a factor pair"
        ));
    };
    let blend = mesh::rcs::blend_state(src, dst);
    // The *first* texture: the ribbon's own coverage. Its alpha is what HD's
    // fragment program multiplies into the output - the file is called
    // `..._alphaistrail.gtf` and the program agrees - so this is the term that
    // gives the ribbon a shape of its own rather than the PSP preset's.
    // Reported either way: a decode failure here draws the ribbon exactly as it
    // drew before, which is a silent regression unless it is stated.
    let mut notes = vec![
        format!(
            "{noise}: {width}x{height} .gtf - the ribbon's noise, named by {model}'s own material"
        ),
        format!("{model}: blend {src:?}/{dst:?} off the material, not the PSP preset's"),
    ];
    let shape = match decode_gtf(archives, &material.texture) {
        Ok(shape) => {
            notes.push(format!(
                "{}: {}x{} .gtf - the ribbon's own coverage, multiplied into its alpha",
                material.texture, shape.width, shape.height
            ));
            Some(shape)
        }
        Err(why) => {
            notes.push(format!(
                "{why} - the ribbon keeps the shape the PSP preset's constants give it"
            ));
            None
        }
    };
    Ok((
        FlareTexture {
            width,
            height,
            rgba: rgba.into_iter().flatten().collect(),
        },
        blend,
        shape,
        notes,
    ))
}

/// One `.gtf` out of the archive set, as pixels the exhaust pipeline can bind.
///
/// The `.gtf` half of [`authored_ribbon`], lifted out when the ribbon needed a
/// second texture and both wanted the same reporting.
fn decode_gtf(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> std::result::Result<FlareTexture, String> {
    let pixels = archives
        .read_name(name)
        .map_err(|e| format!("{name}: not in the archive set ({e})"))?;
    let gtf = oag_formats::gtf::Gtf::parse(&pixels)
        .map_err(|e| format!("{name}: {} bytes, does not parse ({e})", pixels.len()))?;
    let texture = gtf
        .only()
        .ok_or_else(|| format!("{name}: parses, but is not a single texture"))?;
    let rgba = texture
        .to_rgba(&pixels)
        .map_err(|e| format!("{name}: does not decode ({e})"))?;
    let (width, height) = texture.level_size(0);
    Ok(FlareTexture {
        width,
        height,
        rgba: rgba.into_iter().flatten().collect(),
    })
}

/// The flare's sprite texture, from wherever this title keeps it - or nothing
/// at all, for a title that authors the flare as a model instead.
///
/// **A `PerTeam` title is not a failure here and must not read as one.** Until
/// this axis existed every HD load report ended with "the flare falls back to a
/// procedural glow", because the loader asked HD's disc for a Pulse name it
/// does not carry. HD's flare is `Data\Ships\<Team>\engineflare.vex`, loaded
/// per craft by [`crate::livery::flare`] and reported there; the sprite
/// pipeline keeps its stand-in texture for the *rocket* billboard fallback,
/// which is a separate use of the same slot.
pub(super) fn flare_texture(
    archives: &mut oag_assets::Archives,
    title: &oag_title::Title,
    report: &mut Vec<String>,
) -> Option<FlareTexture> {
    match title.flare {
        oag_title::flare::Flare::Sprite(name) => match exhaust_texture(archives, name) {
            Ok((texture, note)) => {
                report.push(note);
                Some(texture)
            }
            Err(why) => {
                // Reported rather than silently swapped for the placeholder. A
                // stand-in that looks plausible is how a decode failure
                // survives review; see the note on `FlareTexture::placeholder`.
                report.push(format!("{why} - the flare falls back to a procedural glow"));
                None
            }
        },
        oag_title::flare::Flare::PerTeam(authored) => {
            report.push(format!(
                "{}: the engine flare is authored geometry here, not a sprite - one \
                 {}.vex per craft, reported with each livery. No flare texture is \
                 loaded and none is missing",
                title.name, authored.stem
            ));
            None
        }
        oag_title::flare::Flare::Unread => {
            report.push(format!(
                "{}: no engine flare is located for this title - the flare falls back \
                 to a procedural glow",
                title.name
            ));
            None
        }
    }
}

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
#[must_use]
pub fn ship_entry_name(team: &str, mode: Mode, zone: oag_title::ZoneCraft) -> String {
    if mode != Mode::Zone {
        return ships::entry_name(team, ships::HULL);
    }
    ships::entry_name(zone.directory(team), zone.hull().unwrap_or(ships::HULL))
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
pub fn boost_entry_name(team: &str, mode: Mode, zone: oag_title::ZoneCraft) -> String {
    if mode != Mode::Zone {
        return ships::entry_name(team, ships::BOOST);
    }
    ships::entry_name(zone.directory(team), zone.boost().unwrap_or(ships::BOOST))
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

/// The report line a Zone race owes about its handling, or `None` for any other
/// mode.
///
/// **Every title ships a Zone-mode handling file and this engine does not read
/// it**, so a Zone race flies on the player's own team's numbers. That is a
/// divergence from the original rather than a simplification, and one that is
/// invisible from a screenshot, so it is stated on every Zone load.
///
/// The directory is `Data\Ships\Zone_01` on both PSP titles and
/// `/data/ships/zone` on HD, all three opening `<Stats team="ZoneMode">` and
/// authoring **no `<Class>` block at all** where a team file authors four or
/// five.
///
/// **The line deliberately names no path**, because on Pulse that directory is
/// not one anything here can derive: [`oag_title::ZoneCraft::directory`] answers
/// with the *player's own team* there, which is correct for the model and wrong
/// for this - Pulse draws `Assegai\Zone.vex` while its ZoneMode block sits in
/// `Zone_01`. An earlier draft of this line interpolated that helper and told
/// every Pulse player that `Assegai\handlingstats.xml` was a ZoneMode file,
/// which it is not. Naming the directory would need a fifth title field for
/// something nothing reads; `crates/game/tests/zone_ground_truth.rs` carries
/// the three spellings as the evidence instead. `oag_gameplay::handling_for` needs a class
/// and panics without one, which is the whole reason it is unread - see
/// [`oag_title::ZoneCraft`] for what the shipped file does carry and why the
/// mode plausibly wants it.
pub(super) fn zone_handling_note(mode: Mode, team: &str) -> Option<String> {
    if mode != Mode::Zone {
        return None;
    }
    Some(format!(
        "zone: flying {team}'s handling. This title ships a ZoneMode handling \
         block of its own and nothing reads it - it authors no <Class>, which \
         is what this engine's per-class conversion needs. See \
         oag_title::ZoneCraft and docs/gameplay/race-modes.md"
    ))
}
