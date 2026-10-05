use super::*;
use crate::tests::race_with_a_grid;
use oag_physics::maglock::ramp;

const SLOT: usize = 1;

/// A grid whose title builds the wake, every hull anchored at its origin.
fn wake_race() -> Race {
    let mut race = race_with_a_grid();
    race.view.magstrip_wake = Some(Wakes::new([Some(Mat4::IDENTITY); MAX_SHIPS]));
    // Mid-track: the fixture's grid sits past the end of its 70-unit straight,
    // where every arc lands far astern and is shed at once.
    race.sim.world.ships[SLOT].physics.body.position = Vec3::new(30.0, 6.0, 0.0);
    race
}

/// Feeds `SLOT` one probe result a tick through the blend's own ramp, the way
/// the simulation does, and returns each tick's `(starts, stops)` for `SLOT`.
fn run(race: &mut Race, probes: &[bool]) -> Vec<(usize, usize)> {
    let mut blend = race.sim.world.ships[SLOT].physics.mag_lock_blend;
    let mut out = Vec::new();
    for &hit in probes {
        blend = ramp(blend, hit);
        race.sim.world.ships[SLOT].physics.mag_lock_blend = blend;
        race.advance_magstrip_wake();
        let cues = race.drain_cues();
        let count = |cue| {
            cues.iter()
                .filter(|e| e.cue == cue && usize::from(e.slot) == SLOT)
                .count()
        };
        out.push((count(Cue::Magstrip), count(Cue::MagstripStop)));
    }
    out
}

#[test]
fn the_arming_law_starts_once_and_stops_once_per_visit() {
    let mut armed = true;
    let mut was_over = false;
    let mut edges = Vec::new();
    for over in [false, true, true, true, false, false, true, true, false] {
        let (edge, next) = sound_edge(armed, was_over, over);
        armed = next;
        was_over = over;
        edges.push(edge);
    }
    use SoundEdge::{Start, Stop};
    assert_eq!(
        edges,
        [
            None,
            Some(Start),
            None,
            None,
            Some(Stop),
            None,
            Some(Start),
            None,
            Some(Stop)
        ]
    );
}

#[test]
fn a_disarmed_craft_does_not_restart_until_it_has_left() {
    let (edge, armed) = sound_edge(false, true, true);
    assert_eq!((edge, armed), (None, false), "still over, still disarmed");
    let (edge, armed) = sound_edge(false, true, false);
    assert_eq!(
        (edge, armed),
        (Some(SoundEdge::Stop), true),
        "leaving re-arms"
    );
}

#[test]
fn the_cue_edges_follow_the_instantaneous_contact_not_the_lingering_blend() {
    let mut race = wake_race();
    // On for four ticks, off for eight (the blend takes five to drain), on again.
    let mut probes = vec![true; 4];
    probes.extend([false; 8]);
    probes.extend([true; 3]);
    let seen = run(&mut race, &probes);
    let starts: Vec<usize> = (0..seen.len()).filter(|&t| seen[t].0 == 1).collect();
    let stops: Vec<usize> = (0..seen.len()).filter(|&t| seen[t].1 == 1).collect();
    assert_eq!(starts, [0, 12], "one start per visit, on the rising tick");
    assert_eq!(
        stops,
        [4],
        "the stop is the first tick the probe misses, not the blend reaching zero"
    );
    assert_eq!(seen.iter().map(|s| s.0 + s.1).sum::<usize>(), 3);
}

#[test]
fn a_title_without_the_class_raises_nothing_and_draws_nothing() {
    let mut race = race_with_a_grid();
    assert!(!race.has_magstrip_wake());
    let seen = run(&mut race, &[true, true, false]);
    assert!(seen.iter().all(|&s| s == (0, 0)), "{seen:?}");
    let (atlas, contact) = race.magstrip_wake_vertices();
    assert!(atlas.is_empty() && contact.is_empty());
}

#[test]
fn arcs_are_drawn_while_over_a_strip_and_gone_after() {
    let mut race = wake_race();
    run(&mut race, &[true; 20]);
    assert!(race.over_magstrip(SLOT));
    assert!(
        race.magstrip_wake_live(SLOT) > 0,
        "no arcs after 20 ticks over a strip"
    );
    let (atlas, contact) = race.magstrip_wake_vertices();
    assert!(!atlas.is_empty() && !contact.is_empty());
    assert_eq!(atlas.len() % 36, 0, "six quads of six vertices an arc");
    assert_eq!(contact.len() % 6, 0);

    run(&mut race, &[false; 120]);
    assert!(!race.over_magstrip(SLOT));
    assert_eq!(
        race.magstrip_wake_live(SLOT),
        0,
        "arcs age out within 1.1 s"
    );
    let (atlas, contact) = race.magstrip_wake_vertices();
    assert!(atlas.is_empty() && contact.is_empty());
}

#[test]
fn a_hull_with_no_anchor_draws_no_wake_but_still_hums() {
    let mut race = race_with_a_grid();
    let mut anchors = [Some(Mat4::IDENTITY); MAX_SHIPS];
    anchors[SLOT] = None;
    race.view.magstrip_wake = Some(Wakes::new(anchors));
    let seen = run(&mut race, &[true; 10]);
    assert_eq!(race.magstrip_wake_live(SLOT), 0);
    assert_eq!(seen[0], (1, 0), "the sound does not need the locator");
}

/// A `.pob` title's grid with `names` loaded as one-emitter stand-ins.
fn pob_race(names: &[&str]) -> Race {
    let mut race = race_with_a_grid();
    race.view.magstrip_wake = Some(Wakes::pob([Some(Mat4::IDENTITY); MAX_SHIPS]));
    for name in names {
        let blob = crate::tests::respawn::one_emitter_pob(name, oag_pob::flags::LOOPING);
        let effect = oag_fx::psys::Effect::parse(&blob, oag_fx::psys::ColourScale::Full)
            .expect("the hand-laid effect parses");
        race.view.effects.insert(name, effect);
    }
    race.sim.world.ships[SLOT].physics.body.position = Vec3::new(30.0, 6.0, 0.0);
    race
}

const BOTH: [&str; 2] = [
    crate::effect_names::MAGSTRIP_SPARKS_EFFECT,
    crate::effect_names::MAGSTRIP_ZONE_EFFECT,
];

#[test]
fn a_pob_title_plays_the_effect_only_while_over_a_strip_and_draws_no_arc() {
    let mut race = pob_race(&BOTH);
    assert!(!race.has_magstrip_wake(), "no arc wake on a .pob title");
    assert!(race.magstrip_pob_of(SLOT).is_none());
    run(&mut race, &[true; 10]);
    assert!(
        race.magstrip_pob_of(SLOT).is_some(),
        "no effect over the strip"
    );
    assert!(
        race.magstrip_pob_of(0).is_none(),
        "a craft off the strip plays none"
    );
    assert_eq!(race.magstrip_wake_live(SLOT), 0, "and no arcs");
    let (atlas, contact) = race.magstrip_wake_vertices();
    assert!(atlas.is_empty() && contact.is_empty());

    run(&mut race, &[false; 3]);
    assert!(race.magstrip_pob_of(SLOT).is_none(), "released on leaving");
}

#[test]
fn a_pob_title_still_raises_the_sound_edges() {
    let mut race = pob_race(&BOTH);
    let seen = run(&mut race, &[true, true, false]);
    assert_eq!(seen[0], (1, 0));
    assert_eq!(seen[2], (0, 1));
}

#[test]
fn a_pob_title_without_the_effect_plays_nothing() {
    let mut race = pob_race(&[]);
    run(&mut race, &[true; 10]);
    assert!(race.magstrip_pob_of(SLOT).is_none());
}

/// Zone picks `WO_MAGSTRIP_ZONE` and every other mode `WO_MAGSTRIP_SPARKS`;
/// told apart by loading only the one.
#[test]
fn zone_picks_the_zone_effect_and_other_modes_the_sparks_one() {
    for (mode, only, plays) in [
        (
            oag_race::Mode::Zone,
            crate::effect_names::MAGSTRIP_ZONE_EFFECT,
            true,
        ),
        (
            oag_race::Mode::Zone,
            crate::effect_names::MAGSTRIP_SPARKS_EFFECT,
            false,
        ),
        (
            oag_race::Mode::SingleRace,
            crate::effect_names::MAGSTRIP_SPARKS_EFFECT,
            true,
        ),
        (
            oag_race::Mode::SingleRace,
            crate::effect_names::MAGSTRIP_ZONE_EFFECT,
            false,
        ),
    ] {
        let mut race = pob_race(&[only]);
        for state in &mut race.sim.world.race {
            state.mode = mode;
        }
        run(&mut race, &[true; 4]);
        assert_eq!(
            race.magstrip_pob_of(SLOT).is_some(),
            plays,
            "{mode:?} with only {only} loaded"
        );
    }
}
