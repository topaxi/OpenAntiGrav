//! `oag_render::gantry::mount` swept across every HD/Fury race circuit, not
//! only Talon's Junction.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(gantry_mount_hd_circuits_ground_truth)'
//! ```
//!
//! # Why this exists, next to `start_gantry_hd_mount_ground_truth.rs`
//!
//! That file measures Talon's Junction alone - `docs/rendering/start-gantry.md`'s
//! own "What is still open" names the other eleven HD race circuits (plus
//! `12_Sol_2`, measured by hand in the same pass but never pinned in a test)
//! as unswept, `just check-test-budget`'s ratchet being the reason the sweep
//! stopped at one circuit rather than the twelve `oag_hd::names::ENVIRONMENTS`
//! actually offers.
//!
//! **One `#[test]` per circuit, not a `for` loop over one `#[test]`** - the
//! same shape `CLAUDE.md`'s own test-budget section asks for: `cargo nextest`
//! runs each test in its own process and parallelises across *tests*, so
//! twelve independent circuit checks spread across twelve cores instead of
//! serialising on one. A macro generates the boilerplate once; each circuit's
//! own assertion is independent of every other's, so splitting this way does
//! not change what is asserted - no `BASELINE` row is needed.
//!
//! Zone's own four environments (`zone_1`..`zone_4`) are out of scope, per
//! the standing instruction elsewhere on this page to implement the plain
//! race before its mode variants.
//!
//! # What "authors a mount" means here, and what would make this fail honestly
//!
//! `docs/ghidra/functions/ps3-hdfury-eu/billboards.md` measured
//! `billboard7.gtf`/`billboard8.gtf` present in **all 17** of HD's own
//! environments, disc-side - so every one of the twelve race circuits here is
//! expected to author a `billboard8.gtf`-bound chunk `oag_render::gantry::mount`
//! can fit a plane to. A circuit that comes back `None` is not assumed to be a
//! bug in `mount` - see `oag_render::gantry::mount`'s own doc comment - but it
//! would be a genuine surprise against that disc-side count, worth a doc
//! update rather than a quiet `#[ignore]`.

use std::path::PathBuf;

use oag_render::mesh::Model;

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

/// The circuit's own track model, built exactly as `race::load` builds it -
/// through `oag_render::mesh::rcs::build_scene` off the `.vex` and its
/// sibling `.rcsmodel` - mirroring `start_gantry_hd_mount_ground_truth.rs`'s
/// own `track_model`, parametrised over the environment instead of pinned to
/// Talon's Junction.
fn track_model(image: &std::path::Path, environment: &str) -> Model {
    let mut archives = oag_hd::open(&image.display().to_string()).expect("opening the disc");
    let track = oag_hd::names::track(environment);
    let track_blob = archives.read_name(&track).expect("the track .vex reads");
    let sibling =
        oag_render::mesh::rcs::sibling_name(&track).expect("a .vex name has an .rcsmodel sibling");
    let geometry = archives
        .read_name(&sibling)
        .expect("the sibling .rcsmodel reads");
    let (model, _report) =
        oag_render::mesh::rcs::build_scene(&track, &track_blob, &geometry, &mut |path| {
            archives.read_name(path).ok()
        })
        .expect("the track model builds");
    model
}

/// Builds the model, measures the mount, prints what it found, and asserts
/// it exists - the report a maintainer reading `--nocapture` output wants,
/// same shape `gantry_mount_ground_truth.rs` already prints for Pulse.
fn assert_circuit_authors_a_mount(environment: &str) {
    let Some(image) = image() else { return };
    let model = track_model(&image, environment);
    let mount = oag_render::gantry::mount(&model);
    match &mount {
        Some(m) => println!(
            "{environment}: mount centre {:?} width {:.2} height {:.2} thickness {:.3} vertices {}",
            m.centre, m.width, m.height, m.thickness, m.vertices
        ),
        None => println!("{environment}: no billboard8 mount found"),
    }
    assert!(
        mount.is_some(),
        "{environment} authors no billboard8.gtf-bound mount - see this file's \
         own doc comment for why that would be a genuine surprise, not an \
         expected gap"
    );
}

/// One `#[test]` per HD race environment, `#[ignore]`d the same way every
/// other ground-truth test on this page is.
macro_rules! circuit_test {
    ($fn_name:ident, $environment:literal) => {
        #[test]
        #[ignore = "needs a decrypted PS3 disc image in data/images"]
        fn $fn_name() {
            assert_circuit_authors_a_mount($environment);
        }
    };
}

// `oag_hd::names::ENVIRONMENTS` minus its four `zone_*` entries - see the
// module doc comment above for why Zone is out of scope. Talon's Junction is
// included deliberately: `start_gantry_hd_mount_ground_truth.rs` already
// covers it with tighter, hand-measured bounds, and re-covering it here at
// this file's own looser "authors a mount at all" bar costs nothing and keeps
// this file a complete sweep of the twelve on its own, readable without
// cross-referencing the other one.
circuit_test!(amphiseum_authors_a_gantry_mount, "amphiseum");
circuit_test!(modesto_heights_authors_a_gantry_mount, "modesto_heights");
circuit_test!(talons_junction_authors_a_gantry_mount, "talons_junction");
circuit_test!(tech_de_ra_authors_a_gantry_mount, "tech_de_ra");
circuit_test!(vineta_k_authors_a_gantry_mount, "01_vineta_k");
circuit_test!(track_02_authors_a_gantry_mount, "02_track");
circuit_test!(track_03_authors_a_gantry_mount, "03_track");
circuit_test!(
    chenghou_project_authors_a_gantry_mount,
    "04_chenghou_project"
);
circuit_test!(ubermall_authors_a_gantry_mount, "05_ubermall");
circuit_test!(sebenco_climb_authors_a_gantry_mount, "10_sebenco_climb");
circuit_test!(sol_2_authors_a_gantry_mount, "12_sol_2");
circuit_test!(anulpha_pass_authors_a_gantry_mount, "15_anulpha_pass");
