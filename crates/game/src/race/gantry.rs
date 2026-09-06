//! The start gantry: loading `321Go_StartFinish.vex`, standing it on the
//! circuit's own mount, and running its authored timeline off the race clock.
//!
//! `docs/rendering/start-gantry.md` has what the asset *does*: one 12.35-second
//! timeline of `Anim Transform` one-shots and a `TEXOFFSET` walk that plays
//! `3`, `2`, `1`, `GO` and then hands the board over to a `FINAL LAP` and a
//! chequered state. What it did not have, for seven passes, was where the thing
//! stands - `TrackStartup.xml`'s slot 8 names the model and carries no
//! transform, and neither does the billboard system that loads it.
//!
//! [`oag_render::gantry`] is the half that answers that, off the circuit's own
//! geometry. This is the half that draws it.
//!
//! # The clock is the race clock, and nothing is offset to make it line up
//!
//! `seconds` here is `world.tick / 60`, the same clock the track's scenery
//! animation rides, with zero at the moment the race starts. That is the only
//! alignment the asset authors and it is used exactly as it is:
//!
//! - The board's `GO` first lights at **3.03-3.6 s**.
//! - The measured thrust gate opens at **272 ticks = 4.533 s**
//!   ([`oag_race::COUNTDOWN_TICKS`], three live captures).
//!
//! So the gantry says `GO` about a second before a craft can move. **That gap
//! is real, unexplained and reproduced rather than papered over**;
//! `docs/rendering/start-gantry.md`'s timing section says outright that nothing
//! measures it and no wiring should assume it. Shifting this clock to make the
//! two coincide would be inventing a constant the disc does not author, and it
//! would hide the discrepancy that is the actual open question.
//!
//! # Why the clock stops, and where
//!
//! The asset's timeline keeps going after the countdown: at frame 560 the
//! `FINAL LAP` board slides in, and at frame 740 the chequered one does. Those
//! are two more states of the same object, and **their triggers are
//! unrecovered** - a lap counter reaching the last lap is the obvious guess and
//! a guess is exactly what `CLAUDE.md` says not to fire an effect on. Running
//! the timeline through would announce the final lap nine seconds into lap one,
//! on every race.
//!
//! So the clock is held at [`CLOCK_LIMIT`], the last frame before the lap
//! board's own first key. The countdown plays in full off the disc's own
//! timing; the two states with no trigger are simply never reached.

use super::*;

use oag_render::gantry::Mount;

/// Where the gantry's authored timeline is held, in seconds.
///
/// **The frame is authored; stopping there is this project's decision.**
/// `docs/rendering/start-gantry.md`'s timeline table has frame 560/561
/// (9.333 s) as `Final_Lap`'s first key, so this is the last frame before the
/// first state whose trigger is unrecovered. Holding rather than looping is
/// what makes the gantry sit in its post-countdown idle - `Board`, `Text` and
/// `Arrow` in place, the digit panel already teleported out of the aperture -
/// for the rest of the race, which is the state the asset itself holds from
/// 7.25 s to 9.333 s.
pub const CLOCK_LIMIT: f32 = 559.0 / 60.0;

/// The gantry, placed: the model and the matrix that stands it on the mount.
///
/// Produced by [`place`] at load and consumed by [`Gantry::new`], so the
/// measurement happens once on the loading screen and not per frame.
pub struct Placed {
    pub(super) model: Model,
    pub(super) matrix: Mat4,
}

impl std::fmt::Debug for Placed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Placed")
            .field("model", &self.model.label)
            .field("matrix", &self.matrix)
            .finish()
    }
}

/// Finds the circuit's mount, loads the gantry and works out its matrix.
///
/// `None` - and a line in `report` saying why - whenever any link in that chain
/// is missing. A circuit whose track authors no mount gets **no gantry**, not a
/// gantry at a coordinate borrowed from a circuit that does: the mount is a
/// per-circuit measurement and eleven of the twelve values differ.
pub(super) fn place(
    archives: &mut oag_assets::Archives,
    name: &str,
    track_model: &Model,
    start: Option<&StartPosition>,
    lod: mesh::Lod,
    report: &mut Vec<String>,
) -> Option<Placed> {
    let Some(mount) = oag_render::gantry::mount(track_model) else {
        report.push(
            "no start gantry: this circuit's track authors no 321backplate/billboard8 \
             surface, so there is nothing measured to stand one on"
                .to_string(),
        );
        return None;
    };
    let Some(start) = start else {
        report.push(
            "no start gantry: the mount was found but the track authors no Start Position, \
             and the mount's plane normal has no sign without one"
                .to_string(),
        );
        return None;
    };
    let model = match load(archives, name, lod) {
        Ok(model) => model,
        Err(e) => {
            report.push(format!("no start gantry: {name} did not load ({e})"));
            return None;
        }
    };
    let mut model = model;
    let parked = oag_render::gantry::clip_to_panel(&mut model, mount.width / 2.0, 0.0);
    let matrix = matrix(&mount, Vec3::from(start.forward));
    report.push(format!(
        "start gantry {name} on node {:?}: centre {:?}, on a panel {:.1} x {:.1}, \
         {:.0} units ahead of the Start Position - measured off this circuit's own \
         geometry (docs/rendering/start-gantry.md). {parked} draw(s) parked outside \
         the panel are not drawn: they are the FINAL LAP and chequered states, whose \
         trigger is unrecovered",
        mount.node,
        mount.centre.to_array().map(|v| (v * 10.0).round() / 10.0),
        mount.width,
        mount.height,
        (mount.centre - Vec3::from(start.position)).length(),
    ));
    Some(Placed { model, matrix })
}

/// Reads the gantry model, trying the manifest's spelling and then the
/// archive's.
///
/// **The two do not match on Pulse and the mismatch is the manifest's, not a
/// decode fault.** `16_Track`'s slot 8 authors
/// `/Data/Environments/321_Go/321Go_StartFinish.vex` - a leading separator and
/// forward slashes - while the WAD hashes its entries under
/// `Data\Environments\321_Go\321Go_StartFinish.vex`. HD's PSARC paths really
/// are `/`-joined, so the authored spelling is tried first and unchanged; the
/// PSP spelling is a fallback rather than a rewrite.
fn load(archives: &mut oag_assets::Archives, name: &str, lod: mesh::Lod) -> Result<Model> {
    let trimmed = name.trim_start_matches(['/', '\\']);
    let mut error = None;
    for candidate in [name, &trimmed.replace('/', "\\"), trimmed] {
        match archives.read_name(candidate) {
            Ok(blob) => return mesh::build_with_textures(name, &blob, None, lod),
            Err(e) => error = error.or(Some(e)),
        }
    }
    Err(error.map_or_else(|| anyhow::anyhow!("{name} is in no archive"), Into::into))
}

/// How far in front of the mount the gantry stands, in track units.
///
/// **Chosen, not measured** - no confidence score, because nothing on the disc
/// says it. What the disc does say is that the mounting surface is a *backing*
/// panel: `321backplate.tga` and the 8x8 `billboard8.tga` stub are two
/// co-located surfaces on one node, and on eleven of the twelve circuits that
/// author both they sit **0.48 to 1.63 units apart** on the same plane
/// (`gantry_mount_ground_truth`). So the artists' own step between stacked
/// surfaces there is about a unit, and this is that order.
///
/// It is not zero because it cannot be: standing the model exactly on the
/// panel makes its board coplanar with the track's own, and the earlier
/// surface wins a `Less` depth test at every distance - which is not a
/// subtlety, it is the whole gantry invisible with the placeholder showing
/// through in its place. That was the first thing this drew.
const CLEARANCE: f32 = 1.0;

/// The model matrix that stands the gantry on `mount` for a race running along
/// `forward`.
///
/// Three decisions, each from a measurement:
///
/// 1. **The board faces the grid, so it faces `-forward`.** The mount sits
///    163-174 units *ahead* of the `Start Position` on all twelve circuits that
///    author one (`gantry_mount_ground_truth`, `ahead-ness` +0.99 to +1.00), so
///    the craft approach it from behind and the side they see is the one facing
///    back down the track.
/// 2. **The model's own board faces its `+Z`.** `start_light_321goShape`'s
///    plane normal is `(0, 0, 1)` and its authored vertex normals agree - the
///    same test's first block.
/// 3. **Scale 1.0, and it is measured rather than assumed.** The gantry's
///    countdown board - `start_light_321goShape`, the one the countdown is
///    actually on - is 34.02 units across, and the panel the track authors is
///    36.30 to 45.50 wide over the twelve circuits that author one. So the
///    board fits its mount at 1:1 on every circuit, and
///    `gantry_mount_ground_truth` asserts exactly that rather than leaving it
///    as a remark. No factor is fitted to improve on it.
///
///    **Not the model's widest piece.** `polySurfaceShape7`, the chequered
///    state, is 43.25 across and overhangs `13_Track`'s 36.30 panel - but that
///    is one of the pieces [`oag_render::gantry::clip_to_panel`] drops, so it
///    never reaches the screen. The claim is about the board being drawn, not
///    about the file's bounding box.
fn matrix(mount: &Mount, forward: Vec3) -> Mat4 {
    let facing = mount.facing(-forward);
    let mut placed = mount.matrix(-forward, Vec3::Z, 1.0);
    placed.w_axis += (facing * CLEARANCE).extend(0.0);
    placed
}

/// The gantry on the GPU: one drawable and the matrix it is drawn at.
#[derive(Debug)]
pub(super) struct Gantry {
    drawable: Drawable,
    matrix: Mat4,
}

impl Gantry {
    /// Builds the pipelines for an already-placed gantry.
    ///
    /// # Errors
    ///
    /// Propagates a pipeline or geometry upload failure, the same as every
    /// other [`Drawable`].
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        placed: Placed,
        format: wgpu::TextureFormat,
        anisotropy: Anisotropy,
        sample_count: u32,
        shadow_map: &oag_render::shadow::map::Map,
    ) -> Result<Self> {
        let drawable = Drawable::new(
            device,
            queue,
            placed.model,
            format,
            anisotropy,
            sample_count,
            mesh_render::Depth::Scene,
            mesh_render::TRANSPARENT_BLEND,
            mesh_render::GlowMask::Protected,
            None,
            Some(shadow_map.view()),
            Some(shadow_map.depth_view()),
            // Scenery, like the track: it stands on the road and the road's
            // own shadow tiers reach it.
            mesh_render::ShadowReceiver::Both,
        )?;
        Ok(Self {
            drawable,
            matrix: placed.matrix,
        })
    }

    /// The fog block this frame, shared with the rest of the scenery.
    pub(super) fn fog(&self) -> &wgpu::Buffer {
        &self.drawable.fog
    }

    /// Writes the frame's matrices and both animation tables.
    ///
    /// `seconds` is the **race** clock, and it is clamped at [`CLOCK_LIMIT`]
    /// here rather than by the caller so there is one place the decision lives.
    pub(super) fn write(
        &self,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        previous_view_projection: Mat4,
        seconds: f32,
    ) {
        self.drawable.write(
            queue,
            view_projection,
            self.matrix,
            previous_view_projection,
        );
        // The gantry is static, so its previous model matrix is this one: the
        // velocity buffer sees only the camera's own motion across it.
        let seconds = seconds.min(CLOCK_LIMIT);
        self.drawable.write_anims(queue, seconds);
        self.drawable.write_node_anims(queue, seconds);
    }

    /// Draws it, frustum-culled and nothing else.
    ///
    /// No PVS: the gantry is not in the track file, so it carries no `section`
    /// id to look one up with - the same exemption the speed pads take.
    pub(super) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        frustum: Option<&oag_core::math::frustum::Frustum>,
    ) -> SceneStats {
        self.drawable.draw(pass, None, None, None, frustum)
    }
}
