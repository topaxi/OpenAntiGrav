//! HD's `Track Select` circuit model: where the camera puts it and how it
//! turns.
//!
//! **Authored**, on the screen's `<Model name="TrackModel">` widget:
//! `OriginX=1308 OriginY=440` and `z=-180`. 1308 and 440 are the centre of the
//! `CIRCUIT MODEL` frame in the 1920 by 1080 grid (the frame spans 862 to 1747
//! across and 180 to 675 down), so they are an absolute screen point, the
//! reading [`crate::flyer::flyer_view_projection`] already makes of the same
//! widget class, and the model sits `-z` in front of the camera.
//!
//! **Chosen, not measured**: the pitch ([`PITCH`]), the turntable's rate
//! ([`TURN_SECONDS`]) and its starting yaw ([`YAW0`]), and the near and far
//! planes. The widget authors none of them. RPCS3 frames of the screen show
//! the circuit seen from above and at a different angle on each visit, so it
//! turns; they do not give a rate.

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
/// How far the circuit is tipped towards the camera, radians (**chosen**).
pub const PITCH: f32 = 0.55;
/// Seconds per revolution (**chosen**).
pub const TURN_SECONDS: f32 = 24.0;
/// The yaw at the screen's first tick, radians (**chosen**).
pub const YAW0: f32 = 0.6;
/// Near and far planes (**chosen**; the model sits within 180 +- 80 units).
const DEPTH: [f32; 2] = [10.0, 2000.0];

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
        seconds: f32,
    ) -> bool {
        let (view_projection, model) = matrices(widget, space, seconds);
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

/// The view-projection and model matrices of the circuit model `seconds` into
/// the screen, for [`crate::preview::Preview::draw_matrices`].
#[must_use]
pub fn matrices(widget: &TrackModel, space: Space, seconds: f32) -> (Mat4, Mat4) {
    let projection = camera::perspective(FOV_Y, space.display_aspect, DEPTH[0], DEPTH[1]);
    let shift = Vec3::new(
        2.0 * widget.origin[0] / space.size.0 - 1.0,
        1.0 - 2.0 * widget.origin[1] / space.size.1,
        0.0,
    );
    let yaw = YAW0 + seconds * std::f32::consts::TAU / TURN_SECONDS;
    let model = Mat4::from_translation(Vec3::new(0.0, 0.0, widget.z))
        * Mat4::from_rotation_x(PITCH)
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
            Texels::Rgba8(texels) => texels
                .chunks_exact(4)
                .map(|texel| [texel[0], texel[1], texel[2], texel[3]])
                .collect(),
            Texels::Chain(levels) => levels
                .first()
                .map(|level| {
                    level
                        .chunks_exact(4)
                        .map(|texel| [texel[0], texel[1], texel[2], texel[3]])
                        .collect()
                })
                .unwrap_or_default(),
            Texels::Blocks { format, levels } => levels
                .first()
                .and_then(|level| format.decode_level(level, texture.width, texture.height))
                .unwrap_or_default(),
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
        }));
        // The colour this writes is the whole surface colour, a tint on the
        // white texture and not HD's baked light term.
        model.vertex_colour_is_light = false;
        for vertex in &mut model.vertices {
            vertex.lit = 0.0;
        }
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
