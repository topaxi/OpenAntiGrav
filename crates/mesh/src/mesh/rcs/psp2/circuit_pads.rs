//! A circuit's own pads as 2048's and Omega's `.rcsmodel` holds them, and which of them a build
//! leaves out.
//!
//! The two containers name no pad class: a pad is a submesh on a pad material, or a shape the
//! artists named `speedup_pad` or `weapon_pad`, drawn in the world pass with everything else.
//! HD's pads are named by `.vex` nodes and built apart (`super::super::pads`).

use oag_rcs::rcsmodel::psp2;

/// Which of a circuit's own pad submeshes a build leaves out. A left-out submesh keeps its
/// empty draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HidePads {
    /// Every pad is drawn.
    #[default]
    None,
    /// The weapon pads, for a race with weapons off: the original neither draws nor arms them
    /// (`oag_race::Mode::weapons_enabled`). Speed pads stay.
    Weapon,
    /// Speed and weapon pads, for a race that draws its own pad model in their place.
    All,
}

impl HidePads {
    pub(super) fn hides(self, decoded: &psp2::Model, submesh: &psp2::SubMesh) -> bool {
        match self {
            Self::None => false,
            Self::Weapon => is_weapon_pad(decoded, submesh),
            Self::All => is_circuit_pad(decoded, submesh),
        }
    }
}

/// The material file's name without its path, extension, case or the PS4 Pro `_1`/`_2` suffix.
fn material_stem(name: &str) -> String {
    let leaf = name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .to_ascii_lowercase();
    let stem = leaf.trim_suffix(".rcsmaterial");
    stem.strip_suffix("_1")
        .or_else(|| stem.strip_suffix("_2"))
        .unwrap_or(stem)
        .to_string()
}

/// Whether a material file is one of the circuit's own pads: `weapon_pads`, `speed_pads` or
/// `speedup_material`.
fn is_circuit_pad_material(name: &str) -> bool {
    matches!(
        material_stem(name).as_str(),
        "weapon_pads" | "speed_pads" | "speedup_material"
    )
}

fn material_of<'a>(decoded: &'a psp2::Model, submesh: &psp2::SubMesh) -> Option<&'a str> {
    submesh
        .material
        .and_then(|m| decoded.materials.get(m))
        .map(|m| m.name.as_str())
}

fn shape_of(decoded: &psp2::Model, submesh: &psp2::SubMesh) -> String {
    submesh
        .mesh
        .and_then(|m| decoded.scene.meshes.get(m))
        .map_or_else(String::new, |m| m.name.to_ascii_lowercase())
}

/// Whether a submesh is one of the circuit's own pads: on a pad material, or a shape the artists
/// named `speedup_pad` or `weapon_pad` (Omega's HD-lineage circuits draw a speed pad on
/// `diffuse_normal_specular_emmissive`, the start-line material, under a shape of that name).
#[must_use]
pub fn is_circuit_pad(decoded: &psp2::Model, submesh: &psp2::SubMesh) -> bool {
    let shape = shape_of(decoded, submesh);
    material_of(decoded, submesh).is_some_and(is_circuit_pad_material)
        || shape.contains("speedup_pad")
        || shape.contains("weapon_pad")
}

/// Whether a submesh is one of the circuit's own **weapon** pads.
///
/// **The shape name wins over the material**: Omega's `zone_1` to `zone_4` circuits draw their
/// speed pads, shapes named `speedup_pad`, on `weapon_pads.rcsmaterial`.
#[must_use]
pub fn is_weapon_pad(decoded: &psp2::Model, submesh: &psp2::SubMesh) -> bool {
    let shape = shape_of(decoded, submesh);
    weapon_pad_by_name(material_of(decoded, submesh).unwrap_or_default(), &shape)
}

fn weapon_pad_by_name(material: &str, shape: &str) -> bool {
    shape.contains("weapon_pad")
        || (material_stem(material) == "weapon_pads" && !shape.contains("speedup_pad"))
}

#[cfg(test)]
mod tests {
    use super::{is_circuit_pad_material, weapon_pad_by_name};

    #[test]
    fn pad_materials_match_in_any_case_separator_and_suffix() {
        for name in [
            "data/environments/x/materials/weapon_pads.rcsmaterial",
            "Data\\Environments2048\\x\\Materials\\Speed_Pads.rcsmaterial",
            "materials/Speedup_Material.rcsmaterial",
            "materials/weapon_pads_1.rcsmaterial",
            "materials/WEAPON_PADS_2.rcsmaterial",
        ] {
            assert!(is_circuit_pad_material(name), "{name}");
        }
        for name in [
            "materials/diffuse_normal_specular_emmissive.rcsmaterial",
            "materials/diffuse_normal_specular_emmissive_alpha.rcsmaterial",
            "materials/chevron_facing_material.rcsmaterial",
            "materials/speed_pads_extra.rcsmaterial",
        ] {
            assert!(!is_circuit_pad_material(name), "{name}");
        }
    }

    #[test]
    fn a_speedup_shape_on_the_weapon_pad_material_is_not_a_weapon_pad() {
        let material = "materials/weapon_pads.rcsmaterial";
        assert!(weapon_pad_by_name(material, "weapon_pad_3shape"));
        assert!(weapon_pad_by_name(material, "polysurface12"));
        assert!(!weapon_pad_by_name(
            material,
            "wohdtrack_speedup_pad_4shape"
        ));
        assert!(weapon_pad_by_name(
            "materials/diffuse.rcsmaterial",
            "wohdtrack_weapon_pad_4shape"
        ));
        assert!(!weapon_pad_by_name(
            "materials/speed_pads.rcsmaterial",
            "wohdtrack_speedup_pad_4shape"
        ));
    }
}
