//! The HD Detonator EMP forks at a split on the circuits that ship one, and each front clears
//! the mines on its own route (`docs/ghidra/functions/ps3-hdfury-eu/detonator-emp.md`, "The
//! fork").

use super::*;
use oag_gameplay::PlayerInputs;
use oag_weapons::detonator::emp::{fire, fork_entry, front_entry, step_wave};
use oag_weapons::detonator::{Placed, Survey};

/// Every HD circuit file whose `WO Track` has a junction with an alternate successor
/// (`Course::branches`), measured on `hdfury-ps3-eu-dec.iso`.
pub(super) const FORKED: [&str; 4] = [
    r"Data\Environments\zone_3\track.vex",
    r"Data\Environments\05_ubermall\track.vex",
    r"Data\Environments\05_ubermall\track_reversed.vex",
    r"Data\Environments\15_anulpha_pass\track.vex",
];

fn start(track: &str) -> Option<Race> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: Mode::Detonator,
        track: Some(track.to_string()),
        ..crate::Options::default()
    })
    .expect("loading an HD Detonator race");
    Some(Race::start(loaded.setup, loaded.view, loaded.modules))
}

fn laid(survey: &Survey, branch_offset: usize, ring: usize) -> [Placed; 2] {
    [
        Placed {
            entry: u16::try_from(
                survey
                    .branch_entry(0, branch_offset)
                    .expect("a branch entry"),
            )
            .expect("fits"),
            lane: 0,
            live: false,
        },
        Placed {
            entry: u16::try_from(ring).expect("fits"),
            lane: 0,
            live: false,
        },
    ]
}

/// **The fork on the disc.** On each circuit with a split: the survey holds the branch beside
/// the ring, it leaves and rejoins where the ring does, and a full EMP fired a few entries
/// before the split clears a mine on the branch and one on the ring past the split, with a
/// second effect and light on the branch's front while it is there.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_emp_wave_forks_at_the_split_and_clears_both_routes() {
    for track in FORKED {
        let Some(mut race) = start(track) else {
            return;
        };
        let detonator = race.core.sim.detonator.as_ref().expect("a Detonator race");
        let survey = detonator.survey.clone();
        let config = detonator.emp.expect("HD authors an EMP");
        let [branch] = survey.branches.as_slice() else {
            panic!("{track}: {} branches", survey.branches.len());
        };
        let total = survey.entries.len();
        let split = usize::from(branch.split);
        let near = |a: &oag_weapons::detonator::Entry, b: &oag_weapons::detonator::Entry| {
            (a.centre - b.centre).length()
        };
        let leave = near(&survey.entries[split], &branch.entries[0]);
        let rejoin = near(
            &survey.entries[usize::from(branch.merge)],
            &branch.entries[branch.entries.len() - 1],
        );
        println!(
            "{track}: {total} ring entries, branch of {} from {split} to {}, leaves {leave:.1} \
             and rejoins {rejoin:.1} units from the ring's",
            branch.entries.len(),
            branch.merge
        );
        assert!(leave < 40.0 && rejoin < 40.0, "{track}");

        let [on_branch, on_ring] = laid(&survey, 30, (split + 30) % total);
        let field = &mut race.core.sim.world.detonator;
        field.mines[0] = on_branch;
        field.mines[1] = on_ring;
        field.live_mines = 2;
        field.emp.charge = config.hi;
        field.emp.gauge = 1.0;
        fire(
            field,
            &survey,
            &config,
            ((split + total - 8) % total) as u16,
        );
        assert!(field.emp.wave.live, "{track}");
        race.view.spu_vertex_lights = true;
        race.set_weapon_lights(true);

        let mut forked_with_effect = false;
        for _ in 0..130 {
            let field = &mut race.core.sim.world.detonator;
            step_wave(field, &survey, &config, 1.0 / 60.0);
            race.view.advance_emp_visual(&race.core);
            let wave = race.core.sim.world.detonator.emp.wave;
            if fork_entry(&wave, &survey).is_some() {
                assert!(front_entry(&wave, &survey).is_some());
                forked_with_effect |=
                    race.view.emp_effect.is_some() && race.view.emp_fork_effect.is_some();
            }
        }
        let field = &race.core.sim.world.detonator;
        assert!(field.emp.wave.forked, "{track}: the wave forked");
        assert_eq!(field.live_mines, 0, "{track}: both routes' mines went");
        assert!(forked_with_effect, "{track}: two effects while forked");
        assert!(
            race.view.emp_fork_effect.is_none() && race.view.emp_effect.is_none(),
            "{track}: both released with the wave"
        );
    }
}

/// A circuit with no split never forks: its survey has no branch and a wave fired anywhere
/// ends unforked.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_circuit_without_a_split_never_forks_its_wave() {
    for track in [
        r"Data\Environments\talons_junction\track.vex",
        r"Data\Environments\15_anulpha_pass\track_reversed.vex",
    ] {
        let Some(mut race) = start(track) else {
            return;
        };
        let detonator = race.core.sim.detonator.as_ref().expect("a Detonator race");
        let survey = detonator.survey.clone();
        let config = detonator.emp.expect("HD authors an EMP");
        assert!(survey.branches.is_empty(), "{track}");
        let field = &mut race.core.sim.world.detonator;
        field.emp.charge = config.hi;
        field.emp.gauge = 1.0;
        fire(field, &survey, &config, 0);
        for _ in 0..130 {
            step_wave(field, &survey, &config, 1.0 / 60.0);
        }
        assert!(!field.emp.wave.forked && !field.emp.wave.live, "{track}");
    }
}

/// Slot 0 held on `entry` past the countdown, charged full, with the mines given and nothing else
/// laid, then L1 pressed and `ticks` run, the craft kept on the spot.
fn fire_from(race: &mut Race, entry: usize, mines: &[Placed], ticks: usize) {
    while oag_race::RaceState::thrust_gated(race.core.sim.world.tick) {
        race.tick(&PlayerInputs::none());
    }
    let at = {
        let survey = &race.core.sim.detonator.as_ref().expect("Detonator").survey;
        survey.entry(entry).expect("an entry").centre
    };
    let hold = |race: &mut Race| {
        let ship = &mut race.core.sim.world.ships[0];
        ship.physics.body.position = at;
        ship.physics.body.linear_velocity = oag_core::math::Vec3::ZERO;
        ship.physics.body.angular_velocity = oag_core::math::Vec3::ZERO;
        ship.physics.shield = ship.handling.dimensions.shield;
    };
    let config = race
        .core
        .sim
        .detonator
        .as_ref()
        .expect("Detonator")
        .emp
        .expect("an EMP");
    let field = &mut race.core.sim.world.detonator;
    field.live_bombs = 0;
    field.live_mines = u16::try_from(mines.len()).expect("fits");
    field.mines[..mines.len()].copy_from_slice(mines);
    field.emp.charge = config.hi;
    field.emp.gauge = 1.0;
    hold(race);
    let mut snapshot = oag_gameplay::InputSnapshot::new();
    snapshot
        .buttons
        .begin_frame(oag_gameplay::input::Button::Flip.bit());
    race.tick(&PlayerInputs::single(snapshot));
    for _ in 0..ticks {
        hold(race);
        race.tick(&PlayerInputs::none());
    }
}

fn mine_at(entry: usize) -> Placed {
    Placed {
        entry: u16::try_from(entry).expect("fits"),
        lane: 0,
        live: false,
    }
}

/// A branch entry well clear of the ring (out of the launch fork's reach, and the road itself
/// apart), with 60 more entries of branch ahead.
fn clear_of_the_ring(survey: &Survey) -> usize {
    let branch = &survey.branches[0];
    (0..branch.entries.len() - 60)
        .find(|&k| {
            let at = branch.entries[k].centre;
            branch.to_primary[k].is_none()
                && survey
                    .entries
                    .iter()
                    .all(|r| (r.centre - at).length() > 20.0)
        })
        .expect("the routes part")
}

/// **A launch on a branch, on the disc.** On each forked circuit, L1 from a branch section past
/// the overlap starts the wave on the branch and clears the mine laid ahead on it, and leaves the
/// ring's.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_emp_fired_on_a_branch_starts_there_and_clears_its_mines() {
    for track in FORKED {
        let Some(mut race) = start(track) else {
            return;
        };
        let survey = race
            .core
            .sim
            .detonator
            .as_ref()
            .expect("Detonator")
            .survey
            .clone();
        let k = clear_of_the_ring(&survey);
        let from = survey.branch_entry(0, k).expect("a branch entry");
        let ahead = survey.branch_entry(0, k + 40).expect("a branch entry");
        let ring_beside = (usize::from(survey.branches[0].split) + k + 40) % survey.entries.len();
        fire_from(
            &mut race,
            from,
            &[mine_at(ahead), mine_at(ring_beside)],
            130,
        );
        let field = &race.core.sim.world.detonator;
        assert_eq!(usize::from(field.emp.wave.from), from, "{track}");
        assert!(!field.emp.wave.forked, "{track}");
        assert_eq!(
            field
                .live_mines()
                .iter()
                .map(|m| usize::from(m.entry))
                .collect::<Vec<_>>(),
            [ring_beside],
            "{track}: the branch's mine goes, the ring's stays"
        );
    }
}

/// **The launch-time fork on the disc.** L1 on either route a few sections past the split, where
/// the two still lie within reach, starts a second front on the other and clears both.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_emp_fired_beside_the_other_route_forks_at_launch() {
    for track in FORKED {
        for on_branch in [false, true] {
            let Some(mut race) = start(track) else {
                return;
            };
            let survey = race
                .core
                .sim
                .detonator
                .as_ref()
                .expect("Detonator")
                .survey
                .clone();
            let split = usize::from(survey.branches[0].split);
            let total = survey.entries.len();
            let route = |offset: usize, branch: bool| {
                if branch {
                    survey.branch_entry(0, offset).expect("a branch entry")
                } else {
                    (split + offset) % total
                }
            };
            let from = route(5, on_branch);
            let other = route(15, !on_branch);
            fire_from(&mut race, from, &[mine_at(other)], 0);
            let field = &race.core.sim.world.detonator;
            assert!(
                field.emp.wave.forked,
                "{track} on_branch={on_branch}: forked at launch"
            );
            fire_ticks(&mut race, 40);
            assert!(
                race.core.sim.world.detonator.live_mines().is_empty(),
                "{track} on_branch={on_branch}: the other route's mine goes"
            );
        }
    }
}

fn fire_ticks(race: &mut Race, ticks: usize) {
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
    }
}

/// **A ring launch away from any split is unchanged**: the wave starts at the ring's nearest
/// entry, unforked, and takes the ring's mines while the branch's stay. On a circuit with no
/// split there is nothing else to take.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_ring_launch_away_from_a_split_is_unchanged() {
    for track in [
        r"Data\Environments\talons_junction\track.vex",
        r"Data\Environments\zone_3\track.vex",
        r"Data\Environments\05_ubermall\track.vex",
    ] {
        let Some(mut race) = start(track) else {
            return;
        };
        let survey = race
            .core
            .sim
            .detonator
            .as_ref()
            .expect("Detonator")
            .survey
            .clone();
        let total = survey.entries.len();
        // Past every split's primary route, or mid-ring with none.
        let from = survey.branches.first().map_or(300, |b| {
            let primary = b.from_primary.len();
            (usize::from(b.split) + primary + 100) % total
        });
        let mut mines = vec![mine_at((from + 40) % total)];
        if !survey.branches.is_empty() {
            mines.push(mine_at(survey.branch_entry(0, 50).expect("a branch entry")));
        }
        fire_from(&mut race, from, &mines, 130);
        let field = &race.core.sim.world.detonator;
        let wave = field.emp.wave;
        assert!(!wave.forked, "{track}");
        assert!(
            usize::from(wave.from).abs_diff(from) <= 1,
            "{track}: from {}",
            wave.from
        );
        assert_eq!(wave.fork, oag_weapons::detonator::emp::Fork::default());
        assert_eq!(
            field.live_mines,
            u16::from(!survey.branches.is_empty()),
            "{track}"
        );
    }
}

/// **The merge end.** On `zone_3` a ring launch near the end of the primary route lies within
/// reach of the branch on its far half: the second front is born travelling backward, which the
/// EMP never advances, so it holds the wave's one fork and the split nine entries on does not
/// fork it again.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_emp_fired_beside_the_merge_never_forks_again() {
    let Some(mut race) = start(r"Data\Environments\zone_3\track.vex") else {
        return;
    };
    let survey = race
        .core
        .sim
        .detonator
        .as_ref()
        .expect("Detonator")
        .survey
        .clone();
    let total = survey.entries.len();
    let from = total - 3;
    let ahead = survey.branch_entry(0, 30).expect("a branch entry");
    fire_from(&mut race, from, &[mine_at(ahead)], 130);
    let field = &race.core.sim.world.detonator;
    assert!(field.emp.wave.forked);
    assert_eq!(
        field.emp.wave.fork,
        oag_weapons::detonator::emp::Fork::INERT
    );
    assert_eq!(field.live_mines, 1, "the branch is left alone");
}

/// **Omega runs the same law** (its sweep `0x0133c600` carries the same launch fork): on its
/// re-shipped forked circuits a branch launch clears the branch's mine and one beside the split
/// forks at once.
#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn an_omega_emp_fired_on_or_beside_a_branch_follows_it() {
    let Some(image) = oag_testdata::image("omega-ps4-eu.pkg") else {
        return;
    };
    for track in [
        r"Data\Environments\05_ubermall\track.vex",
        r"Data\Environments\15_anulpha_pass\track.vex",
    ] {
        let load = || {
            let loaded = crate::load(&crate::Options {
                source: image.display().to_string(),
                class: "VENOM".to_string(),
                mode: Mode::Detonator,
                track: Some(track.to_string()),
                no_effects: true,
                ..crate::Options::default()
            })
            .expect("loading an Omega Detonator race");
            Race::start(loaded.setup, loaded.view, loaded.modules)
        };
        let mut race = load();
        let survey = race
            .core
            .sim
            .detonator
            .as_ref()
            .expect("Detonator")
            .survey
            .clone();
        let k = clear_of_the_ring(&survey);
        let from = survey.branch_entry(0, k).expect("a branch entry");
        let ahead = survey.branch_entry(0, k + 40).expect("a branch entry");
        fire_from(&mut race, from, &[mine_at(ahead)], 130);
        let field = &race.core.sim.world.detonator;
        assert_eq!(usize::from(field.emp.wave.from), from, "{track}");
        assert!(
            field.live_mines().is_empty(),
            "{track}: the branch's mine goes"
        );

        let mut race = load();
        let split = usize::from(survey.branches[0].split);
        let beside = (split + 5) % survey.entries.len();
        let other = survey.branch_entry(0, 15).expect("a branch entry");
        fire_from(&mut race, beside, &[mine_at(other)], 0);
        assert!(race.core.sim.world.detonator.emp.wave.forked, "{track}");
        fire_ticks(&mut race, 40);
        assert!(
            race.core.sim.world.detonator.live_mines().is_empty(),
            "{track}"
        );
    }
}
