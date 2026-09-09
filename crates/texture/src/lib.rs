//! Every pixel format the originals ship, decoded to RGBA8888.
//!
//! Four consoles' worth, and the point of keeping them together is that they
//! overlap rather than partition: [`gtf`] (PS3) and [`gxt`] (Vita) both decode
//! S3TC through the same block maths, and both PSP and PS2 textures come out of
//! the same palette machinery. Splitting by console would cut through that -
//! see [ADR-0050](../../../docs/architecture/adr/0050-format-crates-split-by-format-family.md).
//!
//! [`texture`] is the PSP's palette-indexed `.mip`, [`ps2_texture`] the PS2's
//! GS upload packet, [`fnt`] a bitmap font's swizzled glyph atlas, [`ship_skin`]
//! the alternate liveries, and [`png`] the one *writer* here - what a probe or a
//! screenshot goes out as.
//!
//! The two block codecs, `bcn` (S3TC/BC1-3) and `pvrtc` (PVRTC-II 4bpp), stay
//! private: they are shared *between* the containers above rather than used
//! from outside, and keeping the crate boundary around them rather than through
//! them is what lets them stay that way.
//!
//! Where a texel *is* - the GE's block swizzle - is `oag_formats::swizzle`
//! rather than anything here: that is byte layout, and three crates undo it.

mod bcn;
pub mod fnt;
pub mod gtf;
pub mod gxt;
pub mod png;
pub mod ps2_texture;
mod pvrtc;
pub mod ship_skin;
pub mod texture;
