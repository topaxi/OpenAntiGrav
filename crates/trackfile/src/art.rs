//! A track's art: what the renderer draws and the simulation never reads
//! (ADR-0076). Meshes with baked light, the textures they name, a sun, an
//! ambient colour, a sky borrowed by name and Pure's lens flare.
//!
//! The shapes mirror what Pulse and Pure's renderer takes (`oag_mesh`'s vertex
//! and draw call), not glTF's PBR: a vertex colour is baked light, a material is
//! a texture, a blend class and two flags.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::Error;

/// How far a direction may be from unit length before it is refused.
const UNIT_TOLERANCE: f32 = 1e-3;

/// A track's art. Absent from a [`crate::Track`] when the file has none, which
/// races as flat-shaded collision.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Art {
    /// Art meshes, one draw each, in file order.
    pub meshes: Vec<ArtMesh>,
    /// The textures the meshes and the flare name by index.
    pub textures: Vec<TextureSource>,
    /// A host circuit's `.vex` entry whose authored `Skycube` is drawn, or none.
    pub sky: Option<String>,
    /// The track's one real light.
    pub sun: Option<Sun>,
    /// The ambient light, linear RGB.
    pub ambient: Option<[f32; 3]>,
    /// Pure's sun flare along the sun's direction.
    pub lens_flare: Option<LensFlare>,
}

/// One draw: a run of triangles with one material.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtMesh {
    /// The author's name for it (the glb node is `oag/art/<index>`).
    pub name: String,
    /// World-space positions.
    pub positions: Vec<[f32; 3]>,
    /// One unit normal per position.
    pub normals: Vec<[f32; 3]>,
    /// One texture coordinate per position, texture v down as the disc's are.
    pub texcoords: Vec<[f32; 2]>,
    /// One linear RGBA per position: the baked light, which tints the texture.
    pub colours: Vec<[f32; 4]>,
    /// Triangle list into the vertex arrays.
    pub indices: Vec<u32>,
    /// How it is drawn.
    pub material: ArtMaterial,
    /// What it does with the sun's shadows.
    pub shadow: ShadowRole,
}

/// A mesh's material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtMaterial {
    /// An index into [`Art::textures`], or none for untextured.
    pub texture: Option<usize>,
    /// The blend class.
    pub blend: Blend,
    /// Whether back faces are culled.
    pub culled: bool,
    /// Whether the viewer's light rig applies on top of the baked colour. A
    /// track's baked colours are the light, so this is off for them.
    pub lit: bool,
}

/// The blend classes `oag_vex::vex::BlendClass` and the alpha-test pass name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    /// Depth write on, no blending.
    Opaque,
    /// Depth write on, discard below the alpha-test reference.
    Cutout,
    /// An ordinary alpha blend.
    Alpha,
    /// Additive, source-alpha weighted.
    Additive,
    /// The unblended transparent class.
    Unblended,
}

impl Blend {
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Self::Opaque => "opaque",
            Self::Cutout => "cutout",
            Self::Alpha => "alpha",
            Self::Additive => "additive",
            Self::Unblended => "unblended",
        }
    }

    pub(crate) fn from_tag(tag: &str) -> Option<Self> {
        [
            Self::Opaque,
            Self::Cutout,
            Self::Alpha,
            Self::Additive,
            Self::Unblended,
        ]
        .into_iter()
        .find(|b| b.tag() == tag)
    }
}

/// Whether a mesh casts the sun's shadows, receives them, both or neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadowRole {
    /// Casts and receives.
    #[default]
    Both,
    /// Casts only: a sky dome has no business receiving.
    Cast,
    /// Receives only: a decal that casts is wrong.
    Receive,
    /// Neither.
    None,
}

impl ShadowRole {
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Cast => "cast",
            Self::Receive => "receive",
            Self::None => "none",
        }
    }

    pub(crate) fn from_tag(tag: &str) -> Option<Self> {
        [Self::Both, Self::Cast, Self::Receive, Self::None]
            .into_iter()
            .find(|r| r.tag() == tag)
    }
}

/// Where a texture's pixels come from. Exactly one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextureSource {
    /// The author's own picture: a file beside the manifest. The only form a
    /// committed fixture may use.
    Image(String),
    /// A named `Texture` node of a circuit on the host title's disc, resolved at
    /// race load. The file holds the name, never the pixels.
    Host {
        /// The circuit's `.vex` entry, e.g. `Data\Tracks\03_Track.vex`.
        entry: String,
        /// The texture node's own name.
        name: String,
    },
}

/// The track's sun.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Sun {
    /// Unit vector from the scene towards the sun.
    pub direction: [f32; 3],
    /// Linear RGB.
    pub colour: [f32; 3],
    /// A multiplier on the colour.
    pub intensity: f32,
    /// The sun's angular diameter in degrees: `0` is a point (a hard edge).
    pub angular_diameter: f32,
    /// Whether it may cast shadows.
    pub shadows: bool,
}

/// Pure's sun flare. The ten-ring layout, the fade by angle and the occlusion
/// probe are the renderer's own law.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct LensFlare {
    /// Overrides the sun's colour when given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<[f32; 3]>,
    /// An index into [`Art::textures`]: the 2x2 atlas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub atlas: Option<usize>,
}

/// The manifest's `[art]` table: everything of [`Art`] that is not a glb mesh.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct ArtTable {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sky: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambient: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sun: Option<Sun>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lensflare: Option<LensFlare>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub texture: Vec<TextureSource>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

impl ArtTable {
    pub(crate) fn of(art: &Art) -> Self {
        Self {
            sky: art.sky.clone(),
            ambient: art.ambient,
            sun: art.sun,
            lensflare: art.lens_flare,
            texture: art.textures.clone(),
            extra: BTreeMap::new(),
        }
    }

    pub(crate) fn into_art(self, meshes: Vec<ArtMesh>) -> Art {
        Art {
            meshes,
            textures: self.texture,
            sky: self.sky,
            sun: self.sun,
            ambient: self.ambient,
            lens_flare: self.lensflare,
        }
    }
}

fn finite(what: &str, field: &str, values: impl IntoIterator<Item = f32>) -> Result<(), Error> {
    if values.into_iter().all(f32::is_finite) {
        Ok(())
    } else {
        Err(Error::NonFinite {
            record: what.to_string(),
            field: field.to_string(),
        })
    }
}

fn range(what: &str, why: String) -> Error {
    Error::Range {
        what: what.to_string(),
        why,
    }
}

impl Art {
    /// Checks everything a reader or writer relies on: lengths agree, indices and
    /// texture references are in range, numbers are finite, a direction is a unit
    /// vector.
    ///
    /// # Errors
    ///
    /// The first thing wrong, named.
    pub fn validate(&self) -> Result<(), Error> {
        for (i, t) in self.textures.iter().enumerate() {
            let what = format!("art texture {i}");
            match t {
                TextureSource::Image(file) if file.is_empty() => {
                    return Err(range(&what, "an empty image file name".into()));
                }
                TextureSource::Host { entry, name } if entry.is_empty() || name.is_empty() => {
                    return Err(range(
                        &what,
                        "a host texture needs an entry and a name".into(),
                    ));
                }
                _ => {}
            }
        }
        let texture = |what: &str, i: usize| -> Result<(), Error> {
            if i < self.textures.len() {
                Ok(())
            } else {
                Err(range(
                    what,
                    format!("texture {i}, the file lists {}", self.textures.len()),
                ))
            }
        };
        for (n, m) in self.meshes.iter().enumerate() {
            let what = format!("art mesh {n} ({})", m.name);
            let v = m.positions.len();
            for (field, len) in [
                ("normals", m.normals.len()),
                ("texcoords", m.texcoords.len()),
                ("colours", m.colours.len()),
            ] {
                if len != v {
                    return Err(Error::accessor(
                        &what,
                        field,
                        format!("{len} elements, positions has {v}"),
                    ));
                }
            }
            if v == 0 || m.indices.is_empty() {
                return Err(Error::Unwritable(format!("{what} is empty")));
            }
            if m.indices.len() % 3 != 0 {
                return Err(Error::accessor(&what, "indices", "not a multiple of 3"));
            }
            if m.indices.iter().any(|&i| i as usize >= v) {
                return Err(Error::accessor(
                    &what,
                    "indices",
                    "an index is past the vertices",
                ));
            }
            finite(&what, "positions", m.positions.iter().flatten().copied())?;
            finite(&what, "normals", m.normals.iter().flatten().copied())?;
            finite(&what, "texcoords", m.texcoords.iter().flatten().copied())?;
            finite(&what, "colours", m.colours.iter().flatten().copied())?;
            if let Some(t) = m.material.texture {
                texture(&what, t)?;
            }
        }
        if let Some(sun) = &self.sun {
            finite(
                "art sun",
                "values",
                sun.direction
                    .iter()
                    .chain(&sun.colour)
                    .chain(&[sun.intensity, sun.angular_diameter])
                    .copied(),
            )?;
            let len = sun.direction.iter().map(|c| c * c).sum::<f32>().sqrt();
            if (len - 1.0).abs() > UNIT_TOLERANCE {
                return Err(range(
                    "art sun",
                    format!("direction has length {len}, not 1"),
                ));
            }
            if sun.colour.iter().any(|&c| c < 0.0) || sun.intensity < 0.0 {
                return Err(range("art sun", "a negative colour or intensity".into()));
            }
            if !(0.0..=180.0).contains(&sun.angular_diameter) {
                return Err(range(
                    "art sun",
                    format!(
                        "angular diameter {} is not in 0..=180",
                        sun.angular_diameter
                    ),
                ));
            }
        }
        if let Some(a) = &self.ambient {
            finite("art ambient", "colour", a.iter().copied())?;
            if a.iter().any(|&c| c < 0.0) {
                return Err(range("art ambient", "a negative colour".into()));
            }
        }
        if let Some(flare) = &self.lens_flare {
            if let Some(c) = &flare.colour {
                finite("art lensflare", "colour", c.iter().copied())?;
            }
            if let Some(a) = flare.atlas {
                texture("art lensflare", a)?;
            }
            if flare.colour.is_none() && self.sun.is_none() {
                return Err(range(
                    "art lensflare",
                    "a flare lies along the sun's direction and the file has no sun".into(),
                ));
            }
        }
        Ok(())
    }
}
