//! The Select Team screen's hull: `Team_Definition.xml`'s `<Model name="ShipModel">`.
//!
//! A 3-D craft drawn over a 2-D front-end screen, the same pass
//! ([`crate::preview::Preview`]) every other menu mesh draws through. Evidence page:
//! `docs/ghidra/functions/vita-2048-eu-v104/team-grid.md` ("The hull preview").
//!
//! **Authored** (`Team_Definition.xml`): `OriginX=1400 OriginY=264` (a point in a
//! grid twice the screen's, so (700, 132) of 960 x 544), `x=0 y=0 z=-25`, `nearZ=1`.
//! **Read off the executable** (the screen's update, `0x8113a3b0`): the hull is
//! `<ship dir>/ship_lod.vex` (`"%s/ship_lod.vex"`), and its rotation settles at
//! (`0.546`, `-0.932`) radians (`0x3f0bc01a`-ish literals eased at 1/6 per tick
//! from 1.57 and -1.57); the field at `+0xd8` is set to `40.0`, read here as the
//! vertical field of view in degrees (confidence about 50, fitted by eye against
//! the frame). **Chosen, not measured**: the far plane, and drawing the hull over
//! the 2-D list rather than between its widgets (nothing the hull covers is
//! drawn over it on the frame).
//!
//! Not drawn: the touch turntable (`+0xa8..+0xb0` follow a drag), the livery
//! skin the player picked (`livery1` is the model's own default), and the
//! screen's slide-in.

use std::path::PathBuf;

use anyhow::{Context, Result};
use oag_core::math::{Mat4, Vec3, camera};
use oag_display::space::Space;
use oag_mesh::mesh_render::Anisotropy;

use crate::preview::Preview;

/// The vertical field of view, radians: `40.0` at `+0xd8`, read as degrees.
pub const FOV_Y: f32 = 40.0 * std::f32::consts::PI / 180.0;
/// The settled pitch (`+0xb4`) and yaw (`+0xb8`), radians.
pub const ROT_X: f32 = 0.546;
pub const ROT_Y: f32 = -0.932;
/// The authored depth, `z="-25.0"`.
pub const Z: f32 = -25.0;
/// The authored near plane.
pub const NEAR: f32 = 1.0;
/// The far plane: **chosen** (the widget authors none).
pub const FAR: f32 = 1000.0;

/// The model's entry for `team`'s craft numbered `suffix`: the screen's own
/// `<ship dir>/ship_lod.vex`.
#[must_use]
pub fn entry(ship_dir: &str, team: &str, suffix: &str) -> String {
    format!(r"{ship_dir}\{team}\{suffix}\Ship_LOD.vex")
}

/// The camera and model matrices for a widget at `origin` of a grid twice
/// `space`'s size.
#[must_use]
pub fn matrices(origin: (f32, f32), space: Space) -> (Mat4, Mat4) {
    let projection = camera::perspective(FOV_Y, space.display_aspect, NEAR, FAR);
    let shift = Vec3::new(
        origin.0 / space.size.0 - 1.0,
        1.0 - origin.1 / space.size.1,
        0.0,
    );
    let model = Mat4::from_translation(Vec3::new(0.0, 0.0, Z))
        * Mat4::from_rotation_x(ROT_X)
        * Mat4::from_rotation_y(ROT_Y);
    (Mat4::from_translation(shift) * projection, model)
}

/// The hull on its way to the screen: opens the source on first use and keeps the
/// one model the cursor is on.
pub struct TeamHull {
    source: String,
    dlc: Vec<PathBuf>,
    /// The title's native ship directory (`title.race.ship_dir`).
    ship_dir: &'static str,
    /// The widget's authored `OriginX`/`OriginY`.
    origin: (f32, f32),
    archives: Option<oag_assets::Archives>,
    gave_up: bool,
    shown: Option<(String, Preview)>,
}

impl std::fmt::Debug for TeamHull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TeamHull")
            .field("source", &self.source)
            .field("shown", &self.shown.as_ref().map(|(entry, _)| entry))
            .finish_non_exhaustive()
    }
}

impl TeamHull {
    #[must_use]
    pub fn new(
        source: String,
        dlc: Vec<PathBuf>,
        ship_dir: &'static str,
        origin: (f32, f32),
    ) -> Self {
        Self {
            source,
            dlc,
            ship_dir,
            origin,
            archives: None,
            gave_up: false,
            shown: None,
        }
    }

    fn load(&mut self, entry: &str) -> Result<oag_mesh::mesh::Model> {
        if self.archives.is_none() {
            self.archives = Some(crate::preview::open_archives(&self.source, &self.dlc)?);
        }
        let archives = self.archives.as_mut().expect("opened above");
        crate::preview::psp2_scene::model(archives, entry)
            .with_context(|| format!("reading the Select Team hull {entry}"))
    }

    /// Draws the craft `(team, suffix)` over `view`, loading its model first when the cursor has
    /// moved to another craft. A hull that will not read is logged once and
    /// nothing is drawn in its place.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
        space: Space,
        (team, suffix): (&str, &str),
    ) {
        if self.gave_up {
            return;
        }
        let entry = entry(self.ship_dir, team, suffix);
        if self.shown.as_ref().is_none_or(|(held, _)| *held != entry) {
            let built = self.load(&entry).and_then(|model| {
                Preview::new(device, queue, format, Anisotropy::default(), model)
            });
            match built {
                Ok(preview) => self.shown = Some((entry, preview)),
                Err(error) => {
                    log::warn!("{error:#} - the Select Team screen draws no hull");
                    self.gave_up = true;
                    return;
                }
            }
        }
        let (view_projection, model) = matrices(self.origin, space);
        if let Some((_, preview)) = &mut self.shown {
            preview.draw_matrices(
                device,
                queue,
                encoder,
                view,
                viewport,
                target_size,
                space,
                view_projection,
                model,
                0.0,
            );
        }
    }
}
