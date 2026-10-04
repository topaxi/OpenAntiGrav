//! What a Wipeout 2048 material's own uniforms say scrolls, played through the
//! two tables Wipeout HD's scrolling already rides.
//!
//! **Two shapes, and only the ones the file names.** A material authors its
//! scroll as named uniforms (`oag_rcs::rcsmodel::psp2::material::Param`, the
//! `~crc32` names `KNOWN_PSP2_PARAMETER_NAMES` carries), and what each family
//! does with them is in GPU bytecode nobody here has decoded, so a family is
//! acted on only where its names say which shape it is:
//!
//! - **The additive glow layer** - `Emissive_UV_Offset` and `Emissive_UV_Scale`
//!   beside an emissive sampler and a diffuse one. These are the very uniforms
//!   and the very hashes HD's `uvanim_diffuse_emissive` reads
//!   (`docs/formats/rcsmaterial.md`, "A surface scrolls off an engine
//!   `time`"), so the layer is HD's: [`crate::mesh::Emissive`] and
//!   [`crate::mesh::slots::ADD_SECOND`], sampled at `(u, (v + offset) * scale +
//!   time * rate)` and added to the albedo, gated by the diffuse alpha, tinted
//!   by `GlowTint`.
//! - **The plain V scroll** - `speed_multipliaer` on a single-texture material
//!   (`fc01_effects_vscroll_emissive`, `fc06_effects_vscroll_emissive_alpha`),
//!   played as a texture-transform track.
//!
//! **Chosen, not measured**, and said so in the report: what `rate` is for the
//! glow layer (`TimeScaler` where authored, else the authored `time` value,
//! else `1.0` - the engine clock, which is HD's own default), and the
//! **direction** of the plain scroll (`+v`), because no bytecode was read to
//! settle either. Every material that names `time` and none of these (the bulk
//! of the disc's 188 `uv_anim`/`uvanim` entries) is **drawn still**, the way
//! HD's own reading draws a material whose constants it has not recovered:
//! an invented rate would be a stand-in for a number nobody read.

use std::sync::Arc;

use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2::material::Material;

use super::{Report, decode_material_texture};
use crate::mesh::rcs::Textures;
use crate::mesh::{AnimTrack, Emissive, ModelTexture, slots};

/// One material's glow layer: its index into the model's table, plus one, the
/// decoded emissive texture and the path of the diffuse it is added to.
pub(super) struct Layer {
    pub slot: u32,
    pub texture: Arc<ModelTexture>,
    pub diffuse: String,
}

/// Every material's scroll, resolved once before any vertex is written.
#[derive(Default)]
pub(super) struct Plan {
    pub emissive: Vec<Emissive>,
    pub layers: Vec<Option<Layer>>,
    pub tracks: Vec<AnimTrack>,
    /// Per material: the index into [`Self::tracks`], plus one; `0` for none.
    pub scroll: Vec<u32>,
}

impl Plan {
    /// The bits a vertex of material `index` carries for its glow layer.
    pub fn role(&self, index: Option<usize>) -> u32 {
        index
            .and_then(|i| self.layers.get(i))
            .and_then(Option::as_ref)
            .map_or(0, |l| slots::ADD_SECOND | (l.slot << slots::MATERIAL_SHIFT))
    }

    pub fn anim(&self, index: Option<usize>) -> u32 {
        index.and_then(|i| self.scroll.get(i)).copied().unwrap_or(0)
    }

    pub fn has_layers(&self) -> bool {
        self.layers.iter().any(Option::is_some)
    }
}

const SPEED: &str = "speed_multipliaer";
const TIME: &str = "time";
const TIME_SCALER: &str = "TimeScaler";
const OFFSET: &str = "Emissive_UV_Offset";
const SCALE: &str = "Emissive_UV_Scale";
const TINT: &str = "GlowTint";
const EMISSIVE_SAMPLERS: [&str; 2] = ["EmissiveTexture", "EmissiveMap"];
const DIFFUSE_SAMPLERS: [&str; 3] = ["DiffuseTexture", "DiffuseAlphaMap", "DiffuseMap"];

fn scalar(material: &Material, name: &str) -> Option<f32> {
    material.param(name_hash(name))?.first().copied()
}

fn sampler<'a>(material: &'a Material, names: &[&str]) -> Option<&'a str> {
    names.iter().find_map(|n| material.sampler(name_hash(n)))
}

/// Resolves every material's glow layer and scroll track.
pub(super) fn plan(materials: &[Material], textures: Textures<'_>, report: &mut Report) -> Plan {
    let mut out = Plan::default();
    let mut decoded: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        std::collections::HashMap::new();
    for material in materials {
        let layer = layer_of(material).and_then(|(emissive, path, diffuse)| {
            let texture = decoded
                .entry(path.clone())
                .or_insert_with(|| {
                    textures(&path)
                        .and_then(|blob| decode_material_texture(&path, &blob, report))
                        .map(Arc::new)
                })
                .clone()?;
            let at = out
                .emissive
                .iter()
                .position(|seen| *seen == emissive)
                .or_else(|| {
                    (out.emissive.len() + 1 < crate::mesh::rcs::EMISSIVE_LIMIT).then(|| {
                        out.emissive.push(emissive);
                        out.emissive.len() - 1
                    })
                })?;
            Some(Layer {
                slot: u32::try_from(at + 1).ok()?,
                texture,
                diffuse,
            })
        });
        report.glow_layers += usize::from(layer.is_some());
        out.layers.push(layer);

        let scroll = scroll_of(material).and_then(|rate| {
            let track = AnimTrack::Scroll([0.0, rate]);
            let at = out
                .tracks
                .iter()
                .position(|seen| matches!(seen, AnimTrack::Scroll(r) if *r == [0.0, rate]))
                .or_else(|| {
                    (out.tracks.len() + 1 < crate::mesh::ANIM_TRACK_LIMIT).then(|| {
                        out.tracks.push(track);
                        out.tracks.len() - 1
                    })
                })?;
            u32::try_from(at + 1).ok()
        });
        report.scrolling_materials += usize::from(scroll.is_some());
        out.scroll.push(scroll.unwrap_or(0));
    }
    out
}

/// The glow layer a material authors, with its emissive and diffuse paths.
fn layer_of(material: &Material) -> Option<(Emissive, String, String)> {
    let offset = scalar(material, OFFSET)?;
    let scale = scalar(material, SCALE)?;
    let emissive = sampler(material, &EMISSIVE_SAMPLERS)?;
    let diffuse = sampler(material, &DIFFUSE_SAMPLERS)?;
    let tint = material
        .param(name_hash(TINT))
        .filter(|v| v.len() >= 3)
        .map_or([1.0; 3], |v| [v[0], v[1], v[2]]);
    let rate = scalar(material, TIME_SCALER)
        .or_else(|| scalar(material, TIME))
        .unwrap_or(1.0);
    Some((
        Emissive {
            tint,
            offset,
            scale,
            rate,
        },
        emissive.to_string(),
        diffuse.to_string(),
    ))
}

/// The V rate of a plain scroll: `speed_multipliaer` on a material that names
/// neither half of the glow layer.
fn scroll_of(material: &Material) -> Option<f32> {
    if material.param(name_hash(OFFSET)).is_some() {
        return None;
    }
    scalar(material, SPEED).filter(|r| r.is_finite() && *r != 0.0)
}
