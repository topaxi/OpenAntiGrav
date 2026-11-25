//! The HD Detonator EMP forks at a split on the circuits that ship one, and each front clears
//! the mines on its own route (`docs/ghidra/functions/ps3-hdfury-eu/detonator-emp.md`, "The
//! fork").

use super::*;
use oag_weapons::detonator::emp::{fire, fork_entry, front_entry, step_wave};
use oag_weapons::detonator::{Placed, Survey};

/// Every HD circuit file whose `WO Track` has a junction with an alternate successor
/// (`Course::branches`), measured on `hdfury-ps3-eu-dec.iso`.
const FORKED: [&str; 4] = [
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
            ((split + total - 10) % total) as u16,
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
