//! Every authored particle effect on the disc is accounted for.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! `CLAUDE.md`'s do-not-invent rule says that an effect the disc authors gets
//! played rather than approximated. That rule is only checkable if somebody
//! knows what the disc authors, and a prose list in `HANDOVER.md` goes stale
//! the first time a new archive is mounted or a name is recovered - which is
//! exactly how the invented rocket puffs survived as long as they did.
//!
//! So this makes the **disc** the authority. Every `SYSP` blob is sorted into
//! one of three buckets and the buckets must together be the whole disc, with
//! nothing left over and nothing named that is not there:
//!
//! 1. **Wired** - in [`oag_game::race::RACE_EFFECTS`], loaded at boot and
//!    fired by a recovered trigger.
//! 2. **Embedded** - reached as an emitter *inside* a wired effect's tree, so
//!    already played without being named separately. Computed from the parsed
//!    trees rather than listed, so it cannot go stale.
//! 3. **No trigger recovered** - the asset exists and nothing in the
//!    executable has been read that says when to play it. Listed below with a
//!    reason each.
//!
//! A new effect appearing on a newly mounted archive fails this test rather
//! than passing unnoticed, and that failure is the prompt to decide which
//! bucket it is in. **Adding a name to [`NO_TRIGGER_RECOVERED`] to make the
//! test pass is a decision to leave it unplayed, not a formality** - do it
//! only when the trigger genuinely has not been recovered.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use oag_assets::Archive;
use oag_formats::pob::{self, ParticleSystem};
use oag_game::race::RACE_EFFECTS;

/// `.pob` blobs on the PSP disc, per `docs/formats/pob.md`.
const SYSTEMS: usize = 35;

/// Effects with an asset and no recovered trigger, each with why.
///
/// The bar is the executable, not plausibility: "a mine probably explodes"
/// is not a recovered trigger, and firing one on that basis is the invention
/// this project keeps removing.
const NO_TRIGGER_RECOVERED: &[(&str, &str)] = &[
    (
        "WO_SHIP_COLL_SPARK_NODAMAGE",
        "the trigger IS recovered - `ShipCollisionFx_Trigger` picks this when \
         the contact dealt no damage - but this engine's wall contact carries \
         no damage flag yet, so there is nothing to pick on. The closest of \
         all of these to being wirable.",
    ),
    (
        "WO_SHIP_COLL_SPARK_TRAIL_SMOKE",
        "a sibling of the `_TRAIL` the damage tree embeds, and nothing reads \
         which effect or emitter references it.",
    ),
    (
        "WO_SHIP_EXPLOSION",
        "no craft destruction in this engine; eliminations are not built.",
    ),
    (
        "WO_SHIP_DEATH_SPARKS",
        "same - the death path does not exist.",
    ),
    (
        "WO_SHIP_FXNODE_EXPLO",
        "plays at a hull `Fx` locator; which node and on what event is unread.",
    ),
    (
        "WO_SHIP_SPARK_DAMAGE_LEACHBEAM",
        "the Leach Beam is not built.",
    ),
    ("WO_LEACHBEAM_CHARGING", "the Leach Beam is not built."),
    ("WO_LEACHBEAM_ENERGY", "the Leach Beam is not built."),
    (
        "WO_CANNON_SPARKS",
        "the Cannon is in the weapon table and fires nothing - only the \
         Rocket reaches `projectile::spawn`.",
    ),
    ("WO_MINE_EXPLO", "the Mine is not built."),
    ("WO_MISSILE_HEAD", "the Missile fires nothing yet."),
    ("WO_MISSILE_EXPLO", "the Missile fires nothing yet."),
    ("WO_MISSILE_BOUNCE", "the Missile fires nothing yet."),
    ("WO_PLASMA_HEAD", "the Plasma is not built."),
    ("WO_PLASMA_FLASH", "the Plasma is not built."),
    ("WO_SHURIKEN_HEAD", "the Shuriken is not built."),
    ("WO_SHURIKEN_TRAIL", "the Shuriken is not built."),
    ("WO_SHURIKEN_BOUNCE", "the Shuriken is not built."),
    ("WO_SHURIKEN_EXPIRE", "the Shuriken is not built."),
    ("WO_REPULSER", "the Repulser is not built."),
    ("WO_REPULSER_BLAST", "the Repulser is not built."),
    ("WO_QUAKE", "the Quake is not built."),
    ("WO_BOMB_SMOKERING", "the Bomb is not built."),
    (
        "WO_WEAPON_ABSORB",
        "the Shield absorbs a pickup already (`spend_pickup`), but nothing \
         has been read that ties this effect to that path rather than to the \
         Leach Beam or to a shield hit.",
    ),
    (
        "WO_RAIN",
        "an environment effect. Which track places it, where, and how many, \
         is not in anything decoded - the track format carries no effect \
         placement this project has read.",
    ),
    ("WO_RAIN_LENS", "environment, same as `WO_RAIN`."),
    ("WO_SNOW", "environment, same as `WO_RAIN`."),
    ("WO_BLUE_WELDER", "environment, same as `WO_RAIN`."),
    ("WO_MODESTO_STEAM_A", "environment, same as `WO_RAIN`."),
];

/// The eight the PS2 port authors, the PSP does not, and nothing triggers.
///
/// The port's ninth extra is `WO_SHIP_ENGINEFLARE`, which **is** wired -
/// [`oag_game::race::ENGINE_FLARE_EFFECT`]. Its trigger needs no recovery
/// (an engine flare is on while the craft is), so a PS2-sourced race plays
/// the asset and `oag_render::exhaust`'s procedural flare steps aside; a
/// PSP-sourced one has no such asset and keeps the procedural flare. That
/// asymmetry is the whole reason [`oag_game::race::RACE_EFFECTS`] is a
/// superset across sources rather than one list per disc.
const PS2_EXTRA_NO_TRIGGER: &[(&str, &str)] = &[
    (
        "WO_CANNON_HIT_SHIP",
        "the Cannon fires nothing (see `WO_CANNON_SPARKS`).",
    ),
    (
        "WO_DAMAGE_PLUME",
        "a damaged-hull effect; this engine tracks shield but plays nothing \
         for it and nothing has been read that says when this starts.",
    ),
    ("WO_MODESTO_STEAM_B", "environment, same as `WO_RAIN`."),
    ("WO_UNDERWATER_DEBRIS", "environment, same as `WO_RAIN`."),
    (
        "WO_TRACK_ROCK_DEBRIS",
        "scenery struck by something; no trigger read.",
    ),
    (
        "WO_SHIP_EXPLOSION_DEBRIS",
        "no craft destruction in this engine.",
    ),
    (
        "WO_SHIP_EXPLO_SMOKE",
        "no craft destruction in this engine.",
    ),
    (
        "WO_SHIP_FXNODE_BIGEXPLO",
        "no craft destruction in this engine.",
    ),
];

/// And on the PS2 disc's main archive.
const PS2_SYSTEMS: usize = 41;

/// Which of `RACE_EFFECTS` each disc is expected to actually carry.
///
/// `RACE_EFFECTS` spans every source, so "wired but not on this disc" is
/// only a fault for the entries that disc is supposed to have. These two are
/// the claim about which is which, and a wrong one shows up as a loader line
/// saying an effect will not be drawn.
const PSP_WIRED: &[&str] = &[
    "WO_SHIP_COLL_SPARK_DAMAGE",
    "WO_ROCKET_FLARE",
    "WO_ROCKET_EXPLO_TRACK",
    "WO_ROCKET_EXPLO",
];

/// The PS2 carries all four of the PSP's plus its own engine flare.
const PS2_WIRED: &[&str] = &[
    "WO_SHIP_COLL_SPARK_DAMAGE",
    "WO_ROCKET_FLARE",
    "WO_ROCKET_EXPLO_TRACK",
    "WO_ROCKET_EXPLO",
    "WO_SHIP_ENGINEFLARE",
];

fn image_named(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// Sorts one archive's effects into the three buckets and asserts they cover
/// it exactly.
fn check(label: &str, spec: &str, expected: usize, own: &[(&str, &str)], expected_wired: &[&str]) {
    let mut archive = Archive::open(spec).expect("open archive");

    // Found by scanning for the magic rather than by name: the entry names
    // are hashes, so a name list would only ever find what it already knew.
    let mut on_disc = BTreeSet::new();
    let mut embedded = BTreeSet::new();
    for index in 0..archive.directory().entries.len() {
        let Ok(head) = archive.peek(index, 4) else {
            continue;
        };
        if !pob::looks_like_particle_system(&head) {
            continue;
        }
        let blob = archive.read(index).expect("read blob");
        let system = ParticleSystem::parse(&blob).expect("parse");
        let wired = RACE_EFFECTS.contains(&system.name.as_str());
        if wired {
            // Sub-effects of something already played, taken from the tree
            // rather than from a list so the two cannot disagree.
            for emitter in system.emitters(&blob).expect("emitters") {
                embedded.insert(emitter.name);
            }
        }
        on_disc.insert(system.name);
    }
    assert_eq!(
        on_disc.len(),
        expected,
        "{label}: found {} systems",
        on_disc.len()
    );

    let wired: BTreeSet<&str> = RACE_EFFECTS.iter().copied().collect();
    // Both lists apply: the shared one plus whatever is particular to this
    // disc. A name in both would be a duplicate, which the set collapses and
    // the length check below catches.
    let listed: BTreeSet<&str> = NO_TRIGGER_RECOVERED
        .iter()
        .chain(own)
        .map(|(name, _)| *name)
        .collect();
    let named = NO_TRIGGER_RECOVERED.len() + own.len();
    assert!(
        listed.len() == named || std::ptr::eq(own, NO_TRIGGER_RECOVERED),
        "{label}: a name is listed twice ({named} entries, {} distinct)",
        listed.len()
    );

    // Nothing may be in both buckets.
    let both: Vec<&&str> = wired.intersection(&listed).collect();
    assert!(
        both.is_empty(),
        "{label}: wired *and* listed as untriggered: {both:?}"
    );

    // A *wired* name absent from the disc is a real fault: the library
    // would load nothing and the trigger would fire into silence.
    // `RACE_EFFECTS` is deliberately a superset covering every source, so
    // only the ones this disc is expected to carry are checked - see
    // `expected_wired`.
    for name in expected_wired {
        assert!(
            on_disc.contains(*name),
            "{label}: {name} is wired but is not on this disc"
        );
    }
    // The untriggered list spans both discs, so absence is only a fault for
    // the list that belongs to *this* one - the PSP's
    // `WO_SHIP_COLL_SPARK_TRAIL_SMOKE` has no PS2 counterpart and the PS2's
    // nine extras have no PSP one.
    for (name, _) in own {
        assert!(
            on_disc.contains(*name),
            "{label}: {name} is listed as untriggered on this disc and is \
             not on it"
        );
    }

    // And nothing on the disc may be unaccounted for. This is the assertion
    // the whole file exists for: a new effect is a decision, not a default.
    let missed: Vec<&String> = on_disc
        .iter()
        .filter(|name| {
            !wired.contains(name.as_str())
                && !listed.contains(name.as_str())
                && !embedded.contains(*name)
        })
        .collect();
    assert!(
        missed.is_empty(),
        "{label}: these effects are authored on the disc and neither \
         played nor listed as untriggered: {missed:?}. Decide which they \
         are - see this file's doc comment - rather than adding them to \
         the list to get green."
    );

    println!(
        "{label}: {} on disc, {} wired, {} embedded in a wired tree, {} \
         with no recovered trigger",
        on_disc.len(),
        wired.len(),
        on_disc
            .iter()
            .filter(|n| embedded.contains(*n) && !wired.contains(n.as_str()))
            .count(),
        listed.len(),
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_authored_effect_is_played_or_listed_as_untriggered() {
    let Some(image) = image_named("pulse-psp-usa.chd") else {
        return;
    };
    check(
        "psp",
        &format!("{}:PSP_GAME/USRDIR/Data.wad", image.display()),
        SYSTEMS,
        NO_TRIGGER_RECOVERED,
        PSP_WIRED,
    );
}

/// The PS2 port is a source this engine accepts, and it authors nine effects
/// the PSP does not - so "we have looked at everything the game ships" is a
/// claim about two discs, not one.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn the_ps2_port_s_own_effects_are_accounted_for_too() {
    let Some(image) = image_named("pulse-ps2-eu.chd") else {
        return;
    };
    check(
        "ps2",
        &format!("{}:54748/WADS2.WAD", image.display()),
        PS2_SYSTEMS,
        PS2_EXTRA_NO_TRIGGER,
        PS2_WIRED,
    );
}
