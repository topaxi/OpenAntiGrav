//! The `RCSMODEL` scene: the shape the later titles ship their geometry in.
//!
//! On the PSP and PS2 a `.vex` `Mesh` node's payload *is* its geometry. On the
//! PS3 and Vita that payload shrank to a bounding box and a word, and the
//! vertices moved into a `.rcsmodel` beside the `.vex`. This crate is that file
//! and the three things it cannot be read without: [`rcsmodel`] is the geometry
//! and its vertex declarations, [`rcsmaterial`] the shader-variant table that
//! says how a surface is lit, [`hd_pvs`] the visibility set that decides whether
//! it is drawn at all, and [`gxp`] the compiled shader programs a material
//! selects on the Vita.
//!
//! `rcsmodel` and `rcsmaterial` reference each other - a material needs the
//! model's `VertexDecl`, a model's material needs the material's lightmap
//! sampler - so they are one crate by necessity as well as by subject.
//!
//! **`rcsmodel::psp2` is the Vita's variant and stays inside `rcsmodel`
//! deliberately.** Which layout a file uses is read from the file's own version
//! word, never from a caller naming a console, per
//! [ADR-0022](../../../docs/architecture/adr/0022-title-packages.md); hoisting
//! it into a crate of its own would make the console a code axis, which
//! [ADR-0050](../../../docs/architecture/adr/0050-format-crates-split-by-format-family.md)
//! and ADR-0022 item 5 both forbid.

pub mod gxp;
pub mod hd_pvs;
pub mod points2;
pub mod rcsanimclip;
pub mod rcsmaterial;
pub mod rcsmodel;
pub mod rcsskeleton;
pub mod rig;
