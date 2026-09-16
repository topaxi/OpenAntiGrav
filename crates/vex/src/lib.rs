//! The `.vex` scene tree, and everything authored inside or alongside it.
//!
//! [`vex`] is the format itself: the node tree that carries a Wipeout circuit's
//! geometry, its transforms, its embedded textures and its class-tagged
//! payloads. Which schema a file uses is read from the file's own version word
//! (`vex::classes::{V6, V4, V3}`), never from a caller naming a title, per
//! [ADR-0022](../../../docs/architecture/adr/0022-title-packages.md).
//!
//! Most of the rest of this crate decodes one of those class-tagged payloads,
//! which is why they live here rather than beside each other by subject:
//! [`collision`] is the triangle soup, [`track`] the AI spline graph, [`pads`]
//! the speedup and weapon trigger volumes, [`pvs`] the authored visible set,
//! [`fog`] the fog volume, [`lighting`] the light rig, [`sound_emitters`] the
//! positional sound nodes, [`shadow_occluder`] the convex hulls and [`cloud`]
//! the `cloudCube`/`cloudGroup` sky puffs. Each is a `vex::Node` payload, not a
//! neighbouring format, and splitting them out would cut a tree into chunks.
//!
//! Three are standalone files rather than nodes, kept here because the same
//! scene is what reads them: [`vif`] walks the PS2 VIF1 packets a `.vex` batch
//! header points at, [`kdcol`] is Wipeout 2048's collision (it reuses
//! [`collision`]'s own types, which is one reason this crate is not split by
//! console), and [`pob`] holds the particle systems the scene triggers.
//!
//! See [ADR-0050](../../../docs/architecture/adr/0050-format-crates-split-by-format-family.md).

pub mod cloud;
pub mod collision;
pub mod fog;
pub mod kdcol;
pub mod lighting;
pub mod pads;
pub mod pob;
pub mod pvs;
pub mod shadow_occluder;
pub mod sound_emitters;
pub mod track;
pub mod vex;
pub mod vif;
