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

/// The LeachBeam ribbon's own texture, a literal in the executable - found
/// 2026-09-17, spelled "lee**ch**" on disc rather than "leach", which is why
/// the earlier `(?i)leach` search over `.rodata` missed it.
///
/// At `0x08a7cc60`, loaded by `LeachBeam_LoadTexture` (`0x088730d4`) into
/// `DAT_08b3bfa4`, which `LeachBeam_BuildStrip` (`0x088739b0`) binds right
/// before building the ribbon's own vertices. See
/// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`'s
/// "2026-09-17: the LeachBeam ribbon's own texture" section for the full
/// evidence chain. Under the Cannon's own `Data\Weapons\Textures\` directory
/// rather than the two `Data\Tex\Weapons\` hull-overlay textures beside
/// [`NOISE_TEXTURE`] above - a different directory is the whole reason the
/// earlier search missed it too.
pub const LEACHBEAM_TEXTURE: &str = r"Data\Weapons\Textures\pulse_leechbeam1_ADD.mip";

/// Decodes [`LEACHBEAM_TEXTURE`] out of the archive set, or reports why it did
/// not - the same "absence is reported, not fatal" rule every optional asset
/// in this module follows. No per-title axis the way [`flare_texture`]'s
/// `oag_title::flare::Flare` is: the LeachBeam is Pulse's own weapon and this
/// is its one literal path, found and read on one binary.
pub(super) fn leach_beam_texture(
    archives: &mut oag_assets::Archives,
    report: &mut Vec<String>,
) -> Option<FlareTexture> {
    let own = platform_sibling(archives, LEACHBEAM_TEXTURE);
    let found = match &own {
        Some(entry) => decode_texture(archives, entry)
            .map(|texture| (texture, format!("{entry}: the LeachBeam ribbon's own texture"))),
        None => exhaust_texture(archives, LEACHBEAM_TEXTURE),
    };
    match found {
        Ok((texture, note)) => {
            report.push(note);
            Some(texture)
        }
        Err(why) => {
            report.push(format!("{why} - the LeachBeam draws with no ribbon body"));
            None
        }
    }
}

/// Reads one `Data\Psys\<name>.POB` out of the archive set and parses it
/// into a playable effect.
///
/// The note it returns goes in the loader report next to the other assets',
/// because "the sparks did not appear" and "the file was not found" are
/// otherwise indistinguishable from a screenshot - the failure mode the
/// `--give` flag exists to remove for weapons.
pub(super) fn particle_effect(
    archives: &mut oag_assets::Archives,
    dir: &str,
    name: &str,
) -> std::result::Result<(psys::Effect, String), String> {
    let path = psys::effect_path_in(dir, name);
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
    // Wipeout 2048 ships its sprites beside its effects, `<dir>\Tex\<stem>.gxt`
    // for an authored `...\Tex\<stem>.tga`: all 76 distinct names across its
    // 85 `Particles2048` effects resolve (`psys_2048_ground_truth.rs`). Omega
    // ships `.gnf` under the directory the authored path itself names, which is
    // not always the effect's own (one `particles` effect names a
    // `particles2048` sprite). Every other title's sprites are embedded or not
    // loaded yet, and keep the procedural profile.
    let ext = match archives.layout.platform {
        oag_assets::Platform::Vita => Some("gxt"),
        oag_assets::Platform::Ps4 => Some("gnf"),
        _ => None,
    };
    let (mut sprites, mut absent) = (0usize, Vec::new());
    let effect = psys::Effect::parse_with(&blob, scale, &mut |authored| {
        let ext = ext?;
        let stem = authored.rsplit(['\\', '/']).next()?.rsplit_once('.')?.0;
        let entry = if ext == "gnf" {
            let at = authored.to_ascii_lowercase().find(r"data\")?;
            format!(
                "{}{stem}.gnf",
                &authored[at..authored.len() - stem.len() - 4]
            )
        } else {
            format!(r"{dir}\Tex\{stem}.{ext}")
        };
        let sprite = archives.read_name(&entry).ok().and_then(|blob| {
            if ext == "gnf" {
                psys::sprite::Sprite::from_gnf(&blob)
            } else {
                psys::sprite::Sprite::from_gxt(&blob)
            }
        });
        match sprite {
            Some(sprite) => {
                sprites += 1;
                Some(sprite)
            }
            None => {
                absent.push(entry);
                None
            }
        }
    })
    .map_err(|e| format!("{path}: {} bytes, does not parse ({e})", blob.len()))?;
    let sprite_note = if ext.is_some() {
        let mut note = format!(", {sprites} sprite(s) read");
        if !absent.is_empty() {
            note.push_str(&format!(
                ", {} not read and drawn procedurally: {}",
                absent.len(),
                absent.join(", ")
            ));
        }
        note
    } else {
        String::new()
    };
    let undrawn: Vec<&str> = effect.undrawn_emitters().collect();
    let undrawn_note = if undrawn.is_empty() {
        String::new()
    } else {
        format!(
            "; {} will not be drawn (blend class 8, a distortion the heat-haze shader draws, \
             unrecovered): the other emitters play",
            undrawn.join(", ")
        )
    };
    let note = format!(
        "{name}: {} emitter(s) - {}{sprite_note}{undrawn_note}",
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
                // Vita textures are `.gxt`: the same ribbon, flare and boost
                // code reads 2048, whose own executable was not measured for
                // any of the laws `load` reports under the ribbon.
                if let oag_title::flare::Flare::PerTeam(a) = title.flare
                    && a.sprite.ends_with(".gxt")
                {
                    report.push(
                        "2048 reads its ribbon, flare and boost reveal on HD's laws: \
                         inherited, not measured on 2048 (only the asset names and the \
                         node groups are)"
                            .into(),
                    );
                }
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
    // **Both containers name the same four things**: the coverage texture, the
    // noise texture, and the blend pair. 2048's `.rcsmodel` is another file
    // under the same extension and carries no factor pair of its own, so its
    // ribbon takes HD's pair for the material of the same name -
    // `hd_enginetrail_bluered`, `SrcAlpha`/`One` - the inheritance
    // `oag_rcs::rcsmodel::psp2::lineage_blend` already documents.
    let (coverage, noise, src, dst) = if oag_mesh::mesh::rcs::psp2::is_psp2(&blob) {
        let parsed = oag_rcs::rcsmodel::psp2::parse(&blob)
            .map_err(|e| format!("{model}: {} bytes, does not parse ({e})", blob.len()))?;
        let material = parsed
            .materials
            .first()
            .ok_or_else(|| format!("{model}: parses, but names no material"))?;
        let coverage = material
            .textures
            .first()
            .ok_or_else(|| format!("{model}: its material names no texture"))?;
        let noise = material
            .textures
            .get(1)
            .ok_or_else(|| format!("{model}: its material names no second texture"))?;
        let (src, dst) = oag_rcs::rcsmodel::psp2::lineage_blend::inherited(&material.name)
            .ok_or_else(|| {
                format!(
                    "{model}: its material {} has no blend pair to inherit from HD",
                    material.name
                )
            })?;
        (
            format!("/{}", coverage.to_ascii_lowercase()),
            format!("/{}", noise.to_ascii_lowercase()),
            src,
            dst,
        )
    } else {
        let parsed = oag_rcs::rcsmodel::Model::parse(&blob)
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
            .clone()
            .ok_or_else(|| format!("{model}: its material names no second texture"))?;
        // Through the recovered mapping rather than a second reading of the same
        // bytes: `Material::blend` names the factors and `rcs::blend_state` turns
        // that pair into a pipeline state, and both already draw every HD surface.
        let authored = material.blend();
        let oag_rcs::rcsmodel::Blend::Factors { src, dst } = authored else {
            return Err(format!(
                "{model}: its material's blend is {authored:?} rather than a factor pair"
            ));
        };
        (material.texture.clone(), noise, src, dst)
    };
    let noise = noise.as_str();
    let noise_texture = decode_texture(archives, noise)
        .map_err(|why| format!("{why} (named by {model}'s material)"))?;
    let (width, height) = (noise_texture.width, noise_texture.height);
    let container = if noise.to_ascii_lowercase().ends_with(".gxt") {
        ".gxt"
    } else {
        ".gtf"
    };
    let blend = mesh::rcs::blend_state(src, dst);
    let mut notes = vec![
        format!(
            "{noise}: {width}x{height} {container} - the ribbon's noise, named by {model}'s own material"
        ),
        format!("{model}: blend {src:?}/{dst:?} off the material, not the PSP preset's"),
    ];
    let shape = match decode_texture(archives, &coverage) {
        Ok(shape) => {
            notes.push(format!(
                "{}: {}x{} {} - the ribbon's own coverage, multiplied into its alpha",
                coverage, shape.width, shape.height, container
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
    Ok((noise_texture, blend, shape, notes))
}

/// [`decode_gtf`] or [`decode_gxt`], by the name's own extension.
pub(crate) fn decode_texture(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> std::result::Result<FlareTexture, String> {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".gxt") {
        decode_gxt(archives, name)
    } else if lower.ends_with(".gnf") {
        decode_gnf(archives, name)
    } else {
        decode_gtf(archives, name)
    }
}

/// Pulse's `.mip` entry `name` as this source ships the same texture: the
/// same directory and stem with the platform's own extension, `.gxt` on the
/// Vita and `.gnf` on the PS4. `None` on a platform whose textures keep the
/// name as authored. 2048 and Omega ship `Cannon_bolt`, `cannon_muzzle_flash`
/// and `pulse_leechbeam1_ADD` under `Data\Weapons\Textures` this way, and
/// both executables name `Cannon_bolt` and `Cannon_muzzle`.
pub(crate) fn platform_sibling(archives: &oag_assets::Archives, name: &str) -> Option<String> {
    let ext = match archives.layout.platform {
        oag_assets::Platform::Vita => "gxt",
        oag_assets::Platform::Ps4 => "gnf",
        _ => return None,
    };
    let stem = name.strip_suffix(".mip")?;
    Some(format!("{stem}.{ext}"))
}

/// One PS4 `.gnf` out of the archive set, base level - [`decode_gxt`]'s
/// Omega counterpart.
pub(crate) fn decode_gnf(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> std::result::Result<FlareTexture, String> {
    let blob = archives
        .read_name(name)
        .map_err(|e| format!("{name}: not in the archive set ({e})"))?;
    let sprite = psys::sprite::Sprite::from_gnf(&blob)
        .ok_or_else(|| format!("{name}: {} bytes, does not decode", blob.len()))?;
    Ok(FlareTexture {
        width: u32::from(sprite.width),
        height: u32::from(sprite.height),
        rgba: sprite.rgba.to_vec(),
    })
}

/// One Vita `.gxt` out of the archive set, as pixels the exhaust pipeline can
/// bind - 2048's counterpart of [`decode_gtf`].
pub(crate) fn decode_gxt(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> std::result::Result<FlareTexture, String> {
    let pixels = archives
        .read_name(name)
        .map_err(|e| format!("{name}: not in the archive set ({e})"))?;
    let gxt = oag_texture::gxt::Gxt::parse(&pixels)
        .map_err(|e| format!("{name}: {} bytes, does not parse ({e})", pixels.len()))?;
    let texture = gxt
        .only()
        .ok_or_else(|| format!("{name}: parses, but is not a single texture"))?;
    let rgba = texture
        .to_rgba(&pixels)
        .map_err(|e| format!("{name}: does not decode ({e})"))?;
    Ok(FlareTexture {
        width: u32::from(texture.width),
        height: u32::from(texture.height),
        rgba: rgba.into_iter().flatten().collect(),
    })
}

/// One `.gtf` out of the archive set, as pixels the exhaust pipeline can bind.
///
/// The `.gtf` half of [`authored_ribbon`], lifted out when the ribbon needed a
/// second texture and both wanted the same reporting.
pub(crate) fn decode_gtf(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> std::result::Result<FlareTexture, String> {
    let pixels = archives
        .read_name(name)
        .map_err(|e| format!("{name}: not in the archive set ({e})"))?;
    let gtf = oag_texture::gtf::Gtf::parse(&pixels)
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
/// per craft by [`oag_livery::flare`] and reported there; the sprite
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
                "{}: the engine flare is authored geometry here - one {}.vex per \
                 craft, reported with each livery - and a sprite flare rides on \
                 top of it",
                title.name, authored.stem
            ));
            // HD draws a sprite flare *as well as* the model: the tuning file
            // authors `Enable Flare Sprite` = 1 with its radius and alpha
            // walk, and the flare's own init (`0x002a1528`) loads this
            // texture and four corner pairs. The modulation this engine does
            // not reproduce is listed on `exhaust::hd::Sprite`.
            let sprite = authored.sprite;
            match decode_texture(archives, sprite) {
                Ok(texture) => {
                    report.push(format!(
                        "{sprite}: {}x{} - the sprite flare the \
                         executable loads beside the flare model",
                        texture.width, texture.height
                    ));
                    Some(texture)
                }
                Err(why) => {
                    report.push(format!("{why} - no sprite flare rides the nozzle"));
                    None
                }
            }
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
/// same rule `oag_hud::sprite::Image::decode` follows: a `.mip` that will not
/// parse is far and away the commoner failure, and reporting the PS2 parser's
/// complaint about a PSP blob sends the reader after the wrong decoder.
///
/// The note names the format, because "64x64" alone does not distinguish a PSP
/// disc from a PS2 one in the load report, and the two reach this point by
/// different names.
fn decode_either(name: &str, blob: &[u8]) -> std::result::Result<(FlareTexture, String), String> {
    match oag_texture::texture::Texture::parse(blob) {
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
        Err(psp) => match oag_texture::ps2_texture::parse(blob) {
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
    classes: oag_vex::vex::classes::Classes,
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
        oag_title::ZoneCircuit::Separate(tracks, true) => anyhow::anyhow!(
            "{track} is in none of this source's archives ({layout}). {} offers both \
             its own dedicated zone circuits and its ordinary race ones in zone \
             mode, so the name is simply not on the disc under any name: {} is the \
             one this title opens by default",
            title.name,
            tracks[0]
        ),
        oag_title::ZoneCircuit::Separate(tracks, false) => anyhow::anyhow!(
            "{track} is in none of this source's archives ({layout}). {} keeps its \
             zone circuits apart from its race ones, so a race circuit's name will \
             not do: {} is the one this title opens by default",
            title.name,
            tracks[0]
        ),
        // Zone flies the exact circuit a race would on this title, so a miss
        // here has the same one cause a miss in any other mode does: the name
        // is not on the disc. No special-cased sentence to write.
        oag_title::ZoneCircuit::SameCircuit => {
            anyhow::anyhow!("{track} is in none of this source's archives ({layout})")
        }
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

/// The weapon table this title tunes its weapons from, or `None`.
///
/// **The title's own entry name, not `oag_tables::weapons::RACE_ENTRY`.** That
/// constant is Pulse's spelling and Wipeout HD answers it too, but Pure names one
/// lower-cased `Data\XML\weaponstats.xml` and ships no Eliminator variant - so
/// until 2026-08-26 a Pure race found no table, parsed no weapons and handed out
/// no pickups, with one report line to say so and nothing else. See
/// `oag_title::weapons::Weapons`.
///
/// Read on every mode rather than only the ones with weapons on: it is a
/// per-race asset like any other, and reading it unconditionally is what makes a
/// broken file a reported line on every run rather than one nobody sees until
/// they pick the mode that needs it.
///
/// **`Mode::Eliminator` opens [`oag_title::weapons::Weapons::elimination`]
/// instead, from 2026-09-08.** That is the recovered mechanism behind the
/// disc's own *"weapons do more damage"* in `MSC_EVENT_ELIM` - a whole second
/// table, not a multiplier this build would have to invent - and it is the
/// field `Weapons::elimination`'s own doc comment already named as unread. A
/// title with no second table (`None`, Pure) falls back to
/// [`oag_title::weapons::Weapons::race`] and says so in the report, the same
/// "reported gap, not a silent substitution" shape the rest of this function
/// already uses.
///
/// Two failures, two lines, and a third kind that is neither - see
/// `oag_tables::weapons::WeaponStats::skipped`.
/// The title's `WeaponAIstats.xml` - the odds an opponent fires each weapon
/// at, see [`oag_title::weapons::Weapons::ai`] - or `None`, with a report line
/// saying what that costs.
///
/// Read on every mode for the reason `load_weapons` is: a broken file is a
/// line on every run. A title that names no such file gets a line too, because
/// its opponents fire on this project's own rule rather than the original's.
pub(super) fn load_weapon_ai(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    report: &mut Vec<String>,
) -> Option<oag_tables::weapons::ai::WeaponAiStats> {
    let Some(entry) = title.weapons.ai else {
        report.push(
            "no weapon-AI table named for this title; opponents fire on this \
             build's own rule"
                .to_string(),
        );
        return None;
    };
    let parsed = archives
        .read_name(entry)
        .with_context(|| format!("reading {entry} out of {}", archives.layout.describe()))
        .and_then(|blob| Ok(oag_tables::weapons::ai::from_blob(&blob)?));
    match parsed {
        Ok(stats) => {
            report.push(format!("{entry}: opponents fire on its odds"));
            Some(stats)
        }
        Err(e) => {
            report.push(format!(
                "{entry}: {e}; opponents fire on this build's own rule"
            ));
            None
        }
    }
}

pub(super) fn load_weapons(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    mode: Mode,
    report: &mut Vec<String>,
) -> Option<oag_tables::weapons::WeaponStats> {
    let entry = if mode == Mode::Eliminator {
        match title.weapons.elimination {
            Some(entry) => entry,
            None => {
                report.push(format!(
                    "{}: this title ships no Eliminator weapon table; racing on \
                     its ordinary one instead",
                    title.weapons.race
                ));
                title.weapons.race
            }
        }
    } else {
        title.weapons.race
    };
    let blob = match archives
        .read_name(entry)
        .with_context(|| format!("reading {entry} out of {}", archives.layout.describe()))
    {
        Ok(blob) => blob,
        Err(e) => {
            report.push(format!("{entry}: {e}"));
            return None;
        }
    };
    let stats = match oag_tables::weapons::from_blob(&blob) {
        Ok(stats) => stats,
        Err(e) => {
            report.push(format!("{entry}: {e}"));
            return None;
        }
    };
    report.push(format!(
        "{entry}: {} weapon(s) with an absorb value, {} pickup table(s)",
        stats.absorb.len(),
        stats.pickups.len()
    ));
    // A weapon this title tunes on attributes this build does not read.
    // Reported per weapon, because "the disc has no Bomb" and "this build
    // cannot read this disc's Bomb" are different statements and only one of
    // them is about the disc.
    for (weapon, attribute) in &stats.skipped {
        report.push(format!(
            "{entry}: {weapon:?} authors no {attribute}; this build cannot \
             decode it and the weapon is absent"
        ));
    }
    Some(stats)
}
