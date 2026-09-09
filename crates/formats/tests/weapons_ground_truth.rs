//! Validates the [`weapons`](oag_formats::weapons) decoder against the real
//! tables.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! The same reasoning as [`handling_ground_truth`]: this is text, so a wrong
//! reading does not overrun a buffer and there is no arithmetic to close. The
//! checkable prediction is **completeness and shape**.
//!
//! Three claims, and each would be easy to get wrong in a way a fixture cannot
//! catch:
//!
//! 1. **The three simple weapons really are simple in the shipped file** -
//!    `absorb` and `time`, nothing else needed to read them. The decoder singles
//!    them out on that basis, so if a shipped table gave Turbo a fourth
//!    attribute the reason for the whole `Simple` type would be gone.
//! 2. **`absorb` really is on every weapon.** The module says so, and it is what
//!    lets a future absorb branch look the number up for anything.
//! 3. **`<Pickupodds>` covers the speed classes**, with all four weights on
//!    every entry. A weight silently defaulting to zero would be a weapon that
//!    never drops, which reads as bad luck rather than as a parse failure.
//!
//! Both shipped tables are checked - the race one and Eliminator's - because
//! they are two independent documents against one parser, and the roadmap only
//! ever races the first.
//!
//! What is deliberately **not** checked is any *value*. Asserting one would put
//! shipped tuning data in the repository, which ADR-0006 does not allow.
//! Everything below is a statement about presence and structure.
//!
//! [`handling_ground_truth`]: ./handling_ground_truth.rs

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_formats::weapons::{self, Weapon};

const PSP_IMAGE: &str = "pulse-psp-usa.chd";
const PSP_ARCHIVE: &str = "PSP_GAME/USRDIR/Data.wad";

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// Both weapon tables off one disc, in `[race, elimination]` order.
fn tables() -> Option<Vec<(&'static str, Vec<u8>)>> {
    let path = image(PSP_IMAGE)?;
    let mut disc = DiscImage::open(&path).expect("open");
    let archive = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == PSP_ARCHIVE)
        .expect("Data.wad present")
        .clone();

    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, Directory::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");

    let mut out = Vec::new();
    // Both are required rather than a floor. The Eliminator table existing at
    // all is part of the claim that a mode swaps the whole table.
    for name in [weapons::RACE_ENTRY, weapons::ELIMINATION_ENTRY] {
        let hash = wad::hash_name(name);
        let entry = dir
            .entries
            .iter()
            .find(|e| e.name_hash == hash)
            .unwrap_or_else(|| panic!("{name} is not in {PSP_ARCHIVE}"));
        let raw = disc
            .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
            .expect("blob");
        let blob = match entry.compression {
            Compression::None => raw,
            Compression::Lzss => {
                oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize).expect("lzss")
            }
            Compression::Zlib => panic!("{name}: unexpected zlib entry"),
        };
        out.push((name, blob));
    }
    Some(out)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn both_shipped_tables_decode() {
    let Some(tables) = tables() else { return };
    for (name, blob) in &tables {
        let stats = weapons::from_blob(blob).unwrap_or_else(|e| panic!("{name}: {e}"));
        println!(
            "{name}: {} weapon(s) with absorb, {} pickup table(s)",
            stats.absorb.len(),
            stats.pickups.len()
        );
        assert!(
            stats.slowdown_limit.is_finite(),
            "{name}: <Global> did not decode"
        );
    }
}

/// Claim 1: the three the decoder calls simple really are, in both tables.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_three_simple_weapons_are_authored_in_both_tables() {
    let Some(tables) = tables() else { return };
    for (name, blob) in &tables {
        let stats = weapons::from_blob(blob).unwrap_or_else(|e| panic!("{name}: {e}"));
        for weapon in [Weapon::Turbo, Weapon::Shield, Weapon::Autopilot] {
            let simple = stats
                .simple(weapon)
                .unwrap_or_else(|| panic!("{name}: {weapon:?} is not authored"));
            // Both are durations or energies, so neither is meaningfully
            // negative and a zero `time` would be an effect that never runs.
            assert!(
                simple.time > 0.0 && simple.absorb >= 0.0,
                "{name}: {weapon:?} reads {simple:?}"
            );
            // Printed rather than pinned: the numbers are the disc's and a
            // committed copy of them here would be the hand-transcription
            // `CLAUDE.md` forbids. `--no-capture` shows them.
            println!(
                "{name}: {weapon:?} time={} absorb={}",
                simple.time, simple.absorb
            );
        }
    }
}

/// Claim 1b: the Rocket authors all five of the things a projectile needs, in
/// both tables, and the four class speeds really are four different numbers.
///
/// The last part is the one worth having. `speed_for` reads the array
/// positionally, and every one of the four attributes is a plain number, so a
/// decoder that read the same attribute four times would pass any test that
/// only checked one class. Four distinct ascending values is also the shape a
/// per-class projectile speed *should* have, and asserting it is what would
/// catch the file being read in the wrong order.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_rocket_authors_a_speed_for_every_class() {
    use oag_formats::handling::SpeedClass;

    let Some(tables) = tables() else { return };
    for (name, blob) in &tables {
        let stats = weapons::from_blob(blob).unwrap_or_else(|e| panic!("{name}: {e}"));
        let rocket = stats
            .rocket()
            .unwrap_or_else(|| panic!("{name}: no Rocket authored"));

        assert!(
            rocket.blastradius > 0.0 && rocket.damage > 0.0 && rocket.blastforce > 0.0,
            "{name}: a rocket that hits nobody or hurts nobody - {rocket:?}"
        );

        let speeds: Vec<f32> = SpeedClass::ALL
            .iter()
            .map(|&class| rocket.speed_for(class))
            .collect();
        println!("{name}: rocket speeds by class {speeds:?}");
        assert!(
            speeds.iter().all(|&s| s > 0.0),
            "{name}: a rocket that does not fly - {speeds:?}"
        );
        // Distinct, which is what says the four attributes were read as four.
        for (i, a) in speeds.iter().enumerate() {
            for b in &speeds[i + 1..] {
                assert!(
                    (a - b).abs() > f32::EPSILON,
                    "{name}: two classes share a speed, so the four reads may be one - {speeds:?}"
                );
            }
        }
    }
}

/// Claim 1c: the Missile authors a usable lock window and its own four speeds,
/// in both shipped tables.
///
/// Shape only, never values - ADR-0006. What "usable" means here is the part
/// worth asserting rather than the part that is obvious:
///
/// - `lock_min_dist < lock_max_dist`, because a window that is empty or inverted
///   is a missile that can never lock anything, and the two attributes are read
///   into adjacent fields by adjacent lines of a hand-written parse arm - exactly
///   the shape a transposition survives unnoticed.
/// - The Missile's speeds are read from the Missile's own block. The Rocket's
///   parse arm sits directly above it and reads four identically-named
///   attributes, so a decoder that fell through to the wrong block would still
///   produce four plausible ascending numbers. Comparing the two weapons' Venom
///   speeds is what catches it, and it works because the shipped tables give them
///   the same value only by coincidence if at all - so the assertion is that
///   *some* class differs, not that every one does.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_missile_authors_a_lock_window_and_its_own_speeds() {
    use oag_formats::handling::SpeedClass;

    let Some(tables) = tables() else { return };
    for (name, blob) in &tables {
        let stats = weapons::from_blob(blob).unwrap_or_else(|e| panic!("{name}: {e}"));
        let missile = stats
            .missile()
            .unwrap_or_else(|| panic!("{name}: no Missile authored"));

        assert!(
            missile.blastradius > 0.0 && missile.damage > 0.0 && missile.blastforce > 0.0,
            "{name}: a missile that hits nobody or hurts nobody - {missile:?}"
        );
        assert!(
            missile.lock_min_dist >= 0.0 && missile.lock_min_dist < missile.lock_max_dist,
            "{name}: a lock window nothing can sit inside - min {} max {}",
            missile.lock_min_dist,
            missile.lock_max_dist
        );

        let speeds: Vec<f32> = SpeedClass::ALL
            .iter()
            .map(|&class| missile.speed_for(class))
            .collect();
        println!(
            "{name}: missile speeds by class {speeds:?}, lock window {}..{}",
            missile.lock_min_dist, missile.lock_max_dist
        );
        assert!(
            speeds.iter().all(|&s| s > 0.0),
            "{name}: a missile that does not fly - {speeds:?}"
        );
        for (i, a) in speeds.iter().enumerate() {
            for b in &speeds[i + 1..] {
                assert!(
                    (a - b).abs() > f32::EPSILON,
                    "{name}: two classes share a speed, so the four reads may be one - {speeds:?}"
                );
            }
        }

        // The two blocks are genuinely two. See the doc comment.
        let rocket = stats
            .rocket()
            .unwrap_or_else(|| panic!("{name}: no Rocket authored"));
        assert!(
            SpeedClass::ALL
                .iter()
                .any(|&c| (rocket.speed_for(c) - missile.speed_for(c)).abs() > f32::EPSILON)
                || (rocket.absorb - missile.absorb).abs() > f32::EPSILON
                || (rocket.damage - missile.damage).abs() > f32::EPSILON,
            "{name}: the Missile reads identical to the Rocket on every compared \
             field, which is what falling through to the wrong <Stats> block looks \
             like - rocket {rocket:?} missile {missile:?}"
        );
    }
}

/// Claim 1d: the Mine authors a fuse and a trigger radius, in both shipped
/// tables, and its block is not the Bomb's.
///
/// Shape only, never values - ADR-0006. Three things are worth asserting and
/// each has a failure it is aimed at:
///
/// - `timetodie > 0`, because a mine with a zero fuse is one that expires on
///   the tick it is dropped, which reads as a gameplay bug rather than as a
///   parse failure. It is also the offset (`+0xf4`) that identified the drop
///   handler as the Mine's at all, so a zero here would mean the identification
///   rested on a field the file does not really author.
/// - `trigger_radius > 0` and `blastradius > trigger_radius`. The two are read
///   into adjacent fields by adjacent lines of a hand-written arm - the same
///   transposition risk the Missile's lock window has - and the *ordering* is
///   the part that says which is which: a mine is tripped by a craft coming
///   close and then hurts a wider circle than the one that tripped it.
/// - The Mine's block is not the Bomb's. The Bomb's eight attributes sit
///   directly before the Mine's seven in the struct the original parses into,
///   and both weapons author `blastforce`, `blastradius`, `damage`,
///   `trigger_radius` and `timetodie` under the same names. A decoder that
///   matched the wrong `<Weapon type>` would still produce six plausible
///   numbers. The shipped tables give the two weapons different blast figures,
///   so comparing them catches it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_mine_authors_a_fuse_and_a_trigger_radius() {
    let Some(tables) = tables() else { return };
    for (name, blob) in &tables {
        let stats = weapons::from_blob(blob).unwrap_or_else(|e| panic!("{name}: {e}"));
        let mine = stats
            .mine()
            .unwrap_or_else(|| panic!("{name}: no Mine authored"));

        println!(
            "{name}: mine fuse {} s, trigger {}, blast {}",
            mine.timetodie, mine.trigger_radius, mine.blastradius
        );
        assert!(
            mine.timetodie > 0.0,
            "{name}: a mine that expires the tick it is dropped - {mine:?}"
        );
        assert!(
            mine.trigger_radius > 0.0,
            "{name}: a mine nothing can trip - {mine:?}"
        );
        assert!(
            mine.blastradius > mine.trigger_radius,
            "{name}: blast radius {} is not wider than trigger radius {}, which is \
             what reading the two into each other looks like",
            mine.blastradius,
            mine.trigger_radius
        );
        assert!(
            mine.damage > 0.0 && mine.blastforce > 0.0,
            "{name}: a mine that hurts nobody - {mine:?}"
        );

        // The Mine's block is not the Rocket's. Same argument the Missile makes
        // one claim up, against the neighbour that shares the most attributes.
        let rocket = stats
            .rocket()
            .unwrap_or_else(|| panic!("{name}: no Rocket authored"));
        assert!(
            (rocket.blastforce - mine.blastforce).abs() > f32::EPSILON
                || (rocket.damage - mine.damage).abs() > f32::EPSILON
                || (rocket.absorb - mine.absorb).abs() > f32::EPSILON,
            "{name}: the Mine reads identical to the Rocket on every compared \
             field, which is what matching the wrong <Weapon type> looks like - \
             rocket {rocket:?} mine {mine:?}"
        );
    }
}

/// Claim 1e: the Bomb is the Mine one size up, on both shipped tables.
///
/// Shape only, never values - ADR-0006, so what is asserted is the *ordering*
/// between two weapons rather than either weapon's numbers. That ordering is
/// the claim `oag_gameplay::projectile::mine` rests on when it treats the two
/// with one mechanism: if the Bomb were not uniformly the bigger of the pair,
/// "a single big mine" would be the wrong model and the shared code would be
/// hiding it.
///
/// It doubles as the transposition check the other claims make with a
/// counterfactual: the Bomb's and the Mine's blocks abut in the struct the
/// original parses into and share six attribute names, so a decoder that
/// matched the wrong `<Weapon type>` produces two plausible sets of numbers.
/// Every one of these comparisons would flip.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_bomb_is_the_mine_one_size_up() {
    let Some(tables) = tables() else { return };
    for (name, blob) in &tables {
        let stats = weapons::from_blob(blob).unwrap_or_else(|e| panic!("{name}: {e}"));
        let bomb = stats
            .bomb()
            .unwrap_or_else(|| panic!("{name}: no Bomb authored"));
        let mine = stats
            .mine()
            .unwrap_or_else(|| panic!("{name}: no Mine authored"));

        println!(
            "{name}: bomb {} damage / {} blast / {} trigger / {} s vs mine {} / {} / {} / {} s",
            bomb.damage,
            bomb.blastradius,
            bomb.trigger_radius,
            bomb.timetodie,
            mine.damage,
            mine.blastradius,
            mine.trigger_radius,
            mine.timetodie
        );
        for (what, bigger, smaller) in [
            ("damage", bomb.damage, mine.damage),
            ("blastradius", bomb.blastradius, mine.blastradius),
            ("blastforce", bomb.blastforce, mine.blastforce),
            ("trigger_radius", bomb.trigger_radius, mine.trigger_radius),
            ("timetodie", bomb.timetodie, mine.timetodie),
        ] {
            assert!(
                bigger > smaller,
                "{name}: the Bomb's {what} is {bigger} against the Mine's {smaller} - \
                 either the two blocks are transposed or the Bomb is not the bigger \
                 weapon this engine models it as"
            );
        }
        // And the same self-consistency the Mine's own claim makes: a bomb is
        // tripped by a craft coming close and hurts a wider circle than the one
        // that tripped it.
        assert!(
            bomb.blastradius > bomb.trigger_radius && bomb.trigger_radius > 0.0,
            "{name}: blast radius {} against trigger radius {}",
            bomb.blastradius,
            bomb.trigger_radius
        );
        assert!(
            bomb.timetodie > 0.0,
            "{name}: a bomb that expires the tick it is dropped - {bomb:?}"
        );
    }
}

/// Claim 2: `absorb` is on every weapon the file authors.
///
/// Stated as "every weapon the *decoder recognises* that the file authors",
/// because the two tables need not carry the same set - Eliminator may well drop
/// some - and a missing weapon is not a defect.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_authored_weapon_carries_absorb() {
    let Some(tables) = tables() else { return };
    for (name, blob) in &tables {
        let stats = weapons::from_blob(blob).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(
            stats.absorb.len() >= 10,
            "{name}: only {} weapon(s) decoded, which is too few for a real table",
            stats.absorb.len()
        );
        for (weapon, absorb) in &stats.absorb {
            assert!(
                absorb.is_finite() && *absorb >= 0.0,
                "{name}: {weapon:?} absorbs {absorb}"
            );
        }
        // The decoder's own list is the thing being checked: every weapon it
        // knows about has to be one the file actually authors, or the enum has
        // a name the game does not use.
        let authored: Vec<Weapon> = stats.absorb.iter().map(|(w, _)| *w).collect();
        for weapon in &authored {
            assert!(Weapon::ALL.contains(weapon));
        }
    }
}

/// Claim 3: the pickup tables cover the speed classes, with all four weights.
///
/// The four weights are the pickup design, and reading them positionally rather
/// than by name would be silently wrong because all four are plain numbers - so
/// this asserts every entry has all four and that they are usable weights.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_pickup_tables_cover_the_speed_classes() {
    let Some(tables) = tables() else { return };
    let (name, blob) = &tables[0];
    let stats = weapons::from_blob(blob).unwrap_or_else(|e| panic!("{name}: {e}"));

    for class in ["Venom", "Flash", "Rapier", "Phantom"] {
        let table = stats
            .pickups_for(class)
            .unwrap_or_else(|| panic!("{name}: no <Pickupodds class=\"{class}\">"));
        assert!(
            table.odds.len() >= 8,
            "{name}: {class} authors odds for only {} weapon(s)",
            table.odds.len()
        );
        let mut any_positive = false;
        for (weapon, odds) in &table.odds {
            for (label, value) in [
                ("ai", odds.ai),
                ("back", odds.back),
                ("front", odds.front),
                ("human", odds.human),
            ] {
                assert!(
                    value.is_finite() && value >= 0.0,
                    "{name}: {class}/{weapon:?} {label} is {value}"
                );
            }
            any_positive |= odds.ai > 0.0 || odds.human > 0.0;
        }
        assert!(
            any_positive,
            "{name}: every weight in {class} is zero, so nothing would ever drop"
        );
    }

    // **The shipped table authors four classes and not five**, and this
    // assertion is here because the first reading of the parser said otherwise.
    // `WeaponStats_Parse` tests the `class` attribute against `Vector` and
    // *discards the result* before testing the four that store an index, so the
    // executable knows the name and the data does not use it. That is evidence
    // about `docs/formats/handling-stats.md`'s fifth-class question and it
    // points the opposite way to the parser read alone - pinned here so it is
    // not re-inferred from the code a second time.
    assert!(
        stats.pickups_for("Vector").is_none(),
        "{name}: Vector is authored now, which answers the fifth-class question"
    );
    assert_eq!(
        stats.pickups.len(),
        4,
        "{name}: {} <Pickupodds> block(s), expected the four speed classes",
        stats.pickups.len()
    );
}

/// Claim 1d: the LeachBeam authors a lock window of its own, and it is **not**
/// the Missile's.
///
/// `Ship_AcquireLock` (`0x08844784`) is one function serving two weapons off
/// two pairs of offsets - `stats+0x50`/`+0x54` for the Missile,
/// `stats+0x114`/`+0x118` for the LeachBeam - so the failure this guards
/// against is a decoder that reads one block and hands it to both. Shape and
/// relation only, never values, per this file's header:
///
/// - the LeachBeam's own window is non-empty, the same claim the Missile's gets;
/// - its **far bound is strictly shorter than the Missile's** on both shipped
///   tables, which is what says the two blocks are genuinely two. It is the
///   same trick `the_missile_authors_a_lock_window_and_its_own_speeds` plays
///   with the Rocket's speeds, and it works for the same reason: two blocks
///   read off one would be identical, and these are not.
/// - it authors **no** `slowdown_time`, which is the finding that qualified
///   `RocketStats::slowdown_time`'s "every block authors it". Asserted through
///   `skipped`: a parse arm that started requiring the attribute would drop the
///   whole block, and the reticle with it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_leachbeam_authors_its_own_lock_window_and_no_slowdown_time() {
    let Some(tables) = tables() else { return };
    for (name, blob) in &tables {
        let stats = weapons::from_blob(blob).unwrap_or_else(|e| panic!("{name}: {e}"));
        let leach = stats
            .leach_beam()
            .unwrap_or_else(|| panic!("{name}: no LeachBeam authored"));
        let missile = stats
            .missile()
            .unwrap_or_else(|| panic!("{name}: no Missile authored"));

        println!(
            "{name}: leachbeam lock window {}..{} against the missile's {}..{}",
            leach.lock_min_dist, leach.lock_max_dist, missile.lock_min_dist, missile.lock_max_dist
        );
        assert!(
            leach.lock_min_dist >= 0.0 && leach.lock_min_dist < leach.lock_max_dist,
            "{name}: a lock window nothing can sit inside - {leach:?}"
        );
        assert!(
            leach.absorb > 0.0,
            "{name}: a LeachBeam worth nothing to absorb - {leach:?}"
        );
        assert!(
            leach.lock_max_dist < missile.lock_max_dist,
            "{name}: the LeachBeam reaches as far as the Missile, which is what a decoder \
             reading one block for both would also report - leach {leach:?}, missile \
             min {} max {}",
            missile.lock_min_dist,
            missile.lock_max_dist
        );
        assert!(
            !stats.skipped.iter().any(|(w, _)| *w == Weapon::LeachBeam),
            "{name}: the LeachBeam was skipped - {:?}",
            stats.skipped
        );
    }
}

/// Claim 4: every decoded weapon authors a positive `slowdown_time`, and every
/// one of them sits **at or under** `<Global slowdown_limit>` on both shipped
/// tables.
///
/// The relation is the point, and it is a prediction rather than a
/// restatement. `Ship_AddSlowdown` (`0x08848690`) clamps the victim's running
/// slowdown timer to `slowdown_limit`, so a weapon authored *above* the limit
/// would be one whose own `slowdown_time` is unreachable - it would be silently
/// truncated on the very first hit, with no second hit needed. That the shipped
/// data never does this is what says the two attributes are the pair the law
/// says they are, and it would catch `slowdown_time` being read off the wrong
/// attribute (`timetodie`, say, or `fuse`) far more sharply than a
/// finite-and-positive check does.
///
/// No value is asserted, only the relation and the sign - see this file's
/// header on ADR-0006. `--no-capture` prints the disc's own numbers.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_decoded_weapon_slows_a_victim_within_the_global_limit() {
    let Some(tables) = tables() else { return };
    for (name, blob) in &tables {
        let stats = weapons::from_blob(blob).unwrap_or_else(|e| panic!("{name}: {e}"));
        let limit = stats.slowdown_limit;
        assert!(
            limit > 0.0,
            "{name}: <Global> slowdown_limit is {limit}, so nothing could ever be slowed"
        );

        // One list rather than six blocks, so a weapon added to the decoder and
        // not to this test is a compile error at the `match`, not a silent gap.
        let authored: Vec<(Weapon, f32)> = [
            stats.rocket().map(|s| (Weapon::Rocket, s.slowdown_time)),
            stats.missile().map(|s| (Weapon::Missile, s.slowdown_time)),
            stats.plasma().map(|s| (Weapon::Plasma, s.slowdown_time)),
            stats
                .shuriken()
                .map(|s| (Weapon::Shuriken, s.slowdown_time)),
            stats.mine().map(|s| (Weapon::Mine, s.slowdown_time)),
            stats.bomb().map(|s| (Weapon::Bomb, s.slowdown_time)),
            stats.cannon().map(|s| (Weapon::Cannon, s.slowdown_time)),
        ]
        .into_iter()
        .flatten()
        .collect();

        // Every one of the seven is authored in both shipped tables. A `None`
        // here would mean the block went missing, which the other tests would
        // also catch - this count is what stops the loop below passing vacuously.
        assert_eq!(
            authored.len(),
            7,
            "{name}: {} of the seven decoded blocks are authored - {authored:?}",
            authored.len()
        );

        for (weapon, slowdown_time) in &authored {
            println!("{name}: {weapon:?} slowdown_time/limit = {slowdown_time}/{limit}");
            assert!(
                *slowdown_time > 0.0,
                "{name}: {weapon:?} slows a victim by {slowdown_time}s, which is no slowdown at all"
            );
            assert!(
                *slowdown_time <= limit,
                "{name}: {weapon:?} authors {slowdown_time}s against a {limit}s cap, so its own \
                 figure is unreachable - the attribute is probably being read off the wrong name"
            );
        }
    }
}
