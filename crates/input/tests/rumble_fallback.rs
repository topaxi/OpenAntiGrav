//! Our own rumble patterns (`assets/fallback/Data/XML/Rumble/`) read, and cover
//! every pattern the event map names.

use oag_input::rumble::map::{ROWS, UNFIRED, pattern_names};
use oag_tables::rumble::Library;

const DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/fallback/Data/XML/Rumble"
);

fn library() -> Library {
    let mut library = Library::new();
    for entry in std::fs::read_dir(DIR).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        let pattern =
            oag_mod::rumble::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
        library.insert(&name, pattern);
    }
    library
}

#[test]
fn every_file_reads() {
    assert!(library().len() >= 20);
}

#[test]
fn every_row_of_the_map_has_a_pattern() {
    let library = library();
    for name in pattern_names() {
        assert!(library.index_of(name).is_some(), "{name}");
    }
}

#[test]
fn no_pattern_is_authored_for_a_name_nothing_fires() {
    let library = library();
    for (name, _) in UNFIRED {
        assert!(library.index_of(name).is_none(), "{name}");
    }
}

#[test]
fn the_large_motor_carries_each_pattern() {
    for (name, p) in library().iter() {
        let peak = p.large.iter().copied().max().unwrap_or(0);
        assert!(peak >= 90, "{name} peaks at {peak}");
        assert!(p.large.len() <= 64, "{name}");
    }
}

mod feed {
    use oag_gameplay::cue::CueEvent;
    use oag_input::rumble::feed::{Feed, Levels, Rules};
    use oag_input::rumble::map::{Scope, Set, Trigger};
    use oag_input::rumble::{MotorOutput, Motors, SmallMotor, StepContext};

    use super::{ROWS, library};

    #[derive(Default)]
    struct Recorder(Vec<Motors>);

    impl MotorOutput for Recorder {
        fn set(&mut self, _seat: usize, motors: Motors) {
            self.0.push(motors);
        }
        fn silence(&mut self) {}
    }

    const RACE: StepContext = StepContext {
        race: true,
        paused: false,
        listener: [0.0; 3],
    };

    fn feed() -> Feed {
        let rules = Rules {
            set: Set::Ps4,
            drop_position: false,
        };
        Feed::new(library(), SmallMotor::Sum, rules)
    }

    fn peak_after(feed: &mut Feed, ticks: usize) -> u8 {
        let mut out = Recorder::default();
        for _ in 0..ticks {
            feed.drive(RACE, 0, &mut out);
        }
        out.0.iter().map(|m| m.large).max().unwrap_or(0)
    }

    /// One test per event family: every cue row plays through the large motor
    /// from our own patterns alone.
    #[test]
    fn every_cue_row_rumbles_the_large_motor() {
        for row in ROWS {
            let Trigger::Cue(cue) = row.trigger else {
                continue;
            };
            let mut feed = feed();
            let event = match row.scope {
                Scope::OwnShip => CueEvent {
                    cue,
                    slot: 0,
                    point: None,
                },
                Scope::World => CueEvent {
                    cue,
                    slot: 0,
                    point: Some(oag_core::math::Vec3::ZERO),
                },
            };
            feed.event(&event, 0);
            assert!(peak_after(&mut feed, 60) > 0, "{}", row.pattern);
        }
    }

    #[test]
    fn a_rival_s_weapon_reaches_no_motor() {
        let mut feed = feed();
        feed.event(
            &CueEvent {
                cue: oag_gameplay::cue::Cue::Rocket,
                slot: 3,
                point: None,
            },
            0,
        );
        assert_eq!(peak_after(&mut feed, 30), 0);
    }

    /// The rising edge of the wrong-way warning and of the wreck each play once.
    #[test]
    fn the_wrong_way_and_the_wreck_edges_rumble() {
        for (name, up) in [
            (
                "wrong way",
                Levels {
                    wrong_way: true,
                    ..Levels::default()
                },
            ),
            (
                "wreck",
                Levels {
                    exploding: true,
                    ..Levels::default()
                },
            ),
        ] {
            let mut feed = feed();
            feed.levels(up);
            assert!(peak_after(&mut feed, 60) > 0, "{name}");
        }
    }

    #[test]
    fn the_held_levels_rumble_while_up_and_fall_silent_after() {
        for (name, up) in [
            (
                "mag strip",
                Levels {
                    mag_strip: true,
                    ..Levels::default()
                },
            ),
            (
                "autopilot",
                Levels {
                    autopilot: true,
                    ..Levels::default()
                },
            ),
            (
                "leech beam",
                Levels {
                    leech_beam: true,
                    ..Levels::default()
                },
            ),
        ] {
            let mut feed = feed();
            let mut out = Recorder::default();
            for _ in 0..120 {
                feed.levels(up);
                feed.drive(RACE, 0, &mut out);
            }
            assert!(out.0.last().unwrap().large > 0, "{name} still held");
            let mut after = Recorder::default();
            for _ in 0..120 {
                feed.levels(Levels::default());
                feed.drive(RACE, 0, &mut after);
            }
            assert_eq!(after.0.last().unwrap().large, 0, "{name} released");
        }
    }

    /// Every size of wall contact, on either side, rumbles from our own patterns
    /// while it lasts and falls silent after it.
    #[test]
    fn every_wall_band_rumbles_and_lets_go() {
        for (m, side) in [
            (70.0, 1.0),
            (40.0, 1.0),
            (20.0, 1.0),
            (5.0, 1.0),
            (40.0, -1.0),
            (20.0, -1.0),
            (5.0, -1.0),
        ] {
            let mut feed = feed();
            let mut out = Recorder::default();
            for _ in 0..30 {
                feed.wall(Some((m, side)), 1.0 / 60.0);
                feed.drive(RACE, 0, &mut out);
            }
            assert!(out.0.last().unwrap().large > 0, "contact {m} on {side}");
            for _ in 0..30 {
                feed.wall(None, 1.0 / 60.0);
                feed.drive(RACE, 0, &mut out);
            }
            assert_eq!(out.0.last().unwrap().large, 0, "released {m} on {side}");
        }
    }

    #[test]
    fn every_landing_band_rumbles() {
        for v in [8.0, 5.5, 4.0, 3.2] {
            let mut feed = feed();
            feed.touchdown(v, 1.0 / 60.0);
            assert!(peak_after(&mut feed, 40) > 0, "landing at {v}");
        }
    }
}
