//! Wipeout HD's `absorb` locators - where its weapon-absorb burst plays.
//!
//! HD does not play the burst at the `Ship Collision Fx` set the way Pulse
//! does: `FUN_000d2f58` collects up to six nodes of the `absorb` class
//! (`0x3ee`, [`vex::CLASS_ABSORB`]) and `FUN_000d9398` fires at those. They
//! live beside the collision-fx set in the team's `Locators.vex`, so they are
//! read from the same place, the hull first and then the sibling - see
//! `docs/ghidra/functions/ps3-hdfury-eu/absorb-feedback.md`.

use oag_core::math::Vec3;
use oag_vex::vex;

use super::{LOCATORS_ENTRY, sibling_entry};

/// Every `absorb` locator's position in the hull's own model space, in file
/// order - the depth-first order the original's collector walks. Empty on a
/// source whose format version names no such class (every PSP and PS2 file)
/// and on a hull that authors none, which the report says.
pub(super) fn locators(
    archives: &mut oag_assets::Archives,
    hull_name: &str,
    blob: &[u8],
    report: &mut Vec<String>,
) -> Vec<Vec3> {
    let Ok(Some(_)) = vex::classes_of(blob).map(|classes| classes.absorb) else {
        return Vec::new();
    };
    let mut found = positions(blob);
    let mut source = hull_name.to_string();
    if found.is_empty()
        && let Some(name) = sibling_entry(hull_name, LOCATORS_ENTRY)
        && let Ok(sibling) = archives.read_name(&name)
    {
        found = positions(&sibling);
        source = name;
    }
    report.push(if found.is_empty() {
        format!("{hull_name}: no absorb locators - an absorb plays no burst on this craft")
    } else {
        format!(
            "{source}: {} absorb locator(s) for the absorb burst",
            found.len()
        )
    });
    found
}

fn positions(blob: &[u8]) -> Vec<Vec3> {
    let Ok(Some(class)) = vex::classes_of(blob).map(|classes| classes.absorb) else {
        return Vec::new();
    };
    let Ok(nodes) = vex::nodes(blob) else {
        return Vec::new();
    };
    vex::class_world_transforms(blob, &nodes, class)
        .into_iter()
        .map(|m| Vec3::new(m[12], m[13], m[14]))
        .collect()
}

/// The absorb hull overlay's texture, a literal in the executable: string at
/// `0x08a88378`, loaded by `Texture_LoadEffectSurfaces` (`0x0890cc1c`) into
/// `DAT_08af2800`, which `HullOverlay_DrawAbsorb` binds.
pub const OVERLAY_TEXTURE: &str = r"Data\Tex\Weapons\absorb_surface.mip";

/// The hull redrawn for the absorb overlay - `oag_render::hull_overlay` - or
/// `None`, reported, when `wanted` is false (every title but Pulse), when the
/// texture does not resolve, or when the hull's batches share no one scale
/// (every PS2 hull).
pub(super) fn overlay(
    archives: &mut oag_assets::Archives,
    hull: &oag_render::mesh::Model,
    blob: &[u8],
    wanted: bool,
    report: &mut Vec<String>,
) -> Option<oag_render::mesh::Model> {
    if !wanted {
        return None;
    }
    let Some(scale) = oag_render::hull_overlay::projection_scale(blob) else {
        report.push(format!(
            "{}: no one batch scale to project from - no absorb overlay",
            hull.label
        ));
        return None;
    };
    let texture = oag_pulse::read_image(archives, OVERLAY_TEXTURE)
        .ok()
        .and_then(|blob| oag_texture::texture::Texture::parse(&blob).ok());
    let Some(texture) = texture else {
        report.push(format!(
            "{OVERLAY_TEXTURE}: not readable - no absorb overlay on {}",
            hull.label
        ));
        return None;
    };
    let texture = oag_render::mesh::ModelTexture::rgba8(
        OVERLAY_TEXTURE.to_string(),
        u32::from(texture.width),
        u32::from(texture.height),
        texture.to_rgba(),
        None,
    );
    report.push(format!(
        "{}: absorb overlay over {} triangle(s), projected at batch scale {scale}",
        hull.label,
        hull.indices.len() / 3
    ));
    Some(oag_render::hull_overlay::build(
        hull,
        scale,
        std::sync::Arc::new(texture),
    ))
}
