//! A 3D model drawn inside a rectangle of a UI screen.
//!
//! What the race box's two selection screens need: `Team Selection` shows
//! the chosen craft's own front-end hull (`Data\Ships\<team>\ship_FE.vex`)
//! turning on a turntable, and `Track Creation` shows the circuit's outline
//! ribbon (`Data\Environments\<dir>\FE\<run>.vex`) on its info panel - both
//! the disc's own purpose-built preview meshes, both placed by code on the
//! original rather than by any widget, see `docs/formats/race-setup.md`.
//!
//! One model, one small pass with its own depth buffer, composited over
//! whatever the UI has already drawn - the same shape
//! [`crate::hud::Countdown`] takes for the start-line glyph, and the same
//! pipeline every mesh in this project draws through. The difference is the
//! camera: the countdown's is orthographic over the HUD grid, while this one
//! frames the model from its own bounding sphere the way the asset viewer
//! does (`oag_render::mesh_render::write_uniforms`), and confines the pass
//! to a rectangle given in the screen's own grid.

use anyhow::{Context, Result};
use oag_display::space::Space;
use oag_render::camera::orbit::Orbit;
use oag_render::mesh::Model;
use oag_render::mesh_render::{
    Anisotropy, Built, CutoutPipelines, DEPTH_FORMAT, Depth, GlowMask, NodeAnims, ShadowReceiver,
    TRANSPARENT_BLEND, TexAnims, TransparentPipelines, UNIFORMS_SIZE, Velocity, build,
    write_uniforms,
};
use oag_ui::picker::slideshow::Slideshow;

use crate::render::letterbox_in;

/// How a selection screen's preview is framed at `seconds` into the screen.
///
/// **Both are this build's own numbers**, read off the capture rather than
/// recovered from the executable: the craft turns once every twelve seconds
/// seen slightly from above, and the circuit outline is tilted the way the
/// info panel shows it - see `docs/ui/selection-screens.md`.
#[must_use]
pub fn orbit_for(kind: oag_ui::picker::Kind, seconds: f32) -> Orbit {
    match kind {
        oag_ui::picker::Kind::Ship => Orbit {
            yaw: -0.7 + seconds * std::f32::consts::TAU / 12.0,
            pitch: 0.35,
            zoom: 0.8,
            ..Orbit::default()
        },
        oag_ui::picker::Kind::Track => Orbit {
            yaw: 0.3,
            pitch: 1.05,
            zoom: 0.4,
            ..Orbit::default()
        },
    }
}

/// One still of a slideshow as read off the disc: its entry name and its
/// bytes, the pair `crate::sprite::Sheet::extended` takes.
pub type Still = (String, Vec<u8>);

/// A circuit's slideshow - the stills behind `Track Creation`'s hexagonal
/// window - read off the disc, with every still it names as bytes.
///
/// `Data\Environments\<dir>\screen.xml` is what
/// `TrackDefinition_EnterScreenState` loads into the selected circuit's own
/// state machine; in Zone mode on a circuit that is `availableInZone` it
/// loads `screen_zone.xml` and enters `Zone` instead of `Info`. The PS2
/// authors both chains in the one `screen.xml`, which the fallback below
/// reads the same way: the Zone file first when Zone is asked for, then the
/// plain file, and within it the Zone chain if it has one. See
/// [`oag_ui::picker::slideshow`].
///
/// **`location` is a craft's as readily as a circuit's**, and on Pure it has
/// to be: `Data\Ships\<team>\screen.xml` authors the three-still chain that
/// *is* that title's craft preview. Pulse's and HD's craft files author the
/// stat panel and no `<Image src=>` at all, so the same call returns an empty
/// still list there and their mesh preview stands alone - measured on all
/// three, not assumed.
///
/// **Pure names its Zone chain `screen_z.xml`** - the `%s\screen_z.xml`
/// template in its own executable - and starts it at `Info` like every other
/// chain, rather than at a `Zone` state. Its `screen_m.xml` and
/// `screen_tt.xml` (split screen, time trial) are **not** read: choosing
/// between them needs a race-mode parameter this function does not take, and
/// on the circuit checked all three name the same three stills.
///
/// `globals` are the front end's own `FEGlobals` table: a per-entity
/// `screen.xml` declares none and still names them for its stills' colour.
/// See [`oag_ui::picker::slideshow::Slideshow::read`].
///
/// The stills are read through [`oag_pulse::read_image`], which is what
/// finds a PS2 disc's `.pct` under a `.mip` name. One that is missing is
/// reported and left out, and the card it would have been on is left empty
/// rather than substituted; a file that is missing altogether is an error
/// the caller logs.
///
/// # Errors
///
/// No `screen.xml` (or `screen_zone.xml`) for this circuit, or one with no
/// `Info`/`Zone` chain in it.
pub fn slideshow(
    archives: &mut oag_assets::Archives,
    location: &str,
    zone: bool,
    globals: &[(&str, &str)],
    report: &mut Vec<String>,
) -> Result<(Slideshow, Vec<Still>)> {
    let read = |archives: &mut oag_assets::Archives, file: &str, start: &str| {
        let entry = format!(r"{location}\{file}");
        let blob = archives.read_name(&entry).ok()?;
        let xml = if oag_tables::fexml::is_fexml(&blob) {
            oag_tables::fexml::expand(&blob).ok()?
        } else {
            String::from_utf8(blob).ok()?
        };
        Slideshow::read(&xml, location, start, globals)
    };
    let show = if zone {
        read(archives, "screen_zone.xml", "Zone")
            .or_else(|| read(archives, "screen_z.xml", "Info"))
            .or_else(|| read(archives, "screen.xml", "Zone"))
            .or_else(|| read(archives, "screen.xml", "Info"))
    } else {
        read(archives, "screen.xml", "Info")
    }
    .with_context(|| format!(r"{location}\screen.xml: no slideshow chain to read"))?;
    let mut blobs = Vec::new();
    for src in show.sources() {
        match oag_pulse::read_image(archives, &src) {
            Ok(blob) => blobs.push((src, blob)),
            Err(error) => report.push(format!("{src}: {error} - that card draws nothing")),
        }
    }
    Ok((show, blobs))
}

/// Reads and decodes a preview mesh off the disc - the outline ribbon or
/// the front-end hull - with a PS2 disc's sibling texture set resolved the
/// way a race resolves its own hull's and circuit's
/// ([`crate::race::ps2_texture_set`]): only when the model's own texture
/// slots exist and are all empty, which is that disc's signature and never
/// a PSP model's. Without it both PS2 previews draw flat white. One
/// function so the live screen and the headless capture cannot disagree.
///
/// # Errors
///
/// The entry is missing, or will not decode as a mesh.
pub fn model(archives: &mut oag_assets::Archives, entry: &str) -> Result<Model> {
    let blob = archives
        .read_name(entry)
        .with_context(|| format!("reading the preview mesh {entry}"))?;
    let model = oag_render::mesh::build(entry, &blob)
        .with_context(|| format!("decoding the preview mesh {entry}"))?;
    if !model.textures.is_empty()
        && model.textures.iter().all(Option::is_none)
        && let Some(external) = crate::race::ps2_texture_set(archives, entry)
    {
        return oag_render::mesh::build_with_textures(entry, &blob, Some(&external))
            .map(|mut model| {
                model.keep_nearest();
                model
            })
            .with_context(|| format!("decoding the preview mesh {entry} with its texture set"));
    }
    Ok(model)
}

/// One preview model and the GPU state that draws it.
pub struct Preview {
    model: Model,
    built: Built,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// This pass's own depth attachment, sized to the last target drawn into.
    depth: Option<(wgpu::TextureView, u32, u32)>,
}

impl std::fmt::Debug for Preview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Preview")
            .field("label", &self.model.label)
            .field("triangles", &(self.model.indices.len() / 3))
            .finish()
    }
}

/// Where a grid-space rectangle lands on the target, in pixels: the same
/// letterboxing `ui.wgsl`'s `to_clip` applies to every UI quad, inverted, so
/// the preview sits exactly where the surrounding widgets say it does at any
/// window shape.
#[must_use]
pub fn pixel_rect(
    space: Space,
    viewport: (f32, f32, f32, f32),
    rect: [f32; 4],
) -> (f32, f32, f32, f32) {
    let [scale_x, scale_y] =
        letterbox_in((viewport.2 as u32, viewport.3 as u32), space.display_aspect);
    let to_pixels = |x: f32, y: f32| {
        let clip_x = (2.0 * (x / space.size.0) - 1.0) * scale_x;
        let clip_y = (1.0 - 2.0 * (y / space.size.1)) * scale_y;
        (
            viewport.0 + (clip_x * 0.5 + 0.5) * viewport.2,
            viewport.1 + (0.5 - clip_y * 0.5) * viewport.3,
        )
    };
    let (left, top) = to_pixels(rect[0], rect[1]);
    let (right, bottom) = to_pixels(rect[0] + rect[2], rect[1] + rect[3]);
    (left, top, (right - left).max(1.0), (bottom - top).max(1.0))
}

impl Preview {
    /// Builds the pipelines for an already-loaded model.
    ///
    /// # Errors
    ///
    /// Propagates pipeline creation.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        anisotropy: Anisotropy,
        model: Model,
    ) -> Result<Self> {
        let built = build(
            device,
            queue,
            &model,
            format,
            anisotropy,
            1,
            Depth::Scene,
            TRANSPARENT_BLEND,
            // No bloom behind a menu, and one target: no glow mask, no
            // velocity. No Zone stage, no shadow map, nothing receiving one.
            GlowMask::Protected,
            Velocity::None,
            &oag_render::mesh_render::zone::StageArt::NONE,
            oag_render::mesh_render::ShadowMaps::NONE,
            ShadowReceiver::Never,
        )?;
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("preview uniforms"),
            size: UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("preview uniforms"),
            layout: &built.pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        Ok(Self {
            model,
            built,
            uniform_buffer,
            bind_group,
            depth: None,
        })
    }

    #[must_use]
    pub fn model(&self) -> &Model {
        &self.model
    }

    fn depth_view(&mut self, device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
        let stale = !matches!(&self.depth, Some((_, w, h)) if *w == width && *h == height);
        if stale {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("preview depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth = Some((
                texture.create_view(&wgpu::TextureViewDescriptor::default()),
                width,
                height,
            ));
        }
        self.depth.as_ref().expect("just set above").0.clone()
    }

    /// Draws the model into `rect` of `space`'s grid, over whatever `view`
    /// already holds.
    ///
    /// `viewport` is the aspect rectangle the UI is drawn into and
    /// `target_size` is `view`'s own full size - the two differ on a
    /// pillarboxed window, and the depth attachment has to match the latter
    /// or the pass is invalid. `orbit` frames the model the asset viewer's
    /// way, and `seconds` places its authored texture and node animations.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
        space: Space,
        rect: [f32; 4],
        orbit: Orbit,
        seconds: f32,
    ) {
        if self.model.vertices.is_empty() || self.model.indices.is_empty() {
            return;
        }
        let (x, y, width, height) = pixel_rect(space, viewport, rect);
        // Clamped to the target: a rectangle authored past the grid's edge on
        // a narrow window would otherwise ask for a viewport outside it,
        // which is a validation error rather than a clipped picture.
        let (max_w, max_h) = (target_size.0 as f32, target_size.1 as f32);
        let x0 = x.clamp(0.0, max_w);
        let y0 = y.clamp(0.0, max_h);
        let x1 = (x + width).clamp(0.0, max_w);
        let y1 = (y + height).clamp(0.0, max_h);
        if x1 - x0 < 1.0 || y1 - y0 < 1.0 {
            return;
        }

        write_uniforms(
            queue,
            &self.uniform_buffer,
            &self.model,
            width / height,
            orbit,
        );
        queue.write_buffer(
            &self.built.anim_buffer,
            0,
            bytemuck::bytes_of(&TexAnims::sample(&self.model, seconds)),
        );
        queue.write_buffer(
            &self.built.node_anim_buffer,
            0,
            bytemuck::bytes_of(&NodeAnims::sample(&self.model, seconds)),
        );

        let depth_view = self.depth_view(device, target_size.0.max(1), target_size.1.max(1));
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("preview"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
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
        // The viewport is the unclamped rectangle so the projection keeps its
        // aspect; the scissor is what actually confines the pixels.
        pass.set_viewport(x, y, width, height, 0.0, 1.0);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to the target above"
        )]
        pass.set_scissor_rect(x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32);

        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_bind_group(2, &self.built.fog_bind, &[]);
        pass.set_bind_group(3, &self.built.anim_bind, &[]);
        pass.set_vertex_buffer(0, self.built.vertex_buffer.slice(..));
        pass.set_index_buffer(self.built.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        let bind = |draw: &oag_render::mesh::DrawCall| {
            let slot = draw.texture.map_or(0, |t| t + 1);
            &self.built.texture_binds[if slot < self.built.texture_binds.len() {
                slot
            } else {
                0
            }]
        };
        pass.set_pipeline(&self.built.pipeline);
        for draw in &self.model.draws {
            pass.set_bind_group(1, bind(draw), &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
        // One pipeline per alpha-test reference the model's batches ask for,
        // switched only when it changes - see `mesh_render::cutout`.
        let cutouts = CutoutPipelines {
            default: &self.built.alpha_test_pipeline,
            by_reference: &self.built.cutout_pipelines,
        };
        let mut cutout_set: Option<&wgpu::RenderPipeline> = None;
        for draw in &self.model.alpha_tested_draws {
            let cutout = cutouts.select(draw);
            if !cutout_set.is_some_and(|set| std::ptr::eq(set, cutout)) {
                pass.set_pipeline(cutout);
                cutout_set = Some(cutout);
            }
            pass.set_bind_group(1, bind(draw), &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
        let pipelines = TransparentPipelines {
            alpha_over: &self.built.blend_pipeline,
            additive: &self.built.additive_pipeline,
            unblended: &self.built.unblended_pipeline,
            authored: &self.built.authored_pipelines,
        };
        let mut current: Option<&wgpu::RenderPipeline> = None;
        for draw in &self.model.transparent_draws {
            let pipeline = pipelines.select(draw);
            if !current.is_some_and(|set| std::ptr::eq(set, pipeline)) {
                pass.set_pipeline(pipeline);
                current = Some(pipeline);
            }
            pass.set_bind_group(1, bind(draw), &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grid_rect_lands_where_the_ui_draws_it() {
        let space = Space::PSP;
        // A window exactly the PSP's shape: the grid maps 1:1 scaled.
        let (x, y, w, h) = pixel_rect(space, (0.0, 0.0, 960.0, 544.0), [290.0, 25.0, 170.0, 200.0]);
        assert_eq!((x, y, w, h), (580.0, 50.0, 340.0, 400.0));
        // A wider window pillarboxes: the grid is centred and the rect moves
        // with it, keeping its size.
        let (x, y, w, h) = pixel_rect(space, (0.0, 0.0, 1920.0, 544.0), [0.0, 0.0, 480.0, 272.0]);
        assert!((x - 480.0).abs() < 0.5 && y.abs() < 0.5, "{x} {y}");
        assert!(
            (w - 960.0).abs() < 0.5 && (h - 544.0).abs() < 0.5,
            "{w} {h}"
        );
    }
}
