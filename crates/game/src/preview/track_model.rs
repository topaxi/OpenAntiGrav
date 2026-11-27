//! HD's `Track Select` circuit model: where the camera puts it and how it
//! turns.
//!
//! **Authored**, on the screen's `<Model name="TrackModel">` widget:
//! `OriginX=1308 OriginY=440` and `z=-180`. Omega authors the same widget with
//! `OriginX=960 OriginY=540`, `x=0.525 y=0.125 z=-2.05` and `orthoScaleX/Y/Z=0.012`,
//! which stand 43.8, 10.4 and -170.8 units from the axis. Its `FoV=100` and
//! `nearZ=0.1` are not read: the field of view stays [`FOV_Y`] (see that). 1308 and 440 are the centre of the
//! `CIRCUIT MODEL` frame in the 1920 by 1080 grid (the frame spans 862 to 1747
//! across and 180 to 675 down), so they are an absolute screen point, the
//! reading [`crate::flyer::flyer_view_projection`] already makes of the same
//! widget class, and the model sits `-z` in front of the camera.
//!
//! **Measured**, on a title whose screen update is read
//! ([`oag_title::FrontEnd::circuit_turntable`], HD): the update overwrites the widget's
//! `RotX`, `z` and `OriginY` with the selected circuit's own `RotX`, `PosZ` and
//! `PosY + 440` ([`oag_title::CircuitPose`]) and its `RotY` with one global angle, stepped
//! 0.01 rad every frame and wrapped at +-pi, backwards on a reversed circuit
//! ([`Turntable`]). `docs/ghidra/functions/ps3-hdfury-eu/track-selection-screen.md`, "The
//! circuit model's update".
//!
//! **Chosen, not measured**: on a title whose update is not read (Omega), the pitch
//! ([`PITCH`]), the turntable's rate ([`TURN_SECONDS`]) and its starting yaw ([`YAW0`]);
//! on both, the near and far planes.

use std::sync::Arc;

use anyhow::{Context, Result, bail};
use oag_core::math::{Mat4, Vec3, camera};
use oag_display::space::Space;
use oag_mesh::mesh::{GpuVertex, Model, ModelTexture, Texels};
use oag_ui_screens::picker::hd::track::TrackModel;

/// The vertical field of view, radians: the same **1.0** `Flyer_Item`'s render
/// function builds from `tanf(0.5)` (see [`crate::flyer::FOV_Y`]). That the
/// circuit-model widget uses the same literal is **chosen, not measured**.
pub const FOV_Y: f32 = crate::flyer::FOV_Y;
/// How far the circuit is tipped towards the camera where the title's update is unread,
/// radians (**chosen**).
pub const PITCH: f32 = 0.55;
/// Seconds per revolution where the title's update is unread (**chosen**).
pub const TURN_SECONDS: f32 = 24.0;
/// The yaw at the screen's first tick where the title's update is unread, radians
/// (**chosen**).
pub const YAW0: f32 = 0.6;
/// Near and far planes (**chosen**; the model sits within 180 +- 80 units).
const DEPTH: [f32; 2] = [10.0, 2000.0];

/// How the circuit model is posed this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Posing {
    /// The screen's own update: the circuit's pose, the law it is applied by and the
    /// turntable's angle.
    Entry {
        pose: oag_title::CircuitPose,
        law: oag_title::CircuitTurntable,
        yaw: f32,
    },
    /// The widget as authored, turned at the stand-in rate `seconds` into the screen.
    Standin { seconds: f32 },
}

impl Posing {
    /// How a screen `seconds` in poses the selected circuit: by its entry's `pose` where the
    /// title has the update's `law` and the entry authors a pose, else the stand-in.
    #[must_use]
    pub fn of(
        law: Option<oag_title::CircuitTurntable>,
        pose: Option<oag_title::CircuitPose>,
        turntable: &Turntable,
        seconds: f32,
    ) -> Self {
        match law.zip(pose) {
            Some((law, pose)) => Self::Entry {
                pose,
                law,
                yaw: turntable.angle(),
            },
            None => Self::Standin { seconds },
        }
    }
}

/// The pose the selected entry of a track picker authors for its circuit model and whether it is
/// the reversed one; `(None, false)` for anything that is not a circuit entry.
#[must_use]
pub fn selected_circuit(
    picker: &oag_ui_screens::picker::Picker,
) -> (Option<oag_title::CircuitPose>, bool) {
    match picker.entries().get(picker.index()).map(|e| &e.details) {
        Some(oag_ui_screens::picker::Details::Track {
            model_pose,
            reversed,
            ..
        }) => (*model_pose, *reversed),
        _ => (None, false),
    }
}

impl Posing {
    /// How a track picker `seconds` into its screen poses its circuit model, counting the
    /// turntable from zero in the selected circuit's direction: what a capture draws.
    #[must_use]
    pub fn after(
        front_end: Option<&oag_title::FrontEnd>,
        picker: &oag_ui_screens::picker::Picker,
        seconds: f32,
    ) -> Self {
        let law = front_end.and_then(|front_end| front_end.circuit_turntable);
        let (pose, reversed) = selected_circuit(picker);
        let turntable = law
            .map(|law| Turntable::after(&law, seconds, reversed))
            .unwrap_or_default();
        Self::of(law, pose, &turntable, seconds)
    }
}

/// The global angle the screen's update turns the circuit by: a step per frame, forwards on a
/// circuit and backwards on a reversed one, wrapped at the law's limit.
///
/// Counted in whole frames of the original's update (`dt` times
/// [`oag_title::CircuitTurntable::frames_per_second`], the remainder carried), because the
/// step is per frame and not per second.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Turntable {
    angle: f32,
    carry: f32,
}

impl Turntable {
    /// The angle `seconds` of a screen's frames turn a table that started at zero, in the one
    /// direction.
    #[must_use]
    pub fn after(law: &oag_title::CircuitTurntable, seconds: f32, reversed: bool) -> Self {
        let mut table = Self::default();
        table.advance(law, seconds, reversed);
        table
    }

    /// The angle, radians in `-limit..=limit`: the widget's `RotY`.
    #[must_use]
    pub fn angle(&self) -> f32 {
        self.angle
    }

    /// Runs `seconds` of the screen's frames.
    pub fn advance(&mut self, law: &oag_title::CircuitTurntable, seconds: f32, reversed: bool) {
        // A tick that is exactly one frame must not land a rounding error short of it.
        const EPSILON: f32 = 1e-3;
        self.carry += seconds * law.frames_per_second;
        while self.carry >= 1.0 - EPSILON {
            self.carry -= 1.0;
            self.frame(law, reversed);
        }
    }

    fn frame(&mut self, law: &oag_title::CircuitTurntable, reversed: bool) {
        self.angle += if reversed { -law.step } else { law.step };
        if self.angle > law.limit {
            self.angle = -law.limit;
        } else if self.angle < -law.limit {
            self.angle = law.limit;
        }
    }
}

impl crate::preview::Preview {
    /// Draws the circuit model `seconds` into the screen: its own camera, the
    /// turntable, and (through [`crate::preview::Preview::with_ramp`]) its own
    /// material. `false` when there is nothing to draw.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_track_model(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
        space: Space,
        widget: &TrackModel,
        posing: Posing,
        seconds: f32,
    ) -> bool {
        let (view_projection, model) = matrices(widget, space, posing);
        self.draw_matrices(
            device,
            queue,
            encoder,
            view,
            viewport,
            target_size,
            space,
            view_projection,
            model,
            seconds,
        )
    }
}

/// The view-projection and model matrices of the circuit model, for
/// [`crate::preview::Preview::draw_matrices`].
#[must_use]
pub fn matrices(widget: &TrackModel, space: Space, posing: Posing) -> (Mat4, Mat4) {
    let projection = camera::perspective(FOV_Y, space.display_aspect, DEPTH[0], DEPTH[1]);
    let (origin_y, z, pitch, yaw) = match posing {
        Posing::Entry { pose, law, yaw } => {
            (law.origin_y + pose.pos_y, pose.pos_z, pose.rot_x, yaw)
        }
        Posing::Standin { seconds } => (
            widget.origin[1],
            widget.z,
            PITCH,
            YAW0 + seconds * std::f32::consts::TAU / TURN_SECONDS,
        ),
    };
    let shift = Vec3::new(
        2.0 * widget.origin[0] / space.size.0 - 1.0,
        1.0 - 2.0 * origin_y / space.size.1,
        0.0,
    );
    // `orthoScale` scales the placement, not the model: the same reading the
    // campaign flyer's widget gets (`crate::flyer::campaign_pose`).
    let scale = widget.ortho_scale;
    let model = Mat4::from_translation(Vec3::new(
        widget.offset[0] / scale[0],
        widget.offset[1] / scale[1],
        z / scale[2],
    )) * Mat4::from_rotation_x(pitch)
        * Mat4::from_rotation_y(yaw);
    (Mat4::from_translation(shift) * projection, model)
}

/// `cf_fetracks.rcsmaterial`'s own colour, `ramp(N.V) * tint`, with no light
/// and no blend.
///
/// **Read off the material** (`ps3-microcode.py fp-file`, fragment block #2,
/// the unfogged variant): `DP3`/`DIVSQ` normalise the two interpolators and
/// dot them, `TEX H0.xyz, R0.xxxx unit0` samples the one sampler (`fe_grad.gtf`)
/// at `(N.V, N.V)`, and `MUL H0.xyz, H0, 0x81db67ea` scales it. The rest of the
/// model's state word (`0x006c`/`0x007c`, low two bits 0) says opaque. The
/// ramp is a horizontal gradient from pale blue-grey at `N.V = 0` to dark teal
/// at `N.V = 1`, so a surface facing the camera is dark and a grazing one is
/// bright. The program applies no transfer function, so the sample is the
/// output, which is the stand-in path the mesh shader takes for a prelit
/// vertex (`GpuVertex::lit == 0`) under a white texture.
///
/// Evaluated **per vertex, once a frame**, on the CPU: the mesh shader's
/// material word has no free bit for a ramp-by-facing material (its role bits
/// run out at 22 and the material index starts at 23), and the circuit is one
/// static mesh under one fixed camera.
///
/// **Not read**: the multiplier `0x81db67ea`'s value, which the model's
/// material authors none for and the screen's code presumably sets. It is
/// taken as 1 (**chosen, not measured**); with it the model reads as the
/// neutral grey RPCS3's frames show, 85 to 200 against this ramp's 104 to 197.
///
/// **Which ramp**: `fe_grad.gtf` is on `DATA00` (grey, 197 to 104) and on
/// `DATA02` (blue-grey, 177/198/222 to 84/118/123) for the same eight base
/// circuits; the front end's mount order serves `DATA00`'s, which is the grey
/// the frames show.
#[derive(Debug, Clone)]
pub struct Ramp {
    /// The ramp texture's middle row, left (`N.V = 0`) to right.
    row: Vec<[f32; 3]>,
}

impl Ramp {
    /// The ramp of the circuit model `entry`, whichever material it names: a
    /// PS4 `.rcsmodel` (Omega) carries `FrontEndConstantFranelBlend`'s
    /// uniforms ([`Self::fresnel`]), HD's names `cf_fetracks` and a texture
    /// ([`Self::take`]).
    ///
    /// # Errors
    ///
    /// As [`Self::take`] and [`Self::fresnel`].
    pub fn of(archives: &mut oag_assets::Archives, entry: &str, model: &mut Model) -> Result<Self> {
        let geometry = oag_mesh::mesh::rcs::sibling_name(entry)
            .and_then(|sibling| archives.read_name(&sibling).ok());
        match geometry {
            Some(blob) if oag_mesh::mesh::rcs::psp2::is_psp2(&blob) => {
                let parsed = oag_rcs::rcsmodel::psp2::parse(&blob)
                    .with_context(|| format!("{entry}: reading its materials"))?;
                let material = parsed
                    .materials
                    .first()
                    .with_context(|| format!("{entry}: the circuit model has no material"))?;
                Self::fresnel(material, model)
            }
            _ => Self::take(model),
        }
    }

    /// Omega's circuit model colour, `FrontEndConstantFranelBlend`'s own
    /// pixel shader (`data00.psarc`, read as GCN, the `Shdr` at `0x323c` of
    /// `15_anulpha_pass`'s copy):
    /// `colourDiffAlpha + Constant1 * constantAmbientColour * refl`, with
    /// `refl = ReflectivityMin + ReflectivityScale * (1 - N.V)^ReflectivityPower`
    /// (`1 - N.V` clamped to 0..1), then fogged by the vertex's own factor.
    /// The model's one material authors all of them but the ambient colour
    /// (`Constant1` 1 1 1, `colourDiffAlpha` 0.06 0.048 0.029, min 0.05, scale
    /// 1, power 3 on Anulpha Pass), so the circuit is a dark warm base with a
    /// bright rim. Unlit, like [`Self::take`]'s.
    ///
    /// **Chosen, not measured**: `constantAmbientColour`, which no instance
    /// authors (taken as 1, as the multiplier of HD's `cf_fetracks` is), and
    /// no fog, which the screen's own state would set.
    ///
    /// # Errors
    ///
    /// The material authors none of the uniforms.
    pub fn fresnel(
        material: &oag_rcs::rcsmodel::psp2::material::Material,
        model: &mut Model,
    ) -> Result<Self> {
        let uniform = |name: &str, width: usize| -> Result<Vec<f32>> {
            material
                .param(oag_rcs::rcsmaterial::name_hash(name))
                .filter(|value| value.len() >= width)
                .with_context(|| format!("the material authors no {name}"))
        };
        let base = uniform("colourDiffAlpha", 3)?;
        let constant = uniform("Constant1", 3)?;
        let (min, scale, power) = (
            uniform("ReflectivityMin", 1)?[0],
            uniform("ReflectivityScale", 1)?[0],
            uniform("ReflectivityPower", 1)?[0],
        );
        let ambient = uniform("constantAmbientColour", 3).unwrap_or_else(|_| vec![1.0; 3]);
        const STEPS: usize = 256;
        let row = (0..STEPS)
            .map(|step| {
                let rim = 1.0 - step as f32 / (STEPS - 1) as f32;
                let reflected = min + scale * rim.powf(power);
                std::array::from_fn(|c| base[c] + constant[c] * ambient[c] * reflected)
            })
            .collect();
        Self::unlit(model);
        Ok(Self { row })
    }

    /// Leaves `model` drawn by its vertex colour alone.
    fn unlit(model: &mut Model) {
        model.vertex_colour_is_light = false;
        for vertex in &mut model.vertices {
            vertex.lit = 0.0;
        }
    }

    /// Takes the ramp out of `model`'s first texture and leaves a white one
    /// in its place, so the shader's `texture * colour` is the colour alone.
    ///
    /// # Errors
    ///
    /// The model has no first texture or it is not a plain RGBA8 picture.
    pub fn take(model: &mut Model) -> Result<Self> {
        let Some(slot) = model.textures.first_mut() else {
            bail!("the circuit model names no texture for its ramp");
        };
        let texture = slot.take().context("the ramp texture did not load")?;
        let (width, height) = (texture.width as usize, texture.height as usize);
        let decoded: Vec<[u8; 4]> = match &texture.texels {
            Texels::Rgba8(texels) => texels.as_chunks::<4>().0.to_vec(),
            Texels::Chain(levels) => levels
                .first()
                .map(|level| level.as_chunks::<4>().0.to_vec())
                .unwrap_or_default(),
            Texels::Blocks { format, levels } => levels
                .first()
                .and_then(|level| format.decode_level(level, texture.width, texture.height))
                .unwrap_or_default(),
            #[cfg(not(target_arch = "wasm32"))]
            Texels::Uploaded { .. } => Vec::new(),
        };
        if width == 0 || height == 0 || decoded.len() < width * height {
            bail!("the ramp texture could not be read back as a picture");
        }
        let y = height / 2;
        let row: Vec<[f32; 3]> = (0..width)
            .map(|x| {
                let texel = decoded[y * width + x];
                std::array::from_fn(|c| f32::from(texel[c]) / 255.0)
            })
            .collect();
        *slot = Some(Arc::new(ModelTexture {
            label: "circuit ramp, white".to_string(),
            width: 1,
            height: 1,
            texels: Texels::Rgba8(vec![255; 4]),
            mip_count: None,
            lod_code: 0.0,
        }));
        // The colour this writes is the whole surface colour, a tint on the
        // white texture and not HD's baked light term.
        Self::unlit(model);
        Ok(Self { row })
    }

    /// The ramp's colour at `facing = N.V`, clamped to the picture.
    #[must_use]
    pub fn at(&self, facing: f32) -> [f32; 3] {
        let last = self.row.len() - 1;
        let at = (facing.clamp(0.0, 1.0) * last as f32).round() as usize;
        self.row[at.min(last)]
    }

    /// Writes every vertex's colour for the circuit seen through `model`
    /// from a camera at the origin.
    pub fn shade(&self, vertices: &mut [GpuVertex], model: Mat4) {
        let normal_matrix = oag_core::math::Mat3::from_mat4(model);
        for vertex in vertices {
            let world = model.transform_point3(Vec3::from(vertex.position));
            let normal = (normal_matrix * Vec3::from(vertex.normal)).normalize_or_zero();
            let facing = normal.dot((-world).normalize_or_zero());
            let ramp = self.at(facing);
            vertex.colour = [ramp[0], ramp[1], ramp[2], 1.0];
        }
    }
}

#[cfg(test)]
mod tests;
