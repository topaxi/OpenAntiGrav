//! Collision sparks: a small additive burst on wall impact.
//!
//! Unlike [`crate::exhaust`], this is **not** a reading of the original's
//! `Ship Collision Fx` node (`0x3d0`). The original ships a dedicated particle
//! asset for this exact effect, `Data\Psys\WO_SHIP_COLL_SPARK_DAMAGE.POB`
//! (`docs/formats/wad.md`), and the `.pob`/`SYSP` particle-system format
//! remains undecoded (`docs/formats/README.md`). But the *trigger* is no
//! longer a gap: `ShipCollisionFx_Trigger` (`0x089246b4`), reached from
//! `Ship_DispatchCollisionFx` (`0x0883de90`, previously mislabelled "camera
//! shake" on this page - that function never calls a camera API, it
//! dispatches the spark unconditionally and *separately* plays a proximity
//! sound), is read in full in
//! (`docs/ghidra/functions/psp-pulse/contact-response.md`). It names the
//! three real spark resources, enforces a cooldown, and computes a severity
//! value - see [`COLLISION_COOLDOWN`] below for what is now ported.
//!
//! One more thing the trigger's arguments corroborate, worth recording
//! because it validates this module firing off `oag_physics::wall`
//! specifically: `Ship_DispatchCollisionFx`'s `damaged` flag is
//! `contact_record + 0x30 > 0.0`, and `+0x30` is the same ring-buffer field
//! `contact-response.md` already reads as the *friction* `Body_RecordContact`
//! stored. Wall contacts carry friction `0.035`; floor, magstrip and reset
//! contacts carry `0.0` (same page). So "damaged" is not a health check - it
//! is "this was a wall/track hit, not a floor one", which is exactly the
//! class of contact `evaluated.wall` reports and nothing else. This module's
//! trigger is therefore always in the original's "damaged" branch
//! (`WO_SHIP_COLL_SPARK_DAMAGE`, since `Ship_DispatchCollisionFx` hardcodes
//! `kind = 0` at its call site), which is a second, independent reason this
//! module's own single-variant design does not need to branch.
//!
//! So this effect is still **authored** for its look - the same tier
//! [`crate::exhaust`]'s `Trail` is - because the `.pob` asset itself, and
//! therefore the actual particle count/size/colour/lifetime the original
//! spawns, remain unrecovered. But `ShipCollisionFx_Trigger`'s severity
//! formula, `intensity * 2.0 + 0.4`, **is now ported** ([`SEVERITY_SLOPE`],
//! [`SEVERITY_FLOOR`]) - correcting a wrong conclusion from earlier the same
//! session, worth recording because catching it needed a live capture, not
//! more static reading. That earlier pass read `Ship_DispatchCollisionFx`'s
//! `intensity` argument as the **raw, unscaled** impulse magnitude, and on
//! that premise the formula looked unbounded and unusable: this crate's own
//! `resolve_contact` gives impulses of `1.39`-`165` for realistic speeds, so
//! `intensity * 2.0 + 0.4` would reach `69`-`165`, nowhere near a sane
//! range. **The premise was wrong.** A live PPSSPP capture -
//! `docs/reverse-engineering/ppsspp-debugger.md`'s methodology, breakpointed
//! on `ShipCollisionFx_Trigger`'s own severity write during a real wall
//! scrape - read six real `intensity` values (`0.011`-`0.070`, at craft
//! speeds `21.7`-`112.4` units/s) that are flatly incompatible with an
//! unscaled, unclamped impulse read. Re-reading `FUN_088418e0` fresh (not
//! trusting the earlier pass's paraphrase) found the actual line:
//! `fVar21 = min(|impulse| * 0.0125, 1.0)` - **`intensity` is already
//! clamped to `[0, 1]` before `Ship_DispatchCollisionFx` ever sees it**, the
//! same `0.0125` this module's own [`SEVERITY_SCALE`] already borrows. So
//! `intensity * 2.0 + 0.4` ranges over a sane, bounded `[0.4, 2.4]` - never
//! unbounded - and the capture's own numbers confirm it landing there.
//! [`COLLISION_COOLDOWN`] was already ported for the same reason this now
//! is: both are usable without the still-undecoded `.pob` resource data,
//! once the actual inputs are read correctly.
//!
//! # Two halves, deliberately
//!
//! [`Sparks`] is the state and the maths, with no `wgpu` in it, mirroring
//! [`crate::exhaust::Exhaust`]. [`Pipeline`] is the GPU side.
//!
//! # Trigger discipline lives in the caller, not here
//!
//! [`Sparks::spawn`] does not gate on anything - every call spawns a burst.
//! The caller (`crates/game/src/race.rs`) is responsible for calling it only
//! when [`COLLISION_COOLDOWN`] has elapsed since the last burst, not on every
//! tick of a sustained scrape. `oag_physics::wall::WallResponse::impact` stays
//! `true` for the whole duration of a scrape, and firing on every one of
//! those ticks is exactly the bug `oag_physics::wall::STUN_PER_CONTACT`'s doc
//! comment already records costing a session of play-testing - the fix here
//! is a cooldown timer rather than a one-shot edge latch, because the
//! original itself re-fires periodically rather than staying silent for the
//! rest of the scrape.

use oag_core::Rng;
use oag_core::math::Vec3;

use crate::mesh::GpuVertex;

/// Particles alive at once, across every burst still fading.
///
/// A fixed-size array, not a `Vec` - same no-allocation convention
/// `oag_physics::wall::WallResponse` and `oag_gameplay::World` already follow
/// for their own bounded, plain-data state.
///
/// `96`, not `64`: checked against a real PPSSPP capture
/// (`verification/scenarios/steer-left.inputs`, which ends in a wall hit on
/// Talon's Junction) - the original's burst reads as a much denser cluster
/// than this module's first-cut counts produced. Raised alongside
/// [`BURST_COUNT`]/[`MIN_BURST_COUNT`] rather than guessed independently.
pub const MAX_SPARKS: usize = 96;

/// Particles a full-severity burst spawns. **Authored**, raised from `16`
/// after the same capture comparison [`MAX_SPARKS`] records.
pub const BURST_COUNT: usize = 28;

/// Particles the weakest burst that still fires spawns. **Authored**, raised
/// from `4` for the same reason.
pub const MIN_BURST_COUNT: usize = 8;

/// Seconds a spark lives before fading out. **Authored.**
pub const LIFETIME: f32 = 0.4;

/// Half-angle of the cone particles eject in, around the contact normal.
/// **Authored.**
pub const CONE_HALF_ANGLE: f32 = std::f32::consts::FRAC_PI_4;

/// Downward acceleration applied to live particles, world units per second
/// squared. **Authored** - sparks arc rather than travelling in straight
/// lines, which is what reads as debris rather than as a directional spray.
pub const GRAVITY: Vec3 = Vec3::new(0.0, -30.0, 0.0);

/// Quad half-size at spawn, in world units, before the per-particle
/// [`SIZE_VARIATION`] and the lifetime fade. **Authored.**
///
/// Checked against a real capture twice, not just a formula. `0.06` (a
/// twentieth of the exhaust flare's own half-size range) rasterized to a
/// sub-pixel dot right next to the camera and read as nothing drawing at all -
/// the same trap the fragment shader's fix below describes. `0.3` cleared
/// that check close to the camera but still under-drew at an ordinary
/// contact's distance, several units further out - a real hull-probe contact
/// (`crates/game/src/race.rs`'s `a_sustained_scrape_spawns_sparks_once...`
/// fixture is close-up; most of a lap is not). `1.0` is what survived a
/// PPSSPP-matched capture (`verification/scenarios/steer-left.inputs`) at the
/// distance an actual wall hit happens at, and is still a third of the
/// flare's own `0.75`-`3.1`.
pub const BASE_SIZE: f32 = 1.0;

/// Bounds of the per-particle size multiplier. **Authored.**
pub const SIZE_VARIATION: (f32, f32) = (0.6, 1.4);

/// Bounds of the per-particle ejection-speed multiplier. **Authored.**
pub const EJECTION_VARIATION: (f32, f32) = (0.5, 1.5);

/// Seconds a wall contact must persist before another burst is allowed.
///
/// **Recovered.** `ShipCollisionFx_Trigger` (`0x089246b4`,
/// `docs/ghidra/functions/psp-pulse/contact-response.md`) re-arms exactly
/// this long after every collision-variant spawn: `instance + 100 = now +
/// 0.8`, checked on entry and skipped while still armed. A pure duration, so
/// unlike [`SEVERITY_SCALE`]/[`EJECTION_SCALE`] it needs no unit conversion
/// to reuse - the caller re-fires a burst every time this many seconds of
/// continuous contact pass, rather than once per impact and then silence for
/// the rest of a scrape.
pub const COLLISION_COOLDOWN: f32 = 0.8;

/// Converts a contact's impact speed into `[0, 1]` intensity.
///
/// **Recovered - this is the exact literal `FUN_088418e0` uses**:
/// `fVar21 = min(|impulse| * 0.0125, 1.0)`, the value `Ship_DispatchCollisionFx`
/// and then `ShipCollisionFx_Trigger` receive as `intensity`
/// (`docs/ghidra/functions/psp-pulse/contact-response.md`). Confirmed by a
/// live capture reading the real value at a real wall hit - see the module
/// doc comment. This module's own input is `speed`, not the original's
/// impulse magnitude, so the *scale* is still borrowed across a unit
/// difference; the coefficient and the clamp are not.
pub const SEVERITY_SCALE: f32 = 0.0125;

/// The multiplier `ShipCollisionFx_Trigger` applies to [`SEVERITY_SCALE`]'s
/// clamped intensity. **Recovered**, from the same `intensity * 2.0 + 0.4`
/// literal.
pub const SEVERITY_SLOPE: f32 = 2.0;

/// The floor `ShipCollisionFx_Trigger` adds on top of [`SEVERITY_SLOPE`] - a
/// hit is never zero severity, only ever `0.4` at its gentlest. **Recovered.**
pub const SEVERITY_FLOOR: f32 = 0.4;

/// Converts a contact's impact speed into an ejection speed, world units per
/// second.
///
/// **Borrowed, not recovered**, the same way [`SEVERITY_SCALE`] is: the
/// coefficient is `FUN_088418e0`'s hull-damage amplitude, `|impulse| * 0.05`.
/// Nothing establishes that the original's particle ejection speed - inside
/// the undecoded `.pob` asset - uses this number at all; it is picked because
/// it is the other recovered scale this exact hit already carries.
pub const EJECTION_SCALE: f32 = 0.05;

/// A warm white-orange, roughly what hot metal debris reads as.
/// **Authored.**
pub const COLOUR: [f32; 3] = [1.0, 0.75, 0.35];

/// One live spark. Dead when [`Particle::life`] reaches zero.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Particle {
    position: Vec3,
    velocity: Vec3,
    life: f32,
    max_life: f32,
    size: f32,
}

impl Particle {
    const DEAD: Self = Self {
        position: Vec3::ZERO,
        velocity: Vec3::ZERO,
        life: 0.0,
        max_life: 0.0,
        size: 0.0,
    };

    fn alive(self) -> bool {
        self.life > 0.0
    }
}

/// Per-frame state of one ship's collision sparks.
///
/// Mirrors [`crate::exhaust::Exhaust`] in shape and for the same reason: it is
/// render-only state the game crate owns and advances on the simulation's
/// fixed tick, so it never enters `World` and never touches a determinism
/// hash - see that struct's doc comment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sparks {
    particles: [Particle; MAX_SPARKS],
}

impl Default for Sparks {
    fn default() -> Self {
        Self::new()
    }
}

impl Sparks {
    /// No live particles.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            particles: [Particle::DEAD; MAX_SPARKS],
        }
    }

    /// Spawns one burst at `point`, ejecting into a cone around `normal`
    /// scaled by `speed` (the contact's impact speed, world units per
    /// second).
    ///
    /// Unconditional - see the module doc comment for why the edge-vs-level
    /// trigger discipline belongs to the caller and not here. Slots are taken
    /// from the first dead particle found, or, once the pool is full, from the
    /// particle nearest the end of its life - a burst under pressure replaces
    /// what is about to disappear anyway rather than dropping the new one.
    pub fn spawn(&mut self, point: Vec3, normal: Vec3, speed: f32, rng: &mut Rng) {
        let intensity = (speed * SEVERITY_SCALE).clamp(0.0, 1.0);
        // ShipCollisionFx_Trigger's own recovered shape, intensity*2.0+0.4,
        // renormalized from its [0.4, 2.4] range back to [0, 1] so it still
        // drives this module's own count interpolation - see the module doc
        // comment for why this is now safe to port.
        let severity =
            (intensity * SEVERITY_SLOPE + SEVERITY_FLOOR) / (SEVERITY_SLOPE + SEVERITY_FLOOR);
        let count =
            MIN_BURST_COUNT + (severity * (BURST_COUNT - MIN_BURST_COUNT) as f32).round() as usize;
        let ejection_speed = speed * EJECTION_SCALE;

        for _ in 0..count {
            let slot = self.oldest_slot();
            let direction = cone_direction(normal, rng);
            self.particles[slot] = Particle {
                position: point,
                velocity: direction * (ejection_speed * range(rng, EJECTION_VARIATION)),
                life: LIFETIME,
                max_life: LIFETIME,
                size: BASE_SIZE * range(rng, SIZE_VARIATION),
            };
        }
    }

    /// The index of a dead particle, or the one with the least life left.
    ///
    /// Ascending scan, so the choice depends only on the pool's own state and
    /// never on iteration order that could vary between runs.
    fn oldest_slot(&self) -> usize {
        let mut best = 0;
        let mut best_life = f32::INFINITY;
        for (i, particle) in self.particles.iter().enumerate() {
            if !particle.alive() {
                return i;
            }
            if particle.life < best_life {
                best_life = particle.life;
                best = i;
            }
        }
        best
    }

    /// Ages, moves and expires every live particle by one simulation tick.
    pub fn advance(&mut self, dt: f32) {
        for particle in &mut self.particles {
            if !particle.alive() {
                continue;
            }
            particle.velocity += GRAVITY * dt;
            particle.position += particle.velocity * dt;
            particle.life -= dt;
            if particle.life <= 0.0 {
                *particle = Particle::DEAD;
            }
        }
    }

    /// How many particles are currently live.
    ///
    /// Exists mainly for tests - it is what a caller checks to confirm a
    /// sustained scrape spawned one burst rather than one per tick, since
    /// [`Self::vertices`] would show the same thing only indirectly.
    #[must_use]
    pub fn alive_count(&self) -> usize {
        self.particles.iter().filter(|p| p.alive()).count()
    }

    /// This frame's billboards, one quad per live particle, faded by
    /// remaining life.
    ///
    /// `right` and `up` come from the camera, the same as
    /// [`crate::exhaust::Exhaust::vertices`], so every quad faces the viewer.
    #[must_use]
    pub fn vertices(&self, right: Vec3, up: Vec3) -> Vec<GpuVertex> {
        let mut out = Vec::with_capacity(MAX_SPARKS * 6);
        for particle in &self.particles {
            if !particle.alive() {
                continue;
            }
            let fade = (particle.life / particle.max_life).clamp(0.0, 1.0);
            let half = particle.size * fade;
            out.extend_from_slice(&quad(particle.position, right * half, up * half, fade));
        }
        out
    }
}

/// A value in `lo..hi`, from the seeded generator.
fn range(rng: &mut Rng, (lo, hi): (f32, f32)) -> f32 {
    lo + (hi - lo) * rng.next_f32()
}

/// A unit direction inside [`CONE_HALF_ANGLE`] of `normal`.
///
/// Not solid-angle-uniform - it biases slightly toward the cone's edge - but
/// this is authored geometry with no recovered distribution to match, so the
/// approximation is not worth a rejection sampler over.
fn cone_direction(normal: Vec3, rng: &mut Rng) -> Vec3 {
    let tangent = normal.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
    let bitangent = normal.cross(tangent);

    let theta = range(rng, (0.0, CONE_HALF_ANGLE));
    let phi = range(rng, (0.0, std::f32::consts::TAU));
    let (sin_t, cos_t) = theta.sin_cos();
    let (sin_p, cos_p) = phi.sin_cos();

    (normal * cos_t + (tangent * cos_p + bitangent * sin_p) * sin_t).normalize()
}

/// Six vertices - two triangles - for one camera-facing quad.
///
/// Colour carries the fade in its alpha; the shader's radial falloff supplies
/// the shape, since no authored spark texture exists to sample (the
/// original's is inside the undecoded `.pob` asset - see the module doc
/// comment).
fn quad(centre: Vec3, right: Vec3, up: Vec3, fade: f32) -> [GpuVertex; 6] {
    let corner = |sx: f32, sy: f32, u: f32, v: f32| GpuVertex {
        position: (centre + right * sx + up * sy).to_array(),
        normal: [0.0, 0.0, 1.0],
        colour: [COLOUR[0], COLOUR[1], COLOUR[2], fade],
        texcoord: [u, v],
        // Emissive: sparks must not pick up the mesh light rig.
        lit: 0.0,
        v_cycles: 0.0,
    };
    let bl = corner(-1.0, -1.0, 0.0, 1.0);
    let br = corner(1.0, -1.0, 1.0, 1.0);
    let tl = corner(-1.0, 1.0, 0.0, 0.0);
    let tr = corner(1.0, 1.0, 1.0, 0.0);
    [bl, br, tl, br, tr, tl]
}

/// The additive blend, the same shape [`crate::exhaust::BLEND`] uses: sparks
/// are the same kind of small, bright, emissive point the flare is, and
/// additive is what keeps overlapping ones reading as *brighter* rather than
/// as one occluding another.
pub const BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// The maximum vertices [`Pipeline`]'s buffer holds: one quad per [`MAX_SPARKS`].
pub const MAX_VERTICES: usize = MAX_SPARKS * 6;

/// The sparks' draw pipeline.
///
/// Simpler than [`crate::exhaust::Pipeline`] in one respect: there is no
/// texture to bind, since the shape is a procedural radial falloff computed
/// in the fragment shader rather than sampled - see [`quad`]'s doc comment
/// for why. Otherwise it matches `mesh_render`'s pipeline the same way the
/// exhaust does: same target format, same [`crate::mesh_render::DEPTH_FORMAT`],
/// depth-tested but not depth-writing, for the same transparency-ordering
/// reason `exhaust::Pipeline` documents.
#[derive(Debug)]
pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    /// Vertices actually uploaded by the last [`Pipeline::upload`].
    count: u32,
}

impl Pipeline {
    /// Builds the pipeline.
    ///
    /// `format` must be the target the caller's render pass writes, and
    /// `sample_count` must match its multisample state - see
    /// `mesh_render::build`.
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, sample_count: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sparks"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sparks.wgsl").into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sparks uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sparks"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sparks"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2,
                        4 => Float32
                    ],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(BLEND),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                // A camera-facing quad has no meaningful winding: the basis it
                // is built from flips as the camera orbits.
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::mesh_render::DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sparks uniforms"),
            size: crate::mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sparks uniforms"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sparks vertices"),
            size: (MAX_VERTICES * std::mem::size_of::<GpuVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            uniforms,
            bind_group,
            vertices,
            count: 0,
        }
    }

    /// Uploads this frame's camera matrix and geometry.
    ///
    /// Takes `&mut self` only for the vertex count; both writes go through
    /// `queue`, the same split [`crate::exhaust::Pipeline::upload`] uses.
    pub fn upload(
        &mut self,
        queue: &wgpu::Queue,
        view_projection: &[[f32; 4]; 4],
        vertices: &[GpuVertex],
    ) {
        let mut block = [[0.0f32; 4]; 8];
        block[..4].copy_from_slice(view_projection);
        block[4] = [1.0, 0.0, 0.0, 0.0];
        block[5] = [0.0, 1.0, 0.0, 0.0];
        block[6] = [0.0, 0.0, 1.0, 0.0];
        block[7] = [0.0, 0.0, 0.0, 1.0];
        queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&block));

        let n = vertices.len().min(MAX_VERTICES);
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&vertices[..n]));
        self.count = n as u32;
    }

    /// Draws into a pass the caller already opened.
    ///
    /// Must be issued **after** the opaque geometry, for the same
    /// depth-write-off reason as [`crate::exhaust::Pipeline::draw`].
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..self.count, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng() -> Rng {
        Rng::new(1)
    }

    #[test]
    fn a_full_severity_impact_spawns_the_full_burst() {
        let mut sparks = Sparks::new();
        let mut r = rng();
        // 1.0 / SEVERITY_SCALE saturates intensity to 1.0, which is also
        // where the recovered severity formula reaches its own ceiling
        // (2.4), so the normalized severity is 1.0 either way.
        sparks.spawn(Vec3::ZERO, Vec3::Y, 1.0 / SEVERITY_SCALE, &mut r);
        let alive = sparks.particles.iter().filter(|p| p.alive()).count();
        assert_eq!(alive, BURST_COUNT);
    }

    /// Not [`MIN_BURST_COUNT`] exactly - the recovered severity formula has a
    /// `0.4` floor even at zero intensity (`docs/ghidra/functions/psp-pulse/
    /// contact-response.md`), so a zero-speed "impact" normalizes to
    /// `0.4 / 2.4` rather than `0.0`, landing above the pool's own minimum.
    #[test]
    fn a_gentle_impact_still_spawns_above_the_pools_own_minimum() {
        let mut sparks = Sparks::new();
        let mut r = rng();
        sparks.spawn(Vec3::ZERO, Vec3::Y, 0.0, &mut r);
        let alive = sparks.particles.iter().filter(|p| p.alive()).count();
        let expected = MIN_BURST_COUNT
            + (SEVERITY_FLOOR / (SEVERITY_SLOPE + SEVERITY_FLOOR)
                * (BURST_COUNT - MIN_BURST_COUNT) as f32)
                .round() as usize;
        assert_eq!(alive, expected);
        assert!(
            alive > MIN_BURST_COUNT,
            "the floor should read as more than nothing"
        );
    }

    #[test]
    fn particles_die_after_their_lifetime() {
        let mut sparks = Sparks::new();
        let mut r = rng();
        sparks.spawn(Vec3::ZERO, Vec3::Y, 10.0, &mut r);
        assert!(sparks.particles.iter().any(|p| p.alive()));

        // One giant step clears the whole pool at once.
        sparks.advance(LIFETIME + 1.0);
        assert!(sparks.particles.iter().all(|p| !p.alive()));
    }

    #[test]
    fn a_spawned_particle_starts_at_the_contact_point() {
        let mut sparks = Sparks::new();
        let mut r = rng();
        let point = Vec3::new(3.0, 4.0, 5.0);
        sparks.spawn(point, Vec3::Y, 5.0, &mut r);
        assert!(
            sparks
                .particles
                .iter()
                .filter(|p| p.alive())
                .all(|p| p.position == point)
        );
    }

    #[test]
    fn the_pool_never_grows_past_its_fixed_capacity() {
        let mut sparks = Sparks::new();
        let mut r = rng();
        // Spawn enough bursts to overrun MAX_SPARKS several times over.
        for _ in 0..(MAX_SPARKS / MIN_BURST_COUNT + 4) {
            sparks.spawn(Vec3::ZERO, Vec3::Y, 1.0 / SEVERITY_SCALE, &mut r);
        }
        assert_eq!(sparks.particles.len(), MAX_SPARKS);
        let alive = sparks.particles.iter().filter(|p| p.alive()).count();
        assert!(alive <= MAX_SPARKS);
    }
}
