//! Wipeout 2048's window and awning families on Moa Therma's circuit.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. Run it with `just test-data`.
//!
//! `fc10_Lambert_2UV_Emissive` ends `fog(light * D^2 + D.a * R^2)` on every program of the file,
//! `R` its `ReflectMap` at the world reflection vector, and `dc_diffusetexturealphaemissive` ends
//! `fog(D^2 * (light + 1))` with the texel's alpha as the surface's. The loader gives each a
//! glow-table entry of its own program, the `ReflectMap` on the third binding for the window, and
//! the alpha lane for both. Drop `psp2::emissive_add`'s two rows and every assertion fails.

use std::path::Path;

use oag_mesh::mesh::rcs::psp2;
use oag_mesh::mesh::{Model, PROGRAM_PICTURE_ADD, PROGRAM_REFLECT_MAP, slots};

fn build() -> Option<(Model, psp2::Report)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    let (dlc, base) = (
        root.join("dlc1/PSP2/dlc1.psarc"),
        root.join("base/PSP2/data.psarc"),
    );
    if !dlc.exists() || !base.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} or {} is missing",
            dlc.display(),
            base.display()
        );
        println!("skipping: {} not present", root.display());
        return None;
    }
    let open =
        |p: &Path| oag_assets::psarc::Archive::open(p.to_str().expect("utf-8")).expect("opens");
    let (mut dlc, mut base) = (open(&dlc), open(&base));
    let model = dlc
        .read_path("data/art/published/DLC1/environments/Moa_Therma/track.rcsmodel")
        .expect("model");
    Some(
        psp2::build("moa_therma", &model, None, &mut |p| {
            dlc.read_path(p).or_else(|_| base.read_path(p)).ok()
        })
        .expect("builds"),
    )
}

/// Every vertex whose glow-table entry runs `program`, with the label of the third binding its
/// draw carries.
fn with_program(model: &Model, program: f32) -> Vec<(u32, Option<String>)> {
    let mut found = Vec::new();
    for draw in model
        .draws
        .iter()
        .chain(&model.transparent_draws)
        .filter(|d| !d.range.is_empty())
    {
        let vertex = model.vertices[model.indices[draw.range.start as usize] as usize];
        let index = slots::material_index(vertex.slots) as usize;
        if index == 0 || model.emissive[index - 1].program[0] != program {
            continue;
        }
        let third = draw
            .texture
            .and_then(|t| model.pad_masks.get(t))
            .and_then(Option::as_ref)
            .map(|t| t.label.to_ascii_lowercase());
        found.push((vertex.slots, third));
    }
    found
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_window_family_reads_its_reflect_map_by_the_pictures_alpha() {
    let Some((model, report)) = build() else {
        return;
    };
    let windows = with_program(&model, PROGRAM_REFLECT_MAP);
    assert!(
        windows.len() > 100,
        "{} windows; {}",
        windows.len(),
        report.describe()
    );
    for (packed, third) in &windows {
        assert_eq!((packed >> 3) & 3, 3, "the alpha lane");
        assert!(
            third
                .as_deref()
                .is_some_and(|t| t.contains("sky_moatherma_1")),
            "{third:?}"
        );
    }
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_awning_adds_its_own_square_and_writes_the_pictures_alpha() {
    let Some((model, report)) = build() else {
        return;
    };
    let awnings = with_program(&model, PROGRAM_PICTURE_ADD);
    assert!(!awnings.is_empty(), "{}", report.describe());
    for (packed, third) in &awnings {
        assert_eq!((packed >> 3) & 3, 3, "the alpha lane");
        assert!(third.is_none(), "{third:?}");
    }
}
