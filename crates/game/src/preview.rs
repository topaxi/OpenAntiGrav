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
//! [`oag_hud::Countdown`] takes for the start-line glyph, and the same
//! pipeline every mesh in this project draws through. The difference is the
//! camera: the countdown's is orthographic over the HUD grid, `Team
//! Selection`'s ship still frames the model from its own bounding sphere the
//! way the asset viewer does (`oag_mesh::mesh_render::write_uniforms`) and
//! confines the pass to a rectangle given in the screen's own grid, and
//! `Track Creation`'s outline ribbon uses the disc's own fixed `<Mode3D>`
//! camera instead - see [`mode3d_view_projection`].

use anyhow::{Context, Result};
use oag_core::math::{Mat4, Vec3, camera};
use oag_display::space::Space;
use oag_mesh::mesh::Model;
use oag_mesh::mesh_render::{
    Anisotropy, Built, CutoutPipelines, DEPTH_FORMAT, Depth, GlowMask, NodeAnims, ShadowReceiver,
    TRANSPARENT_BLEND, TexAnims, TransparentPipelines, UNIFORMS_SIZE, Velocity, build,
    write_uniforms, write_uniforms_raw,
};
use oag_mesh::orbit::Orbit;
use oag_ui_screens::picker::slideshow::Slideshow;

use crate::render::letterbox_in;

/// The craft's framing on a title whose `Team Selection` draws the race hull
/// ([`oag_title::FrontEnd::ship_preview_hull`]): **fixed, not a turntable** -
/// the original's hull held the same pose across five seconds of frames
/// (2026-10-08). The angles are **chosen, not measured**, fitted by eye to
/// the original's frame; the widget's authored `RotX`/`RotY` are not yet
/// composed.
#[must_use]
pub fn hull_orbit() -> Orbit {
    Orbit {
        yaw: 5.9,
        pitch: 0.5,
        zoom: 0.6,
        ..Orbit::default()
    }
}

/// Whether `title`'s `kind` selection screen draws the race hull as its craft
/// ([`oag_title::FrontEnd::ship_preview_hull`]) rather than a `ship_FE.vex`.
#[must_use]
pub fn draws_hull(title: &oag_title::Title, kind: oag_ui_screens::picker::Kind) -> bool {
    kind == oag_ui_screens::picker::Kind::Ship
        && title
            .front_end
            .is_some_and(|f| !f.preview_meshes && f.ship_preview_hull.is_some())
}

/// The craft entry a `--menu-page ship-select` capture previews for a team at
/// `location`.
#[must_use]
pub fn ship_entry(title: &oag_title::Title, location: &str) -> String {
    let file = title.front_end.and_then(|f| f.ship_preview_hull);
    format!(r"{location}\{}", file.unwrap_or("ship_FE.vex"))
}

/// The orbit a capture frames a preview with: the fixed hull pose, or the
/// turntable at its first tick.
#[must_use]
pub fn capture_orbit(hull_only: bool, kind: oag_ui_screens::picker::Kind) -> Orbit {
    if hull_only {
        hull_orbit()
    } else {
        orbit_for(kind, 0.0)
    }
}

/// Frames a race hull from the bulk of its vertices instead of all of them.
///
/// HD's `ship.vex` carries one chunk with a stray vertex hundreds of units
/// from the craft (a bound of 257 against a hull some 13 units long, checked
/// 2026-10-08 on `Assegai`), and the bounding sphere the viewer frames from
/// then draws the craft as a speck. The 1st-to-99th percentile box of the
/// vertex positions is the hull's own extent; the stray triangle is left in
/// the mesh and simply sits outside the frame.
pub fn frame_hull(model: &mut Model) {
    let mut lo = [0.0f32; 3];
    let mut hi = [0.0f32; 3];
    for axis in 0..3 {
        let mut values: Vec<f32> = model.vertices.iter().map(|v| v.position[axis]).collect();
        if values.is_empty() {
            return;
        }
        values.sort_by(f32::total_cmp);
        lo[axis] = values[values.len() / 100];
        hi[axis] = values[values.len() * 99 / 100];
    }
    model.centre = std::array::from_fn(|i| (lo[i] + hi[i]) * 0.5);
    model.radius = (0..3)
        .map(|i| (hi[i] - lo[i]) * 0.5)
        .fold(0.0f32, f32::max)
        .max(0.001);
}

/// How the ship preview on `Team Selection` is framed at `seconds` into the
/// screen.
///
/// **This build's own number, read off the capture rather than recovered
/// from the executable**: the craft turns once every twelve seconds seen
/// slightly from above - see `docs/ui/selection-screens.md`. `Track
/// Creation`'s outline ribbon no longer goes through this: see
/// [`mode3d_view_projection`] for the disc's own camera, now read instead.
#[must_use]
pub fn orbit_for(kind: oag_ui_screens::picker::Kind, seconds: f32) -> Orbit {
    match kind {
        oag_ui_screens::picker::Kind::Ship => Orbit {
            yaw: -0.7 + seconds * std::f32::consts::TAU / 12.0,
            pitch: 0.35,
            zoom: 0.8,
            ..Orbit::default()
        },
        oag_ui_screens::picker::Kind::Track => Orbit {
            yaw: 0.3,
            pitch: 1.05,
            zoom: 0.4,
            ..Orbit::default()
        },
    }
}

/// The race box's own fixed `<Mode3D>` camera and the child `<Model>`'s own
/// pose, in place of [`orbit_for`]'s capture-read orbit - `None` when the
/// circuit's own `screen.xml` authors no usable one (no `<Mode3D>` at all, or
/// a degenerate `nearZ`/`farZ`), so a caller can fall back to the orbit.
///
/// **Measured off `psp-pulse-usa`'s own `Mode3D` widget class**
/// (`docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`), not chosen:
///
/// - **The camera itself has no position or rotation of its own** - only
///   `OriginX`/`OriginY` (a viewport-centre shift, in the screen's own
///   pixels) and `nearZ`/`farZ`. It sits at the origin, always looking down
///   `-Z`; what moves is the `<Model>` child's own authored `x y z RotX
///   RotY`.
/// - **The projection is a standard symmetric perspective frustum**, vertical
///   FOV a fixed `1.0` radian (confirmed live: `FUN_088b8548`, the callback
///   `Gfx_FlushRenderManager` actually invokes when the queued draw replays -
///   the one that runs at polygon-submission time, after `FUN_088b7f24`'s
///   own earlier, superseded `~1.134` radian setup), aspect fixed to the
///   screen's own `480/272` regardless of the panel's shape.
/// - **`OriginX`/`OriginY` shift the projection's own centre**, not the
///   model - confirmed live via `FUN_08811220`'s default arguments
///   (`1808, 1912 == 2048 - 240, 2048 - 136`, the PSP's own screen half-
///   extents subtracted from the GE's `2048` viewport centre): a widget's
///   `OriginX`/`OriginY` are added to that pair before the same call, i.e. a
///   plain screen-pixel offset from centre. Applied here as a post-
///   projection NDC shift, the off-axis-frustum equivalent of the same GE
///   viewport-offset register.
///
/// **The PS2's own `OriginX`/`OriginY` are negated relative to the PSP's**,
/// not measured on `psp-pulse-eu`'s own PS2 sibling: this circuit's two
/// pressings' authored values (`-145, 13` PSP vs `193, -21` PS2, within
/// rounding of `-(-145, 13) * (640/480, 448/272)`) suggested it, and applying
/// the PSP's own sign to the PS2's raw values confirmed it live - the outline
/// lands bottom-left of the panel instead of inside it
/// (`data/scratch/pulse-mode3d/shots/track-select-ps2.png` before the flip
/// below). **What is not independently confirmed:** the `<Model>`'s own
/// rotation order and axis handedness - PSP code builds its transform in a
/// form consistent with row-vector composition, `v * RotY * RotX * T`, which
/// this reads into this renderer's column-vector convention as `T * RotX *
/// RotY` applied innermost-first the same way, a plausible transpose rather
/// than a breakpoint-confirmed one. A live PSP screenshot at this reading
/// looks right (`data/scratch/pulse-mode3d/shots/track-select-psp.png`
/// against a live PPSSPP capture of the same panel), which does not prove the
/// order - RotY here is `0.1` rad, small enough that a wrong order would be
/// hard to see on this circuit alone.
#[must_use]
pub fn mode3d_view_projection(model: &oag_ui::screen::Model, space: Space) -> Option<(Mat4, Mat4)> {
    let [near, far] = model.depth;
    if !(near > 0.0 && far > near) {
        return None;
    }
    let fov_y = 1.0; // radians, fixed - see this function's own doc.
    let projection = camera::perspective(fov_y, space.display_aspect, near, far);
    // The PS2 pressing's own OriginX/OriginY read negated relative to the
    // PSP's - see this function's own doc - confirmed live 2026-09-28:
    // applying the PSP sign to the PS2's raw values landed the outline
    // bottom-left of the panel instead of inside it (`data/scratch/pulse-mode3d/`).
    let sign = if space.size == Space::PS2.size {
        -1.0
    } else {
        1.0
    };
    let ndc_shift = Vec3::new(
        sign * -2.0 * model.origin[0] / space.size.0,
        sign * 2.0 * model.origin[1] / space.size.1,
        0.0,
    );
    let view_projection = Mat4::from_translation(ndc_shift) * projection;

    let [x, y, z] = model.position;
    let [rot_x, rot_y] = model.rotation;
    let model_matrix = Mat4::from_translation(Vec3::new(x, y, z))
        * Mat4::from_rotation_x(rot_x)
        * Mat4::from_rotation_y(rot_y);

    Some((view_projection, model_matrix))
}

/// One still of a slideshow as read off the disc: its entry name and its
/// bytes, the pair `oag_hud::sprite::Sheet::extended` takes.
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
/// [`oag_ui_screens::picker::slideshow`].
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
/// `strings` resolves the panel's own `idstring` labels - Pure's stat bars,
/// authored in this same file rather than in `Selection_Definition.xml`. See
/// [`oag_ui_screens::picker::slideshow::Slideshow::read`].
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
    strings: &oag_ui::language::StringTable,
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
        Slideshow::read(&xml, location, start, globals, strings)
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

/// Seconds the hexagonal window's own stills take to fade in on a fresh
/// selection - **chosen, not measured**: the circuit's own `screen.xml`
/// authors no `transition` of its own for them (unlike `Selection_Definition.xml`'s
/// `LeftLayer`s, which do - see [`oag_ui_screens::picker`]'s own reading of that
/// attribute), but a live PPSSPP capture shows them arriving in step with
/// the info panel's own measured `0.5`s (`docs/ui/selection-screens.md`), so
/// this reuses that number rather than inventing an unrelated one.
pub const CARD_FADE_SECONDS: f32 = 0.5;

/// `draw` with every colour it carries multiplied by `alpha` - [`CARD_FADE_SECONDS`]'s
/// own arrival fade, applied after [`Slideshow::draws`] since that method
/// reads no fade of its own. Shared by the live picker stage and the
/// `--menu-page` still capture so the two cannot draw the cards differently.
#[must_use]
pub fn fade_draw(draw: oag_ui::frontend::Draw, alpha: f32) -> oag_ui::frontend::Draw {
    use oag_ui::frontend::Draw;
    if alpha >= 1.0 {
        return draw;
    }
    let scale = |c: [f32; 4]| [c[0], c[1], c[2], c[3] * alpha];
    match draw {
        Draw::Sprite { rect, uv, color } => Draw::Sprite {
            rect,
            uv,
            color: scale(color),
        },
        Draw::Fill { rect, color } => Draw::Fill {
            rect,
            color: scale(color),
        },
        Draw::Text {
            x,
            y,
            scale: text_scale,
            color,
            border,
            align,
            text,
            wrap_width,
        } => Draw::Text {
            x,
            y,
            scale: text_scale,
            color: scale(color),
            border,
            align,
            text,
            wrap_width,
        },
        other => other,
    }
}

/// Reads and decodes a preview mesh off the disc - the outline ribbon or
/// the front-end hull - with a PS2 disc's sibling texture set resolved the
/// way a race resolves its own hull's and circuit's
/// ([`oag_livery::entry::ps2_texture_set`]): only when the model's own texture
/// slots exist and are all empty, which is that disc's signature and never
/// a PSP model's. Without it both PS2 previews draw flat white. One
/// function so the live screen and the headless capture cannot disagree.
///
/// # Errors
///
/// The entry is missing, or will not decode as a mesh.
pub fn model(archives: &mut oag_assets::Archives, entry: &str) -> Result<Model> {
    model_named(archives, entry).map(|(model, _)| model)
}

/// [`model`], and the `.gtf` path of each texture slot in order - what a caller
/// that has to treat one texture's draws differently needs to find them. Empty
/// for a PSP or PS2 model, whose textures are embedded and carry no path.
///
/// # Errors
///
/// As [`model`].
pub fn model_named(
    archives: &mut oag_assets::Archives,
    entry: &str,
) -> Result<(Model, Vec<String>)> {
    let blob = archives
        .read_name(entry)
        .with_context(|| format!("reading the preview mesh {entry}"))?;
    // A PS3 `.vex` keeps its geometry in the `.rcsmodel` beside it, and its
    // materials and textures in other entries of the same archive set - which
    // is why this takes `Archives` rather than a blob. `build` below is the
    // PSP/PS2 path and cannot draw one.
    if oag_mesh::mesh::geometry_is_external(&blob) {
        return ps3_model(archives, entry, &blob);
    }
    let named = |model| (model, Vec::new());
    let model = oag_mesh::mesh::build(entry, &blob)
        .with_context(|| format!("decoding the preview mesh {entry}"))?;
    if !model.textures.is_empty()
        && model.textures.iter().all(Option::is_none)
        && let Some(external) = oag_livery::entry::ps2_texture_set(archives, entry)
    {
        return oag_mesh::mesh::build_with_textures(entry, &blob, Some(&external))
            .map(|mut model| {
                model.keep_nearest();
                named(model)
            })
            .with_context(|| format!("decoding the preview mesh {entry} with its texture set"));
    }
    Ok(named(model))
}

/// [`model`]'s PS3 branch: the scene `.vex` and the `.rcsmodel` beside it,
/// with every material and `.gtf` read through the same archive set a race
/// reads them through.
///
/// `build_scene` and not `build`: the viewer and the race both take the scene
/// path, and a flyer is all first-pass (every mesh node addresses a chunk), so
/// the two give the same model; the scene path is the one the rest of this
/// project's HD ground truth is pinned against.
fn ps3_model(
    archives: &mut oag_assets::Archives,
    entry: &str,
    blob: &[u8],
) -> Result<(Model, Vec<String>)> {
    let sibling = oag_mesh::mesh::rcs::sibling_name(entry)
        .with_context(|| format!("{entry} names no .rcsmodel"))?;
    let geometry = archives
        .read_name(&sibling)
        .with_context(|| format!("{entry}: reading its {sibling}"))?;
    // The `.gtf` requests come in texture-slot order, which is how a draw
    // names its texture.
    let mut textures = Vec::new();
    let (mut model, report) =
        oag_mesh::mesh::rcs::build_scene(entry, blob, &geometry, &mut |path| {
            if path.to_ascii_lowercase().ends_with(".gtf") {
                textures.push(path.to_string());
            }
            archives.read_name(path).ok()
        })
        .with_context(|| format!("decoding the preview mesh {entry}"))?;
    log::debug!("preview {entry}: {}", report.describe());
    model.keep_nearest();
    Ok((model, textures))
}

/// Paints the skin file `entry` onto a preview `model` built from
/// `model_name`, the same swap a race makes - the PSP's four-block upload or
/// the PS2's whole-atlas sibling, whichever the hull's own slots call for.
/// Returns the report lines; a skin that will not apply leaves the hull's
/// own paint and says so in them. See `oag_livery::ship_skin`.
pub fn paint(
    archives: &mut oag_assets::Archives,
    entry: &str,
    model_name: &str,
    model: &mut Model,
) -> Vec<String> {
    let mut report = Vec::new();
    oag_livery::ship_skin::apply(archives, entry, model_name, model, &mut report);
    report
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
/// letterboxing `ui.wesl`'s `to_clip` applies to every UI quad, inverted, so
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
            &oag_mesh::mesh_render::zone::StageArt::NONE,
            oag_mesh::mesh_render::ShadowMaps::NONE,
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
        self.render(
            device,
            queue,
            encoder,
            view,
            target_size,
            seconds,
            (x, y, width, height),
            (x0, y0, x1, y1),
        );
    }

    /// [`Self::draw_mode3d`], falling back to [`Self::draw`] when `mode3d` is
    /// `None` or its own `screen.xml` authors no usable `<Mode3D>` - the one
    /// call both selection screens' draw sites make (live and `--menu-page`).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_auto(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
        space: Space,
        mode3d: Option<&oag_ui::screen::Model>,
        rect: [f32; 4],
        orbit: Orbit,
        seconds: f32,
    ) {
        let drew_mode3d = mode3d.is_some_and(|model| {
            self.draw_mode3d(
                device,
                queue,
                encoder,
                view,
                viewport,
                target_size,
                space,
                model,
                seconds,
            )
        });
        if !drew_mode3d {
            self.draw(
                device,
                queue,
                encoder,
                view,
                viewport,
                target_size,
                space,
                rect,
                orbit,
                seconds,
            );
        }
    }

    /// [`Self::draw`], with the disc's own `<Mode3D>` camera in place of an
    /// [`Orbit`] - see [`mode3d_view_projection`]. `None` when the circuit's
    /// `screen.xml` authors no usable `<Mode3D>`, so a caller falls back to
    /// [`Self::draw`].
    ///
    /// **Draws into the full letterboxed screen, not `rect`** - unlike
    /// [`Self::draw`]'s panel-confined pass, this camera's own
    /// `OriginX`/`OriginY` are a shift of the *screen's* own centre (see
    /// [`mode3d_view_projection`]), so confining the viewport to the panel
    /// would apply that shift, and the fixed FOV/aspect, against the wrong
    /// rectangle.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_mode3d(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
        space: Space,
        model: &oag_ui::screen::Model,
        seconds: f32,
    ) -> bool {
        if self.model.vertices.is_empty() || self.model.indices.is_empty() {
            return false;
        }
        let Some((view_projection, model_matrix)) = mode3d_view_projection(model, space) else {
            return false;
        };
        self.draw_matrices(
            device,
            queue,
            encoder,
            view,
            viewport,
            target_size,
            space,
            view_projection,
            model_matrix,
            seconds,
        )
    }

    /// [`Self::draw_mode3d`]'s second half: draws into the full letterboxed
    /// screen under a camera the caller already built. One seam for every
    /// 3-D thing a menu places with a pose of its own - a circuit's
    /// `<Mode3D>` here, a campaign flyer in [`crate::flyer::flyer_view_projection`].
    ///
    /// `false` when the model is empty or the screen rectangle is degenerate.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_matrices(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
        space: Space,
        view_projection: Mat4,
        model_matrix: Mat4,
        seconds: f32,
    ) -> bool {
        if self.model.vertices.is_empty() || self.model.indices.is_empty() {
            return false;
        }
        let (x, y, width, height) =
            pixel_rect(space, viewport, [0.0, 0.0, space.size.0, space.size.1]);
        let (max_w, max_h) = (target_size.0 as f32, target_size.1 as f32);
        let x0 = x.clamp(0.0, max_w);
        let y0 = y.clamp(0.0, max_h);
        let x1 = (x + width).clamp(0.0, max_w);
        let y1 = (y + height).clamp(0.0, max_h);
        if x1 - x0 < 1.0 || y1 - y0 < 1.0 {
            return false;
        }

        write_uniforms_raw(queue, &self.uniform_buffer, view_projection, model_matrix);
        self.render(
            device,
            queue,
            encoder,
            view,
            target_size,
            seconds,
            (x, y, width, height),
            (x0, y0, x1, y1),
        );
        true
    }

    /// The render pass both [`Self::draw`] and [`Self::draw_mode3d`] run,
    /// once each has written its own uniforms - texture/node animation
    /// sampling, the pass itself, and the three draw-call lists in the
    /// model's own paint order. Split out so the two only duplicate the
    /// camera maths that actually differs between them.
    #[allow(clippy::too_many_arguments)]
    fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        target_size: (u32, u32),
        seconds: f32,
        viewport_px: (f32, f32, f32, f32),
        scissor_px: (f32, f32, f32, f32),
    ) {
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
        let (x, y, width, height) = viewport_px;
        pass.set_viewport(x, y, width, height, 0.0, 1.0);
        let (x0, y0, x1, y1) = scissor_px;
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

        let bind = |draw: &oag_mesh::mesh::DrawCall| {
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
