//! Simulation mods in a race ([`crate::extensions`]): a vanilla race hashes the same
//! with the `extensions` feature on and off, a modded race is deterministic, and a
//! module that runs out of fuel is turned off at the same tick every time.
//!
//! The first test runs in both builds: `cargo nextest run -p oag-raceplay` and the
//! same with `--features extensions` must both pass it against one constant.

use super::*;
use oag_gameplay::PlayerInputs;

/// The fixture grid, with an engine on slot 1 so the race has something to scale.
fn grid(extensions: crate::extensions::Prepared) -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.extensions = extensions;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    let mut race = Race::start(setup);
    let engine = &mut race.sim.world.ships[1].handling.engine;
    engine.amount = 20.0;
    engine.accelcap = 1000.0;
    race
}

/// Every tick's state hash over `ticks` ticks, folded into one, and the hashes.
fn run(mut race: Race, ticks: u32) -> (Race, Vec<u64>) {
    let hashes = (0..ticks)
        .map(|_| {
            race.tick(&PlayerInputs::none());
            race.sim.state_hash()
        })
        .collect();
    (race, hashes)
}

fn fold(hashes: &[u64]) -> u64 {
    let mut hasher = oag_core::hash::StateHasher::new();
    for hash in hashes {
        hasher.write_u64(*hash);
    }
    hasher.finish()
}

/// The fixture race's 600 tick hashes, folded. Recorded from the build **without**
/// the `extensions` feature on 2026-10-29; the build with it must reproduce it.
const VANILLA_FOLD: u64 = 0x0898_d286_7629_23f9;

#[test]
fn a_vanilla_race_hashes_the_same_with_the_feature_on_or_off_and_builds_no_engine() {
    let (race, hashes) = run(grid(crate::extensions::Prepared::default()), 600);
    assert!(!race.is_modded());
    assert_eq!(fold(&hashes), VANILLA_FOLD, "{:#018x}", fold(&hashes));
    #[cfg(feature = "extensions")]
    assert_eq!(
        oag_modhost::engines_created(),
        0,
        "a vanilla race built a wasm engine"
    );
}

#[cfg(feature = "extensions")]
mod on {
    use std::sync::Arc;

    use super::*;
    use crate::extensions::{Prepared, Source, prepare};

    const STATE: i32 = 1024 + oag_ext_abi::STATE_AT as i32;
    const REPLY: i32 = 1024 + oag_ext_abi::REPLY_AT as i32;
    const POOL: i32 = 1024 + oag_ext_abi::POOL_AT as i32;

    fn module(id: &str, body: &str) -> Source {
        let wasm = wat::parse_str(format!(
            r#"(module
                (memory (export "memory") 1)
                (func (export "oag_abi") (result i32) i32.const {word})
                (func (export "oag_io") (result i32) i32.const 1024)
                (func (export "tick") (result i32) {body}))"#,
            word = oag_ext_abi::ABI_WORD,
        ))
        .unwrap();
        Source {
            mod_id: id.to_string(),
            module: "test.wasm".to_string(),
            titles: Vec::new(),
            api: 1,
            memory_pages: 1,
            state_bytes: 8,
            scope: crate::extensions::Scope::Rules,
            read: Arc::new(move || Ok(wasm.clone())),
            strings: Arc::new(|_| Default::default()),
            icons: Vec::new(),
            effects: Vec::new(),
            files: Arc::new(|_| Err("no files".into())),
        }
    }

    /// Asks for slot 1's thrust at `factor` every tick.
    fn scaler(id: &str, factor: f32) -> Source {
        module(
            id,
            &format!(
                "(i32.store (i32.const {REPLY}) (i32.const 1))
                 (i32.store8 (i32.const {cmd}) (i32.const 1))
                 (i32.store8 (i32.const {slot}) (i32.const 1))
                 (f32.store (i32.const {f}) (f32.const {factor}))
                 i32.const 0",
                cmd = REPLY + 4,
                slot = REPLY + 5,
                f = REPLY + 8,
            ),
        )
    }

    #[test]
    fn a_factor_above_one_makes_a_craft_faster_and_one_changes_nothing() {
        let speed = |race: &Race| {
            race.sim.world.ships[1]
                .physics
                .body
                .linear_velocity
                .length()
        };
        let (vanilla, plain) = run(grid(Prepared::default()), 600);
        let (boosted, _) = run(grid(prepared(&[scaler("test.boost", 1.5)])), 600);
        assert!(
            speed(&boosted) > speed(&vanilla),
            "{} against {}",
            speed(&boosted),
            speed(&vanilla)
        );
        // A factor of exactly 1.0 is no command at all for the craft: its whole
        // trajectory is the vanilla one (the race hash differs only by the module's
        // own state).
        let (neutral, _) = run(grid(prepared(&[scaler("test.neutral", 1.0)])), 600);
        assert_eq!(neutral.sim.world.ships, vanilla.sim.world.ships);
        assert_eq!(fold(&plain), VANILLA_FOLD);
    }

    const CRAFTS: i32 = 1024 + oag_ext_abi::CRAFTS_AT as i32;

    /// On its first call takes slot 1's whole pool (`-shield_max`, read from its craft
    /// record), then nothing.
    fn drainer() -> Source {
        module(
            "test.drainer",
            &format!(
                "(if (i32.eqz (i32.load (i32.const {STATE})))
                   (then
                     (i32.store (i32.const {STATE}) (i32.const 1))
                     (i32.store (i32.const {REPLY}) (i32.const 1))
                     (i32.store8 (i32.const {cmd}) (i32.const 2))
                     (i32.store8 (i32.const {slot}) (i32.const 1))
                     (f32.store (i32.const {f}) (f32.neg (f32.load (i32.const {max}))))))
                 i32.const 0",
                cmd = REPLY + 4,
                slot = REPLY + 5,
                f = REPLY + 8,
                max = CRAFTS + 16 + 8,
            ),
        )
    }

    #[test]
    fn a_shield_delta_that_empties_the_pool_wrecks_the_craft_as_a_hit_does() {
        let mut race = grid(prepared(&[drainer()]));
        // The fixture's hulls carry no pool; give slot 1 a full one.
        race.sim.world.ships[1].handling.dimensions.shield = 100.0;
        race.sim.world.ships[1].physics.shield = 100.0;
        assert_eq!(
            race.sim.world.ships[1].physics.craft_state,
            oag_physics::damage::CraftState::Racing
        );
        race.tick(&PlayerInputs::none());
        let ship = &race.sim.world.ships[1];
        assert_eq!(ship.physics.shield, 0.0);
        assert_eq!(
            ship.physics.craft_state,
            oag_physics::damage::CraftState::Destroyed
        );
        assert_eq!(race.extension_reports()[0].faulted, None);
    }

    const EVENTS: i32 = 1024 + oag_ext_abi::EVENTS_AT as i32;

    /// Gives slot 0 the mod pickup while it holds nothing and nothing has been fired or
    /// absorbed yet, and counts the `pickup-fire` (blob word 0) and `pickup-absorbed`
    /// (word 1) events it is sent.
    fn giver(id: &str) -> Source {
        module(
            id,
            &format!(
                "(local $i i32) (local $kind i32)
                 (block $done (loop $next
                   (br_if $done (i32.ge_u (local.get $i) (i32.load (i32.const {count}))))
                   (local.set $kind (i32.load8_u (i32.add (i32.const {EVENTS}) (i32.mul (local.get $i) (i32.const 8)))))
                   (if (i32.eq (local.get $kind) (i32.const 2))
                     (then (i32.store (i32.const {STATE}) (i32.add (i32.load (i32.const {STATE})) (i32.const 1)))))
                   (if (i32.eq (local.get $kind) (i32.const 3))
                     (then (i32.store (i32.const {s4}) (i32.add (i32.load (i32.const {s4})) (i32.const 1)))))
                   (local.set $i (i32.add (local.get $i) (i32.const 1)))
                   (br $next)))
                 (if (i32.eqz (i32.or (i32.load8_u (i32.const {CRAFTS}))
                                      (i32.or (i32.load (i32.const {STATE})) (i32.load (i32.const {s4})))))
                   (then
                     (i32.store (i32.const {REPLY}) (i32.const 1))
                     (i32.store8 (i32.const {cmd}) (i32.const 3))
                     (i32.store8 (i32.const {slot}) (i32.const 0))))
                 i32.const 0",
                count = 1024 + 24,
                s4 = STATE + 4,
                cmd = REPLY + 4,
                slot = REPLY + 5,
            ),
        )
    }

    /// Every tick: `pickup-roll` every second Turbo.
    fn roller() -> Source {
        module(
            "test.roller",
            &format!(
                "(i32.store (i32.const {REPLY}) (i32.const 1))
                 (i32.store8 (i32.const {cmd}) (i32.const 9))
                 (i32.store8 (i32.const {slot}) (i32.const 0))
                 (i32.store16 (i32.const {arg}) (i32.const 5))
                 (i32.store (i32.const {word}) (i32.const 2))
                 i32.const 0",
                cmd = REPLY + 4,
                slot = REPLY + 5,
                arg = REPLY + 6,
                word = REPLY + 12,
            ),
        )
    }

    fn pressed(button: oag_gameplay::input::Button) -> PlayerInputs {
        let mut buttons = oag_gameplay::input::Input::new();
        buttons.begin_frame(button.bit());
        PlayerInputs::single(oag_gameplay::InputSnapshot {
            buttons,
            ..oag_gameplay::InputSnapshot::new()
        })
    }

    #[test]
    fn the_mods_own_pickup_is_the_mods_to_fire_and_an_absorb_pays_nothing() {
        use oag_gameplay::input::Button;
        use oag_tables::weapons::Weapon;
        let mut race = grid(prepared(&[giver("test.giver")]));
        race.tick(&PlayerInputs::none()); // the give
        assert_eq!(
            race.sim.world.ships[0].pickup.weapon,
            Some(Weapon::Extension)
        );
        assert_eq!(
            race.sim.world.ships[0].pickup.last, None,
            "the repeat rule's memory is the title's"
        );
        race.tick(&pressed(Button::Square));
        assert_eq!(
            race.sim.world.ships[0].pickup.weapon,
            Some(Weapon::Extension),
            "fire does not spend it"
        );
        assert_eq!(
            race.sim.world.ships[0].physics.turbo_timer, 0.0,
            "nor does the engine fire anything"
        );
        let shield = race.sim.world.ships[0].physics.shield;
        race.tick(&pressed(Button::Circle));
        assert_eq!(
            race.sim.world.ships[0].pickup.weapon, None,
            "absorb takes it away"
        );
        assert_eq!(
            race.sim.world.ships[0].physics.shield, shield,
            "and pays nothing"
        );
        race.tick(&PlayerInputs::none());
        let blob = race.sim.extensions.blob_for_tests(0);
        assert_eq!(
            &blob[..8],
            &[1, 0, 0, 0, 1, 0, 0, 0],
            "one fire, one absorb"
        );
    }

    #[test]
    fn a_pickup_roll_turns_every_nth_grant_of_its_weapon_into_the_mods_pickup() {
        use oag_tables::weapons::Weapon;
        let mut race = grid(prepared(&[roller()]));
        assert!(
            !race.sim.extensions.rolls(Weapon::Turbo),
            "no rule before the first call"
        );
        race.tick(&PlayerInputs::none());
        let rolled: Vec<bool> = [
            Weapon::Turbo,
            Weapon::Rocket,
            Weapon::Turbo,
            Weapon::Turbo,
            Weapon::Turbo,
        ]
        .into_iter()
        .map(|weapon| race.sim.extensions.rolls(weapon))
        .collect();
        assert_eq!(
            rolled,
            [false, false, true, false, true],
            "every second Turbo, nothing else"
        );
        // Re-sent every tick, the same rule keeps its count.
        race.tick(&PlayerInputs::none());
        assert!(!race.sim.extensions.rolls(Weapon::Turbo));
        assert!(race.sim.extensions.rolls(Weapon::Turbo));
    }

    #[test]
    fn a_second_module_reaching_for_the_mods_pickup_faults_and_the_first_keeps_it() {
        let mut race = grid(prepared(&[giver("test.a"), giver("test.b")]));
        race.tick(&PlayerInputs::none());
        let reports = race.extension_reports();
        assert!(reports[0].faulted.is_none(), "{:?}", reports[0].faulted);
        let (_, why) = reports[1]
            .faulted
            .clone()
            .expect("the second giver faulted");
        assert!(why.contains("test.a"), "{why}");
        assert_eq!(
            race.sim.world.ships[0].pickup.weapon,
            Some(oag_tables::weapons::Weapon::Extension)
        );
    }

    #[test]
    fn a_race_without_weapons_gives_no_mod_pickup() {
        let mut setup_race = grid(prepared(&[giver("test.giver")]));
        setup_race.sim.weapons_on = false;
        setup_race.tick(&PlayerInputs::none());
        assert_eq!(setup_race.sim.world.ships[0].pickup.weapon, None);
    }

    /// Every tick: `fire-effect` with bit 15 set, the mod's own effect 2, for slot 1.
    fn mod_effect_firer(id: &str, fires: bool) -> Source {
        let body = if fires {
            format!(
                "(i32.store (i32.const {REPLY}) (i32.const 1))
                 (i32.store8 (i32.const {c}) (i32.const 6)) (i32.store8 (i32.const {s}) (i32.const 1))
                 (i32.store16 (i32.const {a}) (i32.const 0x8002))
                 i32.const 0",
                c = REPLY + 4,
                s = REPLY + 5,
                a = REPLY + 6,
            )
        } else {
            "i32.const 0".to_string()
        };
        module(id, &body)
    }

    #[test]
    fn a_mod_effect_is_queued_under_its_mod_and_a_race_firing_one_hashes_like_one_firing_none() {
        let mut loud = grid(prepared(&[mod_effect_firer("test.loud", true)]));
        let mut quiet = grid(prepared(&[mod_effect_firer("test.quiet", false)]));
        for _ in 0..120 {
            loud.tick(&PlayerInputs::none());
            quiet.tick(&PlayerInputs::none());
            assert_eq!(loud.sim.state_hash(), quiet.sim.state_hash());
        }
        loud.tick_sim_only(&PlayerInputs::none());
        assert_eq!(
            loud.sim.extensions.effects_for_tests(),
            [(
                1,
                crate::extension_effects::Fired::Own {
                    mod_id: "test.loud".to_string(),
                    index: 2
                }
            )]
        );
        assert!(quiet.sim.extensions.effects_for_tests().is_empty());
    }

    /// Every tick: a Turbo cue, the wall sparks, a red pickup backdrop for slot 1, the
    /// three race-wide outputs (rain at 0.5, sun at 0.5, sky grey at 0.25), and for slot 1
    /// an energy-bar shake of 2, a thrust-bar share of 0.5 into red, the text "hi" and the
    /// icon "Ab", both at the centre.
    fn outputter(id: &str, outputs: bool) -> Source {
        let body = if outputs {
            let hud = |index: i32, kind: i32, arg: i32, value: u32, word: u32| {
                let at = REPLY + 4 + 12 * index;
                let pooled = if kind >= 19 && index == 9 { 3 } else { 0 };
                let arg = if kind >= 19 { pooled } else { arg };
                format!(
                    "(i32.store8 (i32.const {at}) (i32.const {kind})) (i32.store8 (i32.const {s}) (i32.const 1))
                     (i32.store16 (i32.const {a}) (i32.const {arg}))
                     (i32.store (i32.const {v}) (i32.const {value})) (i32.store (i32.const {w}) (i32.const {word}))",
                    s = at + 1,
                    a = at + 2,
                    v = at + 4,
                    w = at + 8,
                )
            };
            let race_wide = |index: i32, kind: i32, value: f32| {
                let at = REPLY + 4 + 12 * index;
                format!(
                    "(i32.store8 (i32.const {at}) (i32.const {kind}))
                     (f32.store (i32.const {v}) (f32.const {value}))",
                    v = at + 4,
                )
            };
            let pool = format!(
                    "(i32.store8 (i32.const {p}) (i32.const 2)) (i32.store8 (i32.const {q}) (i32.const 104)) (i32.store8 (i32.const {r}) (i32.const 105))
                     (i32.store8 (i32.const {u}) (i32.const 2)) (i32.store8 (i32.const {v}) (i32.const 65)) (i32.store8 (i32.const {w}) (i32.const 98))",
                    p = POOL,
                    q = POOL + 1,
                    r = POOL + 2,
                    u = POOL + 3,
                    v = POOL + 4,
                    w = POOL + 5,
                );
            format!(
                "(i32.store (i32.const {REPLY}) (i32.const 10))
                 (i32.store8 (i32.const {c0}) (i32.const 5)) (i32.store8 (i32.const {s0}) (i32.const 1))
                 (i32.store16 (i32.const {a0}) (i32.const 1))
                 (i32.store8 (i32.const {c1}) (i32.const 6)) (i32.store8 (i32.const {s1}) (i32.const 1))
                 (i32.store16 (i32.const {a1}) (i32.const 0))
                 (i32.store8 (i32.const {c2}) (i32.const 16)) (i32.store8 (i32.const {s2}) (i32.const 1))
                 (i32.store16 (i32.const {a2}) (i32.const 4))
                 (i32.store (i32.const {b2}) (i32.const 0xff0000ff))
                 {weather} {sun} {grade}
                 {shake} {extra} {text} {icon} {pool}
                 i32.const 0",
                c0 = REPLY + 4,
                s0 = REPLY + 5,
                a0 = REPLY + 6,
                c1 = REPLY + 16,
                s1 = REPLY + 17,
                a1 = REPLY + 18,
                c2 = REPLY + 28,
                s2 = REPLY + 29,
                a2 = REPLY + 30,
                b2 = REPLY + 36,
                shake = hud(6, 17, 3, 2.0_f32.to_bits(), 0),
                extra = hud(7, 18, 2, 0.5_f32.to_bits(), 0xff20_20e0),
                text = hud(8, 19, 0, 0x0050_0344, 0xffff_ffff),
                icon = hud(9, 20, 3, 0x0050_0344, 0xffff_ffff),
                weather = race_wide(3, 10, 0.5),
                sun = race_wide(4, 11, 0.5),
                grade = race_wide(5, 12, 0.25),
            )
        } else {
            "i32.const 0".to_string()
        };
        module(id, &body)
    }

    #[test]
    fn output_commands_reach_the_cues_effects_and_hud_and_leave_the_hash_alone() {
        let mut loud = grid(prepared(&[outputter("test.loud", true)]));
        let mut quiet = grid(prepared(&[outputter("test.quiet", false)]));
        for _ in 0..120 {
            loud.tick(&PlayerInputs::none());
            quiet.tick(&PlayerInputs::none());
            assert_eq!(loud.sim.state_hash(), quiet.sim.state_hash());
        }
        assert!(
            loud.sim
                .cues
                .iter()
                .any(|cue| cue.cue == oag_gameplay::cue::Cue::Turbo && cue.slot == 1),
            "the Turbo cue is queued"
        );
        // A drawn tick hands the effect to the view at once; a tick that draws nothing
        // leaves this tick's queued.
        loud.tick_sim_only(&PlayerInputs::none());
        assert_eq!(
            loud.sim.extensions.effects_for_tests(),
            [(
                1,
                crate::extension_effects::Fired::Title(oag_title::effects::Trigger::CollisionSpark)
            )]
        );
        let tick = loud.sim.world.tick;
        let hud = loud.sim.extensions.hud(1, tick);
        assert_eq!(
            hud.tint(oag_hud::ext_draws::Element::Pickup),
            Some([1.0, 0.0, 0.0, 1.0])
        );
        assert_eq!(hud.shake[oag_hud::ext_draws::Element::Energy as usize], 2.0);
        let extra = hud.extra(oag_hud::ext_draws::Element::Thrust).unwrap();
        assert_eq!(extra.extra, 0.5);
        assert_eq!(
            extra.color,
            [224.0 / 255.0, 32.0 / 255.0, 32.0 / 255.0, 1.0]
        );
        assert_eq!(
            hud.items
                .iter()
                .map(|item| (item.anchor, item.x, item.y, item.scale, item.what.clone()))
                .collect::<Vec<_>>(),
            [
                (
                    oag_hud::ext_draws::Anchor::Center,
                    3.0,
                    5.0,
                    1.0,
                    oag_hud::ext_draws::What::Text("hi".into())
                ),
                (
                    oag_hud::ext_draws::Anchor::Center,
                    3.0,
                    5.0,
                    1.0,
                    oag_hud::ext_draws::What::Icon("Ab".into())
                ),
            ]
        );
        assert!(
            loud.sim.extensions.hud(0, tick).is_quiet(),
            "only the slot it names"
        );
        assert!(quiet.sim.extensions.hud(1, tick).is_quiet());
        // The race-wide outputs are held for the view half and absent without a module.
        assert_eq!(
            loud.extension_sky(),
            crate::extensions::Sky {
                rain: Some(0.5),
                sun: 0.5,
                grey: 0.25
            }
        );
        assert_eq!(quiet.extension_sky(), crate::extensions::Sky::default());
        assert_eq!(
            grid(Prepared::default()).extension_sky(),
            crate::extensions::Sky::default()
        );
    }

    /// Slot 1 says the mod's own string "boost" and shows its own icon "badge", both at
    /// the centre.
    fn namer() -> Source {
        let at = |index: i32| REPLY + 4 + 12 * index;
        let body = format!(
            "(i32.store (i32.const {REPLY}) (i32.const 2))
             (i32.store8 (i32.const {t}) (i32.const 19)) (i32.store8 (i32.const {ts}) (i32.const 1))
             (i32.store16 (i32.const {ta}) (i32.const 32768))
             (i32.store (i32.const {tv}) (i32.const 68)) (i32.store (i32.const {tw}) (i32.const 4294967295))
             (i32.store8 (i32.const {i}) (i32.const 20)) (i32.store8 (i32.const {is}) (i32.const 1))
             (i32.store16 (i32.const {ia}) (i32.const 32774))
             (i32.store (i32.const {iv}) (i32.const 68)) (i32.store (i32.const {iw}) (i32.const 4294967295))
             (i32.store8 (i32.const {p0}) (i32.const 5))
             (i32.store8 (i32.const {p1}) (i32.const 98)) (i32.store8 (i32.const {p2}) (i32.const 111))
             (i32.store8 (i32.const {p3}) (i32.const 111)) (i32.store8 (i32.const {p4}) (i32.const 115))
             (i32.store8 (i32.const {p5}) (i32.const 116))
             (i32.store8 (i32.const {q0}) (i32.const 5))
             (i32.store8 (i32.const {q1}) (i32.const 98)) (i32.store8 (i32.const {q2}) (i32.const 97))
             (i32.store8 (i32.const {q3}) (i32.const 100)) (i32.store8 (i32.const {q4}) (i32.const 103))
             (i32.store8 (i32.const {q5}) (i32.const 101))
             i32.const 0",
            t = at(0),
            ts = at(0) + 1,
            ta = at(0) + 2,
            tv = at(0) + 4,
            tw = at(0) + 8,
            i = at(1),
            is = at(1) + 1,
            ia = at(1) + 2,
            iv = at(1) + 4,
            iw = at(1) + 8,
            p0 = POOL,
            p1 = POOL + 1,
            p2 = POOL + 2,
            p3 = POOL + 3,
            p4 = POOL + 4,
            p5 = POOL + 5,
            q0 = POOL + 6,
            q1 = POOL + 7,
            q2 = POOL + 8,
            q3 = POOL + 9,
            q4 = POOL + 10,
            q5 = POOL + 11,
        );
        let mut source = module("test.names", &body);
        source.strings = Arc::new(|language| {
            let text = if language.is_some_and(|l| l.eq_ignore_ascii_case("german")) {
                "SCHUB"
            } else {
                "BOOST"
            };
            [("boost".to_string(), text.to_string())].into()
        });
        source
    }

    #[test]
    fn a_mods_own_string_is_shown_in_the_games_language_or_english_and_its_icon_is_its_own() {
        let shown = |language: Option<&str>, sources: &[Source]| {
            let mut report = Vec::new();
            let props = crate::extensions::TrackProps::default();
            let track = crate::extensions::Track {
                name: "test",
                props: &props,
                speedup_pads: &[],
                weapon_pads: &[],
            };
            let prepared = prepare(sources, "wipeout-pulse", None, track, language, &mut report);
            let mut race = grid(prepared);
            race.tick(&PlayerInputs::none());
            let tick = race.sim.world.tick;
            race.sim.extensions.hud(1, tick).items
        };
        let text = |items: &[oag_hud::ext_draws::Item]| {
            items.iter().find_map(|item| match &item.what {
                oag_hud::ext_draws::What::Text(text) => Some(text.clone()),
                _ => None,
            })
        };
        for (language, want) in [
            (Some("German"), "SCHUB"),
            (Some("english"), "BOOST"),
            (Some("French"), "BOOST"),
            (None, "BOOST"),
        ] {
            assert_eq!(
                text(&shown(language, &[namer()])).as_deref(),
                Some(want),
                "{language:?}"
            );
        }
        let items = shown(Some("German"), &[namer()]);
        assert_eq!(
            items[1].what,
            oag_hud::ext_draws::What::ModIcon("mod:test.names/badge".into())
        );
        let mut bare = namer();
        bare.strings = Arc::new(|_| Default::default());
        assert_eq!(
            text(&shown(Some("German"), &[bare])),
            None,
            "no table, no text"
        );
    }

    #[test]
    fn a_mods_icons_are_read_and_decoded_for_the_sheet_and_a_bad_one_is_named() {
        let png = oag_texture::png::encode_rgba(2, 1, &[255, 0, 0, 255, 0, 0, 255, 128]);
        let mut source = namer();
        source.icons = vec![
            ("badge".to_string(), Arc::new(move || Ok(png.clone()))),
            ("broken".to_string(), Arc::new(|| Ok(b"not a png".to_vec()))),
            (
                "gone".to_string(),
                Arc::new(|| Err("no such file".to_string())),
            ),
        ];
        let mut report = Vec::new();
        let icons =
            crate::extensions::mod_icons(&[source.clone()], "wipeout-pulse", None, &mut report);
        assert_eq!(icons.len(), 1);
        assert_eq!(icons[0].src, "mod:test.names/badge");
        assert_eq!((icons[0].width, icons[0].height), (2, 1));
        assert_eq!(icons[0].rgba, [255, 0, 0, 255, 0, 0, 255, 128]);
        let said = report.join("\n");
        assert!(
            said.contains("icon broken") && said.contains("icon gone"),
            "{said}"
        );
        assert!(said.contains("draws nothing"));
        let mut other = source;
        other.titles = vec!["wipeout-pulse".to_string()];
        assert!(
            crate::extensions::mod_icons(&[other], "wipeout-pure", None, &mut Vec::new())
                .is_empty(),
            "a mod for another title loads none of its icons"
        );
        let mut notes = Vec::new();
        let sheet = oag_hud::sprite::Sheet::build_with(&[], icons, &mut notes);
        assert!(sheet.get("mod:test.names/badge").is_some(), "{notes:?}");
    }

    const GOOD_EFFECT: &str = r##"toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/effect.tosd", version = "1.0.0" }
[[emitter]]
duration = 4
lifetime = [10, 2]
speed = [0.5, 0.1]
spread = 20
size = 0.1
alpha = [[0.0, 1.0], [1.0, 0.0]]
colour = "#ff2010"
"##;

    #[test]
    fn a_mods_effect_files_are_loaded_and_a_broken_one_draws_nothing_and_is_named() {
        let mut source = namer();
        source.effects = [
            "a_good",
            "b_broken",
            "c_no_texture",
            "d_gone",
            "e_host",
            "f_no_host",
        ]
        .map(|name| (name.to_string(), format!("fx/{name}.toml")))
        .to_vec();
        source.files = Arc::new(|path| match path {
            "fx/a_good.toml" => Ok(GOOD_EFFECT.as_bytes().to_vec()),
            "fx/b_broken.toml" => Ok(GOOD_EFFECT
                .replace("duration = 4", "duration = 0")
                .into_bytes()),
            "fx/c_no_texture.toml" => Ok(GOOD_EFFECT
                .replace("size = 0.1", "size = 0.1\ntexture = \"fx/x.png\"")
                .into_bytes()),
            "fx/e_host.toml" => Ok(GOOD_EFFECT
                .replace(
                    "size = 0.1",
                    "size = 0.1\ntexture = { entry = \"Track.vex\", name = \"spark.tga\" }",
                )
                .into_bytes()),
            "fx/f_no_host.toml" => Ok(GOOD_EFFECT
                .replace(
                    "size = 0.1",
                    "size = 0.1\ntexture = { entry = \"Track.vex\", name = \"nope.tga\" }",
                )
                .into_bytes()),
            other => Err(format!("{other}: not in the mod")),
        });
        let mut library = oag_fx::psys::Library::new();
        let mut report = Vec::new();
        let loaded = crate::extension_effects::load(
            &[source.clone()],
            "wipeout-pulse",
            None,
            &mut library,
            &mut |entry, name| match (entry, name) {
                ("Track.vex", "spark.tga") => Ok(oag_fx::psys::sprite::Sprite {
                    width: 1,
                    height: 1,
                    rgba: vec![255; 4].into(),
                }),
                _ => Err(format!("{entry} has no texture node {name}")),
            },
            &mut report,
        );
        assert!(loaded.get("test.names", 0).is_some(), "the good file plays");
        assert!(
            loaded.get("test.names", 4).is_some(),
            "a disc texture by name plays"
        );
        for index in [1, 2, 3, 5] {
            assert!(
                loaded.get("test.names", index).is_none(),
                "{index} draws nothing"
            );
        }
        assert!(library.get("mod:test.names/a_good").is_some());
        assert_eq!(
            library.len(),
            2,
            "only the two good ones reached the library"
        );
        let said = report.join("\n");
        for name in [
            "effect b_broken",
            "effect c_no_texture",
            "effect d_gone",
            "effect f_no_host",
        ] {
            assert!(said.contains(name), "{name} is named: {said}");
        }
        assert!(
            said.contains("duration") && said.contains("fx/x.png"),
            "{said}"
        );
        assert!(said.contains("draws nothing"));

        let mut other = source;
        other.titles = vec!["wipeout-pulse".to_string()];
        let none = crate::extension_effects::load(
            &[other],
            "wipeout-pure",
            None,
            &mut oag_fx::psys::Library::new(),
            &mut |_, _| Err("none".to_string()),
            &mut Vec::new(),
        );
        assert!(
            none.get("test.names", 0).is_none(),
            "another title's mod loads none"
        );
    }

    /// Copies slot 1's lap and laps target, as its craft record carries them, into the
    /// first two blob bytes every tick.
    fn lap_reader() -> Source {
        module(
            "test.laps",
            &format!(
                "(i32.store8 (i32.const {STATE}) (i32.load8_u (i32.const {lap})))
                 (i32.store8 (i32.const {target}) (i32.load8_u (i32.const {laps})))
                 i32.const 0",
                lap = CRAFTS + 16 + 2,
                laps = CRAFTS + 16 + 3,
                target = STATE + 1,
            ),
        )
    }

    #[test]
    fn the_craft_record_carries_the_lap_and_the_laps_target() {
        let mut race = grid(prepared(&[lap_reader()]));
        let target = race
            .sim
            .world
            .laps_target()
            .expect("a single race has laps");
        race.sim.world.ships[1].standing.lap = 2;
        race.tick(&PlayerInputs::none());
        let blob = race.sim.extensions.blob_for_tests(0);
        assert_eq!(blob[..2], [2, u8::try_from(target).unwrap()]);
    }

    /// Counts its calls in the blob and halves slot 1's thrust every tick.
    fn halver() -> Source {
        module(
            "test.halver",
            &format!(
                "(i32.store (i32.const {STATE}) (i32.add (i32.load (i32.const {STATE})) (i32.const 1)))
                 (i32.store (i32.const {REPLY}) (i32.const 1))
                 (i32.store8 (i32.const {cmd}) (i32.const 1))
                 (i32.store8 (i32.const {slot}) (i32.const 1))
                 (f32.store (i32.const {f}) (f32.const 0.5))
                 i32.const 0",
                cmd = REPLY + 4,
                slot = REPLY + 5,
                f = REPLY + 8,
            ),
        )
    }

    /// Runs normally for 100 calls, then loops forever.
    fn runaway() -> Source {
        module(
            "test.runaway",
            &format!(
                "(i32.store (i32.const {STATE}) (i32.add (i32.load (i32.const {STATE})) (i32.const 1)))
                 (if (i32.gt_u (i32.load (i32.const {STATE})) (i32.const 100))
                     (then (loop $spin (br $spin))))
                 i32.const 0"
            ),
        )
    }

    fn prepared(sources: &[Source]) -> Prepared {
        let mut report = Vec::new();
        let props = crate::extensions::TrackProps::default();
        let track = crate::extensions::Track {
            name: "test",
            props: &props,
            speedup_pads: &[],
            weapon_pads: &[],
        };
        let prepared = prepare(sources, "wipeout-pulse", None, track, None, &mut report);
        assert!(
            report.iter().all(|line| !line.contains("left out")),
            "{report:?}"
        );
        prepared
    }

    #[test]
    fn a_modded_race_run_twice_hashes_the_same_every_tick_and_the_command_lands() {
        let (first, a) = run(grid(prepared(&[halver()])), 600);
        let (_, b) = run(grid(prepared(&[halver()])), 600);
        assert!(first.is_modded());
        assert_eq!(a, b);
        assert_ne!(fold(&a), VANILLA_FOLD, "the blob is in the hash");
        let (vanilla, _) = run(grid(Prepared::default()), 600);
        let speed = |race: &Race| {
            race.sim.world.ships[1]
                .physics
                .body
                .linear_velocity
                .length()
        };
        assert!(
            speed(&first) < speed(&vanilla),
            "halved thrust {} against {}",
            speed(&first),
            speed(&vanilla)
        );
        assert_eq!(oag_modhost::engines_created(), 1);
        let reports = first.extension_reports();
        assert_eq!(reports.len(), 1);
        assert!(reports[0].fuel_used > 0 && reports[0].faulted.is_none());
    }

    #[test]
    fn a_runaway_module_faults_at_the_same_tick_every_time_and_the_race_goes_on() {
        let (first, a) = run(grid(prepared(&[runaway(), halver()])), 600);
        let (second, b) = run(grid(prepared(&[runaway(), halver()])), 600);
        assert_eq!(a, b);
        let reports = first.extension_reports();
        // Id order: `test.halver` runs before `test.runaway`.
        assert_eq!(reports[0].mod_id, "test.halver");
        assert!(
            reports[0].faulted.is_none(),
            "the other module keeps running"
        );
        let (tick, why) = reports[1].faulted.clone().expect("the runaway faulted");
        assert_eq!(why, "out of fuel");
        assert_eq!(
            second.extension_reports()[1].faulted.as_ref().unwrap().0,
            tick
        );
        assert!(
            first.sim.world.tick > tick + 400,
            "the race went on past the fault"
        );
    }
}

#[cfg(feature = "extensions")]
mod triggers {
    use std::sync::Arc;

    use super::*;
    use crate::extensions::{Prepared, Source, Track, TrackProps, prepare};

    const IO: usize = 1024;
    const TRACK: usize = 8192;

    /// Counts slot 1's `trigger-enter` (blob byte 0) and `trigger-exit` (byte 1), and
    /// copies the track region's trigger count (byte 2) and its first property's value
    /// (bytes 4..8).
    fn counter() -> Source {
        let events = IO + oag_ext_abi::EVENTS_AT;
        let state = IO + oag_ext_abi::STATE_AT;
        let wasm = wat::parse_str(format!(
            r#"(module
                (memory (export "memory") 1)
                (func (export "oag_abi") (result i32) i32.const {word})
                (func (export "oag_io") (result i32) i32.const {IO})
                (func (export "oag_track") (result i32) i32.const {TRACK})
                (func $bump (param $at i32)
                  (i32.store8 (local.get $at) (i32.add (i32.load8_u (local.get $at)) (i32.const 1))))
                (func (export "tick") (result i32) (local $i i32) (local $p i32)
                  (block $done (loop $next
                    (br_if $done (i32.ge_u (local.get $i) (i32.load (i32.const {count}))))
                    (local.set $p (i32.add (i32.const {events}) (i32.mul (local.get $i) (i32.const 8))))
                    (if (i32.eq (i32.load8_u (i32.add (local.get $p) (i32.const 1))) (i32.const 1))
                      (then
                        (if (i32.eq (i32.load8_u (local.get $p)) (i32.const 7))
                          (then (call $bump (i32.const {state}))))
                        (if (i32.eq (i32.load8_u (local.get $p)) (i32.const 8))
                          (then (call $bump (i32.const {exit}))))))
                    (local.set $i (i32.add (local.get $i) (i32.const 1)))
                    (br $next)))
                  (i32.store8 (i32.const {triggers}) (i32.load16_u (i32.const {trigger_count})))
                  (i32.store (i32.const {value}) (i32.load (i32.const {first_value})))
                  i32.const 0))"#,
            word = oag_ext_abi::ABI_WORD,
            count = IO + oag_ext_abi::EVENT_COUNT_AT,
            exit = state + 1,
            triggers = state + 2,
            value = state + 4,
            trigger_count = TRACK + oag_ext_abi::track::TRIGGERS_AT,
            first_value = TRACK + oag_ext_abi::track::PROPS_AT + 4,
        ))
        .unwrap();
        Source {
            mod_id: "test.triggers".to_string(),
            module: "test.wasm".to_string(),
            titles: Vec::new(),
            api: 1,
            memory_pages: 1,
            state_bytes: 8,
            scope: crate::extensions::Scope::Rules,
            read: Arc::new(move || Ok(wasm.clone())),
            strings: Arc::new(|_| Default::default()),
            icons: Vec::new(),
            effects: Vec::new(),
            files: Arc::new(|_| Err("no files".into())),
        }
    }

    /// A box straddling the road between `from` and `to` along x, with `gate = 0.25`.
    fn gate(from: f32, to: f32) -> TrackProps {
        let mut to_world = oag_ext_abi::track::IDENTITY;
        to_world[12] = (from + to) * 0.5;
        TrackProps {
            triggers: vec![oag_trackfile::props::Trigger {
                name: "gate".into(),
                to_world,
                min: [(from - to) * 0.5, -100.0, -100.0],
                max: [(to - from) * 0.5, 100.0, 100.0],
                props: [("gate".to_string(), 0.25)].into(),
            }],
            ..TrackProps::default()
        }
    }

    fn prepared_on(props: &TrackProps) -> Prepared {
        let mut report = Vec::new();
        let track = Track {
            name: "test",
            props,
            speedup_pads: &[],
            weapon_pads: &[],
        };
        let prepared = prepare(
            &[counter()],
            "wipeout-pulse",
            None,
            track,
            None,
            &mut report,
        );
        assert!(report.iter().all(|l| !l.contains("left out")), "{report:?}");
        prepared
    }

    #[test]
    fn a_craft_passing_through_a_trigger_enters_and_leaves_it_once_and_the_guest_reads_it() {
        // Where slot 1 is at 300 and at 380 ticks of the fixture race: the gate sits
        // between, so it is driven into and out of once (the fixture's ring sends the
        // craft back round after that).
        let x_at = |ticks| {
            run(grid(Prepared::default()), ticks).0.sim.world.ships[1]
                .physics
                .body
                .position
                .x
        };
        let (from, to) = (x_at(300), x_at(380));
        assert!(to > from + 10.0, "slot 1 drives along +x: {from} to {to}");
        let props = gate(from + (to - from) * 0.25, from + (to - from) * 0.5);
        let (race, _) = run(grid(prepared_on(&props)), 380);
        let blob = race.extension_state(0).unwrap();
        assert_eq!(
            blob[..3],
            [1, 1, 1],
            "one enter, one exit, one trigger in the region"
        );
        assert_eq!(blob[4..8], 0.25_f32.to_le_bytes(), "its gate property");
        // A gate the craft never reaches is never entered.
        let (race, _) = run(grid(prepared_on(&gate(to + 1000.0, to + 1010.0))), 380);
        assert_eq!(race.extension_state(0).unwrap()[..2], [0, 0]);
    }

    #[test]
    fn a_track_past_a_cap_leaves_every_module_out_naming_the_track() {
        let mut props = gate(0.0, 10.0);
        props.triggers = vec![props.triggers[0].clone(); oag_ext_abi::track::MAX_TRIGGERS + 1];
        let mut report = Vec::new();
        let track = Track {
            name: "foreign:crowded",
            props: &props,
            speedup_pads: &[],
            weapon_pads: &[],
        };
        let prepared = prepare(
            &[counter()],
            "wipeout-pulse",
            None,
            track,
            None,
            &mut report,
        );
        assert!(!crate::extensions::Extensions::new(prepared).any());
        assert!(
            report
                .iter()
                .any(|l| l.contains("track foreign:crowded carries 65 triggers")),
            "{report:?}"
        );
    }
}
