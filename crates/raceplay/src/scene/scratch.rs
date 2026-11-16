//! [`Scratch`]: the per-frame vertex lists a race frame is assembled into.
//!
//! Split out of `race/scene.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// The per-frame vertex lists [`Scene`] hands to its two effect pipelines.
///
/// One owner rather than four locals, so the buffers survive the frame that
/// filled them - see [`Scene::scratch`].
#[derive(Debug, Default)]
pub(crate) struct Scratch {
    /// Engine flares, HD sprite quads and projectile billboards: the
    /// `exhaust::Pipeline`'s sprite buffer.
    pub(crate) vertices: Vec<oag_mesh::mesh::GpuVertex>,
    /// The engine flares drawn among the circuit's blended list, six vertices each, in the
    /// order of the view's flare keys.
    pub(crate) flares: Vec<oag_mesh::mesh::GpuVertex>,
    /// The engine ribbons, the same pipeline's second buffer.
    pub(crate) trail: Vec<oag_mesh::mesh::GpuVertex>,
    /// The particle pipeline's two blend classes.
    pub(crate) additive: Vec<oag_mesh::mesh::GpuVertex>,
    pub(crate) alpha: Vec<oag_mesh::mesh::GpuVertex>,
    /// The Cannon round's bolt streak and muzzle flash -
    /// `oag_fx::weapon_quads::Pipeline`'s own two buffers.
    pub(crate) cannon_bolt: Vec<oag_mesh::mesh::GpuVertex>,
    pub(crate) cannon_flash: Vec<oag_mesh::mesh::GpuVertex>,
    /// Which weapon pads currently hand out a pickup, one per pad node.
    pub(crate) pads_ready: Vec<bool>,
    /// The span of vertices whichever recolour or reshape is running has just
    /// built, on its way to a `write_buffer`.
    ///
    /// **One buffer for all five of them** - the airbrake flaps, the shield
    /// shell, the cockpit sphere, each weapon pad and each speed pad -
    /// because [`Scene::render`] runs them one after another and none of
    /// them reads what the last one wrote. Five buffers would be five
    /// high-water marks kept alive to save nothing.
    ///
    /// Each of the five cleared and refilled it every frame before this
    /// existed: 85 KiB a frame for the pads alone on Talon's Junction, plus a
    /// pair of flap writes on every frame of every race and a whole shell's
    /// vertices for every craft with its shield up.
    ///
    /// Each caller clears it itself rather than trusting the state it is
    /// handed, so a span is never written from another mesh's leftovers.
    pub(crate) recoloured: Vec<oag_mesh::mesh::GpuVertex>,
}

impl Scratch {
    /// Empties every list while keeping what they have already grown to.
    pub(crate) fn clear(&mut self) {
        self.vertices.clear();
        self.flares.clear();
        self.trail.clear();
        self.additive.clear();
        self.alpha.clear();
        self.cannon_bolt.clear();
        self.cannon_flash.clear();
        self.pads_ready.clear();
        self.recoloured.clear();
    }
}

/// Where a craft's engine flare quad is recorded this frame. With the ribbons first the flare
/// is queued among the circuit's blended list at `key` (`ExhaustFlare_Submit`) and the pass
/// draws it there, so its quad goes into `among` with its key; otherwise it rides the sprite
/// buffer and draws after the blended list. A flare in both buffers draws twice, in neither
/// not at all.
pub(crate) fn queue_flare(
    ribbon_first: bool,
    quad: &[oag_mesh::mesh::GpuVertex; 6],
    key: u32,
    sprites: &mut Vec<oag_mesh::mesh::GpuVertex>,
    (among, keys): (&mut Vec<oag_mesh::mesh::GpuVertex>, &mut Vec<u32>),
) {
    if ribbon_first {
        among.extend_from_slice(quad);
        keys.push(key);
    } else {
        sprites.extend_from_slice(quad);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;
    use oag_mesh::mesh::GpuVertex;

    fn queued(ribbon_first: bool) -> (usize, usize, Vec<u32>) {
        let (mut sprites, mut among, mut keys) = (Vec::new(), Vec::new(), Vec::new());
        queue_flare(
            ribbon_first,
            &[GpuVertex::zeroed(); 6],
            0x4d0d_ca46,
            &mut sprites,
            (&mut among, &mut keys),
        );
        (sprites.len(), among.len(), keys)
    }

    #[test]
    fn with_the_ribbons_first_a_flare_is_queued_with_its_key_and_not_in_the_sprite_draw() {
        assert_eq!(queued(true), (0, 6, vec![0x4d0d_ca46]));
    }

    #[test]
    fn otherwise_a_flare_stays_in_the_sprite_draw() {
        assert_eq!(queued(false), (6, 0, vec![]));
    }
}
