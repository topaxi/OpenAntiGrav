//! Which of Anulpha Pass's material slots `mesh::rcs::skin::roles` marks
//! `slots::LIT_ALPHA` - the programs that light the albedo's alpha and tint
//! the specular with its RGB - and which it leaves on the colour path.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'test(/^hd_lit_alpha_ground_truth::/)'
//! ```
//!
//! # What this pins
//!
//! `docs/rendering/hd-track-tone.md`, "The blue cast": Anulpha Pass's walls
//! and floor plates (`reversed_diffuse_specular_normal`, `cf_chevron_pulse`)
//! end their lit block on `MAD H0.xyz,
//! light, albedo.wwww, albedo.rgb * specular`, where the pit floor
//! (`diffuse_normal_specular`) and the panelling (`diffuse_specular`) end on
//! `MUL albedo.rgb, light` and `MAD specular, albedo.wwww, ...`. Lighting the
//! RGB of a `_sc` texture painted its blue specular tint as the surface. If a
//! slot below changes side, the fragment reader
//! (`oag_rcs::rcsmaterial::fragment::Program::lit_albedo`) or the resolver
//! moved, and this is where it shows first.
use crate::archive_cache;
use oag_mesh::mesh::{self, slots};
use std::path::PathBuf;

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";
const TRACK: &str = "/data/environments/15_anulpha_pass/track.vex";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn anulphas_reversed_walls_light_their_alpha_and_its_floor_lights_its_colour() {
    let Some(image) = image() else { return };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let Some(data) = archive_cache::read(&spec, TRACK) else {
        return;
    };
    let Some((model, _report)) =
        mesh::rcs::scene_from(&spec, TRACK, &data).expect("Anulpha Pass's track should build")
    else {
        panic!("{TRACK}: not a PS3 model");
    };
    let geometry = mesh::rcs::sibling_geometry(&spec, TRACK, &data).expect("sibling .rcsmodel");
    let source = oag_rcs::rcsmodel::Model::parse(&geometry).expect("the .rcsmodel parses");

    // Read by hand off the lit-race block of each (`scripts/ps3-microcode.py
    // fp-file`): the slot, its material, and which part of the albedo the
    // light multiplies.
    let read: &[(usize, &str, bool)] = &[
        (485, "reversed_diffuse_specular_normal", true),
        (486, "reversed_diffuse_specular_normal", true),
        (472, "reversed_diffuse_specular_normal", true),
        (442, "cf_chevron_pulse", true),
        (483, "cf_chevron_pulse", true),
        // Lights `constant * normal.a` off a normal map at unit 0; the
        // renderer binds that family's constant as its picture, so the flag
        // stays off and the normal map's alpha is an open, unwired factor.
        (151, "m_diffuse_normal_constantspecular", false),
        (476, "diffuse_normal_specular", false),
        (492, "diffuse_specular", false),
        (518, "diffuse_specular", false),
        (162, "constantdiffuse_specular_normal", false),
    ];
    for &(slot, material, alpha) in read {
        let name = &source.materials[slot].name;
        assert!(
            name.ends_with(&format!("/{material}.rcsmaterial")),
            "slot {slot} is {name}, not {material}: the material table moved"
        );
        let word = model.material_slots[slot];
        assert_eq!(
            word & slots::LIT_ALPHA != 0,
            alpha,
            "slot {slot} ({material}) should {}light the albedo's alpha",
            if alpha { "" } else { "not " }
        );
        assert_eq!(
            slots::material_index(word),
            0,
            "slot {slot} ({material}) carries a glow entry the flag must not be confused with"
        );
    }

    // Every slot of a reversed material on the circuit, not only the ones
    // in the frame, and none of the colour-path families.
    let mut by_family: std::collections::BTreeMap<&str, (usize, usize)> = Default::default();
    for (slot, word) in model.material_slots.iter().enumerate() {
        let Some(material) = source.materials.get(slot) else {
            continue;
        };
        let family = material
            .name
            .rsplit('/')
            .next()
            .unwrap_or("")
            .trim_end_matches(".rcsmaterial");
        let entry = by_family.entry(family).or_default();
        entry.0 += 1;
        if word & slots::LIT_ALPHA != 0 {
            entry.1 += 1;
        }
    }
    for (family, (slots, alpha)) in &by_family {
        if *alpha > 0 {
            println!("  {family}: {alpha} of {slots} slot(s) light their alpha");
        }
    }
    // Measured when this guard was written (2026-10-21, `hd-blue-tone`): three
    // families light their alpha, on every slot each has, and no other does.
    let alpha_families: Vec<(&str, (usize, usize))> = by_family
        .iter()
        .filter(|(_, (_, alpha))| *alpha > 0)
        .map(|(f, c)| (*f, *c))
        .collect();
    assert_eq!(
        alpha_families,
        [
            ("cf_chevron_pulse", (8, 8)),
            ("chevron_pulse", (4, 4)),
            ("reversed_diffuse_specular_normal", (8, 8)),
        ]
    );
}
