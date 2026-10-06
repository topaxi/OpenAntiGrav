//! Every pixel format the originals ship, decoded to RGBA8888.
//!
//! Four consoles' worth, kept together because they overlap: [`gtf`] (PS3) and
//! [`gxt`] (Vita) share S3TC block maths, and PSP and PS2 share palette
//! machinery. See
//! [ADR-0050](../../../docs/architecture/adr/0050-format-crates-split-by-format-family.md).
//!
//! [`texture`] is the PSP's palette-indexed `.mip`, [`ps2_texture`] the PS2's
//! GS upload packet, [`fnt`] a bitmap font's swizzled glyph atlas, [`ship_skin`]
//! the alternate liveries, and [`png`] the one *writer* here - what a probe or a
//! screenshot goes out as.
//!
//! The two block codecs, `bcn` (S3TC/BC1-3) and `pvrtc` (PVRTC-II 4bpp), stay
//! private: they are shared between the containers, not used from outside.
//!
//! The GE's block swizzle is `oag_formats::swizzle`: byte layout, which three
//! crates undo.

mod bcn;
pub mod fnt;
pub mod gnf;
pub mod gtf;
pub mod gxt;
pub mod png;
pub mod ps2_texture;
mod pvrtc;
pub mod ship_skin;
pub mod texture;
