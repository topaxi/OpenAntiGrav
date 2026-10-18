//! Circuit `11`, which arrives with a downloadable pack, mounts its start
//! gantry on its board and not on a fit across two quads.
//!
//! `11` binds `billboard8.tga` on the board (171 indices, 164.5 units ahead of
//! the Start Position, 14.7 above, straight ahead) and on a 75-index quad 795
//! units away. One plane fitted across both read "44.6 x 643.7" and hung the
//! board over the grid (`docs/rendering/start-gantry.md`, 2026-10-18,
//! `pulse-mount-11`). `11` is not offered by the original's Track Select, so the
//! evidence is the asset: the same board quad every other circuit authors.

use oag_core::math::Vec3;
use oag_raceplay as race;

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and data/dlc"]
fn circuit_11_mounts_on_its_board_quad_not_a_fit_across_two() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let Some(dlc) = oag_testdata::image("data/dlc").filter(|d| d.is_dir()) else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        dlc: vec![dlc],
        track: Some(r"Data\Environments\11_Track\track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let model = &loaded.track_model;
    let mount = oag_render::gantry::mount(model).expect("11 authors a billboard8 surface");

    let start = loaded.setup.start_position.as_ref().expect("a Start Position");
    let from_start = (mount.centre - Vec3::from(start.position)).length();
    assert!(
        (150.0..180.0).contains(&from_start),
        "the mount stands {from_start:.1} units from the Start Position; every other circuit's \
         board is 164-165"
    );
    assert!(
        (30.0..60.0).contains(&mount.width) && (8.0..20.0).contains(&mount.height),
        "{:.1} x {:.1} is not the 45 x 13 board the other circuits author",
        mount.width,
        mount.height
    );
    assert!(
        mount.vertices < 200,
        "{} vertices: the far 75-index quad is back in the fit",
        mount.vertices
    );
}
