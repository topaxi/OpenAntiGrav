//! Which of an Omega material's textures is its tangent-space normal map.
//!
//! The pixel programs say which samplers they decode `2x - 1` ([`crate::gcn::signed_samplers`]);
//! the material instance says which of those it binds and to what file. A normal map is a
//! signed sampler the instance binds that is neither its diffuse picture nor its lightmap
//! (`shadowMap` is signed too, and an instance never binds it). Exactly one such file is the
//! answer; none, or several, is named as such and nothing is bound.

use crate::gcn::SignedSamplers;
use crate::rcsmaterial::name_hash;

use super::material::Material;

/// [`choose`]'s answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice<'m> {
    /// The one normal map the instance binds: its file.
    Bound(&'m str),
    /// Several signed samplers the instance binds are neither diffuse nor lightmap (a material
    /// that blends two normal maps, or scales a second picture the same way): their files.
    /// Which one the surface's tangent frame belongs to is not read, so none is bound.
    Ambiguous(Vec<&'m str>),
    /// None of the instance's samplers is signed by any program.
    Absent,
}

/// The normal map of `material`, given what its file's programs decode `2x - 1`.
#[must_use]
pub fn choose<'m>(material: &'m Material, signed: &SignedSamplers) -> Choice<'m> {
    let diffuse = material.diffuse_texture();
    let lightmap = material.lightmap.as_deref();
    let mut files: Vec<&str> = Vec::new();
    for name in &signed.names {
        let Some(path) = material.sampler(name_hash(name)) else {
            continue;
        };
        if Some(path) != diffuse && Some(path) != lightmap && !files.contains(&path) {
            files.push(path);
        }
    }
    match files.as_slice() {
        [] => Choice::Absent,
        [one] => Choice::Bound(one),
        _ => Choice::Ambiguous(files),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(samplers: &[(&str, &str)], lightmap: Option<&str>) -> Material {
        Material {
            name: "m.rcsmaterial".to_string(),
            technique: None,
            textures: samplers.iter().map(|(_, p)| (*p).to_string()).collect(),
            lightmap: lightmap.map(str::to_string),
            params: Vec::new(),
            samplers: samplers
                .iter()
                .map(|(n, p)| (name_hash(n), (*p).to_string()))
                .collect(),
            state: None,
        }
    }

    fn signed(names: &[&str]) -> SignedSamplers {
        SignedSamplers {
            names: names.iter().map(|n| (*n).to_string()).collect(),
            unresolved: 0,
        }
    }

    #[test]
    fn the_one_signed_sampler_the_instance_binds_is_the_normal_map() {
        let m = material(
            &[("DiffuseMap", "d.gnf"), ("NormalMap", "n.gnf")],
            Some("l-lmap.gnf"),
        );
        // `shadowMap` is signed too but an instance never binds it.
        assert_eq!(
            choose(&m, &signed(&["NormalMap", "shadowMap"])),
            Choice::Bound("n.gnf")
        );
    }

    #[test]
    fn a_signed_diffuse_or_lightmap_is_not_a_normal_map() {
        let m = material(
            &[("DiffuseMap", "d.gnf"), ("lightmap", "l-lmap.gnf")],
            Some("l-lmap.gnf"),
        );
        assert_eq!(
            choose(&m, &signed(&["DiffuseMap", "lightmap"])),
            Choice::Absent
        );
    }

    #[test]
    fn two_signed_samplers_are_named_as_ambiguous_and_neither_is_bound() {
        let m = material(
            &[
                ("DiffuseMap", "d.gnf"),
                ("RockColourNormal", "a.gnf"),
                ("RockDetailNormal", "b.gnf"),
            ],
            None,
        );
        assert_eq!(
            choose(&m, &signed(&["RockColourNormal", "RockDetailNormal"])),
            Choice::Ambiguous(vec!["a.gnf", "b.gnf"])
        );
    }
}
