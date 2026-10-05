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

/// The LeachBeam hull overlay's texture, the other half of the same pair:
/// string at `0x08a8839c`, loaded by `Texture_LoadEffectSurfaces` into
/// `DAT_08af2804`, which `HullOverlay_SubmitLeachBeamBatched` (`0x0890e140`)
/// binds - measured reading it about ten times a frame for the whole life of
/// a beam. See `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
pub const LEACH_OVERLAY_TEXTURE: &str = r"Data\Tex\Weapons\leachbeam_surface.mip";

/// Which of the two hull overlays [`overlay`] builds: its texture entry and
/// the word the load report names it by.
pub(super) const ABSORB: (&str, &str) = (OVERLAY_TEXTURE, "absorb");
/// See [`ABSORB`].
pub(super) const LEACH: (&str, &str) = (LEACH_OVERLAY_TEXTURE, "LeachBeam");

/// The hull redrawn under one of the two hull-overlay textures -
/// `oag_render::hull_overlay`, [`ABSORB`] or [`LEACH`] - or `None`, reported,
/// when `wanted` is false (every title but Pulse), when the texture does not
/// resolve, or when the hull's batches share no one scale (every PS2 hull).
pub(super) fn overlay(
    archives: &mut oag_assets::Archives,
    hull: &oag_mesh::mesh::Model,
    blob: &[u8],
    (entry, what): (&str, &str),
    wanted: bool,
    report: &mut Vec<String>,
) -> Option<oag_mesh::mesh::Model> {
    if !wanted {
        return None;
    }
    let projection = oag_render::hull_overlay::projection_scale(blob)
        .zip(oag_render::hull_overlay::overlaid_meshes(hull, blob));
    let Some((scale, ships)) = projection else {
        report.push(format!(
            "{}: no one batch scale to project from, or meshes that do not map back \
             to the file - no {what} overlay",
            hull.label
        ));
        return None;
    };
    if ships.is_empty() {
        report.push(format!(
            "{}: no list-0 mesh to overlay - no {what} overlay",
            hull.label
        ));
        return None;
    }
    let texture = oag_pulse::read_image(archives, entry)
        .ok()
        .and_then(|blob| oag_texture::texture::Texture::parse(&blob).ok());
    let Some(texture) = texture else {
        report.push(format!(
            "{entry}: not readable - no {what} overlay on {}",
            hull.label
        ));
        return None;
    };
    let mut texels = texture.to_rgba();
    oag_render::hull_overlay::glow_texels(&mut texels);
    let texture = oag_mesh::mesh::ModelTexture::rgba8(
        entry.to_string(),
        u32::from(texture.width),
        u32::from(texture.height),
        texels,
        None,
    );
    report.push(format!(
        "{}: {what} overlay over {} mesh(es), projected at batch scale {scale}",
        hull.label,
        ships.len()
    ));
    Some(oag_render::hull_overlay::build(
        hull,
        scale,
        &ships,
        std::sync::Arc::new(texture),
    ))
}

/// The stem `ShipAbsorbShell_Load` (`0x000dba30`) formats under the team's own
/// directory: `"%s\\AbsorbEffect.vex"`, the literal at `0x00782190`.
pub const SHELL_STEM: &str = "AbsorbEffect";

/// Wipeout HD's absorb shell for one team - see [`oag_render::absorb_shell`] -
/// or `None`, reported, when `wanted` is false (every title but HD), when the
/// pair does not resolve or build, or when its material is not the program
/// that module reproduces.
///
/// **The material gate is all or nothing**, the flame's rule: every material
/// the model names must declare `ShieldColour` and blend `SrcAlpha`/`One`,
/// which is `hd_absorbinternal` on every team of the base game. Detonator's
/// shell is an opaque `nitro_emissive_outlines` with no `ShieldColour`, so it
/// is refused rather than drawn as something it is not.
pub(super) fn shell(
    archives: &mut oag_assets::Archives,
    team: &str,
    ship_dir: &str,
    wanted: bool,
    report: &mut Vec<String>,
) -> Option<oag_mesh::mesh::Model> {
    if !wanted {
        return None;
    }
    let name = oag_pulse::race::ships::entry_name_in(ship_dir, team, SHELL_STEM);
    let pair = archives.read_name(&name).ok().and_then(|blob| {
        let sibling = oag_mesh::mesh::rcs::sibling_name(&name)?;
        Some((blob, archives.read_name(&sibling).ok()?))
    });
    let Some((blob, geometry)) = pair else {
        report.push(format!("{name}: no .vex/.rcsmodel pair - no absorb shell"));
        return None;
    };
    let built = oag_mesh::mesh::rcs::build(
        &name,
        &blob,
        &geometry,
        &mut |path| archives.read_name(path).ok(),
        |c| c.mesh,
    );
    let (mut model, built) = match built {
        Ok(pair) => pair,
        Err(error) => {
            report.push(format!(
                "{name}: does not build ({error}) - no absorb shell"
            ));
            return None;
        }
    };
    let materials = oag_rcs::rcsmodel::Model::parse(&geometry)
        .map(|parsed| parsed.materials)
        .unwrap_or_default();
    let is_shell = |m: &oag_rcs::rcsmodel::Material| {
        m.blend() == ADDITIVE
            && m.parameters
                .iter()
                .any(|p| p.hash == oag_render::absorb_shell::SHIELD_COLOUR)
    };
    if materials.is_empty() || !materials.iter().all(is_shell) || model.indices.is_empty() {
        let names: Vec<&str> = materials.iter().map(|m| m.name.as_str()).collect();
        report.push(format!(
            "{name}: its material ({}) is not the additive ShieldColour program - no \
             absorb shell on this craft",
            names.join(", ")
        ));
        return None;
    }
    super::flare::alpha_ramp(&mut model);
    model.absorb_shell = true;
    report.push(format!(
        "{name}: {} - the absorb shell, faded by ShieldColour off the absorb timer \
         (oag_render::absorb_shell)",
        built.describe()
    ));
    Some(model)
}

/// `hd_absorbinternal`'s factor pair, `0x0302`/`0x0001`.
const ADDITIVE: oag_rcs::rcsmodel::Blend = oag_rcs::rcsmodel::Blend::Factors {
    src: oag_rcs::rcsmodel::Factor::SrcAlpha,
    dst: oag_rcs::rcsmodel::Factor::One,
};
