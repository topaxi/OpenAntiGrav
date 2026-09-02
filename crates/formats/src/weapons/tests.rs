//! What the `WeaponStats_*.xml` reader in [`super`] is asserted to do: the
//! weapon table, the pickup odds, and the shapes a malformed file must not
//! produce.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `weapons.rs`: the tests are 213 lines, well past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::*;

/// Every number here is **invented** - a plain 1, 2, 3 in document order.
/// Per `docs/architecture/adr/0006-no-copyrighted-content.md` no shipped
/// value is reproduced in this repository, and a fixture that copied one
/// would also stop testing the parser and start testing the disc.
const FIXTURE: &str = r#"<WeaponStats>
<Weapon type="Global"><Stats slowdown_limit="1"/></Weapon>
<Weapon type="Rocket"><Stats absorb="2" blastforce="3" blastradius="4" damage="5" slowdown_time="6" venomspeed="100" flashspeed="200" rapierspeed="300" phantomspeed="400" launchSpeed="7" spread="0.25"/></Weapon>
<Weapon type="Missile"><Stats absorb="22" blastforce="23" blastradius="24" damage="25" slowdown_time="26" venomspeed="500" flashspeed="600" rapierspeed="700" phantomspeed="800" launchSpeed="27" lock_min_dist="28" lock_max_dist="29"/></Weapon>
<Weapon type="Plasma"><Stats absorb="52" blastforce="53" blastradius="54" charge_time="55" damage="56" slowdown_time="57" venomspeed="900" flashspeed="1000" rapierspeed="1100" phantomspeed="1200" launchSpeed="58"/></Weapon>
<Weapon type="Bomb"><Stats absorb="40" blastforce="41" blastradius="42" damage="43" damageradius="44" slowdown_time="45" timetodie="46" trigger_radius="47"/></Weapon>
<Weapon type="Mine"><Stats absorb="30" blastforce="31" blastradius="32" damage="33" slowdown_time="34" timetodie="35" trigger_radius="36"/></Weapon>
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

/// Asking a rocket for a simple pair is a caller error, and answering with a
/// defaulted zero would hide it.
#[test]
fn a_weapon_that_is_not_simple_has_no_simple_stats() {
    let stats = parse(FIXTURE).expect("the fixture parses");
    assert_eq!(stats.simple(Weapon::Rocket), None);
}

/// `absorb` is on every weapon, simple or not - it is what the absorb branch
/// spends whatever was absorbed.
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
            launch_speed: 7.0,
            spread: 0.25,
            speeds: [100.0, 200.0, 300.0, 400.0],
        }
    );
}

/// The four class speeds are read positionally, so a test that only checked
/// one of them would pass with the array in any order. Each fixture value is
/// distinct, and the class is the index.
#[test]
fn each_speed_class_reads_its_own_projectile_speed() {
    use crate::handling::SpeedClass;
    let rocket = parse(FIXTURE).expect("parses").rocket().expect("a Rocket");
    assert_eq!(rocket.speed_for(SpeedClass::Venom), 100.0);
    assert_eq!(rocket.speed_for(SpeedClass::Flash), 200.0);
    assert_eq!(rocket.speed_for(SpeedClass::Rapier), 300.0);
    assert_eq!(rocket.speed_for(SpeedClass::Phantom), 400.0);
}

/// A file with no Rocket is not an error, the same way one with no Turbo is
/// not: the two shipped tables need not carry the same set, and a caller
/// with no stats hands out no rocket.
#[test]
fn a_file_without_a_rocket_has_no_rocket_stats() {
    let no_rocket =
        "<WeaponStats><Weapon type=\"Global\"><Stats slowdown_limit=\"1\"/></Weapon></WeaponStats>";
    assert_eq!(parse(no_rocket).expect("parses").rocket(), None);
}

/// And a Rocket missing one of the five is an error rather than a zero -
/// a zero `blastradius` is a rocket that hits nobody, which reads as a
/// gameplay bug rather than as a parse failure.
/// `spread` is the fan angle and has to survive the parse, because three
/// rockets at zero degrees apart is one rocket wearing a disguise.
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
            launch_speed: 27.0,
            lock_min_dist: 28.0,
            lock_max_dist: 29.0,
            speeds: [500.0, 600.0, 700.0, 800.0],
        }
    );
}

/// The same positional argument [`each_speed_class_reads_its_own_projectile_speed`]
/// makes for the Rocket, and it needs making twice: the two structs fill their
/// arrays in two separate places, so one being right says nothing about the other.
///
/// The fixture's missile speeds are deliberately **different numbers from the
/// rocket's**, so a parser that filled the missile's array from the rocket's block
/// - the copy-paste this arm was written by - fails here rather than passing.
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

/// The two lock distances are what make the Missile a Missile rather than a
/// Rocket that costs more, so losing one has to fail loudly.
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

/// The Mine's seven, and the two that make it a mine rather than a small rocket.
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
            timetodie: 35.0,
            trigger_radius: 36.0,
        }
    );
}

/// `trigger_radius` and `blastradius` are two different distances and the
/// fixture gives them two different numbers, so a parser that read one into
/// both fails here. They are also the pair a caller is most likely to conflate:
/// a mine is *tripped* inside one and *hurts* inside the other.
#[test]
fn a_mines_trigger_radius_is_not_its_blast_radius() {
    let mine = parse(FIXTURE).expect("parses").mine().expect("a Mine");
    assert_ne!(mine.trigger_radius, mine.blastradius);
}

/// A mine with no fuse never expires, so the field cannot quietly default - and
/// it does not: the Mine comes back absent, and *recorded*.
///
/// **This asserted a whole-file error until 2026-08-26**, and the reasoning was
/// right while the mechanism was too blunt. Wipeout Pure authors a Bomb with
/// `damageradius` and no `timetodie` - a charge with no fuse, which sits until
/// something trips it - and failing the file for it cost Pure its Missile, its
/// Rocket and its pickup odds too. A block this build cannot decode now costs
/// that weapon alone.
///
/// The rule the old test was defending survives intact, and this pins both
/// halves of it: nothing defaults, and nothing is silent.
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

/// A *present* attribute that will not parse still fails the whole file.
///
/// The other side of the line above: a file that does not carry a field is a
/// dialect, and one with rubbish in a field is broken. Nothing about tolerating
/// the first should tolerate the second.
#[test]
fn a_mine_whose_fuse_is_not_a_number_is_still_an_error() {
    let broken = FIXTURE.replace(r#"timetodie="35""#, r#"timetodie="soon""#);
    assert!(
        matches!(parse(&broken), Err(Error::NotANumber { .. })),
        "a Mine with a nonsense fuse parsed"
    );
}

/// A file with no Mine is not an error, for
/// [`a_file_without_a_rocket_has_no_rocket_stats`]'s reason.
#[test]
fn a_file_without_a_mine_has_no_mine_stats() {
    let none =
        "<WeaponStats><Weapon type=\"Global\"><Stats slowdown_limit=\"1\"/></Weapon></WeaponStats>";
    assert_eq!(parse(none).expect("parses").mine(), None);
}

/// The Bomb's six decoded attributes, and the two the fixture authors that it
/// must **not** pick up.
///
/// `damageradius` and `slowdown_time` are both in the fixture and both absent
/// from [`BombStats`], deliberately - see that struct. A parser that started
/// reading either would have to change this test, which is the point: the
/// omission is a decision and a decision should be hard to undo by accident.
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
            timetodie: 46.0,
            trigger_radius: 47.0,
        }
    );
}

/// The Bomb's block is not the Mine's, and the fixture gives them disjoint
/// numbers so a parser that matched the wrong `<Weapon type>` fails here.
///
/// The two share six attribute names out of seven and eight, which is exactly
/// the shape a copy-pasted parse arm gets wrong silently.
#[test]
fn the_bomb_and_the_mine_read_their_own_blocks() {
    let stats = parse(FIXTURE).expect("parses");
    let bomb = stats.bomb().expect("a Bomb");
    let mine = stats.mine().expect("a Mine");
    assert_ne!(bomb.damage, mine.damage);
    assert_ne!(bomb.timetodie, mine.timetodie);
    assert_ne!(bomb.trigger_radius, mine.trigger_radius);
}

/// A file with no Bomb is not an error, for
/// [`a_file_without_a_rocket_has_no_rocket_stats`]'s reason.
#[test]
fn a_file_without_a_bomb_has_no_bomb_stats() {
    let none =
        "<WeaponStats><Weapon type=\"Global\"><Stats slowdown_limit=\"1\"/></Weapon></WeaponStats>";
    assert_eq!(parse(none).expect("parses").bomb(), None);
}

/// A file with no Missile is not an error, for [`a_file_without_a_rocket_has_no_rocket_stats`]'s
/// reason - and asking a Missile for a simple pair is still a caller error.
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

/// The Missile authors no `spread` and the Rocket no lock, which is the whole
/// reason the two are separate structs. Pinned because a later widening that
/// merged them would have to answer this test rather than quietly default a
/// field to zero.
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

/// The four weights are kept apart and in the right order. Reading them
/// positionally would be silently wrong, since all four are plain numbers.
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

/// One table per class, and a class the file omits is absent rather than
/// empty - a caller must be able to tell "no odds authored" from "zero odds".
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

/// The three that damage nobody are exactly the three with a simple schema.
/// If that ever stops holding, the reason this module singles them out has
/// gone with it.
#[test]
fn the_simple_weapons_are_the_harmless_ones() {
    for weapon in Weapon::ALL {
        let stats = parse(FIXTURE).expect("parses");
        let simple = matches!(weapon, Weapon::Turbo | Weapon::Shield | Weapon::Autopilot);
        assert_eq!(!weapon.damages(), simple, "{weapon:?}");
        if simple {
            assert!(stats.simple(weapon).is_some());
        }
    }
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
            launch_speed: 58.0,
            speeds: [900.0, 1000.0, 1100.0, 1200.0],
        }
    );
}

/// The Plasma's own block, not the Rocket's, and the fixture gives them
/// disjoint numbers so a parse arm that read the wrong `<Weapon type>` fails
/// here. Their schemas differ by exactly one attribute each way - `spread`
/// against `charge_time` - which is the shape a copy-paste gets wrong quietly.
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

/// Pure's dialect: one `speed` for every class and no `launchSpeed`, which is
/// what `Data\XML\weaponstats.xml` authors. Measured 2026-09-02 on
/// `pure-psp-usa.chd`; the *shape* is reproduced here, not its values.
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

/// A file with no Plasma is not an error, for
/// [`a_file_without_a_rocket_has_no_rocket_stats`]'s reason.
#[test]
fn a_file_without_a_plasma_has_no_plasma_stats() {
    let none =
        "<WeaponStats><Weapon type=\"Global\"><Stats slowdown_limit=\"1\"/></Weapon></WeaponStats>";
    assert_eq!(parse(none).expect("parses").plasma(), None);
}
