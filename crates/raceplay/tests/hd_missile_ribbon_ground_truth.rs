//! Wipeout HD's Missile ribbons on a real disc: a fired Missile lays a stub and a
//! helix at each of its two heads (four `FatLine` elements), the helices' alpha
//! reads the opacity ramp, and the ribbons outlive the missile.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^hd_missile_ribbon_ground_truth::/)'
//! ```
//!
//! The pool is read from `Missile_Init_q` (`0x0011c5d0`): two elements of each of
//! `RibbonEffects` pools `+0x50` and `+0x68` per missile
//! (`docs/ghidra/functions/ps3-hdfury-eu/missile-ribbon.md`).

use oag_core::math::Vec3;
use oag_gameplay::{InputSnapshot, PlayerInputs};
use oag_raceplay as race;

/// The tick the missile is fired on, after the countdown.
const FIRE: u64 = 421;

fn loaded() -> Option<race::Loaded> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in loaded
        .report
        .iter()
        .filter(|l| l.contains("missile ribbons"))
    {
        println!("{line}");
    }
    Some(loaded)
}

#[test]
#[ignore = "needs a disc image"]
fn the_disc_loads_the_thickline_texture_and_the_helix_config() {
    let Some(loaded) = loaded() else {
        return;
    };
    let assets = loaded
        .ribbon_textures
        .missile_smoke
        .as_ref()
        .expect("ThickLine_Red.gtf and the opacity ramp must both load");
    assert!(assets.texture.width > 0 && assets.texture.height > 0);
    assert_eq!(assets.trail.helix.chains, 3);
    assert!(
        loaded
            .report
            .iter()
            .any(|l| l.contains("missile ribbons: Data/Tex/ThickLine_Red.gtf")),
        "the loader report names the texture"
    );
}

#[test]
#[ignore = "needs a disc image"]
fn a_missile_lays_four_ribbons_that_draw_and_outlive_it() {
    let Some(loaded) = loaded() else {
        return;
    };
    let ramp = loaded
        .ribbon_textures
        .missile_smoke
        .as_ref()
        .expect("assets")
        .ramp
        .clone();
    let mut race = race::Race::start(loaded.setup);
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Missile);
    let eye = Vec3::new(0.0, 5.0, 0.0);
    let mut buttons = oag_core::buttons::Input::new();
    let mut peak = 0;
    let mut lit = false;
    let mut flying_until = None;
    let mut emptied_at = None;
    for tick in 1..=FIRE + 900 {
        let held = if tick == FIRE {
            1 << oag_core::buttons::Button::Square as u32
        } else {
            0
        };
        buttons.begin_frame(held);
        race.tick(&PlayerInputs::single(InputSnapshot {
            buttons,
            ..InputSnapshot::EMPTY
        }));
        let live = race.missile_smoke_live();
        peak = peak.max(live);
        let airborne = race
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .any(|p| p.kind == Some(oag_tables::weapons::Weapon::Missile));
        if airborne {
            flying_until = Some(tick);
        }
        if tick > FIRE + 60 && tick % 30 == 0 {
            let vertices = race.missile_smoke_vertices(eye, &ramp);
            assert!(
                vertices
                    .iter()
                    .all(|v| v.position.iter().all(|c| c.is_finite())),
                "a vertex is not finite at tick {tick}"
            );
            lit |= vertices.iter().any(|v| v.colour[3] > 0.2);
        }
        if tick > FIRE + 1 && live == 0 && emptied_at.is_none() {
            emptied_at = Some(tick);
        }
    }
    assert_eq!(peak, 4, "a stub and a helix at each of the two heads");
    assert!(lit, "the ribbons never drew a visible vertex");
    let flew = flying_until.expect("the missile never flew");
    let emptied = emptied_at.expect("the ribbons never freed themselves");
    // The helix lives 2.8 s past its last node: about 168 ticks after the missile.
    assert!(
        emptied >= flew + 160,
        "freed at {emptied}, missile gone at {flew}"
    );
}
