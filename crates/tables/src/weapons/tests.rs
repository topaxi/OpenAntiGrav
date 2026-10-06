//! What the `WeaponStats_*.xml` reader in [`super`] is asserted to do: the
//! weapon table, pickup odds, and the shapes a malformed file must not produce.
//! Its own file because the tests exceed the 200-line inline limit
//! (`scripts/check-file-size.py`).

use super::*;

/// Every number is **invented** (1, 2, 3 in document order): no shipped value
/// may be reproduced (`docs/architecture/adr/0006-no-copyrighted-content.md`),
/// and a copied one would test the disc, not the parser.
const FIXTURE: &str = r#"<WeaponStats>
<Weapon type="Global"><Stats slowdown_limit="1"/></Weapon>
<Weapon type="Rocket"><Stats absorb="2" blastforce="3" blastradius="4" damage="5" slowdown_time="6" venomspeed="100" flashspeed="200" rapierspeed="300" phantomspeed="400" launchSpeed="7" spread="0.25"/></Weapon>
<Weapon type="Missile"><Stats absorb="22" blastforce="23" blastradius="24" damage="25" slowdown_time="26" venomspeed="500" flashspeed="600" rapierspeed="700" phantomspeed="800" launchSpeed="27" lock_min_dist="28" lock_max_dist="29"/></Weapon>
<Weapon type="Plasma"><Stats absorb="52" blastforce="53" blastradius="54" charge_time="55" damage="56" slowdown_time="57" venomspeed="900" flashspeed="1000" rapierspeed="1100" phantomspeed="1200" launchSpeed="58"/></Weapon>
<Weapon type="Shuriken"><Stats absorb="60" rhicochetForce="61" blastForce="62" blastradius="63" rhicochetdamage="64" blastdamage="65" slowdown_time="66" venomspeed="1300" flashspeed="1400" rapierspeed="1500" phantomspeed="1600" launchSpeed="67" fuse="68"/></Weapon>
<Weapon type="Bomb"><Stats absorb="40" blastforce="41" blastradius="42" damage="43" damageradius="44" slowdown_time="45" timetodie="46" trigger_radius="47"/></Weapon>
<Weapon type="Mine"><Stats absorb="30" blastforce="31" blastradius="32" damage="33" slowdown_time="34" timetodie="35" trigger_radius="36"/></Weapon>
<Weapon type="LeachBeam"><Stats repair="70" absorb="71" damage="72" lock_max_dist="74" lock_min_dist="73" slowShipFactor="75" range="76" active_time="77" energy_multiplier="78"/></Weapon>
<Weapon type="Turbo"><Stats absorb="4" time="5"/></Weapon>
<Weapon type="Shield"><Stats absorb="6" time="7"/></Weapon>
<Weapon type="Autopilot"><Stats absorb="8" time="9"/></Weapon>
<Pickupodds class="Venom">
<Weapon type="Turbo"><Stats ai="10" back="11" front="12" human="13"/></Weapon>
<Weapon type="Rocket"><Stats ai="14" back="15" front="16" human="17"/></Weapon>
</Pickupodds>
<Pickupodds class="Phantom">
<Weapon type="Turbo"><Stats ai="18" back="19" front="20" human="21"/></Weapon>
</Pickupodds>
</WeaponStats>"#;

#[test]
fn the_three_simple_weapons_read_their_pair() {
    let stats = parse(FIXTURE).expect("the fixture parses");
    assert_eq!(
        stats.simple(Weapon::Turbo),
        Some(Simple {
            absorb: 4.0,
            time: 5.0
        })
    );
    assert_eq!(
        stats.simple(Weapon::Shield),
        Some(Simple {
            absorb: 6.0,
            time: 7.0
        })
    );
    assert_eq!(
        stats.simple(Weapon::Autopilot),
        Some(Simple {
            absorb: 8.0,
            time: 9.0
        })
    );
}

/// Asking a rocket for a simple pair is a caller error; a defaulted zero would
/// hide it.
#[test]
fn a_weapon_that_is_not_simple_has_no_simple_stats() {
    let stats = parse(FIXTURE).expect("the fixture parses");
    assert_eq!(stats.simple(Weapon::Rocket), None);
}

/// `absorb` is on every weapon, simple or not.
#[test]
fn every_weapon_carries_absorb() {
    let stats = parse(FIXTURE).expect("the fixture parses");
    assert_eq!(stats.absorb(Weapon::Rocket), Some(2.0));
    assert_eq!(stats.absorb(Weapon::Turbo), Some(4.0));
    assert_eq!(stats.absorb(Weapon::Mine), Some(30.0));
    assert_eq!(stats.absorb(Weapon::Quake), None, "the fixture omits it");
}

#[test]
fn the_rocket_reads_the_five_things_a_projectile_needs() {
    let rocket = parse(FIXTURE)
        .expect("the fixture parses")
        .rocket()
        .expect("the fixture authors a Rocket");
    assert_eq!(
        rocket,
        RocketStats {
            absorb: 2.0,
            blastforce: 3.0,
            blastradius: 4.0,
            damage: 5.0,
            slowdown_time: 6.0,
            launch_speed: 7.0,
            spread: 0.25,
            speeds: [100.0, 200.0, 300.0, 400.0],
            class_independent: false,
        }
    );
}

/// The four class speeds are read positionally, so each fixture value is
/// distinct and the class is the index.
#[test]
fn each_speed_class_reads_its_own_projectile_speed() {
    use crate::handling::SpeedClass;
    let rocket = parse(FIXTURE).expect("parses").rocket().expect("a Rocket");
    assert_eq!(rocket.speed_for(SpeedClass::Venom), 100.0);
    assert_eq!(rocket.speed_for(SpeedClass::Flash), 200.0);
    assert_eq!(rocket.speed_for(SpeedClass::Rapier), 300.0);
    assert_eq!(rocket.speed_for(SpeedClass::Phantom), 400.0);
}

/// A file with no Rocket is not an error: the two shipped tables need not carry
/// the same set.
#[test]
fn a_file_without_a_rocket_has_no_rocket_stats() {
    let no_rocket =
        "<WeaponStats><Weapon type=\"Global\"><Stats slowdown_limit=\"1\"/></Weapon></WeaponStats>";
    assert_eq!(parse(no_rocket).expect("parses").rocket(), None);
}

/// A Rocket missing one of the five is an error, not a zero. `spread` must
/// survive the parse: three rockets at zero degrees apart is one rocket.
#[test]
fn the_rocket_reads_its_fan_angle() {
    let rocket = parse(FIXTURE).expect("parses").rocket().expect("a Rocket");
    assert_eq!(rocket.spread, 0.25);
}

#[test]
fn the_missile_reads_its_blast_its_speeds_and_its_lock() {
    let missile = parse(FIXTURE)
        .expect("the fixture parses")
        .missile()
        .expect("the fixture authors a Missile");
    assert_eq!(
        missile,
        MissileStats {
            absorb: 22.0,
            blastforce: 23.0,
            blastradius: 24.0,
            damage: 25.0,
            slowdown_time: 26.0,
            launch_speed: 27.0,
            lock_min_dist: 28.0,
            lock_max_dist: 29.0,
            speeds: [500.0, 600.0, 700.0, 800.0],
            class_independent: false,
        }
    );
}

/// The positional argument again for the Missile, since the two structs fill
/// their arrays separately. Its fixture speeds differ from the rocket's so a
/// copy-pasted arm fails.
#[test]
fn each_speed_class_reads_its_own_missile_speed() {
    use crate::handling::SpeedClass;
    let missile = parse(FIXTURE)
        .expect("parses")
        .missile()
        .expect("a Missile");
    assert_eq!(missile.speed_for(SpeedClass::Venom), 500.0);
    assert_eq!(missile.speed_for(SpeedClass::Flash), 600.0);
    assert_eq!(missile.speed_for(SpeedClass::Rapier), 700.0);
    assert_eq!(missile.speed_for(SpeedClass::Phantom), 800.0);
}

/// The lock distances make a Missile a Missile; losing one must fail loudly.
#[test]
fn a_missile_missing_a_lock_distance_is_an_error() {
    let broken = FIXTURE.replace(r#" lock_max_dist="29""#, "");
    assert_eq!(
        parse(&broken),
        Err(Error::MissingAttribute {
            element: "Stats",
            attribute: "lock_max_dist"
        })
    );
}

/// The Mine's seven.
#[test]
fn the_mine_reads_its_blast_its_fuse_and_its_trigger() {
    let mine = parse(FIXTURE)
        .expect("the fixture parses")
        .mine()
        .expect("the fixture authors a Mine");
    assert_eq!(
        mine,
        MineStats {
            absorb: 30.0,
            blastforce: 31.0,
            blastradius: 32.0,
            damage: 33.0,
            slowdown_time: 34.0,
            timetodie: 35.0,
            trigger_radius: 36.0,
        }
    );
}

/// `trigger_radius` and `blastradius` get different fixture numbers so a parser
/// reading one into both fails: a mine is *tripped* inside one and *hurts*
/// inside the other.
#[test]
fn a_mines_trigger_radius_is_not_its_blast_radius() {
    let mine = parse(FIXTURE).expect("parses").mine().expect("a Mine");
    assert_ne!(mine.trigger_radius, mine.blastradius);
}

/// A mine with no fuse comes back absent and *recorded* (in `skipped`), not
/// defaulted. This asserted a whole-file error until 2026-08-26; Pure's Bomb
/// (`damageradius`, no `timetodie`) showed that cost Pure its Missile, Rocket
/// and pickup odds too. A block this build cannot decode now costs that weapon
/// alone: nothing defaults, nothing is silent.
#[test]
fn a_mine_missing_its_fuse_is_absent_rather_than_defaulted() {
    let broken = FIXTURE.replace(r#" timetodie="35""#, "");
    let stats = parse(&broken).expect("one undecodable weapon does not fail the file");
    assert!(
        stats.mine().is_none(),
        "a Mine with no fuse was given one from somewhere"
    );
    assert!(
        stats.skipped.contains(&(Weapon::Mine, "timetodie")),
        "the Mine went missing without saying why: {:?}",
        stats.skipped
    );
    // And the rest of the file is still there, which is the half the old
    // whole-file error threw away.
    assert!(stats.missile().is_some(), "the Missile went with it");
    assert!(stats.rocket().is_some(), "the Rocket went with it");
}

/// A *present* attribute that will not parse still fails the whole file: a
/// missing field is a dialect, rubbish in one is broken.
#[test]
fn a_mine_whose_fuse_is_not_a_number_is_still_an_error() {
    let broken = FIXTURE.replace(r#"timetodie="35""#, r#"timetodie="soon""#);
    assert!(
        matches!(parse(&broken), Err(Error::NotANumber { .. })),
        "a Mine with a nonsense fuse parsed"
    );
}

/// A file with no Mine is not an error; see
/// [`a_file_without_a_rocket_has_no_rocket_stats`].
#[test]
fn a_file_without_a_mine_has_no_mine_stats() {
    let none =
        "<WeaponStats><Weapon type=\"Global\"><Stats slowdown_limit=\"1\"/></Weapon></WeaponStats>";
    assert_eq!(parse(none).expect("parses").mine(), None);
}

/// The Bomb's six decoded attributes. `damageradius` and `slowdown_time` are in
/// the fixture and absent from [`BombStats`] on purpose; changing that should
/// mean changing this test.
#[test]
fn the_bomb_reads_six_of_its_eight() {
    let bomb = parse(FIXTURE)
        .expect("the fixture parses")
        .bomb()
        .expect("the fixture authors a Bomb");
    assert_eq!(
        bomb,
        BombStats {
            absorb: 40.0,
            blastforce: 41.0,
            blastradius: 42.0,
            damage: 43.0,
            slowdown_time: 45.0,
            timetodie: Some(46.0),
            trigger_radius: 47.0,
        }
    );
}

/// The Bomb's block is not the Mine's: disjoint fixture numbers catch a
/// copy-pasted parse arm (six shared attribute names).
#[test]
fn the_bomb_and_the_mine_read_their_own_blocks() {
    let stats = parse(FIXTURE).expect("parses");
    let bomb = stats.bomb().expect("a Bomb");
    let mine = stats.mine().expect("a Mine");
    assert_ne!(bomb.damage, mine.damage);
    assert_ne!(bomb.timetodie, Some(mine.timetodie));
    assert_ne!(bomb.trigger_radius, mine.trigger_radius);
}

/// A file with no Bomb is not an error; see
/// [`a_file_without_a_rocket_has_no_rocket_stats`].
#[test]
fn a_file_without_a_bomb_has_no_bomb_stats() {
    let none =
        "<WeaponStats><Weapon type=\"Global\"><Stats slowdown_limit=\"1\"/></Weapon></WeaponStats>";
    assert_eq!(parse(none).expect("parses").bomb(), None);
}

/// A file with no Missile is not an error, and asking a Missile for a simple
/// pair is still a caller error.
#[test]
fn a_file_without_a_missile_has_no_missile_stats() {
    let none =
        "<WeaponStats><Weapon type=\"Global\"><Stats slowdown_limit=\"1\"/></Weapon></WeaponStats>";
    assert_eq!(parse(none).expect("parses").missile(), None);
    assert_eq!(
        parse(FIXTURE).expect("parses").simple(Weapon::Missile),
        None
    );
}

/// The Missile has no `spread` and the Rocket no lock: the reason they are
/// separate structs. A merge would have to answer this test.
#[test]
fn the_missile_and_the_rocket_do_not_share_a_schema() {
    let stats = parse(FIXTURE).expect("parses");
    let rocket = stats.rocket().expect("a Rocket");
    let missile = stats.missile().expect("a Missile");
    // Distinct blocks, not one block read twice.
    assert_ne!(rocket.absorb, missile.absorb);
    assert_ne!(
        rocket.speed_for(crate::handling::SpeedClass::Venom),
        missile.speed_for(crate::handling::SpeedClass::Venom)
    );
    // A missile whose lock window is empty or inverted could never lock anything.
    assert!(missile.lock_min_dist < missile.lock_max_dist);
}

#[test]
fn a_rocket_missing_an_attribute_is_an_error() {
    let broken = FIXTURE.replace(r#" blastradius="4""#, "");
    assert_eq!(
        parse(&broken),
        Err(Error::MissingAttribute {
            element: "Stats",
            attribute: "blastradius"
        })
    );
}

#[test]
fn the_global_block_is_read() {
    assert_eq!(parse(FIXTURE).expect("parses").slowdown_limit, 1.0);
}

/// The four weights keep their order; reading positionally would be silently
/// wrong.
#[test]
fn the_pickup_weights_keep_their_four_meanings() {
    let stats = parse(FIXTURE).expect("the fixture parses");
    let venom = stats.pickups_for("Venom").expect("a Venom table");
    assert_eq!(
        venom.get(Weapon::Turbo),
        Some(PickupOdds {
            ai: 10.0,
            back: 11.0,
            front: 12.0,
            human: 13.0
        })
    );
}

/// One table per class; an omitted class is absent, not empty ("no odds" vs
/// "zero odds").
#[test]
fn each_class_gets_its_own_table() {
    let stats = parse(FIXTURE).expect("the fixture parses");
    assert_eq!(stats.pickups.len(), 2);
    assert!(stats.pickups_for("Phantom").is_some());
    assert!(stats.pickups_for("Flash").is_none());
}

/// A missing required attribute is an error, not a zero.
#[test]
fn a_missing_attribute_is_an_error() {
    let broken = FIXTURE.replace(r#"<Stats absorb="4" time="5"/>"#, r#"<Stats absorb="4"/>"#);
    assert_eq!(
        parse(&broken),
        Err(Error::MissingAttribute {
            element: "Stats",
            attribute: "time"
        })
    );
}

/// And a broken one is an error rather than passing as an older schema.
#[test]
fn a_broken_number_is_an_error() {
    let broken = FIXTURE.replace(r#"time="5""#, r#"time="oops""#);
    assert!(matches!(parse(&broken), Err(Error::NotANumber { .. })));
}

#[test]
fn a_document_that_is_not_this_one_is_refused() {
    assert_eq!(
        parse("<Handling><Stats/></Handling>"),
        Err(Error::MissingRoot)
    );
}

/// The three that damage nobody are the three with a simple schema. The
/// Disruptor is the fourth harmless weapon and not simple (a projectile with no
/// `damage`); see `Weapon::damages`.
#[test]
fn the_simple_weapons_are_the_harmless_ones() {
    for weapon in Weapon::ALL {
        let stats = parse(FIXTURE).expect("parses");
        let simple = matches!(weapon, Weapon::Turbo | Weapon::Shield | Weapon::Autopilot);
        let harmless = simple || weapon == Weapon::Disruptor;
        assert_eq!(!weapon.damages(), harmless, "{weapon:?}");
        if simple {
            assert!(stats.simple(weapon).is_some());
        }
    }
}

/// Pure's Bomb (`damageradius`, no `timetodie`) parses to a Bomb with no fuse,
/// not a skipped weapon. Shape only.
#[test]
fn a_bomb_without_a_fuse_is_a_bomb_that_never_times_out() {
    let pure = FIXTURE.replace(r#" timetodie="46""#, "");
    let stats = parse(&pure).expect("parses");
    let bomb = stats.bomb().expect("a fuse-less Bomb is still a Bomb");
    assert_eq!(bomb.timetodie, None);
    assert_eq!(bomb.trigger_radius, 47.0);
    assert!(
        !stats.skipped.iter().any(|(w, _)| *w == Weapon::Bomb),
        "the Bomb was skipped for want of a fuse: {:?}",
        stats.skipped
    );
    // Pulse's Bomb, unchanged: the fuse is still read where it is authored.
    assert_eq!(
        parse(FIXTURE).expect("parses").bomb().map(|b| b.timetodie),
        Some(Some(46.0))
    );
}

#[test]
fn every_type_name_round_trips() {
    for weapon in Weapon::ALL {
        assert_eq!(Weapon::from_type(weapon.as_type()), Some(weapon));
    }
    assert_eq!(Weapon::from_type("Global"), None);
    assert_eq!(Weapon::from_type("Nonsense"), None);
}

#[test]
fn the_plasma_reads_the_rockets_schema_without_the_fan() {
    let plasma = parse(FIXTURE)
        .expect("the fixture parses")
        .plasma()
        .expect("the fixture authors a Plasma");
    assert_eq!(
        plasma,
        PlasmaStats {
            absorb: 52.0,
            blastforce: 53.0,
            blastradius: 54.0,
            damage: 56.0,
            slowdown_time: 57.0,
            launch_speed: 58.0,
            speeds: [900.0, 1000.0, 1100.0, 1200.0],
            class_independent: false,
        }
    );
}

/// The Plasma's own block, not the Rocket's: their schemas differ by `spread`
/// against `charge_time`, so disjoint numbers catch a copy-pasted arm.
#[test]
fn the_plasma_and_the_rocket_read_their_own_blocks() {
    let stats = parse(FIXTURE).expect("parses");
    let plasma = stats.plasma().expect("a Plasma");
    let rocket = stats.rocket().expect("a Rocket");
    assert_ne!(plasma.damage, rocket.damage);
    assert_ne!(plasma.blastradius, rocket.blastradius);
    assert_ne!(
        plasma.speed_for(crate::handling::SpeedClass::Venom),
        rocket.speed_for(crate::handling::SpeedClass::Venom)
    );
}

/// Pure's dialect: one `speed` for every class and no `launchSpeed`, as
/// `Data\XML\weaponstats.xml` authors (measured 2026-09-02 on
/// `pure-psp-usa.chd`). Shape only.
#[test]
fn a_plasma_authored_with_one_speed_reads_it_for_every_class() {
    let one = concat!(
        "<WeaponStats><Weapon type=\"Global\"><Stats slowdown_limit=\"1\"/></Weapon>",
        "<Weapon type=\"Plasma\"><Stats absorb=\"1\" blastforce=\"2\" blastradius=\"3\" ",
        "charge_time=\"4\" damage=\"5\" slowdown_time=\"6\" speed=\"700\"/></Weapon></WeaponStats>"
    );
    let plasma = parse(one).expect("parses").plasma().expect("a Plasma");
    for class in crate::handling::SpeedClass::ALL {
        assert_eq!(plasma.speed_for(class), 700.0, "{class:?}");
    }
    assert_eq!(
        plasma.launch_speed, 0.0,
        "a file that omits `launchSpeed` adds nothing rather than failing"
    );
}

/// A file with no Plasma is not an error; see
/// [`a_file_without_a_rocket_has_no_rocket_stats`].
#[test]
fn a_file_without_a_plasma_has_no_plasma_stats() {
    let none =
        "<WeaponStats><Weapon type=\"Global\"><Stats slowdown_limit=\"1\"/></Weapon></WeaponStats>";
    assert_eq!(parse(none).expect("parses").plasma(), None);
}

/// The Shuriken reads ten of its thirteen, the **blast** pair and not the
/// ricochet pair. The pairs have adjacent distinct numbers: a parser grabbing
/// `rhicochetdamage` for `blastdamage` yields a plausible struct that only
/// disjoint values catch.
#[test]
fn the_shuriken_reads_its_blast_pair_and_not_its_ricochet_pair() {
    let shuriken = parse(FIXTURE)
        .expect("the fixture parses")
        .shuriken()
        .expect("the fixture authors a Shuriken");
    assert_eq!(
        shuriken,
        ShurikenStats {
            absorb: 60.0,
            blastforce: 62.0,
            blastradius: 63.0,
            blastdamage: 65.0,
            slowdown_time: 66.0,
            launch_speed: 67.0,
            fuse: 68.0,
            speeds: [1300.0, 1400.0, 1500.0, 1600.0],
            class_independent: false,
        }
    );
    // Said again as an inequality, because the assertion above only fails
    // *usefully* if a reader knows which of the two pairs 64 and 61 were.
    assert_ne!(shuriken.blastdamage, 64.0, "that is `rhicochetdamage`");
    assert_ne!(shuriken.blastforce, 61.0, "that is `rhicochetForce`");
}

/// A file with no Shuriken is not an error; see
/// [`a_file_without_a_rocket_has_no_rocket_stats`].
#[test]
fn a_file_without_a_shuriken_has_no_shuriken_stats() {
    let none =
        "<WeaponStats><Weapon type=\"Global\"><Stats slowdown_limit=\"1\"/></Weapon></WeaponStats>";
    assert_eq!(parse(none).expect("parses").shuriken(), None);
}

/// A Shuriken missing its fuse is absent, not zero (a blade reaped on the tick
/// it was thrown).
#[test]
fn a_shuriken_missing_its_fuse_is_absent_rather_than_defaulted() {
    let no_fuse = FIXTURE.replace(" fuse=\"68\"", "");
    let stats = parse(&no_fuse).expect("the rest of the file still parses");
    assert_eq!(stats.shuriken(), None);
    assert!(
        stats.skipped.contains(&(Weapon::Shuriken, "fuse")),
        "the skip must name the attribute: {:?}",
        stats.skipped
    );
    assert!(
        stats.rocket().is_some(),
        "one undecodable weapon must not cost the file its others"
    );
}

#[test]
fn the_leachbeam_reads_its_whole_block() {
    let stats = parse(FIXTURE).expect("the fixture parses");
    // Every attribute distinct in the fixture, so an arm wired to the wrong
    // offset fails rather than passing on a coincidence - which is the failure
    // mode a nine-attribute block invites. The fixture also authors
    // `lock_max_dist` *before* `lock_min_dist`, so document order is not what
    // this reads by.
    assert_eq!(
        stats.leach_beam(),
        Some(LeachBeamStats {
            repair: 70.0,
            absorb: 71.0,
            damage: 72.0,
            lock_min_dist: 73.0,
            lock_max_dist: 74.0,
            slow_ship_factor: 75.0,
            range: 76.0,
            active_time: 77.0,
            energy_multiplier: 78.0,
        })
    );
}

/// The block authors no `slowdown_time` on any shipped table, so the parser must
/// not ask for one. Otherwise `optional_block` would put the LeachBeam in
/// `skipped` and every Pulse race would lose its reticle with only a loader line.
#[test]
fn the_leachbeam_is_not_skipped_for_want_of_a_slowdown_time() {
    assert!(
        !FIXTURE
            .lines()
            .any(|line| line.contains("LeachBeam") && line.contains("slowdown_time")),
        "the fixture must keep authoring no slowdown_time on this block"
    );
    let stats = parse(FIXTURE).expect("the fixture parses");
    assert!(
        !stats.skipped.iter().any(|(w, _)| *w == Weapon::LeachBeam),
        "the LeachBeam was skipped: {:?}",
        stats.skipped
    );
}

/// A file with no LeachBeam keeps the rest of its table, which is Pure's case.
#[test]
fn a_file_with_no_leachbeam_loses_only_the_leachbeam() {
    let without: Vec<&str> = FIXTURE
        .lines()
        .filter(|line| !line.contains("type=\"LeachBeam\""))
        .collect();
    let stats = parse(&without.join("\n")).expect("a table without a LeachBeam still parses");
    assert_eq!(stats.leach_beam(), None);
    assert!(stats.missile().is_some(), "the Missile survives");
    assert!(
        !stats.skipped.iter().any(|(w, _)| *w == Weapon::LeachBeam),
        "an absent block is not a skipped one"
    );
}

/// A LeachBeam missing a lock bound is *skipped*, not fatal - the Bomb's rule.
#[test]
fn a_leachbeam_without_a_lock_window_is_skipped_rather_than_fatal() {
    let broken = FIXTURE.replace(" lock_max_dist=\"74\"", "");
    let stats = parse(&broken).expect("one unreadable block does not fail the file");
    assert_eq!(stats.leach_beam(), None);
    assert!(
        stats
            .skipped
            .contains(&(Weapon::LeachBeam, "lock_max_dist")),
        "the skip names the attribute: {:?}",
        stats.skipped
    );
}
