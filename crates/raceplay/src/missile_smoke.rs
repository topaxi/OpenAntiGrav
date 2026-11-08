//! Wipeout HD's Missile ribbons in a race: each of a missile's two heads lays a
//! short straight stub and a helix, both [`oag_fx::fat_line`] elements, a node a
//! tick while the missile flies, and they age out after it. The law is
//! `oag_fx::fat_line`'s; this is the pools, the ownership and the assets.
//!
//! Render-side state, like [`crate::rocket_smoke`]'s: nothing here reaches
//! [`oag_gameplay::World`] or a state hash. Only a title whose
//! [`oag_title::weapons::WeaponModels::missile_trail`] is set and whose texture and
//! ramp load draws it, which today is HD alone.

use oag_core::math::Vec3;
use oag_fx::exhaust::FlareTexture;
use oag_fx::fat_line::{Ribbon, Spec};
use oag_fx::rocket_smoke::OpacityRamp;
use oag_mesh::mesh::GpuVertex;
use oag_title::weapons::{FatLineSpec, MissileTrail, WeaponModels};
use oag_weapons::projectile::{MAX_FLIGHT_SECONDS, MAX_PROJECTILES};

use crate::Race;
use crate::rocket_smoke::rocket_rows;
use crate::weapons::missile_flare_anchors;

/// Elements in each of the two pools a missile draws from: `RibbonEffects` pools
/// `+0x50` (stubs) and `+0x68` (helices) both hold six
/// (`docs/ghidra/functions/ps3-hdfury-eu/missile-ribbon.md`). A missile fired
/// while a pool is full flies without that element, as the original's acquire
/// fails.
pub const POOL: usize = 6;

/// What the Missile's ribbons draw with: the material's texture, the opacity
/// ramp the helix's alpha reads, and the title's element config the scene sizes
/// its buffer from.
#[derive(Debug, Clone)]
pub struct MissileSmokeAssets {
    pub texture: FlareTexture,
    pub ramp: OpacityRamp,
    /// The title's config, with [`MissileTrail::blast_streak`] cleared where its texture
    /// did not load.
    pub trail: MissileTrail,
    /// The explosion's streak texture ([`MissileTrail::blast_streak`]); it loads, or
    /// fails, apart from the ribbons' own.
    pub streak_texture: Option<FlareTexture>,
}

/// Reads the title's [`WeaponModels::missile_trail`] texture, taking the ramp the
/// Rocket's load already read (one global table in the original), and reporting
/// what it found or why nothing draws.
pub(crate) fn load(
    archives: &mut oag_assets::Archives,
    models: &WeaponModels,
    ramp: Option<&OpacityRamp>,
    report: &mut Vec<String>,
) -> Option<MissileSmokeAssets> {
    let mut trail = models.missile_trail?;
    let platform = archives.layout.platform;
    let texture = archives
        .read_name(trail.texture)
        .ok()
        .and_then(|blob| crate::load::magstrip_wake::decode(&blob, platform));
    let Some((texture, ramp)) = texture.zip(ramp.cloned()) else {
        report.push(format!(
            "missile ribbons: {} or the opacity ramp missing or unreadable - nothing draws",
            trail.texture
        ));
        return None;
    };
    report.push(format!(
        "missile ribbons: {} {}x{}",
        trail.texture, texture.width, texture.height
    ));
    let streak_texture = trail.blast_streak.and_then(|streak| {
        let decoded = archives
            .read_name(streak.texture)
            .ok()
            .and_then(|blob| crate::load::magstrip_wake::decode(&blob, platform));
        match &decoded {
            Some(texture) => report.push(format!(
                "missile explosion streaks: {} {}x{}, {} elements",
                streak.texture, texture.width, texture.height, streak.pool
            )),
            None => report.push(format!(
                "missile explosion streaks: {} missing or unreadable - they do not draw",
                streak.texture
            )),
        }
        decoded
    });
    if streak_texture.is_none() {
        trail.blast_streak = None;
    }
    Some(MissileSmokeAssets {
        texture,
        ramp,
        trail,
        streak_texture,
    })
}

pub(crate) fn spec(spec: FatLineSpec) -> Spec {
    Spec {
        width: spec.width,
        lifetime: spec.lifetime,
        u_per_unit: spec.u_per_unit,
        chains: spec.chains as usize,
        capacity: spec.capacity as usize,
        phase_step: spec.phase_step,
        radius: spec.radius,
        ramp: spec.ramp,
    }
}

/// The pool slots a missile's two heads took, `(stub, helix)` each; `None` where
/// the pool was full.
type HeadRibbons = [(Option<usize>, Option<usize>); 2];

/// Which ribbons a projectile slot's missile owns, keyed by the age it was last
/// seen at so a new missile in a reused slot is told from the old one.
#[derive(Debug, Clone, Copy)]
struct Owner {
    age: f32,
    heads: HeadRibbons,
}

/// The two pools, and which missile owns each element.
#[derive(Debug, Clone)]
pub(crate) struct Smoke {
    trail: MissileTrail,
    stubs: [Option<Ribbon>; POOL],
    helices: [Option<Ribbon>; POOL],
    owners: [Option<Owner>; MAX_PROJECTILES],
}

fn acquire(pool: &mut [Option<Ribbon>; POOL], spec: Spec) -> Option<usize> {
    let free = pool.iter().position(Option::is_none)?;
    pool[free] = Some(Ribbon::new(spec));
    Some(free)
}

impl Smoke {
    pub(crate) fn new(trail: MissileTrail) -> Self {
        Self {
            trail,
            stubs: std::array::from_fn(|_| None),
            helices: std::array::from_fn(|_| None),
            owners: [None; MAX_PROJECTILES],
        }
    }

    /// Live elements, owned or ageing out.
    #[must_use]
    pub(crate) fn live(&self) -> usize {
        self.stubs.iter().flatten().count() + self.helices.iter().flatten().count()
    }
}

impl Race {
    /// One tick of every missile's ribbons: `Missile`'s update pushes a node into
    /// each of its four elements at the two heads' places, then
    /// `RibbonEffects_UpdatePools` ages every element and frees the empty ones.
    ///
    /// The heads sit on [`missile_flare_anchors`] and the basis is the velocity's
    /// ([`rocket_rows`]): **chosen, not measured**, as the head model's own placement
    /// is (`Race::missile_head_model_matrices`); the original's stores its two
    /// matrices at `missile + 0xf0` and `+0x130` (`0x0011e640`, unread).
    pub(crate) fn advance_missile_smoke(&mut self) {
        let dt = self.sim.dt;
        let Some(smoke) = self.view.missile_smoke.as_mut() else {
            return;
        };
        let (stub, helix) = (spec(smoke.trail.stub), spec(smoke.trail.helix));
        for (slot, projectile) in self.sim.world.projectiles.slots.iter().enumerate() {
            if projectile.kind != Some(oag_tables::weapons::Weapon::Missile) {
                smoke.owners[slot] = None;
                continue;
            }
            let fresh = smoke.owners[slot].is_none_or(|owner| projectile.age < owner.age);
            if fresh {
                let mut heads: HeadRibbons = [(None, None); 2];
                for head in &mut heads {
                    *head = (
                        acquire(&mut smoke.stubs, stub),
                        acquire(&mut smoke.helices, helix),
                    );
                }
                smoke.owners[slot] = Some(Owner {
                    age: projectile.age,
                    heads,
                });
            }
            let Some(owner) = smoke.owners[slot].as_mut() else {
                continue;
            };
            owner.age = projectile.age;
            let Some(forward) = projectile.velocity.try_normalize() else {
                continue;
            };
            let (_, up) = rocket_rows(forward, projectile.surface);
            let flight = MAX_FLIGHT_SECONDS - projectile.lifetime;
            let (a, b) = missile_flare_anchors(projectile.position, projectile.velocity, flight);
            for (head, anchor) in owner.heads.iter().zip([a, b]) {
                let (stub, helix) = *head;
                if let Some(ribbon) = stub.and_then(|i| smoke.stubs[i].as_mut()) {
                    ribbon.push(anchor, forward, up, 1.0);
                }
                if let Some(ribbon) = helix.and_then(|i| smoke.helices[i].as_mut()) {
                    ribbon.push(anchor, forward, up, 1.0);
                }
            }
        }
        for pool in [&mut smoke.stubs, &mut smoke.helices] {
            for element in pool.iter_mut() {
                if let Some(live) = element.as_mut() {
                    live.advance(dt);
                    if live.is_empty() {
                        *element = None;
                    }
                }
            }
        }
    }

    /// Every live element's triangle list, built for a camera at `eye`, alphas
    /// from `ramp`. Empty where the title draws no Missile ribbons.
    #[must_use]
    pub fn missile_smoke_vertices(&self, eye: Vec3, ramp: &OpacityRamp) -> Vec<GpuVertex> {
        let mut out = Vec::new();
        if let Some(smoke) = &self.view.missile_smoke {
            for ribbon in smoke.stubs.iter().chain(&smoke.helices).flatten() {
                ribbon.extend_vertices(&mut out, eye, ramp);
            }
        }
        out
    }

    /// How many elements are alive, owned or ageing out.
    #[must_use]
    pub fn missile_smoke_live(&self) -> usize {
        self.view.missile_smoke.as_ref().map_or(0, Smoke::live)
    }
}
