//! The particle effects a circuit places itself: `ParticleSystem` (`0x3c4`)
//! nodes in its own `.vex`, each naming a `Data\Psys\<Name>.POB`.
//!
//! **The original plays every one of these from the moment the circuit loads.**
//! `PsysNode_Init` (`0x089156a0`, the class's `init` slot, via the generic
//! per-node spawner) reads the node's `Name` string attribute with `strcasecmp`,
//! builds `Data\Psys\%s.POB`, loads it and, unless the node's parent is an
//! `animationTrigger` (`0x3dc`, tag `0x08a727cc`), spawns one instance at the
//! node's own matrix (`node+0x50`, the 64-byte payload). Its update slot
//! (`0x08915cdc`) hands the instance the node's world matrix every frame and
//! destroys the node once the instance finishes: a looping effect runs the whole
//! race, a burst plays once. Confirmed live on PPSSPP: Basilico Black loads
//! `WO_BLUE_WELDER` three times, with local matrices byte-identical to the three
//! nodes here. See `docs/ghidra/functions/psp-pulse-usa/placed-particle-systems.md`.
//!
//! This module hands back the placement alone (the name and how to compose the
//! node's world matrix at a time), so the caller that plays it decides nothing
//! about where or when.

use crate::vex::{self, Node};

/// Class id of an `animationTrigger` node, version 6.
///
/// A `ParticleSystem` parented to one is not spawned at load: the trigger starts
/// its instance, by something unread.
pub const CLASS_ANIMATION_TRIGGER: u32 = 0x3dc;

/// One step of a node's ancestry, node first.
#[derive(Debug, Clone, PartialEq)]
enum Link {
    /// A fixed local matrix: the node's own payload, or a `Transform`'s.
    Fixed([f32; 16]),
    /// An `Anim Transform`, sampled on the scenery clock.
    Anim(Box<vex::AnimTransform>),
}

/// A `ParticleSystem` node the circuit spawns at load.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    /// The node's index in [`vex::nodes`].
    pub node: usize,
    /// Its `Name` attribute: the effect, `Data\Psys\<name>.POB`.
    pub name: String,
    /// The node's own matrix, then each ancestor's, root last.
    chain: Vec<Link>,
}

impl Placed {
    /// The node's world matrix at `seconds` into the scenery clock,
    /// row-major with the translation in row 3.
    ///
    /// The product [`vex::world_transforms_at`] builds, restricted to this node's
    /// chain with its own payload as the local matrix: `Transform` and `Anim
    /// Transform` contribute theirs, every other class the identity.
    #[must_use]
    pub fn world_at(&self, seconds: f32) -> [f32; 16] {
        self.chain.iter().fold(vex::IDENTITY, |world, link| {
            let local = match link {
                Link::Fixed(m) => *m,
                Link::Anim(anim) => anim.sample(seconds),
            };
            vex::multiply(&world, &local)
        })
    }

    /// Whether any ancestor moves, so [`Self::world_at`] depends on time.
    #[must_use]
    pub fn moves(&self) -> bool {
        self.chain.iter().any(|link| matches!(link, Link::Anim(_)))
    }
}

/// Every `ParticleSystem` node in `data` that the original spawns at load.
///
/// Version-6 files only (Pulse): another version returns nothing, not a guess. A
/// node with no `Name`, no 64-byte payload, or an `animationTrigger` parent is
/// left out.
#[must_use]
pub fn placed(data: &[u8], nodes: &[Node]) -> Vec<Placed> {
    let Ok(classes) = vex::classes_of(data) else {
        return Vec::new();
    };
    if classes.version != 6 {
        return Vec::new();
    }
    let order = vex::byte_order(data);
    let fixed = |node: &Node| {
        data.get(node.payload())
            .and_then(|payload| vex::transform(payload, order))
    };
    let mut out = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        if node.class_id != vex::CLASS_PARTICLE_SYSTEM {
            continue;
        }
        if node
            .parent
            .is_some_and(|p| nodes[p].class_id == CLASS_ANIMATION_TRIGGER)
        {
            continue;
        }
        let Some(name) = vex::node_string_attribute(data, node, "Name") else {
            continue;
        };
        let Some(own) = fixed(node) else {
            continue;
        };
        let mut chain = vec![Link::Fixed(own)];
        let mut at = node.parent;
        while let Some(p) = at {
            let ancestor = &nodes[p];
            if Some(ancestor.class_id) == classes.transform {
                chain.push(Link::Fixed(fixed(ancestor).unwrap_or(vex::IDENTITY)));
            } else if Some(ancestor.class_id) == classes.anim_transform {
                chain.push(
                    vex::anim_transform_of(data, ancestor)
                        .map_or(Link::Fixed(vex::IDENTITY), |anim| {
                            Link::Anim(Box::new(anim))
                        }),
                );
            }
            at = ancestor.parent;
        }
        out.push(Placed {
            node: index,
            name,
            chain,
        });
    }
    out
}
