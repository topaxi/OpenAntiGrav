use oag_tables::rumble::{Class, Kind, Library, Pattern, Priority};

use super::*;
use crate::rumble::SmallMotor;

const DT: f32 = 1.0 / 60.0;
fn library() -> Library {
    let mut library = Library::new();
    for (i, name) in WALL.iter().enumerate() {
        library.insert(
            name,
            Pattern {
                kind: Kind::Looping,
                loop_start: 0,
                loop_end: 2,
                class: Some(Class::Game),
                priority: Some(Priority::High),
                positional: None,
                large: vec![10 * (i as u8 + 1); 4],
                small: vec![],
            },
        );
    }
    for (i, name) in LANDING.iter().enumerate() {
        library.insert(
            name,
            Pattern {
                kind: Kind::OneShot,
                loop_start: 0,
                loop_end: 0,
                class: Some(Class::Game),
                priority: Some(Priority::High),
                positional: None,
                large: vec![100 + i as u8; 3],
                small: vec![],
            },
        );
    }
    library
}

fn rig() -> (Selector, Player) {
    let library = library();
    (
        Selector::new(&library),
        Player::new(library, SmallMotor::OnOff),
    )
}

#[test]
fn contact_bands_pick_the_pattern_by_size_and_side() {
    assert_eq!(wall_band(60.0, 1.0), Some(0));
    assert_eq!(wall_band(-60.0, -1.0), Some(0), "heavy has no side");
    assert_eq!(wall_band(40.0, 1.0), Some(1));
    assert_eq!(wall_band(40.0, -1.0), Some(4));
    assert_eq!(wall_band(15.0, 1.0), Some(2));
    assert_eq!(wall_band(15.0, -1.0), Some(5));
    assert_eq!(wall_band(5.0, 1.0), Some(3));
    assert_eq!(wall_band(5.0, -1.0), Some(6));
    assert_eq!(wall_band(2.0, 1.0), None);
}

#[test]
fn landing_bands_pick_the_pattern_by_speed() {
    let bands: Vec<usize> = [7.0, 5.0, 4.0, 3.2, 3.0].map(landing_band).to_vec();
    assert_eq!(bands, [0, 1, 2, 3, 4]);
}

#[test]
fn a_held_contact_is_replaced_only_when_its_band_changes_and_stops_after_1_5_s() {
    let (mut sel, mut player) = rig();
    let mut starts = 0;
    for _ in 0..60 {
        starts += usize::from(sel.contact(&mut player, Some((15.0, 1.0)), DT).is_some());
    }
    assert_eq!(starts, 1, "the band held, one voice");
    assert_eq!(player.active_voices(), 1);
    assert!(sel.contact(&mut player, Some((40.0, 1.0)), DT).is_some());
    assert_eq!(player.active_voices(), 1, "replaced, not stacked");
    for _ in 0..60 {
        sel.contact(&mut player, Some((40.0, 1.0)), DT);
    }
    assert_eq!(player.active_voices(), 0, "past 1.5 s of contact");
}

#[test]
fn losing_contact_stops_the_voice_after_a_tenth_of_a_second() {
    let (mut sel, mut player) = rig();
    sel.contact(&mut player, Some((15.0, 1.0)), DT);
    for _ in 0..5 {
        sel.contact(&mut player, None, DT);
    }
    assert_eq!(player.active_voices(), 1, "still within 0.1 s");
    sel.contact(&mut player, None, DT);
    sel.contact(&mut player, None, DT);
    assert_eq!(player.active_voices(), 0);
}

#[test]
fn a_new_contact_after_a_gap_starts_a_fresh_voice() {
    let (mut sel, mut player) = rig();
    sel.contact(&mut player, Some((15.0, 1.0)), DT);
    for _ in 0..30 {
        sel.contact(&mut player, None, DT);
    }
    assert!(sel.contact(&mut player, Some((15.0, 1.0)), DT).is_some());
}

#[test]
fn only_a_stronger_landing_plays_until_the_one_second_rearm() {
    let (mut sel, mut player) = rig();
    assert_eq!(sel.landing(&mut player, 4.0, DT), Some(2));
    assert_eq!(sel.landing(&mut player, 3.2, DT), None, "weaker");
    assert_eq!(sel.landing(&mut player, 4.0, DT), None, "equal");
    assert_eq!(sel.landing(&mut player, 7.0, DT), Some(0), "stronger");
    for _ in 0..61 {
        sel.landing(&mut player, 0.0, DT);
    }
    assert_eq!(sel.landing(&mut player, 3.2, DT), Some(3), "re-armed");
}
