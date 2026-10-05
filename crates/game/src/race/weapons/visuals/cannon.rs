//! What a Cannon round draws: its model, its bolt streak and its muzzle
//! flash, on whichever title's own terms the loaded [`CannonDraw`] names.
//!
//! **Two readings, one per executable, and they do not agree on the model.**
//! Pulse hangs `pulse_muzzleflash.vex` on the round for its whole flight and
//! centres its flash quad on the round
//! (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`). Wipeout
//! HD's `CannonBullet` (`docs/ghidra/functions/ps3-hdfury-eu/cannon.md`)
//! instead re-places `hd_muzzleflash` at the firing craft's `cannon_flash`
//! locator every tick the round is under `0.1` s old and hides it after, and
//! centres the flash quad on the same locator - so on HD the round in flight
//! is the bolt streak alone. `oag_title::weapons::CannonLook` carries which.

use oag_title::weapons::{CannonBody, CannonLook};

use super::*;

/// The Cannon's loaded textures and its title's own reading, in one value:
/// `crate::race::load::weapon_models::cannon_quads`' result, unnamed as a
/// tuple for the line budget the loader's own file has none of.
pub(crate) type CannonAssets = (
    Option<FlareTexture>,
    Option<FlareTexture>,
    Option<CannonLook>,
);

/// What drawing a Cannon round needs beyond the simulation: the title's own
/// reading and, for a title that flashes at the muzzle, every slot's two
/// `cannon_flash` locators in its hull's model space.
///
/// Built once from the loaded liveries; see `crate::livery::Livery`'s own
/// `cannon_flash` field.
#[derive(Debug, Clone, Default)]
pub(crate) struct CannonDraw {
    /// `None` draws on Pulse's terms - see
    /// `oag_title::weapons::WeaponModels::cannon_look`.
    pub(crate) look: Option<CannonLook>,
    /// Per slot, `[left, right]`.
    pub(crate) muzzles: Vec<[Option<Mat4>; 2]>,
}

impl Race {
    /// Where each Cannon model is drawn this frame.
    ///
    /// **On Pulse, one per live round, riding it.** `Cannon_Construct`
    /// (`0x088651d8`) loads [`CANNON_MODEL_ENTRY`] into every round instance's
    /// own scene node, velocity-oriented like the Rocket's: a round is a body
    /// in flight with a direction of travel, and
    /// `oag_gameplay::projectile::cannon::launch` gives it no independent
    /// orientation to read.
    ///
    /// **On Wipeout HD, one per round under the flash window, at the muzzle** -
    /// see [`Self::cannon_muzzle_matrix`].
    #[must_use]
    pub(crate) fn cannon_model_matrices(&self, draw: &CannonDraw) -> Vec<Mat4> {
        let Some(look) = draw.look else {
            return self.projectile_model_matrices(oag_tables::weapons::Weapon::Cannon);
        };
        let CannonBody::Muzzle { stretch_range } = look.body else {
            return self.projectile_model_matrices(oag_tables::weapons::Weapon::Cannon);
        };
        use oag_render::weapon_quads::{geometry, random};
        let tick = self.sim.world.tick as u32;
        self.flashing_cannon_rounds()
            .filter_map(|(slot, projectile)| {
                let (position, axes) = self.cannon_muzzle_matrix(draw, projectile)?;
                let (_, half_size, _) =
                    random::ranged_flash_roll(slot as u32, tick, look.flash_size_range);
                let scale = half_size / geometry::FLASH_SIZE_SCALE;
                let stretch = random::stretch_roll(slot as u32, tick, stretch_range);
                Some(Mat4::from_cols(
                    (axes[0] * scale).extend(0.0),
                    (axes[1] * scale).extend(0.0),
                    (axes[2] * (scale * stretch)).extend(0.0),
                    position.extend(1.0),
                ))
            })
            .collect()
    }

    /// This frame's vertices for every live Cannon round's two hand-built
    /// quads: the bolt streak into `bolt`, the muzzle flash into `flash`
    /// while the round's age is under
    /// [`oag_render::weapon_quads::geometry::FLASH_WINDOW_SECONDS`].
    ///
    /// `right`/`up` are the camera's own basis vectors, the same ones
    /// [`Self::projectile_sprites`] takes. **The flash's rotation, size and
    /// alpha are rolled from [`oag_render::weapon_quads::random`], seeded from
    /// the round's own slot index and [`World::tick`] - not from
    /// `self.sim.world.rng`.** Rolling it from the simulation's own seeded
    /// stream would advance that stream once per live round per tick for a
    /// value nothing in `oag_gameplay` ever reads back, moving every committed
    /// determinism hash for a purely cosmetic reason.
    ///
    /// The previous tick's position - the bolt's other endpoint - is derived
    /// as `position - velocity * dt` rather than stored on `Projectile`, since
    /// both are already there and adding a field for one draw call would be
    /// new simulation state for a render-only need.
    ///
    /// On HD the streak takes the title's own half-width and runs the whole
    /// segment, and the flash is centred on the muzzle rather than the round,
    /// drawing nothing for a craft with no `cannon_flash` locator.
    pub(crate) fn cannon_quad_vertices(
        &self,
        draw: &CannonDraw,
        right: Vec3,
        up: Vec3,
        bolt: &mut Vec<oag_mesh::mesh::GpuVertex>,
        flash: &mut Vec<oag_mesh::mesh::GpuVertex>,
    ) {
        use oag_render::weapon_quads::{geometry, random};
        let dt = TickRate::DEFAULT.dt();
        let tick = self.sim.world.tick as u32;
        for projectile in &self.sim.world.projectiles.slots {
            if projectile.kind != Some(oag_tables::weapons::Weapon::Cannon) {
                continue;
            }
            let curr = projectile.position;
            let prev = curr - projectile.velocity * dt;
            bolt.extend(match draw.look {
                Some(look) => geometry::shaped_bolt_vertices(
                    prev,
                    curr,
                    right,
                    up,
                    look.bolt_half_width,
                    look.bolt_near_fraction,
                ),
                None => geometry::bolt_vertices(prev, curr, right, up),
            });
        }
        for (slot, projectile) in self.flashing_cannon_rounds() {
            let (centre, (rotation, half_size, alpha)) = match draw.look {
                Some(look) => {
                    let Some((position, _)) = self.cannon_muzzle_matrix(draw, projectile) else {
                        continue;
                    };
                    let roll = random::ranged_flash_roll(slot as u32, tick, look.flash_size_range);
                    (position, roll)
                }
                None => (projectile.position, random::flash_roll(slot as u32, tick)),
            };
            flash.extend(geometry::flash_vertices(
                centre, right, up, half_size, rotation, alpha,
            ));
        }
    }

    /// Every live Cannon round still inside the flash window, with its slot.
    fn flashing_cannon_rounds(
        &self,
    ) -> impl Iterator<Item = (usize, &oag_gameplay::projectile::Projectile)> {
        use oag_render::weapon_quads::geometry::FLASH_WINDOW_SECONDS;
        self.sim
            .world
            .projectiles
            .slots
            .iter()
            .enumerate()
            .filter(|(_, projectile)| {
                projectile.kind == Some(oag_tables::weapons::Weapon::Cannon)
                    && oag_gameplay::projectile::MAX_FLIGHT_SECONDS - projectile.lifetime
                        < FLASH_WINDOW_SECONDS
            })
    }

    /// The muzzle a round left from, in world space: its position and its
    /// three unit axes, orthonormalised the way `CannonBullet`'s update does
    /// before it scales them (`0x001320e8`: X normalised, Y made orthogonal
    /// to it and normalised, Z their cross product).
    ///
    /// **The muzzle is the firing craft's `cannon_flash` locator, carried by
    /// the craft's live pose**, as `FUN_00130b50` reads its node's world
    /// matrix every tick rather than the pose at the shot.
    ///
    /// **Which of the two is chosen, not measured.** `CannonManager` picks it
    /// off one bit of the craft's own round count (`0x0010fc4c`), and this
    /// engine's spawn already picked a side off the same parity when it laid
    /// the round a quarter-hull to the left or right of the nose
    /// (`oag_gameplay::projectile::cannon::launch`). The side is read back
    /// from which side of the craft the round is on rather than carried as
    /// new simulation state, so the flash sits on the side the round's own
    /// streak starts from; whether this engine's left is the original's
    /// `left` node is not checked against a capture.
    ///
    /// Z keeps the locator's own handedness: the cross product is flipped
    /// when it disagrees with the authored Z, rather than trusting that the
    /// authored basis and this engine's hull matrix are both right-handed.
    fn cannon_muzzle_matrix(
        &self,
        draw: &CannonDraw,
        projectile: &oag_gameplay::projectile::Projectile,
    ) -> Option<(Vec3, [Vec3; 3])> {
        let slot = usize::from(projectile.owner);
        let body = &self.sim.world.ships.get(slot)?.physics.body;
        let left = (projectile.position - body.position).dot(body.right()) < 0.0;
        let local = draw.muzzles.get(slot)?[usize::from(!left)]?;
        let world = self.ship_model_matrix_of(slot) * local;
        let x = world.x_axis.truncate().normalize_or_zero();
        let authored_y = world.y_axis.truncate();
        let y = (authored_y - x * x.dot(authored_y)).normalize_or_zero();
        let z = x.cross(y);
        let z = if z.dot(world.z_axis.truncate()) < 0.0 {
            -z
        } else {
            z
        };
        Some((world.w_axis.truncate(), [x, y, z]))
    }
}
