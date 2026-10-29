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

pub mod motion;

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
    /// The track's own sky, or none: never together with [`Self::sky`].
    pub skybox: Option<Skybox>,
    /// The track's one real light.
    pub sun: Option<Sun>,
    /// The ambient light, linear RGB.
    pub ambient: Option<[f32; 3]>,
    /// Lights beyond the sun and the ambient, in file order: further
    /// directionals (the second and later of a disc circuit's).
    pub lights: Vec<Light>,
    /// Pure's sun flare along the sun's direction.
    pub lens_flare: Option<LensFlare>,
    /// Keyframed motion of the meshes and the sun, played against the race
    /// clock (decision 10). Visual only.
    pub animations: Vec<motion::Animation>,
    /// The sun node's rest rotation (`x, y, z, w`), present exactly when an
    /// animation turns the sun: its direction at a time is [`Sun::direction`]
    /// turned by the rotation from this to the node's rotation then.
    pub sun_rotation: Option<[f32; 4]>,
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
    /// One RGBA per position: the baked light, which tints the texture. RGB is
    /// linear (glTF's `COLOR_0`), alpha is coverage and on no curve.
    pub colours: Vec<[f32; 4]>,
    /// Triangle list into the vertex arrays.
    pub indices: Vec<u32>,
    /// How it is drawn.
    pub material: ArtMaterial,
    /// What it does with the sun's shadows.
    pub shadow: ShadowRole,
    /// The render layer it is drawn in: draws go by layer, then by file order.
    /// `0` unless authored; a disc export writes the `.vex` layer.
    pub layer: u32,
    /// The node's rest pose, only on a mesh an animation moves: its vertices
    /// are then in the node's own space. `None` is the identity (a static
    /// mesh's vertices are always in world space).
    pub transform: Option<motion::Trs>,
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
    /// The alpha-test reference in `0..=255` (a texel passes above it), or
    /// none for the renderer's default. Read by the `cutout` pass.
    pub alpha_ref: Option<u8>,
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
    /// Whether it may cast shadows. `true` unless written: an authored sun is
    /// the shadow light by definition.
    #[serde(default = "yes")]
    pub shadows: bool,
}

fn yes() -> bool {
    true
}

/// An `[[art.light]]` row: a light beyond the sun and the ambient.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Light {
    /// A light from a direction, as the sun is, with no size and no shadows.
    Directional {
        /// Unit vector from the scene towards the light.
        direction: [f32; 3],
        /// Linear RGB.
        colour: [f32; 3],
        /// A multiplier on the colour.
        intensity: f32,
    },
}

/// A track's own sky (ADR-0076 decision 14): a camera-centred cube, each face
/// one of the author's textures or the vertical gradient from `horizon` to
/// `zenith`. At least one of the two forms is given; a face with no texture
/// needs the gradient.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Skybox {
    /// A texture row per face, in cubemap order `+X, -X, +Y (zenith), -Y
    /// (nadir), +Z, -Z` (the GL cubemap face table HD's own sky reads), or
    /// `None` for a face drawn as the gradient. All `None` when the file
    /// gives only a gradient.
    pub faces: [Option<usize>; 6],
    /// The gradient's colour overhead, linear RGB.
    pub zenith: Option<[f32; 3]>,
    /// The gradient's colour at and below the horizon, linear RGB.
    pub horizon: Option<[f32; 3]>,
}

impl Skybox {
    /// Whether it carries the gradient (both colours).
    #[must_use]
    pub fn gradient(&self) -> Option<([f32; 3], [f32; 3])> {
        self.zenith.zip(self.horizon)
    }
}

/// The manifest's `[art.skybox]`: `faces` as texture rows with `-1` for none
/// (TOML has no null), absent when no face has a texture.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct SkyboxTable {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faces: Option<[i64; 6]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zenith: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizon: Option<[f32; 3]>,
}

impl SkyboxTable {
    fn of(sky: &Skybox) -> Self {
        Self {
            faces: sky.faces.iter().any(Option::is_some).then(|| {
                sky.faces
                    .map(|f| f.map_or(-1, |i| i64::try_from(i).unwrap_or(i64::MAX)))
            }),
            zenith: sky.zenith,
            horizon: sky.horizon,
        }
    }

    fn into_skybox(self) -> Result<Skybox, Error> {
        let mut faces = [None; 6];
        if let Some(given) = self.faces {
            for (i, v) in given.into_iter().enumerate() {
                faces[i] = match v {
                    -1 => None,
                    v => Some(usize::try_from(v).map_err(|_| {
                        range(
                            &format!("art.skybox.faces[{i}]"),
                            format!("{v} is not -1 or a texture row"),
                        )
                    })?),
                };
            }
        }
        Ok(Skybox {
            faces,
            zenith: self.zenith,
            horizon: self.horizon,
        })
    }
}

/// Pure's sun flare. The ten-ring layout, the fade by angle and the occlusion
/// probe are the renderer's own law.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct LensFlare {
    /// Overrides the sun's direction when given (a unit vector towards the
    /// flare): Pure authors its flare apart from its first directional light.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<[f32; 3]>,
    /// Overrides the sun's colour when given, linear RGB.
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
    pub skybox: Option<SkyboxTable>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambient: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sun: Option<Sun>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lensflare: Option<LensFlare>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub light: Vec<Light>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub texture: Vec<TextureSource>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

impl ArtTable {
    pub(crate) fn of(art: &Art) -> Self {
        Self {
            sky: art.sky.clone(),
            skybox: art.skybox.as_ref().map(SkyboxTable::of),
            ambient: art.ambient,
            sun: art.sun,
            lensflare: art.lens_flare,
            light: art.lights.clone(),
            texture: art.textures.clone(),
            extra: BTreeMap::new(),
        }
    }

    pub(crate) fn into_art(self, meshes: Vec<ArtMesh>) -> Result<Art, Error> {
        Ok(Art {
            meshes,
            textures: self.texture,
            sky: self.sky,
            skybox: self.skybox.map(SkyboxTable::into_skybox).transpose()?,
            sun: self.sun,
            ambient: self.ambient,
            lights: self.light,
            lens_flare: self.lensflare,
            animations: Vec::new(),
            sun_rotation: None,
        })
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

/// A finite unit `direction`, a non-negative `colour` and `intensity`.
fn light(what: &str, direction: [f32; 3], colour: [f32; 3], intensity: f32) -> Result<(), Error> {
    finite(
        what,
        "values",
        direction.iter().chain(&colour).chain(&[intensity]).copied(),
    )?;
    unit(what, direction)?;
    if colour.iter().any(|&c| c < 0.0) || intensity < 0.0 {
        return Err(range(what, "a negative colour or intensity".into()));
    }
    Ok(())
}

fn unit(what: &str, direction: [f32; 3]) -> Result<(), Error> {
    let len = direction.iter().map(|c| c * c).sum::<f32>().sqrt();
    if (len - 1.0).abs() > UNIT_TOLERANCE {
        return Err(range(what, format!("direction has length {len}, not 1")));
    }
    Ok(())
}

fn range(what: &str, why: String) -> Error {
    Error::Range {
        what: what.to_string(),
        why,
    }
}

impl Art {
    /// Whether the art borrows anything from a title's disc by name: a host
    /// texture or a host circuit's sky. Such a track must name its host
    /// (`Track::host`, ADR-0076 decision 13).
    #[must_use]
    pub fn borrows_from_a_disc(&self) -> bool {
        self.sky.is_some()
            || self
                .textures
                .iter()
                .any(|t| matches!(t, TextureSource::Host { .. }))
    }

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
        if let Some(sky) = &self.skybox {
            let what = "art.skybox";
            if self.sky.is_some() {
                return Err(range(
                    what,
                    "a track draws one sky: the host form (art.sky) or its own (art.skybox), not both"
                        .into(),
                ));
            }
            if sky.zenith.is_some() != sky.horizon.is_some() {
                return Err(range(
                    what,
                    "the gradient needs both zenith and horizon".into(),
                ));
            }
            if let Some((zenith, horizon)) = sky.gradient() {
                finite(what, "gradient", zenith.iter().chain(&horizon).copied())?;
                if zenith.iter().chain(&horizon).any(|&c| c < 0.0) {
                    return Err(range(what, "a negative colour".into()));
                }
            }
            for (i, face) in sky.faces.iter().enumerate() {
                match face {
                    Some(t) => texture(&format!("art.skybox.faces[{i}]"), *t)?,
                    None if sky.gradient().is_none() => {
                        return Err(range(
                            &format!("art.skybox.faces[{i}]"),
                            "a face with no texture needs the gradient (zenith and horizon)".into(),
                        ));
                    }
                    None => {}
                }
            }
        }
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
            light("art sun", sun.direction, sun.colour, sun.intensity)?;
            finite("art sun", "angular_diameter", [sun.angular_diameter])?;
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
        for (i, l) in self.lights.iter().enumerate() {
            match *l {
                Light::Directional {
                    direction,
                    colour,
                    intensity,
                } => light(&format!("art light {i}"), direction, colour, intensity)?,
            }
        }
        if let Some(flare) = &self.lens_flare {
            if let Some(c) = &flare.colour {
                finite("art lensflare", "colour", c.iter().copied())?;
            }
            if let Some(d) = flare.direction {
                finite("art lensflare", "direction", d)?;
                unit("art lensflare", d)?;
            }
            if let Some(a) = flare.atlas {
                texture("art lensflare", a)?;
            }
            if (flare.colour.is_none() || flare.direction.is_none()) && self.sun.is_none() {
                return Err(range(
                    "art lensflare",
                    "a flare lies along the sun's direction and the file has no sun".into(),
                ));
            }
        }
        motion::validate(self)
    }
}
