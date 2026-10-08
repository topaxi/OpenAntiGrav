//! Wipeout HD's Rocket smoke ribbon in a race: one [`Trail`] per rocket, laid
//! a node per tick while the rocket flies and left to age out after it hits.
//! The law is `oag_fx::rocket_smoke`'s; this is the ownership and the assets.
//!
//! Render-side state, like the LeachBeam ribbon's: nothing here reaches
//! [`oag_gameplay::World`] or a state hash. The CPU half runs on every title
//! and is a few vectors of nodes; only a title whose [`RibbonTextures`] carry
//! [`RocketSmokeAssets`] draws it, which today is HD alone - see
//! `oag_title::weapons::WeaponModels::rocket_trail`.

use oag_core::math::Vec3;
use oag_core::rng::Rng;
use oag_fx::exhaust::FlareTexture;
use oag_fx::rocket_smoke::{MAX_TRAILS, OpacityRamp, Trail, UNLIT_RGB};
use oag_mesh::mesh::GpuVertex;
use oag_title::weapons::WeaponModels;
use oag_weapons::projectile::MAX_PROJECTILES;

use crate::Race;

/// The ribbon's own jitter generator's seed, deliberately not `world.rng`:
/// the same separation `LEACH_BEAM_SEED` keeps.
pub const ROCKET_SMOKE_SEED: u64 = 0x5_70_4e_08;

/// What the Rocket's smoke draws with: its material's race texture and the
/// opacity ramp. Both or neither - see [`load`].
#[derive(Debug, Clone)]
pub struct RocketSmokeAssets {
    pub texture: FlareTexture,
    pub ramp: OpacityRamp,
}

/// The ribbon effects' textures a race hands its scene: the HD-lineage
/// magstrip arc wake's `[atlas, contact]` (see `load::magstrip_wake`) and the
/// Rocket's smoke. `None` draws nothing rather than a stand-in.
#[derive(Debug, Clone, Default)]
pub struct RibbonTextures {
    pub magstrip: Option<[FlareTexture; 2]>,
    pub rocket_smoke: Option<RocketSmokeAssets>,
}

/// Reads the title's [`WeaponModels::rocket_trail`] entries, reporting what
/// it found or why nothing draws.
pub(crate) fn load(
    archives: &mut oag_assets::Archives,
    models: &WeaponModels,
    report: &mut Vec<String>,
) -> Option<RocketSmokeAssets> {
    let trail = models.rocket_trail?;
    let platform = archives.layout.platform;
    let texture = archives
        .read_name(trail.texture)
        .ok()
        .and_then(|blob| crate::load::magstrip_wake::decode(&blob, platform));
    let ramp = archives
        .read_name(trail.opacity_ramp)
        .ok()
        .and_then(|blob| OpacityRamp::from_tga(&blob));
    let Some((texture, ramp)) = texture.zip(ramp) else {
        report.push(format!(
            "rocket smoke ribbon: {} or {} missing or unreadable - nothing draws",
            trail.texture, trail.opacity_ramp
        ));
        return None;
    };
    report.push(format!(
        "rocket smoke ribbon: {} {}x{}, opacity ramp {} (entry 0 = {})",
        trail.texture,
        texture.width,
        texture.height,
        trail.opacity_ramp,
        ramp.table()[0]
    ));
    Some(RocketSmokeAssets { texture, ramp })
}

/// Which ribbon a projectile slot's rocket owns, keyed by the age it was last
/// seen at, so a new rocket in a reused slot is told from the old one.
#[derive(Debug, Clone, Copy)]
struct Owner {
    age: f32,
    /// `None` when every ribbon was busy at the shot: the original's pool
    /// acquire fails and that rocket flies without one for its whole life.
    trail: Option<usize>,
}

/// Pool 4: [`MAX_TRAILS`] ribbons, and which rocket owns each.
#[derive(Debug, Clone)]
pub(crate) struct Smoke {
    trails: [Option<Trail>; MAX_TRAILS],
    owners: [Option<Owner>; MAX_PROJECTILES],
    rng: Rng,
}

impl Smoke {
    pub(crate) fn new() -> Self {
        Self {
            trails: std::array::from_fn(|_| None),
            owners: [None; MAX_PROJECTILES],
            rng: Rng::new(ROCKET_SMOKE_SEED),
        }
    }

    /// Live ribbons, owned or ageing out.
    #[must_use]
    pub(crate) fn live(&self) -> usize {
        self.trails.iter().flatten().count()
    }
}

impl Race {
    /// One tick of every rocket's ribbon: `Rocket_Update` pushes a node at the
    /// rocket each update, then `RibbonEffects_UpdatePools` ages every chain
    /// and frees the empty ones - the order the dump's newest node (born
    /// `1.85 - dt`, alpha 0) says.
    ///
    /// The node sits at the rocket's own origin, **chosen, not measured**: the
    /// original adds a scaled vector to the position it pushes
    /// (`0x00124880..0x00124888`), unread. The basis is the one the Rocket's
    /// model draws on (`projectile_model_matrices`): row 0 `surface x
    /// forward`, row 1 `forward x row 0`.
    pub(crate) fn advance_rocket_smoke(&mut self) {
        let dt = self.sim.dt;
        let smoke = &mut self.view.rocket_smoke;
        for (slot, projectile) in self.sim.world.projectiles.slots.iter().enumerate() {
            if projectile.kind != Some(oag_tables::weapons::Weapon::Rocket) {
                smoke.owners[slot] = None;
                continue;
            }
            let fresh = smoke.owners[slot].is_none_or(|owner| projectile.age < owner.age);
            if fresh {
                let free = smoke.trails.iter().position(Option::is_none);
                if let Some(index) = free {
                    smoke.trails[index] = Some(Trail::new());
                }
                smoke.owners[slot] = Some(Owner {
                    age: projectile.age,
                    trail: free,
                });
            }
            let Some(owner) = smoke.owners[slot].as_mut() else {
                continue;
            };
            owner.age = projectile.age;
            let (Some(index), Some(forward)) = (owner.trail, projectile.velocity.try_normalize())
            else {
                continue;
            };
            let reference = [projectile.surface, Vec3::Y]
                .into_iter()
                .find(|axis| forward.dot(*axis).abs() < 0.999)
                .unwrap_or_else(|| forward.any_orthonormal_vector());
            let right = reference.cross(forward).normalize_or_zero();
            let up = forward.cross(right);
            if let Some(trail) = smoke.trails[index].as_mut() {
                trail.push(projectile.position, right, up, UNLIT_RGB, &mut smoke.rng);
            }
        }
        for trail in &mut smoke.trails {
            if let Some(live) = trail.as_mut() {
                live.advance(dt);
                if live.is_empty() {
                    *trail = None;
                }
            }
        }
    }

    /// Every live ribbon's triangle list, alphas from `ramp`.
    #[must_use]
    pub fn rocket_smoke_vertices(&self, ramp: &OpacityRamp) -> Vec<GpuVertex> {
        let mut out = Vec::new();
        for trail in self.view.rocket_smoke.trails.iter().flatten() {
            trail.extend_vertices(&mut out, ramp);
        }
        out
    }

    /// How many ribbons are alive, owned or ageing out.
    #[must_use]
    pub fn rocket_smoke_live(&self) -> usize {
        self.view.rocket_smoke.live()
    }
}
