//! Wipeout HD's vertex-side texture scroll, from the material record to a pixel.
//!
//! **`#[ignore]`d and never run in CI.** It needs a decrypted PS3 disc image.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(hd_vertex_scroll_ground_truth)'
//! ```
//!
//! The law is on `docs/formats/rcsmaterial.md`, "HD's vertex programs scroll
//! the texture coordinate". The reach below is pinned per circuit so a change
//! that switched on a family nobody read, or dropped the wiring, moves a number.

mod archive_cache;

use std::path::{Path, PathBuf};

use oag_mesh::mesh::{self, AnimTrack};

const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

fn build(image: &Path, archive: &str, circuit: &str) -> Option<(mesh::Model, usize)> {
    let path = format!("/data/environments/{circuit}/track.vex");
    for a in [archive, "DATA02.PSARC"] {
        let spec = format!("{}:PS3_GAME/USRDIR/{a}", image.display());
        let Some(data) = archive_cache::read(&spec, &path) else {
            continue;
        };
        let geometry = mesh::rcs::sibling_geometry(&spec, &path, &data)?;
        let (model, report) = mesh::rcs::build_scene(&path, &data, &geometry, &mut |name| {
            archive_cache::read(&spec, name)
        })
        .expect("the scene builds");
        return Some((model, report.vertex_scrolls));
    }
    None
}

/// One test per circuit, so the matrix is the test axis rather than a loop in
/// one test: `(name, archive, circuit directory, materials scrolled by an
/// authored rate)`.
macro_rules! reach {
    ($($name:ident: $archive:literal, $circuit:literal, $expected:literal;)*) => {$(
        #[test]
        #[ignore = "needs a decrypted PS3 disc image in data/images"]
        fn $name() {
            let Some(image) = image() else { return };
            reaches(&image, $archive, $circuit, $expected);
        }
    )*};
}

reach! {
    tech_de_ra_scrolls_five: "DATA00.PSARC", "tech_de_ra", 5;
    amphiseum_scrolls_fifteen: "DATA00.PSARC", "amphiseum", 15;
    modesto_heights_scrolls_seven: "DATA00.PSARC", "modesto_heights", 7;
    talons_junction_scrolls_two: "DATA00.PSARC", "talons_junction", 2;
    vineta_k_scrolls_seven: "DATA00.PSARC", "01_vineta_k", 7;
    track_02_scrolls_one: "DATA00.PSARC", "02_track", 1;
    track_03_scrolls_seven: "DATA00.PSARC", "03_track", 7;
    chenghou_scrolls_two: "DATA00.PSARC", "04_chenghou_project", 2;
    ubermall_scrolls_eleven: "DATA00.PSARC", "05_ubermall", 11;
    sebenco_scrolls_one: "DATA00.PSARC", "10_sebenco_climb", 1;
    sol_2_scrolls_one: "DATA00.PSARC", "12_sol_2", 1;
    anulpha_pass_scrolls_seventeen: "DATA00.PSARC", "15_anulpha_pass", 17;
    zone_1_scrolls_none: "DATA00.PSARC", "zone_1", 0;
    zone_2_scrolls_none: "DATA00.PSARC", "zone_2", 0;
}

fn reaches(image: &Path, archive: &str, circuit: &str, expected: usize) {
    let (model, scrolled) =
        build(image, archive, circuit).unwrap_or_else(|| panic!("{circuit} builds"));
    assert_eq!(scrolled, expected, "{circuit}: materials scrolled");
    let scrolls: Vec<[f32; 2]> = model
        .anim_tracks
        .iter()
        .filter_map(|t| match t {
            AnimTrack::Scroll(r) => Some(*r),
            _ => None,
        })
        .collect();
    assert_eq!(scrolls.is_empty(), expected == 0, "{circuit}: tracks");
    let driven = model
        .vertices
        .iter()
        .filter(|v| {
            matches!(
                model.anim_tracks.get((v.anim as usize).wrapping_sub(1)),
                Some(AnimTrack::Scroll(_))
            )
        })
        .count();
    assert_eq!(driven > 0, expected > 0, "{circuit}: vertices driven");
    match circuit {
        // `basic_uv_scroll`'s authored `USpeed`.
        "tech_de_ra" => assert!(scrolls.contains(&[0.45, 0.0]), "{scrolls:?}"),
        // `hologram`'s authored `Speed`.
        "amphiseum" => assert!(scrolls.contains(&[0.5, 0.0]), "{scrolls:?}"),
        _ => {}
    }
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_vertex_scroll_moves_pixels_and_the_clock_decides_where() {
    let Some(image) = image() else { return };
    let shot = |model: &mesh::Model, seconds: f32| {
        oag_mesh::mesh_render::capture_pixels_from(
            model,
            320,
            240,
            0.0,
            0.3,
            oag_mesh::mesh_render::Anisotropy::default(),
            seconds,
        )
        .expect("the offscreen capture runs")
    };
    let differing = |a: &[u8], b: &[u8]| {
        a.as_chunks::<4>()
            .0
            .iter()
            .zip(b.as_chunks::<4>().0)
            .filter(|(x, y)| x != y)
            .count()
    };
    let mut attributed = 0usize;
    for circuit in ["tech_de_ra", "amphiseum", "15_anulpha_pass"] {
        let (model, _) = build(&image, "DATA00.PSARC", circuit).expect("builds");
        let mut still = model.clone();
        for v in &mut still.vertices {
            if matches!(
                model.anim_tracks.get((v.anim as usize).wrapping_sub(1)),
                Some(AnimTrack::Scroll(_))
            ) {
                v.anim = 0;
            }
        }
        let on = shot(&model, 0.6);
        let off = shot(&still, 0.6);
        let contributed = differing(&on, &off);
        println!("{circuit}: the scroll paints {contributed} pixels at 0.6 s");
        attributed += contributed;
        // At clock 0 an offset of zero is the identity: nothing differs.
        assert_eq!(
            differing(&shot(&model, 0.0), &shot(&still, 0.0)),
            0,
            "{circuit}: the scroll shows at clock 0"
        );
    }
    assert!(attributed > 0, "the scroll changed no pixel on any circuit");
}
