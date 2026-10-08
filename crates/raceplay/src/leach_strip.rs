//! Wipeout HD's LeachBeam strip in a race: every craft's anchor trail,
//! recorded each tick, and the strip built from the target's trail while a
//! locked beam lives. The law is `oag_fx::leach_strip`'s; this is the
//! ownership and the assets.
//!
//! Render-side state like the Rocket smoke's: nothing here reaches
//! [`oag_gameplay::World`] or a state hash. Only a title whose
//! [`RibbonTextures`](crate::rocket_smoke::RibbonTextures) carry
//! [`LeachStripAssets`] draws it, today HD alone - see
//! `oag_title::weapons::WeaponModels::leach_strip`. Where it draws, Pulse's own
//! ribbon does not.

use oag_core::math::{Mat4, Vec3};
use oag_fx::exhaust::FlareTexture;
use oag_fx::leach_strip::{self as law, AnchorTrail, Phase};
use oag_gameplay::MAX_SHIPS;
use oag_mesh::mesh::GpuVertex;
use oag_title::weapons::WeaponModels;
use oag_weapons::projectile::leach_beam::Kind;

use crate::Race;

/// Vertices the strip's buffer holds: a full 299-sample path, three fins.
pub const VERTICES: usize = (law::RING - 2) * oag_fx::rocket_smoke::FINS * 6;

/// What the strip draws with: unit 1's glow and unit 0's noise.
#[derive(Debug, Clone)]
pub struct LeachStripAssets {
    pub glow: FlareTexture,
    pub noise: FlareTexture,
}

/// Reads the title's [`WeaponModels::leach_strip`] textures, reporting what it
/// found or why nothing draws.
pub(crate) fn load(
    archives: &mut oag_assets::Archives,
    models: &WeaponModels,
    report: &mut Vec<String>,
) -> Option<LeachStripAssets> {
    let strip = models.leach_strip?;
    let platform = archives.layout.platform;
    let mut read = |name: &str| {
        archives
            .read_name(name)
            .ok()
            .and_then(|blob| crate::load::magstrip_wake::decode(&blob, platform))
    };
    let (Some(glow), Some(noise)) = (read(strip.glow), read(strip.noise)) else {
        report.push(format!(
            "leach strip: {} or {} missing or unreadable - Pulse's beam ribbon draws instead",
            strip.glow, strip.noise
        ));
        return None;
    };
    report.push(format!(
        "leach strip: glow {} {}x{}, noise {} {}x{}",
        strip.glow, glow.width, glow.height, strip.noise, noise.width, noise.height
    ));
    Some(LeachStripAssets { glow, noise })
}

/// Every craft's trail, the hull locators the anchors are taken from, and the
/// wobble's three phases.
#[derive(Debug, Clone)]
pub(super) struct Strip {
    anchors: [Option<Mat4>; MAX_SHIPS],
    trails: Vec<AnchorTrail>,
    phases: [f32; 3],
}

impl Strip {
    pub(super) fn new(anchors: [Option<Mat4>; MAX_SHIPS]) -> Self {
        Self {
            anchors,
            trails: (0..MAX_SHIPS).map(|_| AnchorTrail::new()).collect(),
            phases: [0.0; 3],
        }
    }
}

impl Race {
    /// Whether this race draws HD's strip instead of Pulse's ribbon.
    #[must_use]
    pub(crate) fn has_leach_strip(&self) -> bool {
        self.view.leach_strip.is_some()
    }

    /// The craft's anchor this tick: its `arc_anchor_point` node's position
    /// pushed [`law::ANCHOR_OFFSET`] along the node's row 1. A hull with no
    /// locator takes the body the same way, as the original's own fallback
    /// does (`0x001169f8`).
    fn leach_anchor(&self, anchors: &[Option<Mat4>; MAX_SHIPS], slot: usize) -> Vec3 {
        let model = self.ship_model_matrix_of(slot);
        let node = anchors[slot].map_or(model, |anchor| model * anchor);
        law::anchor_point(
            node.w_axis.truncate(),
            node.y_axis.truncate().normalize_or_zero(),
        )
    }

    /// The craft's progress along the course, unwrapped across laps: **this
    /// engine's equivalent** of the original's 0..1 lap fraction with a lap fix
    /// (`craft + 0x7020`), the same ordering.
    fn leach_progress(&self, slot: usize) -> f32 {
        let standing = &self.sim.world.ships[slot].standing;
        standing.lap.saturating_sub(1) as f32 * self.leach_course_length()
            + standing.progress.unwrap_or(0.0)
    }

    fn leach_course_length(&self) -> f32 {
        self.sim
            .course
            .as_ref()
            .map_or(0.0, |course| course.length())
    }

    /// The craft's trail record as of now: anchor, progress and the body's
    /// rows 0 and 1 (`+0x1d0`, `+0x1e0`), here the physics body's own axes.
    fn leach_record(&self, anchors: &[Option<Mat4>; MAX_SHIPS], slot: usize) -> law::Record {
        let orientation = self.sim.world.ships[slot].physics.body.orientation;
        law::Record {
            anchor: self.leach_anchor(anchors, slot),
            progress: self.leach_progress(slot),
            row0: orientation * Vec3::X,
            row1: orientation * Vec3::Y,
        }
    }

    /// Whether a beam the strip draws is alive: locked and not broken.
    fn leach_beam_live(&self) -> Option<oag_weapons::projectile::leach_beam::Beam> {
        self.sim
            .world
            .leach_beam
            .filter(|beam| beam.kind == Kind::Locked && beam.disconnected_at.is_none())
    }

    /// `LeachTrail_RecordAnchor`, every tick, every craft; and the wobble's
    /// phases, which step once a tick while a beam lives and start again with
    /// the next.
    pub(crate) fn advance_leach_strip(&mut self) {
        let Some(mut strip) = self.view.leach_strip.take() else {
            return;
        };
        for slot in 0..usize::from(self.sim.world.ship_count).min(MAX_SHIPS) {
            if !self.ship_active(slot) {
                continue;
            }
            let record = self.leach_record(&strip.anchors, slot);
            strip.trails[slot].record(record);
        }
        if self.leach_beam_live().is_some() {
            law::advance_wobble(&mut strip.phases);
        } else {
            strip.phases = [0.0; 3];
        }
        self.view.leach_strip = Some(strip);
    }

    /// The strip's triangle list for this frame, empty while there is no
    /// live beam, while the ball still flies, or once the link broke (HD draws
    /// states 3 and 4 only: no linger fade).
    ///
    /// The path is walked on the trail of whichever of the two craft is
    /// **ahead** (`0x001179c8` swaps the pair by the sign of a progress
    /// difference), back to the other's progress.
    #[must_use]
    pub(crate) fn leach_strip_vertices(&self) -> Vec<GpuVertex> {
        let (Some(strip), Some(beam)) = (&self.view.leach_strip, self.leach_beam_live()) else {
            return Vec::new();
        };
        let Some((reveal, window)) = Phase::at(beam.age).reveal() else {
            return Vec::new();
        };
        let (shooter, target) = (usize::from(beam.owner), usize::from(beam.target));
        let (shooter_record, target_record) = (
            self.leach_record(&strip.anchors, shooter),
            self.leach_record(&strip.anchors, target),
        );
        let (leader, leader_record, trailer_record) =
            if target_record.progress >= shooter_record.progress {
                (target, target_record, shooter_record)
            } else {
                (shooter, shooter_record, target_record)
            };
        let mut samples = law::walk(
            &strip.trails[leader],
            leader_record,
            trailer_record.anchor,
            trailer_record.progress,
        );
        law::bend(&mut samples, trailer_record.anchor);
        law::wobble(&mut samples, &strip.phases, self.leach_course_length());
        let nodes = law::nodes(&samples, reveal, window);
        let mut out =
            Vec::with_capacity(nodes.len().saturating_sub(1) * oag_fx::rocket_smoke::FINS * 6);
        law::extend_vertices(&mut out, &nodes);
        out
    }
}
