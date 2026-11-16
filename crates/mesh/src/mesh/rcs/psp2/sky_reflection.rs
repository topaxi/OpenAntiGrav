//! Omega's sky reflection map: the circuit's `skyCube.gnf`, which the circuit loader opens beside
//! `skycube.rcsmodel` and binds as `cubeReflectionTex`
//! (`docs/ghidra/functions/ps4-omega-eu/panel-displacement.md`, "The CPU side" and
//! "`skyCube.gnf`"). Four cubes, each one blur step past the last, become the four levels of a
//! [`crate::mesh::sky_paraboloid`] map on the scene's sky binding, which the panel-map road reads
//! at `lod = 2 * Blurrines` ([`super::panel_map`]).
//!
//! **Read at a coarse level, chosen, not measured**: cube `k` at mip `2 + k` (256 texels a face for
//! the sharp cube), which oversamples each level of the 512x256 map about twice; the original
//! samples the 1024 face at the level its derivatives pick.

use std::sync::Arc;

use crate::mesh::rcs::Textures;
use crate::mesh::sky_paraboloid;

/// The cubes `skyCube.gnf` holds, and the mip each is read at.
const CUBES: u32 = 4;
const FIRST_LEVEL: u32 = 2;

/// The map for a PS4 `skycube.rcsmodel` named `label`; `None` for any other model or a file that
/// does not read.
pub(super) fn load(label: &str, ps4: bool, textures: Textures<'_>) -> Option<Arc<Vec<u8>>> {
    let (dir, file) = label.rsplit_once(['/', '\\'])?;
    if !ps4 || !file.eq_ignore_ascii_case("skycube.rcsmodel") {
        return None;
    }
    let blob = textures(&format!("{dir}/skyCube.gnf"))?;
    let texture = oag_texture::gnf::Texture::parse(&blob).ok()?;
    let cubes = (0..CUBES)
        .map(|cube| texture.cube_faces(&blob, FIRST_LEVEL + cube, cube).ok())
        .collect::<Option<Vec<_>>>()?;
    let side = (texture.width >> FIRST_LEVEL).max(1);
    Some(Arc::new(sky_paraboloid::from_cubes(&cubes, side)))
}
