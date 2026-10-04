//! `cloudCube` `0x3d8` and `cloudGroup` `0x3d9`: the sky's cloud puffs.
//!
//! Before this module, `docs/formats/skycube.md`'s Open section recorded both
//! as undecoded, and speculated `cloudCube` might have no registration site at
//! all - the same absence `exhaust.md` found for `engine_fire`/`exitglow`/`gate`.
//! That guess is refuted: both classes are registered
//! (`CloudCube_RegisterClass` `0x08932138`, `CloudGroup_RegisterClass`
//! `0x0893471c`), and `CloudGroup_Init` (`0x08933048`) is a real, substantial
//! function, not a compiler stub. See
//! `docs/ghidra/functions/psp-pulse-usa/clouds.md` for the full evidence and
//! confidence per claim; this module implements what that page describes.
//!
//! # Census
//!
//! **Exactly one Pulse circuit authors any cloud node on either PSP pressing**:
//! `05_Track`, both layouts (forward and reversed share the finding). Swept
//! against every `.vex` entry in `Data.wad`, `FEData.wad`, `BEData.wad` and
//! `FE.wad` on `pulse-psp-usa.chd` and `pulse-psp-eu.chd` - see
//! `crates/vex/tests/cloud_ground_truth.rs`. 5 `cloudCube` leaves and 3
//! `cloudGroup` nodes per layout, matching the count `docs/formats/skycube.md`
//! already quoted from the census that motivated this module.
//!
//! Pure's class-ID table is renumbered (`docs/formats/vex.md`), so a `0x3d8`
//! search there would look for the wrong class entirely; Pure is **not**
//! checked here; see the doc page.
//!
//! # `cloudCube`'s payload: 16 bytes, byte-identical on every shipped instance
//!
//! ```text
//! +0x00  u32  kind, 2 on all 10 shipped instances - meaning not established,
//!             since it never varies
//! +0x04  f32  scale, 1.0 on all 10 shipped instances - likewise
//! +0x08  u32  zero on all 10
//! +0x0c  u32  zero on all 10
//! ```
//!
//! A `cloudCube` carries none of its own placement or visual parameters - see
//! [Where the real parameters live](#where-the-real-parameters-live).
//!
//! # `cloudGroup`'s payload: a `Transform`-shaped 4x4, exactly like `fogCube`'s
//!
//! 0 bytes (identity) or 64 bytes, row-major, translation in row 3 with `w =
//! 1.0` - the same convention [`vex::transform`] already reads for
//! [`vex::CLASS_TRANSFORM`] and for `fogCube`'s leading matrix
//! (`docs/formats/skycube.md`). Confirmed arithmetically on `05_Track`'s outer
//! `cloudGroup`: rows 0-2 are unit-length up to floating-point noise
//! (`0.9997`..`1.0002`) and row 3 is a real world position, `(-308, 138,
//! 1379)`.
//!
//! A `cloudGroup` can itself be a child of another `cloudGroup` - `05_Track`
//! nests one inside another - so placing a `cloudCube` means composing every
//! `Transform` **and** `cloudGroup` matrix on its ancestor chain, not just the
//! nearest `Transform`. [`vex::class_world_transforms`] cannot do this alone:
//! it composes one class against [`vex::world_transforms`]'s stock chain, which
//! treats every other class - `cloudGroup` included - as the identity, so a
//! `cloudGroup` nested inside another would silently drop the outer one's
//! placement. [`cloud_world_transforms`] is the same short loop
//! `vex::world_transforms_at` runs, generalised to treat `cloudGroup` as a
//! second matrix-contributing class alongside `Transform`.
//!
//! # Where the real parameters live: the node header's named-attribute list
//!
//! `CloudGroup_Init` reads six colour channels, two alphas, a sprite radius and
//! its variance, a blend midpoint, an overlap and a random seed - **from the
//! node's own [`vex::node_attributes`] list, by name, not from its 4x4
//! payload.** `cloudCube` carries no attribute list of its own; every
//! attribute is read off the owning `cloudGroup`. This is the same
//! `header+0x06`/`+0x0e` named-`f32`-list mechanism `AnimTransform_Bind`
//! already uses for `LoopEnd`/`AnimEnd`/`FixedFrames` - confirmed here by a
//! second, independent consumer (`CloudGroup_Init`) reading the same list
//! shape for entirely different names, and by every name below appearing
//! verbatim in the binary at `0x08a88aa4`.
//!
//! **One divergence from that mechanism's own doc comment, recorded rather
//! than resolved**: [`vex::node_attributes`]'s doc says the lookup is case
//! **sensitive** in the original (`AnimTransform_Bind`'s `strcmp`).
//! `CloudGroup_Init` looks its names up with `strcasecmp` instead - a
//! case-*insensitive* compare. It makes no difference to any shipped file,
//! because every cloud attribute name on `05_Track` already matches
//! [`vex::node_attributes`]'s case exactly, so this module reuses that
//! function unchanged rather than adding a second, case-folding walker for a
//! difference no shipped byte exercises.
//!
//! [`CloudAttributes::from_node`] reads, in the order `CloudGroup_Init` does:
//!
//! | Name | Field | `05_Track`'s values |
//! | --- | --- | --- |
//! | `Overlap` | [`overlap`](CloudAttributes::overlap) | `0.65` |
//! | `Seed` | [`seed`](CloudAttributes::seed) | `0` (unset; the original falls back to `Psys_RandIntRange(1, 9999)`) |
//! | `SpriteRadius` | [`sprite_radius`](CloudAttributes::sprite_radius) | `4.0` |
//! | `SpriteRadiusVar` | [`sprite_radius_var`](CloudAttributes::sprite_radius_var) | `0.2` |
//! | `HiColourR/G/B`, `HiAlpha` | [`hi_colour`](CloudAttributes::hi_colour), [`hi_alpha`](CloudAttributes::hi_alpha) | warm near-white, alpha `1.0` |
//! | `MidColourR/G/B` | [`mid_colour`](CloudAttributes::mid_colour) | warm mid-grey |
//! | `LoColourR/G/B`, `LoAlpha` | [`lo_colour`](CloudAttributes::lo_colour), [`lo_alpha`](CloudAttributes::lo_alpha) | warm shadow, alpha `1.0` |
//! | `Midpoint` | [`midpoint`](CloudAttributes::midpoint) | `0.5` |
//!
//! A name absent from the list reads as `0.0`, matching `CloudGroup_Init`'s own
//! "attribute not found -> 0" fallback for every one of these except `Seed`
//! (which the constructor re-rolls when it reads `0`, a runtime step this
//! module does not reproduce - see the doc page's Open section).
//!
//! Two of `05_Track`'s three `cloudGroup` nodes carry a **flat neutral**
//! palette (`1.0, 1.0, 1.0` / `0.8, 0.8, 0.8` / `0.6, 0.6, 0.6`) rather than the
//! warm one above; both are shipped and this module reads whichever a given
//! node authors rather than assuming the warm set.
//!
//! # What the executable draws with it, and what this module does not attempt
//!
//! `cloudGroup`'s own method table (`0x08ad2a24`) overrides `draw` at `+0x44`
//! (`0x0893280c`); `cloudCube`'s (`0x08ad299c`) does not - that slot is still
//! the inherited `Transform` default. So **`cloudGroup` is what draws**, and it
//! draws a rotating camera-facing billboard per instance in its own list
//! (`vsin_s`/`vcos_s` on a running phase, `vtfm4_q` through the camera's own
//! view-matrix stack), not a static quad. This module's renderer
//! (`oag_render::cloud`) draws a **static**, camera-independent billboard
//! instead - the rotation is recovered structurally but its exact phase and
//! speed constant are not, so animating it would be a guess dressed as a
//! measurement. See the doc page's Open section.

use crate::vex::{self, IDENTITY, Node, byte_order, multiply, node_attributes, transform};

/// Class ID of a `cloudCube` node.
///
/// Kept local to this module rather than added to `vex.rs`'s `CLASS_*` list,
/// which another pass is editing concurrently.
pub const CLASS_CLOUD_CUBE: u32 = 0x3d8;

/// Class ID of a `cloudGroup` node.
pub const CLASS_CLOUD_GROUP: u32 = 0x3d9;

/// Length of a `cloudCube` payload.
pub const CLOUD_CUBE_PAYLOAD_LEN: usize = 0x10;

/// A `cloudCube` leaf: a placement and the two fields its payload carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloudCube {
    /// `+0x00`, constant `2` on every shipped instance; meaning not established.
    pub kind: u32,
    /// `+0x04`, constant `1.0` on every shipped instance; meaning not
    /// established beyond "a scale of some kind".
    pub scale: f32,
    /// World-space position: the translation row of the composed
    /// `Transform`/`cloudGroup` chain above this node. A `cloudCube` payload
    /// carries no matrix of its own.
    pub world_position: [f32; 3],
}

/// Decodes a `cloudCube` payload. `None` if it is not the one shipped shape.
#[must_use]
pub fn cloud_cube(payload: &[u8], order: oag_formats::ByteOrder) -> Option<(u32, f32)> {
    if payload.len() < CLOUD_CUBE_PAYLOAD_LEN {
        return None;
    }
    Some((order.u32(payload, 0), order.f32(payload, 4)))
}

/// The colour-and-size parameters `CloudGroup_Init` (`0x08933048`) reads off a
/// `cloudGroup` node's named-attribute list.
///
/// Every field defaults to `0.0` when its name is absent, matching the
/// original's own fallback - see the module doc's table.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CloudAttributes {
    /// `Overlap`.
    pub overlap: f32,
    /// `Seed`. `0.0` when unset, which is what every shipped instance is; the
    /// original re-rolls a random seed at construction time in that case
    /// rather than using `0` itself. Not reproduced here - see the doc page.
    pub seed: f32,
    /// `SpriteRadius`.
    pub sprite_radius: f32,
    /// `SpriteRadiusVar`.
    pub sprite_radius_var: f32,
    /// `HiColourR/G/B`.
    pub hi_colour: [f32; 3],
    /// `HiAlpha`.
    pub hi_alpha: f32,
    /// `MidColourR/G/B`.
    pub mid_colour: [f32; 3],
    /// `LoColourR/G/B`.
    pub lo_colour: [f32; 3],
    /// `LoAlpha`.
    pub lo_alpha: f32,
    /// `Midpoint`.
    pub midpoint: f32,
}

impl CloudAttributes {
    /// Reads every field off `node`'s own attribute list, by exact name.
    #[must_use]
    pub fn from_node(data: &[u8], node: &Node) -> Self {
        let attrs = node_attributes(data, node);
        let get = |name: &str| {
            attrs
                .iter()
                .find(|(n, _)| n == name)
                .map_or(0.0, |(_, v)| *v)
        };
        Self {
            overlap: get("Overlap"),
            seed: get("Seed"),
            sprite_radius: get("SpriteRadius"),
            sprite_radius_var: get("SpriteRadiusVar"),
            hi_colour: [get("HiColourR"), get("HiColourG"), get("HiColourB")],
            hi_alpha: get("HiAlpha"),
            mid_colour: [get("MidColourR"), get("MidColourG"), get("MidColourB")],
            lo_colour: [get("LoColourR"), get("LoColourG"), get("LoColourB")],
            lo_alpha: get("LoAlpha"),
            midpoint: get("Midpoint"),
        }
    }
}

/// World matrix of every node, treating **both** [`vex::CLASS_TRANSFORM`] and
/// [`CLASS_CLOUD_GROUP`] as matrix-contributing classes.
///
/// [`vex::world_transforms`] cannot be reused directly for this: it treats
/// every class but `Transform`/`Anim Transform` as the identity, so a
/// `cloudGroup` nested inside another `cloudGroup` (which `05_Track` authors)
/// would compose with the outer one's own placement dropped. This is the same
/// loop `vex::world_transforms_at` runs, with one more class recognised - see
/// that function's doc for why every *other* class still resolves to the
/// identity rather than a guess.
#[must_use]
pub fn cloud_world_transforms(data: &[u8], nodes: &[Node]) -> Vec<[f32; 16]> {
    let order = byte_order(data);
    let mut out: Vec<[f32; 16]> = Vec::with_capacity(nodes.len());
    for node in nodes {
        let parent = node
            .parent
            .and_then(|p| out.get(p).copied())
            .unwrap_or(IDENTITY);
        let local = if node.class_id == vex::CLASS_TRANSFORM || node.class_id == CLASS_CLOUD_GROUP {
            data.get(node.payload())
                .and_then(|payload| transform(payload, order))
                .unwrap_or(IDENTITY)
        } else {
            IDENTITY
        };
        out.push(multiply(&local, &parent));
    }
    out
}

/// Every `cloudCube` in `nodes`, placed and coloured.
///
/// A leaf's colour and size come from the nearest `cloudGroup` on its ancestor
/// chain, itself included were it ever a `cloudGroup` (it never is - the class
/// is a leaf in every shipped file). A `cloudCube` with no `cloudGroup`
/// ancestor at all is skipped: it is not a shape any shipped file authors, and
/// drawing it with no parameters would be inventing them.
#[must_use]
pub fn clouds(data: &[u8], nodes: &[Node]) -> Vec<(CloudCube, CloudAttributes)> {
    let order = byte_order(data);
    let world = cloud_world_transforms(data, nodes);
    let mut out = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        if node.class_id != CLASS_CLOUD_CUBE {
            continue;
        }
        let Some(payload) = data.get(node.payload()) else {
            continue;
        };
        let Some((kind, scale)) = cloud_cube(payload, order) else {
            continue;
        };
        let Some(group_index) = nearest_cloud_group(nodes, node.parent) else {
            continue;
        };
        let attrs = CloudAttributes::from_node(data, &nodes[group_index]);
        let m = world[index];
        out.push((
            CloudCube {
                kind,
                scale,
                world_position: [m[12], m[13], m[14]],
            },
            attrs,
        ));
    }
    out
}

/// How many `cloudCube` nodes one `cloudGroup` collects at most: the `0x20`
/// capacity `CloudGroup_Init` (`0x08933048`) passes `FUN_08a72d1c`.
pub const MAX_CUBES_PER_GROUP: usize = 0x20;

/// One `cloudCube` as a `cloudGroup` collects it, with what its build reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroupCube {
    /// The payload's `+0x00` word, the sprite-variant branch
    /// `CloudGroup_BuildDisplayList` switches on (`2` on every shipped cube).
    pub kind: u32,
    /// The payload's `+0x04` float, multiplied by `10` into every half-size.
    pub scale: f32,
    /// The cube's composed world matrix, row-major, translation in row 3. The
    /// cube carries no matrix of its own, so this is its parent chain's.
    pub world: [f32; 16],
    /// `SpriteRadius` and `SpriteRadiusVar` of the cube's **nearest**
    /// `cloudGroup` ancestor (the cube's `+0x74`), which may not be the group
    /// building it: a nested group's cubes are collected by every group above
    /// them too, and the half-size reads the nearest one's attributes.
    pub sprite_radius: f32,
    /// See [`GroupCube::sprite_radius`].
    pub sprite_radius_var: f32,
}

/// One `cloudGroup` and every `cloudCube` it builds a field from.
#[derive(Debug, Clone, PartialEq)]
pub struct CloudGroup {
    /// Index of the group's node in the `nodes` slice it was read from.
    pub node: usize,
    /// The group's own attributes: overlap, seed and colour ramp.
    pub attributes: CloudAttributes,
    /// The cubes it collects, in the order the original visits them.
    pub cubes: Vec<GroupCube>,
}

/// Every `cloudGroup` in `nodes`, in file order, each with the cubes it builds
/// from.
///
/// `CloudGroup_Init` collects cubes with `FUN_08a72d1c`, a preorder walk of
/// the group's **whole** subtree (the node, its first child `+0x10`, then each
/// sibling `+0xc`) that keeps every node of `cloudCube`'s class, up to
/// [`MAX_CUBES_PER_GROUP`]. A nested group's cubes are therefore built by the
/// outer group as well: live on `05_Track` both the outer group and the group
/// inside it held the same one cube, each building its own 46 records from its
/// own seed and colouring them from its own ramp. A group that collects no
/// cube is destroyed by the original and is skipped here. Evidence and
/// confidence are on `docs/ghidra/functions/psp-pulse-usa/clouds.md`.
#[must_use]
pub fn cloud_groups(data: &[u8], nodes: &[Node]) -> Vec<CloudGroup> {
    let order = byte_order(data);
    let world = cloud_world_transforms(data, nodes);
    let mut out = Vec::new();
    for (index, group) in nodes.iter().enumerate() {
        if group.class_id != CLASS_CLOUD_GROUP {
            continue;
        }
        // `vex::nodes` is a preorder list, so the subtree is every later node
        // with `index` on its ancestor chain, already in the walk's order.
        let cubes: Vec<GroupCube> = nodes
            .iter()
            .enumerate()
            .skip(index + 1)
            .take_while(|(_, node)| is_descendant(nodes, node.parent, index))
            .filter(|(_, node)| node.class_id == CLASS_CLOUD_CUBE)
            .filter_map(|(at, node)| {
                let (kind, scale) = cloud_cube(data.get(node.payload())?, order)?;
                let nearest = nearest_cloud_group(nodes, node.parent)?;
                let size = CloudAttributes::from_node(data, &nodes[nearest]);
                Some(GroupCube {
                    kind,
                    scale,
                    world: world[at],
                    sprite_radius: size.sprite_radius,
                    sprite_radius_var: size.sprite_radius_var,
                })
            })
            .take(MAX_CUBES_PER_GROUP)
            .collect();
        if cubes.is_empty() {
            continue;
        }
        out.push(CloudGroup {
            node: index,
            attributes: CloudAttributes::from_node(data, group),
            cubes,
        });
    }
    out
}

/// Whether `ancestor` is on the chain starting at `start`.
fn is_descendant(nodes: &[Node], start: Option<usize>, ancestor: usize) -> bool {
    let mut at = start;
    while let Some(index) = at {
        if index == ancestor {
            return true;
        }
        at = nodes[index].parent;
    }
    false
}

/// Walks `start` and its ancestors for the nearest [`CLASS_CLOUD_GROUP`] node.
fn nearest_cloud_group(nodes: &[Node], start: Option<usize>) -> Option<usize> {
    let mut at = start;
    while let Some(index) = at {
        let node = &nodes[index];
        if node.class_id == CLASS_CLOUD_GROUP {
            return Some(index);
        }
        at = node.parent;
    }
    None
}

#[cfg(test)]
mod tests;
