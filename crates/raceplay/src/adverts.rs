//! The billboard adverts: each slot's model is drawn through its own camera
//! into a small texture, and that texture is what the circuit's placeholder
//! quads show.
//!
//! **What the original does, read off a live PPSSPP frame** (Pulse PSP,
//! Talon's Junction, 2026-10-06, `docs/ghidra/functions/psp-pulse-usa/billboards.md`):
//! the GE list renders twice into a 128 x 128 offscreen buffer (`FBPTR`
//! `0x04154000`, width `0x80`, viewport scale 64) before the scene pass samples
//! that buffer on two of the track's quads. The advert's matrix there is the
//! inverse of the model's own `camera1` transform and its projection is built
//! from the model's own `Camera` node (`VexCamera_BuildProjection`,
//! [`oag_vex::camera::Camera::fov_degrees`]), so **the adverts have no placement
//! transform to find**: the thing the constructor writes as an identity matrix
//! is the camera's cached view matrix, filled in on the first update. Where the
//! picture lands is decided by the track, which authors a quad textured
//! `billboardN.tga` for slot N - the same quad `oag_render::gantry` measures
//! for slot 8's mount.
//!
//! This module is the load half ([`load`]: model and camera per slot) and the GPU
//! half ([`Cards`]: the targets, the per-frame pass and the rebinding of the
//! track's own placeholder materials).

use super::*;

/// The side of a card's target, in pixels: `FUN_0891fe14(object + 0x98, 0x80,
/// 0x80)` in `Billboard_ConstructResource`, and the `0x80` buffer width and
/// 64.0 viewport scale of the captured pass.
pub(super) const SIDE: u32 = 128;

/// The near plane of every `.vex` camera projection: the literal `1.2` in
/// `VexCamera_BuildProjection`.
const NEAR: f32 = 1.2;

/// The far plane the captured matrices imply: solving `-2fn/(f-n)` =
/// -2.40143 for `n` = 1.2. The engine reads it from the display's own state
/// (`g_display + 0x1698`); a flat card does not depend on it.
const FAR: f32 = 2017.7;

/// What a circuit's billboard slots become: the start gantry on slot 8's quad,
/// and an advert drawn into each of the other slots' quads that has a model.
#[derive(Debug, Default)]
pub struct Billboards {
    /// Slot 8: [`crate::gantry`].
    pub gantry: Option<crate::gantry::Placed>,
    /// The other slots, see [`Card`].
    pub adverts: Vec<Card>,
}

/// The slot a card fills, drawn through the camera its own file authors.
pub struct Card {
    /// The billboard slot, the `num` of the manifest entry.
    pub slot: u32,
    /// The advert model, as loaded.
    pub model: Model,
    /// The projection times the inverse of the camera's world transform.
    pub view_projection: Mat4,
}

impl std::fmt::Debug for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Card")
            .field("slot", &self.slot)
            .field("model", &self.model.label)
            .finish_non_exhaustive()
    }
}

/// The matrix a card draws with, from its model's camera.
///
/// `None` when the camera is orthographic or its field of view is a curve of
/// more than one key: nothing here interpolates one, and a card that cannot be
/// framed is drawn nothing rather than framed by a guess.
#[must_use]
pub fn view_projection(camera: &oag_vex::camera::Camera) -> Option<Mat4> {
    let aspect = camera.aspect();
    if !(aspect.is_finite() && aspect > 0.0) {
        return None;
    }
    let tan_half = (camera.fov_degrees()?.to_radians() / 2.0).tan();
    // The original's matrix has an x scale of `1 / tan(fov / 2)` and a y scale
    // `aspect` times that; `perspective` takes the vertical field of view.
    let fov_y = 2.0 * (tan_half / aspect).atan();
    let view = Mat4::from_cols_array(&camera.to_world).inverse();
    Some(oag_core::math::camera::perspective(fov_y, aspect, NEAR, FAR) * view)
}

/// Loads the advert every `Location` slot of `manifest` names, for the slots
/// whose placeholder this circuit's track authors.
///
/// Slot 8 is the start gantry and is left to [`crate::gantry`]. A slot that
/// names a colour is left unfilled: which advert a colour picks is
/// `Billboard_CreateFromColour`'s walk of a per-track pool, unrecovered. Every
/// refusal is a line in `report`.
pub(super) fn load(
    archives: &mut oag_assets::Archives,
    manifest: &oag_tables::trackstartup::TrackStartup,
    track_model: &Model,
    report: &mut Vec<String>,
) -> Vec<Card> {
    let placeholders = oag_render::gantry::placeholder_texture_slots(track_model);
    let mut cards = Vec::new();
    for billboard in &manifest.billboards {
        if billboard.num == 8 || !placeholders.iter().any(|&(_, n)| n == billboard.num) {
            continue;
        }
        let Some(name) = billboard.location() else {
            report.push(format!(
                "billboard slot {} names a colour ({:?}): which advert a colour picks is \
                 unrecovered, so its placeholder draws nothing",
                billboard.num, billboard.fill
            ));
            continue;
        };
        match load_card(archives, billboard.num, name, report) {
            Ok(card) => {
                report.push(format!(
                    "billboard slot {}: {name} drawn through its own camera into a {SIDE}x{SIDE} \
                     target shown on the circuit's billboard{}.tga quad(s)",
                    card.slot, card.slot
                ));
                cards.push(card);
            }
            Err(why) => report.push(format!(
                "billboard slot {}: {name} not drawn ({why}) - its placeholder draws nothing",
                billboard.num
            )),
        }
    }
    cards
}

fn load_card(
    archives: &mut oag_assets::Archives,
    slot: u32,
    name: &str,
    report: &mut Vec<String>,
) -> Result<Card> {
    let (model, blob) = super::gantry::load_with_blob(archives, name, report)?;
    let cameras = oag_vex::camera::cameras(&blob);
    let camera = match cameras.as_slice() {
        [one] => one,
        other => anyhow::bail!("{} cameras, expected one", other.len()),
    };
    let view_projection =
        view_projection(camera).context("its camera is orthographic or has a multi-key curve")?;
    Ok(Card {
        slot,
        model,
        view_projection,
    })
}

struct CardGpu {
    slot: u32,
    drawable: Drawable,
    view_projection: Mat4,
    colour: wgpu::TextureView,
}

/// The cards on the GPU: one target each, drawn once a frame ahead of the
/// scene pass.
pub(super) struct Cards {
    cards: Vec<CardGpu>,
    depth: wgpu::TextureView,
    velocity: wgpu::TextureView,
}

impl std::fmt::Debug for Cards {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cards")
            .field(
                "slots",
                &self.cards.iter().map(|c| c.slot).collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}

/// A card's colour format: raw, as the capture path draws, because the mesh
/// shader writes gamma-space values to a target that is not linear and the
/// track samples this as an ordinary albedo.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Draws every card of `cards`, if the circuit has any, ahead of the scene pass
/// that samples them, at `seconds` on the scenery clock.
pub(super) fn render(
    cards: &Option<Cards>,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    seconds: f32,
) {
    if let Some(cards) = cards {
        cards.render(queue, encoder, seconds);
    }
}

impl Cards {
    /// Builds a drawable and a target per card.
    ///
    /// # Errors
    ///
    /// Propagates a pipeline or geometry upload failure.
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cards: Vec<Card>,
        anisotropy: Anisotropy,
    ) -> Result<Self> {
        let size = wgpu::Extent3d {
            width: SIDE,
            height: SIDE,
            depth_or_array_layers: 1,
        };
        let target = |label: &str, format, usage| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let depth = target(
            "advert depth",
            mesh_render::DEPTH_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let velocity = target(
            "advert velocity",
            oag_gpu::formats::VELOCITY_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let mut gpu = Vec::with_capacity(cards.len());
        for card in cards {
            let drawable = Drawable::new(
                device,
                queue,
                card.model,
                FORMAT,
                anisotropy,
                1,
                mesh_render::Depth::Scene,
                mesh_render::TRANSPARENT_BLEND,
                mesh_render::GlowMask::Protected,
                &mesh_render::zone::StageArt::NONE,
                mesh_render::ShadowMaps::NONE,
                mesh_render::ShadowReceiver::Never,
            )?;
            let colour = target(
                "advert",
                FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            );
            gpu.push(CardGpu {
                slot: card.slot,
                drawable,
                view_projection: card.view_projection,
                colour,
            });
        }
        Ok(Self {
            cards: gpu,
            depth,
            velocity,
        })
    }

    /// Points every placeholder material of `track` whose slot has a card at
    /// that card's target. `placeholders` is
    /// [`oag_render::gantry::placeholder_texture_slots`] of the track model.
    pub(super) fn bind_into(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        track: &mut Drawable,
        placeholders: &[(usize, u32)],
    ) {
        for &(texture_slot, number) in placeholders {
            if let Some(card) = self.cards.iter().find(|c| c.slot == number) {
                track.set_albedo(device, queue, texture_slot, &card.colour);
            }
        }
    }

    /// Draws every card into its target at `seconds` on the scenery clock.
    pub(super) fn render(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        seconds: f32,
    ) {
        for card in &self.cards {
            card.drawable.write(
                queue,
                card.view_projection,
                Mat4::IDENTITY,
                card.view_projection,
            );
            card.drawable.write_anims(queue, seconds);
            card.drawable.write_node_anims(queue, seconds);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("advert"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &card.colour,
                        depth_slice: None,
                        resolve_target: None,
                        // The captured pass clears with a sprite of colour 0.
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.velocity,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Discard,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            card.drawable.draw(&mut pass, None, None, None, None);
        }
    }
}

#[cfg(test)]
mod tests;
