//! Eight craft, eight teams, off a real disc image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all livery
//! ```
//!
//! # Why it needs the disc
//!
//! `oag_livery`'s unit tests cover which team lands in which slot, which is
//! arithmetic over a list of strings. What they cannot see is whether those
//! ids resolve to *different hulls* on a real source, and that is the whole
//! feature: before this landed, `race::load` read one `Ship.vex` and
//! `Scene::new` cloned it eight times, so a grid was one team's ship eight
//! times over.
//!
//! **The assertion is on geometry, and that is a deliberate choice.** Two hulls
//! could differ only in the texture painted on them - that is exactly what
//! Zone mode does, where every team's `Zone.vex` decodes to the same 1213
//! vertices and 1149 triangles. It is not what the race hulls do: the eight
//! teams the PSP disc declares range from 845 to 1,497 triangles. So counting
//! distinct triangle counts is a real test here, and would be a vacuous one if
//! pointed at Zone.

use std::path::{Path, PathBuf};

use oag_raceplay as race;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// A single race, which is the mode that fields a grid.
fn load() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        // Deliberately empty: `load` then reads the disc's own plugin
        // definition, which is the path `--race`, a capture and this test all
        // take. An entry point that forgot to pass a list is how the whole
        // grid ended up in one livery the first time.
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

#[test]
#[ignore = "needs a disc image"]
fn every_grid_slot_gets_its_own_teams_hull() {
    let Some(loaded) = load() else {
        return;
    };
    assert_eq!(
        loaded.liveries.len(),
        oag_gameplay::MAX_SHIPS,
        "one livery per grid slot"
    );

    let teams: Vec<&str> = loaded
        .liveries
        .iter()
        .map(|livery| livery.team.as_str())
        .collect();
    let distinct: std::collections::BTreeSet<&&str> = teams.iter().collect();
    assert_eq!(
        distinct.len(),
        oag_gameplay::MAX_SHIPS,
        "the disc declares eight raceable teams and the grid is eight, so every \
         slot should fly its own: {teams:?}"
    );

    // The feature, not the plumbing: eight *different models*. Triangle counts
    // are the cheapest thing that cannot be equal by accident - see the module
    // docs on why this would be the wrong assertion for Zone.
    let shapes: std::collections::BTreeSet<usize> = loaded
        .liveries
        .iter()
        .map(|livery| livery.hull.indices.len() / 3)
        .collect();
    assert!(
        shapes.len() >= 6,
        "the grid drew {} distinct hull(s) across eight teams, so most craft are \
         wearing somebody else's ship: {:?}",
        shapes.len(),
        loaded
            .liveries
            .iter()
            .map(|livery| (livery.team.as_str(), livery.hull.indices.len() / 3))
            .collect::<Vec<_>>()
    );
}

/// The other half of a livery, and the one a shared hull hid: each team's
/// exhaust hangs off *its own* nozzle.
///
/// A single locator carried onto eight different hulls puts the flare inside
/// the fuselage on the ones it was not authored for, which is invisible in a
/// test that only counts triangles.
#[test]
#[ignore = "needs a disc image"]
fn every_hull_carries_its_own_nozzle_and_plume() {
    let Some(loaded) = load() else {
        return;
    };
    for livery in &loaded.liveries {
        assert!(
            livery.nozzle.is_some(),
            "{}: no Engine Flare locator, so this craft would burn nothing",
            livery.team
        );
        assert!(
            livery.boost.is_some(),
            "{}: no boost plume beside its hull",
            livery.team
        );
    }

    let nozzles: std::collections::BTreeSet<[u32; 3]> = loaded
        .liveries
        .iter()
        .filter_map(|livery| livery.nozzle)
        .map(|at| [at.x.to_bits(), at.y.to_bits(), at.z.to_bits()])
        .collect();
    assert!(
        nozzles.len() > 1,
        "all eight nozzles are at the same point, which is what a single shared \
         locator looks like: {:?}",
        loaded
            .liveries
            .iter()
            .map(|livery| (livery.team.as_str(), livery.nozzle))
            .collect::<Vec<_>>()
    );
}

/// **Zone mode's hull is the one exception the module docs call out**: every
/// team's `Zone.vex` is the same 1213-vertex model with only its paint
/// varying, unlike the race hulls. That raises the question the race-mode
/// test above cannot answer: does `Zone.vex` carry an `Engine Flare` locator
/// at all?
///
/// The stakes are higher than a single craft's flare. `Scene::draw`'s exhaust
/// loop (`crates/raceplay/src/scene/frame.rs`) `break`s out of the whole
/// field the first time a slot's nozzle is `None`, reasoning that a shared
/// hull means a missing locator is missing for every craft - true for Zone,
/// where the hull really is shared. If `Zone.vex` authors no locator, that
/// `break` fires on slot 0 and **no craft's trail draws for the entire
/// race**, which is what a reported "the zone craft draws no exhaust trail"
/// symptom looks like from the outside.
#[test]
#[ignore = "needs a disc image"]
fn the_zone_hull_carries_its_own_nozzle_too() {
    let Some(image) = image() else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Zone,
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the zone race");
    for line in &loaded.report {
        println!("{line}");
    }

    for livery in &loaded.liveries {
        assert!(
            livery.nozzle.is_some(),
            "{}: no Engine Flare locator on Zone.vex, so this craft's exhaust \
             trail would not draw - and because the draw loop breaks on the \
             first missing nozzle, neither would anyone else's",
            livery.team
        );
    }
}

/// The player flies what the player picked, whatever the grid does around them.
#[test]
#[ignore = "needs a disc image"]
fn slot_zero_is_the_team_the_options_asked_for() {
    let Some(image) = image() else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    let teams = oag_raceplay::catalogue::teams(&definition);
    assert!(teams.len() > 1, "the disc declares more than one team");

    // The *last* team, so a pass could not come from it happening to be first.
    let chosen = teams.last().expect("a team").id.clone();
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        team: Some(chosen.clone()),
        ..race::Options::default()
    })
    .expect("loading the race");

    assert_eq!(loaded.liveries[0].team, chosen, "slot 0 is the player's");
    assert!(
        loaded.liveries[1..]
            .iter()
            .all(|livery| livery.team != chosen),
        "the player's own team should not also be flown by an opponent: {:?}",
        loaded
            .liveries
            .iter()
            .map(|livery| livery.team.as_str())
            .collect::<Vec<_>>()
    );
}

/// **The RACE page's Normal/Concept axis, off the real disc.** `--variant
/// extra` draws Assegai's `extra.vex` for the player alone - a different
/// hull from the baseline `Ship.vex`, and different from what every
/// opponent still wears, since nothing offered them a choice of their own.
/// See `oag_title::race::HullVariant`.
#[test]
#[ignore = "needs a disc image"]
fn a_hull_variant_swaps_the_players_own_hull_and_nobody_elses() {
    let Some(image) = image() else {
        return;
    };
    let baseline = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        team: Some("Assegai".to_string()),
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the baseline race");

    let concept = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        team: Some("Assegai".to_string()),
        hull_variant: Some("extra".to_string()),
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the Concept race");

    let triangles =
        |loaded: &race::Loaded, slot: usize| loaded.liveries[slot].hull.indices.len() / 3;
    assert_ne!(
        triangles(&baseline, 0),
        triangles(&concept, 0),
        "extra.vex should decode to a different triangle count from Ship.vex \
         for the player's own slot"
    );
    assert_eq!(
        baseline.liveries[0].team, concept.liveries[0].team,
        "the team's own identity is unaffected by the hull-file override"
    );

    // Slot 1's team was not asked for a variant, so its hull must be
    // unaffected by the player's own pick - the property that says this is
    // a per-slot override and not a global one.
    if baseline.liveries[1].team == concept.liveries[1].team {
        assert_eq!(
            triangles(&baseline, 1),
            triangles(&concept, 1),
            "an opponent's own hull should not change when only the player's \
             variant does"
        );
    }
}

/// **Wipeout HD keeps its locator nodes in a file of their own.** Pulse and
/// the PS2 port put every `Engine Flare` and `Ship Collision Fx` node in the
/// hull's own `.vex`; HD's `Ship.vex` carries the class ids and no nodes of
/// either class, and `Locators.vex` beside it carries them.
///
/// This is the test that turns that into a run rather than a reading. Before
/// the sibling was read, HD's `WO_SHIP_ENGINEFLARE` loaded and then had no
/// nozzle to attach to, so the flare never drew - a missing asset and a
/// missing anchor look identical on screen, and only one of them was true.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn an_hd_hull_takes_its_locators_from_the_file_beside_it() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/hdfury-ps3-eu-dec.iso");
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        println!("skipping: {} not present", path.display());
        return;
    }

    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_hd::TITLE)
        .expect("the archives open");
    let teams = vec!["Detonator".to_string()];
    let mut report = Vec::new();
    let liveries = oag_livery::load(
        &mut archives,
        &teams,
        &oag_livery::LoadContext {
            race: oag_hd::TITLE.race,
            mode: oag_race::Mode::SingleRace,
            flare: oag_hd::TITLE.flare,
            hull_overlay: false,
            hull_shine: false,
            hull_wreck: false,
            absorb_shell: false,
            zone_liveries: &[],
        },
        None,
        None,
        &mut report,
    )
    .expect("the livery loads");
    for line in &report {
        println!("{line}");
    }

    let livery = &liveries[0];
    assert!(
        livery.nozzle.is_some(),
        "no Engine Flare locator, so nothing would anchor WO_SHIP_ENGINEFLARE"
    );
    // Ten, which is the count `Ship_DispatchCollisionFx` picks the nearest of
    // and the same number `oag_physics::wall::HULL_PROBES` samples.
    assert_eq!(
        livery.collision_fx.len(),
        10,
        "the Ship Collision Fx locators did not come through"
    );
    assert!(
        report
            .iter()
            .any(|line| line.contains("Locators.vex") && line.contains("engine_flare locator")),
        "the report does not say the sibling supplied them: {report:?}"
    );
}

/// **The RACE page's other axis: the paint, off the real disc.** `--skin
/// Alternative` repaints Assegai's own `Ship.vex` from
/// `Data\Ships\Assegai\ship_alt.dat` for the player alone - the same geometry,
/// different texels - and leaves every opponent's textures byte-identical.
///
/// # What this asserts that "a texture is bound" would not
///
/// Four separate things, because three of them can pass while the ship on
/// screen is still wrong:
///
/// 1. **The right team's file.** The `.dat`'s own `0x20` header holds the
///    team's *display* name, so parsing it and comparing to the team asked
///    for proves the path resolved to Assegai's skin and not merely to some
///    skin.
/// 2. **The right size.** The file declares no width or height anywhere -
///    the original reads them off the target texture's descriptor, and
///    `mesh::ship_skin::apply` reads them off the block. Those agree only if
///    the hull's own textures really are 128x128 and 64x64, and a mismatch
///    would stretch the paint across the hull rather than fail.
/// 3. **A visible amount of it.** A repaint that changes 0.1% of the texels
///    is a bug that passes every binding check there is. The fraction is
///    printed and asserted substantial, which is the closest a headless test
///    gets to looking at the ship.
/// 4. **Slot 0 alone.** Every opponent's texels stay byte-identical, which is
///    the property that says this is a per-slot override rather than a global
///    one - and it is only expressible because the renderer keys its uploads
///    on a texture's address rather than on its label.
///
/// **Which skin a race flies is this project's choice, not the original's**,
/// and no unlock is checked - see `oag_livery`'s `ship_skin` module.
#[test]
#[ignore = "needs a disc image"]
fn a_skin_repaints_the_players_own_hull_and_nobody_elses() {
    let Some(image) = image() else {
        return;
    };
    let options = |skin: Option<&str>| race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        team: Some("Assegai".to_string()),
        skin: skin.map(str::to_string),
        ..race::Options::default()
    };
    let baseline = race::load(&options(None)).expect("loading the baseline race");
    let painted = race::load(&options(Some("Alternative"))).expect("loading the painted race");

    // 1. The right team's file, proved by the header the parser reads.
    let mut archives =
        oag_pulse::open(&image.display().to_string()).expect("mounting the disc again");
    let blob = archives
        .read_name(r"Data\Ships\Assegai\ship_alt.dat")
        .expect("the skin the definition names");
    let skin = oag_texture::ship_skin::parse(&blob).expect("it parses");
    assert_eq!(
        skin.team_name, "Assegai",
        "the resolved path must be this team's own skin, not some other team's"
    );

    // 2. The right size: the block's dimensions against the hull's own.
    let mut checked = 0;
    for slot in baseline.liveries[0].hull.textures.iter().flatten() {
        let Some(index) = oag_mesh::mesh::ship_skin::slot_of(&slot.label) else {
            continue;
        };
        let block = &skin.blocks[index];
        assert_eq!(
            (slot.width, slot.height),
            (block.width as u32, block.height as u32),
            "{} is {}x{} and its skin block is {}x{} - the paint would be stretched",
            slot.label,
            slot.width,
            slot.height,
            block.width,
            block.height
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "Ship.vex names no texture1.tga..texture4.tga slot, so nothing could be repainted"
    );
    println!("{checked} of Assegai's Ship.vex texture slot(s) take a skin block");

    // 3. A visible amount of the player's own paint actually changed.
    let texels = |loaded: &race::Loaded, slot: usize| -> Vec<(String, Vec<u8>)> {
        loaded.liveries[slot]
            .hull
            .textures
            .iter()
            .flatten()
            .map(|texture| {
                (
                    texture.label.clone(),
                    texture.rgba().map(<[u8]>::to_vec).unwrap_or_default(),
                )
            })
            .collect()
    };
    let (before, after) = (texels(&baseline, 0), texels(&painted, 0));
    assert_eq!(
        before.len(),
        after.len(),
        "the same hull, so the same slots"
    );
    let mut differing = 0u64;
    let mut total = 0u64;
    for ((label, before), (_, after)) in before.iter().zip(&after) {
        if oag_mesh::mesh::ship_skin::slot_of(label).is_none() {
            continue;
        }
        assert_eq!(before.len(), after.len(), "{label}: same size either way");
        differing += before
            .as_chunks::<4>()
            .0
            .iter()
            .zip(after.as_chunks::<4>().0)
            .filter(|(a, b)| a != b)
            .count() as u64;
        total += (before.len() / 4) as u64;
    }
    let fraction = differing as f64 / total.max(1) as f64;
    println!(
        "skin: {differing} of {total} texel(s) differ on the player's own hull ({:.1}%)",
        fraction * 100.0
    );
    assert!(
        fraction > 0.2,
        "only {:.2}% of the player's texels changed - a livery a player would notice \
         repaints far more of the hull than that",
        fraction * 100.0
    );

    // 4. Nobody else's paint moved.
    for slot in 1..baseline.liveries.len() {
        if baseline.liveries[slot].team != painted.liveries[slot].team {
            continue;
        }
        assert_eq!(
            texels(&baseline, slot),
            texels(&painted, slot),
            "slot {slot} ({}) was never offered a paint job and must be untouched",
            baseline.liveries[slot].team
        );
    }
}

/// A Zone race flies a different hull that no `PI_ModelSkin` names a file
/// for, so a skin asked for there is refused out loud rather than painted
/// onto a surface the disc does not specify. See `livery::ship_skin::resolve`.
#[test]
#[ignore = "needs a disc image"]
fn a_zone_race_refuses_a_skin_and_says_so() {
    let Some(image) = image() else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Zone,
        team: Some("Assegai".to_string()),
        skin: Some("Alternative".to_string()),
        ..race::Options::default()
    })
    .expect("loading the Zone race");
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("ignored on a Zone race")),
        "{:?}",
        loaded.report
    );
}
