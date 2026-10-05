//! Pulse's `EndRace Rewards` trophies, off the real disc: `TrophyPanel`'s
//! three `<Model>`s (`Data\FE\trophies\gold.vex`/`silver.vex`/`bronze.vex`)
//! decode through `oag_game::preview::model`, each stands for the medal its
//! widget name says, and the disc's own `Mode3D` camera - `OriginX="145"
//! OriginY="60"`, no depth authored so `Mode3D_ReadValues`' `20`/`100` -
//! lands each model's origin on `MedalImg`'s own 32x32 square (`x="80"
//! y="60"`), the icon an earned trophy takes the place of. Two widgets the
//! file places independently agreeing is the check that the camera law and
//! its defaults are read right.
//!
//! **`#[ignore]`d and never run in CI** - it needs game content. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     endrace_trophy_ground_truth
//! ```

use oag_core::math::Vec4;
use oag_tables::race_campaign::Medal;

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulses_three_trophies_decode_and_sit_on_the_medal_icon() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let source = image.display().to_string();
    let mut archives = oag_game::title::open_source(&source, Vec::new(), Vec::new())
        .expect("Pulse's own disc opens")
        .archives;
    let space = oag_display::space::Space::PSP;
    let screens = oag_game::endrace::load(
        &mut archives,
        &oag_ui::language::StringTable::default(),
        oag_ui_screens::picker::FaceScales::default(),
        [space.size.0, space.size.1],
        &oag_game::sprite::Sheet::default(),
        &[],
        oag_pulse::TITLE,
    )
    .expect("Pulse's own EndRace screens read off the real disc");

    let medals: Vec<Medal> = screens.trophies.iter().map(|t| t.medal).collect();
    assert_eq!(medals, [Medal::Gold, Medal::Silver, Medal::Bronze]);

    let rewards = screens.rewards.as_ref().expect("Pulse authors Rewards");
    let icon = rewards
        .screen
        .images
        .iter()
        .find(|image| image.name.as_deref() == Some("MedalImg"))
        .expect("MedalImg is on the screen");
    let (icon_w, icon_h) = (icon.width.unwrap_or(32.0), icon.height.unwrap_or(32.0));

    for trophy in &screens.trophies {
        assert!(
            !trophy.mesh.vertices.is_empty() && !trophy.mesh.indices.is_empty(),
            "{} decodes to triangles",
            trophy.placement.model.src
        );
        assert_eq!(trophy.placement.model.depth, [20.0, 100.0]);
        let (view_projection, model) =
            oag_game::preview::mode3d_view_projection(&trophy.placement.model, space)
                .expect("the trophy's camera is usable");
        let clip = view_projection * model * Vec4::new(0.0, 0.0, 0.0, 1.0);
        let (ndc_x, ndc_y) = (clip.x / clip.w, clip.y / clip.w);
        let x = (ndc_x + 1.0) * 0.5 * space.size.0;
        let y = (1.0 - ndc_y) * 0.5 * space.size.1;
        assert!(
            (icon.x..=icon.x + icon_w).contains(&x) && (icon.y..=icon.y + icon_h).contains(&y),
            "{} lands at ({x}, {y}), MedalImg is {:?}",
            trophy.placement.model.src,
            [icon.x, icon.y, icon_w, icon_h]
        );
    }
}
