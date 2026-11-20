//! Omega's crowd rigs: one `Data/Crowd/Rigs` figure at each of a track's crowd-entity nodes, in
//! its bind pose.
//!
//! The original's track loader replaces every `trackcrowd:crowdentity<N>[s][w]_<M>Shape`
//! placeholder (mesh flags `0x0401`) with an animated, skinned rig instance at the placeholder's
//! node (`docs/formats/omega-status.md`, "The crowd-entity shapes" and "The crowd rigs"). This
//! merges one rig per entity into the decoded model before the build plans it, as ordinary
//! node-bound submeshes, so every plan that reads materials and submeshes sees them.
//!
//! **Which rig** (`FUN_0162a390`, confidence 85, read from the v1.07 eboot): the digits `N` after
//! `CrowdEntity` and the letters `s` and `w` (each looked for at the two characters after the
//! digits) make a group index `N - 1 + 15 * (s + 2 * w)`, clamped to 59, and the table
//! `FUN_0163b720` fills gives the group a rig slot and a clip. The slot names one of 40
//! rigs: `Male01..05`, `Female01..05`, their `_Shadow` twins, then the same for `06..10`. So
//! the digits pick the person, `s` the shadowed twin and `w` the second texture group
//! (`06..10`). There is no random choice of rig: `rand()` only draws the clip's start phase.
//!
//! **The placement** (confidence 70): the rig's vertices are authored at one hundredth of a metre
//! (the skeleton's root bone carries scale 100, the figure is 1.69 to 1.75 tall after it), and the
//! entity node's bind matrix carries the circuit's unit scale (about 2.3 on Sol; the placeholder is
//! 1.74 tall in the node's own space, so 4.0 units in the world). A rig vertex is scaled by 100 and
//! then placed by the entity's node.
//!
//! **The pose is the bind pose**: the skin weights and bone indices are decoded
//! ([`psp2::SubMesh::skin_weights`]) and not applied, since at bind every skin matrix is the
//! identity. The group's clip (`male_01_folded_arms` and five more) does not play.

use std::collections::HashMap;

use oag_rcs::rcsmodel::psp2;

use super::Report;
use crate::mesh::rcs::Textures;

/// The skeleton's root scale: rig vertices are authored in hundredths.
const RIG_SCALE: f32 = 100.0;

/// How many numbers a letter variant spans, and how many groups the loader's table holds.
const PER_VARIANT: usize = 15;
const GROUPS: usize = 4 * PER_VARIANT;

/// Which of the ten rigs of a texture group (`Male01..05`, `Female01..05`) group number
/// `1..=15` stands at, from the loader's table (`FUN_0163b720`: the first element of each pair).
const SLOT: [u8; PER_VARIANT] = [0, 1, 1, 2, 2, 3, 3, 4, 4, 0, 5, 6, 7, 8, 9];

/// The loader's group index for a crowd-entity shape's name, or `None` for a name that has no
/// `CrowdEntity` in it. Mirrors the loader's reading, including its clamp to the 60th group.
fn group(name: &str) -> Option<usize> {
    let lower = name.to_ascii_lowercase();
    let rest = &lower[lower.find("crowdentity")? + "crowdentity".len()..];
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    // `sscanf("%d")` leaves the preset 1 when no digit is there.
    let number: usize = rest[..digits].parse().unwrap_or(1);
    let mut after = rest.bytes().skip(digits);
    let letters = [after.next(), after.next()];
    let has = |c: u8| letters.contains(&Some(c));
    let variant = match (has(b's'), has(b'w')) {
        (false, false) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (true, true) => 3,
    };
    Some(
        (number + variant * PER_VARIANT)
            .saturating_sub(1)
            .min(GROUPS - 1),
    )
}

/// `<male|female><NN>[_shadow]`, the rig a group index stands at.
fn rig_stem(group: usize) -> String {
    let variant = group / PER_VARIANT;
    let slot = usize::from(SLOT[group % PER_VARIANT]);
    let (shadow, second) = (variant & 1 == 1, variant & 2 == 2);
    let gender = if slot < 5 { "male" } else { "female" };
    let number = slot % 5 + 1 + if second { 5 } else { 0 };
    format!("{gender}{number:02}{}", if shadow { "_shadow" } else { "" })
}

/// Adds one rig submesh per crowd-entity shape that sits on a node, with the rig's material
/// appended to the model's table. Does nothing for a model that is not Omega's, or when a rig's
/// file is not there.
pub(super) fn instance(
    decoded: &mut psp2::Model,
    ps4: bool,
    textures: Textures<'_>,
    report: &mut Report,
) {
    if !ps4 {
        return;
    }
    let entities: Vec<(usize, Option<usize>)> = decoded
        .scene
        .meshes
        .iter()
        .filter(|m| m.is_crowd_entity())
        .filter_map(|m| Some((m.node?, group(&m.name))))
        .collect();
    if entities.is_empty() {
        return;
    }
    let mut rigs: HashMap<String, Option<psp2::Model>> = HashMap::new();
    let mut materials: HashMap<String, usize> = HashMap::new();
    let mut record = usize::MAX / 2;
    for (node, group) in entities {
        let Some(group) = group else {
            report.crowd_rigs_missing += 1;
            continue;
        };
        let stem = rig_stem(group);
        let rig = rigs.entry(stem.clone()).or_insert_with(|| {
            let blob = textures(&format!("Data/crowd/Rigs/{stem}.rcsmodel"))?;
            psp2::parse(&blob).ok()
        });
        let Some(rig) = rig else {
            report.crowd_rigs_missing += 1;
            continue;
        };
        let (Some(sub), Some(material)) = (
            rig.submeshes.first(),
            rig.submeshes
                .first()
                .and_then(|s| s.material)
                .and_then(|m| rig.materials.get(m)),
        ) else {
            report.crowd_rigs_missing += 1;
            continue;
        };
        let index = *materials.entry(stem).or_insert_with(|| {
            decoded
                .materials
                .iter()
                .position(|m| m == material)
                .unwrap_or_else(|| {
                    decoded.materials.push(material.clone());
                    decoded.materials.len() - 1
                })
        });
        let mut placed = sub.clone();
        record += 1;
        placed.record = record;
        placed.mesh = None;
        placed.node = Some(node);
        placed.material = Some(index);
        for p in &mut placed.positions {
            *p = p.map(|c| c * RIG_SCALE);
        }
        placed.skin_weights.clear();
        placed.skin_indices.clear();
        report.crowd_rig_vertices += placed.positions.len();
        decoded.submeshes.push(placed);
        report.crowd_rigs += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_digits_and_letters_make_the_loaders_group_index() {
        assert_eq!(group("trackcrowd:crowdentity6_10Shape"), Some(5));
        assert_eq!(group("trackcrowd:crowdentity6s_10Shape"), Some(20));
        assert_eq!(group("trackcrowd:crowdentity9w_3Shape"), Some(38));
        assert_eq!(group("trackcrowd:crowdentity9sw_3Shape"), Some(53));
        assert_eq!(
            group("trackcrowd:crow_to_import:crowdentity15_3Shape"),
            Some(14)
        );
        assert_eq!(group("trackcrowd:crowdentity99_3Shape"), Some(GROUPS - 1));
        assert_eq!(group("trackcrowd:stand_3Shape"), None);
    }

    #[test]
    fn a_group_names_its_rig_as_the_loaders_table_does() {
        // Slots 0..4 `Male01..05`, 5..9 `Female01..05`, +10 the shadow twin, +20 the second set.
        assert_eq!(rig_stem(0), "male01");
        assert_eq!(rig_stem(9), "male01");
        assert_eq!(rig_stem(5), "male04");
        assert_eq!(rig_stem(10), "female01");
        assert_eq!(rig_stem(14), "female05");
        assert_eq!(rig_stem(15), "male01_shadow");
        assert_eq!(rig_stem(25), "female01_shadow");
        assert_eq!(rig_stem(30), "male06");
        assert_eq!(rig_stem(59), "female10_shadow");
    }

    #[test]
    fn every_group_names_a_distinct_slot_of_the_forty() {
        let stems: std::collections::BTreeSet<String> = (0..GROUPS).map(rig_stem).collect();
        assert_eq!(stems.len(), 40);
    }
}
