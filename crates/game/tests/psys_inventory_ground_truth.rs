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
//! 1. **Wired** - in [`oag_raceplay::RACE_EFFECTS`], loaded at boot and
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
use std::path::PathBuf;

use oag_assets::Archive;
use oag_pob::{self as pob, ParticleSystem};
use oag_raceplay::RACE_EFFECTS;

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
        "the trigger IS read: `Ship_DispatchCollisionFx` picks it for a contact \
         whose combined friction is not positive (`record+0x30`, `0x0883df38`) - \
         a floor or magstrip contact, the same predicate that spares it damage. \
         320 live contacts on Outpost 7 never produced one, so it waits for a \
         live `damaged == 0` hit before it is wired.",
    ),
    (
        "WO_SHIP_COLL_SPARK_TRAIL_SMOKE",
        "a sibling of the `_TRAIL` the damage tree embeds, and nothing reads \
         which effect or emitter references it.",
    ),
    // **Not `WO_CANNON_SPARKS`** - that one is wired, off `Cannon_UpdateRound`
    // (`0x0886593c`), on the wall/track hit path only. The Cannon landed
    // 2026-09-08 (round + model), the sparks 2026-09-09.
    // **Not `WO_PLASMA_HEAD`** - that one is wired, off `Plasma_Init`
    // (`0x0885bd18`), and this list is the *untriggered* half. The Plasma
    // landed 2026-09-02.
    // **Not `WO_PLASMA_FLASH`** either, as of 2026-09-09 - it is the Plasma's
    // *detonation*, spawned by `PlasmaBlast_Construct` (`0x0885fd90`) off
    // `Plasma_SpawnDetonation` (`0x0886ac88`) in `Plasmas_Update`'s
    // (`0x0886b490`) teardown pass, and it is wired through `Race::blast_for`.
    // It sat in this list from 2026-09-02 to 2026-09-09 with "no call site was
    // found for it"; the call site is the pool teardown that read had not
    // followed.
    // **Not `WO_SHURIKEN_HEAD` or `WO_SHURIKEN_BOUNCE`** - both are wired,
    // off `Shuriken_Init` (`0x08877280`) and `Shuriken_Bounce` (`0x088778ac`).
    // The Shuriken landed 2026-09-02; its other two files are below.
    (
        "WO_SHURIKEN_TRAIL",
        "the Shuriken flies and rides its `_HEAD`, but this is attached at a \
         *second* anchor whose basis `Shuriken_Init` rotates by -pi/2 about the \
         blade and `Shuriken_Update` rebuilds every tick. A `Projectile` here \
         carries a position and a velocity and no roll, so there is nowhere to \
         hang it. Playing the head's file at a second point instead would be \
         invention wearing a real asset.",
    ),
    // **Not `WO_SHURIKEN_EXPIRE`** - wired 2026-09-30 off `ShurikenPool_Update`
    // (`0x0886ff38`): the teardown `FUN_08870c78` spawns it at the blade on
    // both endings, a fuse running out and a craft hit. See
    // `oag_raceplay::SHURIKEN_EXPIRE_EFFECT`.
    // **Not `WO_REPULSER` or `WO_REPULSER_BLAST`** - wired 2026-10-04 off
    // `Repulser_SpawnWaves` (`0x08876300`) and `Repulser_SpawnBlastEffect`
    // (`0x088761d8`). See `oag_raceplay::REPULSER_EFFECT` and
    // `docs/ghidra/functions/psp-pulse-usa/repulser.md`.
    // **Not `WO_BOMB_SMOKERING`** - wired 2026-09-23, off `Bomb_Detonate`
    // (`0x088640c8`) -> `BombBlast_Construct` (`0x08872078`), read in full
    // alongside the two `.vex` models the same detonation loads - see
    // `oag_raceplay::bomb_blast` and
    // `docs/ghidra/functions/psp-pulse-usa/mine.md`.
    // **Not `WO_WEAPON_ABSORB`** - wired 2026-09-23, off
    // `Ship_PlayAbsorbFeedback` (`0x08840640`), which both absorb paths call:
    // one instance per `Ship Collision Fx` node, staggered 0.1 s. See
    // `oag_raceplay::absorb` and `docs/ghidra/functions/psp-pulse-usa/shield.md`.
    // **Not `WO_RAIN`, `WO_RAIN_LENS` or `WO_SNOW`** - wired 2026-10-02 off a
    // circuit's `TrackStartup` `<Weather>` element (`Weather_Construct`
    // `0x088f184c`, `Weather_Update` `0x088f1e58`) - see
    // `oag_raceplay::scenery_fx::weather` and `::lens`. Pulse PSP only: the PS2
    // disc authors the same element on three circuits and plays none yet, and
    // Wipeout HD carries none of the three.
    // **Not `WO_BLUE_WELDER` or `WO_MODESTO_STEAM_A`** - wired 2026-10-02.
    // The circuits place them as `ParticleSystem` (`0x3c4`) nodes and
    // `PsysNode_Init` (`0x089156a0`) spawns each at load; three Basilico
    // welders confirmed live. See `oag_raceplay::scenery_fx`. `RAIN`,
];

/// The eight the PS2 port authors, the PSP does not, and nothing triggers.
///
/// The port's ninth extra is `WO_SHIP_ENGINEFLARE`, which **is** wired -
/// [`oag_raceplay::ENGINE_FLARE_EFFECT`]. Its trigger needs no recovery
/// (an engine flare is on while the craft is), so a PS2-sourced race plays
/// the asset and `oag_fx::exhaust`'s procedural flare steps aside; a
/// PSP-sourced one has no such asset and keeps the procedural flare. That
/// asymmetry is the whole reason [`oag_raceplay::RACE_EFFECTS`] is a
/// superset across sources rather than one list per disc.
const PS2_EXTRA_NO_TRIGGER: &[(&str, &str)] = &[
    (
        "WO_CANNON_HIT_SHIP",
        "the PSP craft-hit path (`FUN_08857f2c`/`FUN_08857e90`, reached from \
         `Cannon_UpdateRound`'s own `CannonPool_Update`) applies damage and a \
         `CANNONEXPLSHIP` sound cue but calls no `Psys_Spawn_q` at all - see \
         `WO_CANNON_SPARKS` above, which is the wall/track hit's own effect. \
         No string reference to this name exists in either the PSP or the PS2 \
         executable (`search_strings` on both came back empty), so unlike \
         every other name in this file it cannot be chased from a string xref; \
         whether the PS2 port fires it on a ship hit by a numeric class id \
         instead is open.",
    ),
    (
        "WO_DAMAGE_PLUME",
        "a damaged-hull effect; this engine tracks shield but plays nothing \
         for it and nothing has been read that says when this starts.",
    ),
    (
        "WO_MODESTO_STEAM_B",
        "environment; the PS2 port's second steam, placed by no node read.",
    ),
    (
        "WO_UNDERWATER_DEBRIS",
        "environment; the PS2 port's, no trigger read.",
    ),
    (
        "WO_TRACK_ROCK_DEBRIS",
        "scenery struck by something; no trigger read.",
    ),
    (
        "WO_SHIP_EXPLOSION_DEBRIS",
        "the PS2 port's craft explosion; its destruction path is unread. The PSP's is `Ship_SpawnExplosionSmall`/`Big`.",
    ),
    (
        "WO_SHIP_EXPLO_SMOKE",
        "the PS2 port's craft explosion; its destruction path is unread. The PSP's is `Ship_SpawnExplosionSmall`/`Big`.",
    ),
    (
        "WO_SHIP_FXNODE_BIGEXPLO",
        "the PS2 port's craft explosion; its destruction path is unread. The PSP's is `Ship_SpawnExplosionSmall`/`Big`.",
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
    "WO_MISSILE_HEAD",
    "WO_ROCKET_EXPLO_TRACK",
    "WO_ROCKET_EXPLO",
    "WO_MISSILE_EXPLO",
    "WO_MISSILE_BOUNCE",
    "WO_MINE_EXPLO",
    "WO_SHURIKEN_EXPIRE",
    // The Repulser's blast and its two waves - see `oag_raceplay::REPULSER_EFFECT`.
    "WO_REPULSER",
    "WO_REPULSER_BLAST",
    "WO_WEAPON_ABSORB",
    // `Ship_Damage`'s weapon branch on a LeachBeam drain (`craft+0x138 == 7`);
    // see `oag_raceplay::hit_sparks`.
    "WO_SHIP_SPARK_DAMAGE_LEACHBEAM",
    // What a craft throws at each wreck node as it goes out - see
    // `oag_raceplay::wreck_fx`.
    "WO_SHIP_FXNODE_EXPLO",
    "WO_SHIP_DEATH_SPARKS",
    // The big blast 1.5 s later, at the live model's matrix - the same module.
    "WO_SHIP_EXPLOSION",
    // Placed by the circuits themselves - see `oag_raceplay::scenery_fx`.
    "WO_BLUE_WELDER",
    "WO_MODESTO_STEAM_A",
    // A circuit's `<Weather>` element names them - see
    // `oag_raceplay::scenery_fx::weather`.
    "WO_RAIN",
    "WO_RAIN_LENS",
    "WO_SNOW",
];

/// The PS2 carries all ten of the PSP's plus its own engine flare.
const PS2_WIRED: &[&str] = &[
    "WO_SHIP_COLL_SPARK_DAMAGE",
    "WO_ROCKET_FLARE",
    "WO_MISSILE_HEAD",
    "WO_ROCKET_EXPLO_TRACK",
    "WO_ROCKET_EXPLO",
    "WO_MISSILE_EXPLO",
    "WO_MISSILE_BOUNCE",
    "WO_MINE_EXPLO",
    "WO_WEAPON_ABSORB",
    "WO_SHIP_SPARK_DAMAGE_LEACHBEAM",
    "WO_SHIP_ENGINEFLARE",
];

fn image_named(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
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

/// Wipeout HD/Fury, and deliberately **not** the three-bucket sweep above.
///
/// HD authors 82 distinct particle systems and 22 are wired (`RACE_EFFECTS`'
/// current length - up from 11 when this comment was first written; Plasma,
/// Shuriken and Quake landed since, so recompute from the source rather than
/// trusting a number here) - the weapon table alone (Detonator, Nitro,
/// per-damage-tier hull states) is mostly unbuilt in this engine, and writing
/// a `NO_TRIGGER_RECOVERED`-style reason for each of the other 67 on the
/// strength of "that weapon isn't built" would be exactly the
/// plausibility-over-evidence shortcut this file's own doc comment forbids
/// for the two discs it already covers. That bucketing is still owed, save
/// six names read against the executable rather than inferred (`just psarc
/// cat … | xxd` for the internal name; `strings -a` and Ghidra's own
/// `search_strings` for the trigger check) - `docs/formats/pob.md`'s HD
/// corpus section carries the evidence for all six:
///
/// - `NUMBERS`, `TEST_BOMBSPIKES` (`DATA02` only): disc-authored debug/test
///   assets - confidence 82. Both string checks agree on zero hits, and a
///   46-of-82 load-path control (unbuilt weapon names like
///   `WO_LEACHBEAM_CHARGING` *are* in that 46) shows the check actually
///   discriminates here.
/// - `WO_BLUE_WELDER`, `WO_MODESTO_STEAM_A` (`DATA02`), `WO_DustMotes`
///   (`DATA00`, mixed-case internally, unlike every wired name), and
///   `WO_UNDERWATER_GODRAYS` (`DATA02`, new to HD): environment effects, no
///   recovered placement trigger - confidence 65, lower than the first two.
///   The same string checks come back zero, but the control fails for this
///   category: *no* environment name is in the 46, wired or not, and the
///   wired `WO_SHIP_ENGINEFLARE` (plus its two unwired HD siblings
///   `WO_ENGINE_FLARE`/`WO_ENGINE_JETFLARE`) is absent too - a continuous or
///   ambient effect's trigger just doesn't leave a load-path string, so zero
///   hits proves nothing either way here. The bucket assignment instead
///   rests on the same basis `WO_RAIN`'s PSP entry already does: the
///   track-placement format is unread, not a string search - `WO_BLUE_WELDER`
///   and `WO_MODESTO_STEAM_A` reuse that reasoning under the same name,
///   `WO_DustMotes`/`WO_UNDERWATER_GODRAYS` extend it to two names new to HD
///   on naming and asset-pairing grounds alone.
///
/// 61 remain unbucketed - two of those, `WO_ENGINE_FLARE`/`WO_ENGINE_JETFLARE`,
/// carry an open correctness question rather than just an unread trigger
/// (see the handover thread's Next Steps). Not added to a bucket list here:
/// this module
/// doesn't carry an HD-side `NO_TRIGGER_RECOVERED` const yet, and six of 82
/// isn't the completed sweep that would justify assembling one.
///
/// What *is* checkable without inventing a reason for anything: that every
/// [`RACE_EFFECTS`] name this engine already fires is really on the disc (so
/// a wired trigger can never fire into silence), and the disc's own archive
/// membership for the two-colour `WO_TRAIL_HITSHIP` pair - which independently
/// corroborates `engine-trail.md`'s "red variant on a Fury skin" reading:
/// the archive that carries only Fury/team content is the only one that
/// carries the red variant at all.
mod hd {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use oag_pob::ParticleSystem;

    /// Distinct system names across all seven archives.
    ///
    /// **Not the same number as `crates/pob/tests/pob_ground_truth.rs`'s
    /// `HD_SYSTEMS = 88`** - that counts every `.pob` blob, one per archive
    /// entry; this counts each name once. `the_inventory_count_is_pinned`
    /// prints which five names those six extra blobs belong to: three are the
    /// expected kind, the same effect stored in more than one archive
    /// (`WO_TRAIL_HITSHIP` in three, `WO_DAMAGE_MILD` and `WO_DEBRIS_SPARKS`
    /// in two), but two are a different thing entirely - `WO_SHIP_EXPLOSION`
    /// and `WO_SHIP_COLL_SPARK_DAMAGE` each have a *second* file **inside
    /// `DATA02` alone** whose internal name field was never changed off the
    /// original it was copied from: `wo_ship_explosion_lightshafts.pob` still
    /// names itself `WO_SHIP_EXPLOSION`, and `stesparkstest.pob` - a name that
    /// reads as a developer test asset - still names itself
    /// `WO_SHIP_COLL_SPARK_DAMAGE`. The two numbers disagreeing by exactly the
    /// duplicate count is the corroboration, not a bug to reconcile away.
    const SYSTEMS: usize = 82;

    fn image() -> Option<PathBuf> {
        oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
    }

    /// Every particle system's name across all seven archives, mapped to
    /// which archive(s) carry it - **not deduplicated**, because which
    /// archive(s) hold a name is exactly what the red-variant assertion below
    /// needs.
    fn scan(image: &Path) -> BTreeMap<String, Vec<&'static str>> {
        let mut found: BTreeMap<String, Vec<&'static str>> = BTreeMap::new();
        for archive_path in oag_hd::archives::ALL {
            let spec = format!("{}:{archive_path}", image.display());
            let mut archive = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
            let paths: Vec<String> = archive
                .paths()
                .iter()
                .filter(|p| p.to_ascii_lowercase().ends_with(".pob"))
                .cloned()
                .collect();
            for path in paths {
                let blob = archive.read_path(&path).expect("the entry reads");
                let system = ParticleSystem::parse(&blob).expect("parse");
                found.entry(system.name).or_default().push(archive_path);
            }
        }
        found
    }

    /// **A wired name absent from the disc would fire into silence.**
    ///
    /// Every one of [`RACE_EFFECTS`] is on this disc - a superset of the
    /// PSP's eight and the PS2's nine, per `oag_raceplay::RACE_EFFECTS`'s
    /// own doc comment. The count above drifts as more get wired; recompute
    /// from `RACE_EFFECTS.len()` rather than trusting either number. Kept
    /// separate from [`SYSTEMS`]'s tripwire below so a
    /// changed inventory count fails *that* test rather than masking this
    /// more serious one under the same red.
    #[test]
    #[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
    fn every_wired_effect_is_on_the_disc() {
        let Some(image) = image() else { return };
        let found = scan(&image);
        // **HD's own table, with no exemptions.** It used to inherit Pulse's
        // `WO_RAIN`, `WO_RAIN_LENS`, `WO_SNOW` and 2048's magstrip pair, none
        // of which any of the seven archives carries, and this loop skipped
        // them by name; `oag_hd::effects::EFFECTS` no longer asks for them.
        for name in oag_hd::effects::EFFECTS.names() {
            assert!(
                found.contains_key(name),
                "{name}: wired but not on this disc"
            );
        }
        for name in [
            "WO_RAIN",
            "WO_RAIN_LENS",
            "WO_SNOW",
            "WO_MAGSTRIP_SPARKS",
            "WO_MAGSTRIP_ZONE",
        ] {
            assert!(
                !found.contains_key(name),
                "{name} is on HD's disc now: its table should name it"
            );
        }
        println!(
            "hd: all {} wired names present across {} distinct particle system(s)",
            oag_hd::effects::EFFECTS.names().len(),
            found.len()
        );
    }

    /// The tripwire the PSP and PS2 discs get too: a newly mounted archive or
    /// a renamed effect changes this number and fails here rather than
    /// passing unnoticed.
    #[test]
    #[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
    fn the_inventory_count_is_pinned() {
        let Some(image) = image() else { return };
        let found = scan(&image);
        let duplicated: Vec<(&String, &Vec<&str>)> = found
            .iter()
            .filter(|(_, archives)| archives.len() > 1)
            .collect();
        println!(
            "hd: {} name(s) stored in more than one archive: {duplicated:?}",
            duplicated.len()
        );
        assert_eq!(
            found.len(),
            SYSTEMS,
            "hd: found {} distinct particle system(s), expected {SYSTEMS} - a new or \
             renamed one needs bucketing (see this test's doc comment) before this \
             number moves",
            found.len()
        );
    }

    /// **The red hit-spark variant is Fury-only on the disc itself**, not
    /// just by the executable's variant-select instruction.
    ///
    /// `DATA00`-`DATA02` and `DATA04`-`DATA05` carry the base game; `DATA03`
    /// and `DATA06` are the four-team and Fury/campaign archives per
    /// `oag_hd::archives`. `WO_TRAIL_HITSHIP` (no colour) is in the base set
    /// and duplicated into both team archives; `WO_TRAIL_HITSHIP_RED` is
    /// authored nowhere else. See "the sparks are red exactly when the ribbon
    /// is" in `engine-trail.md`.
    #[test]
    #[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
    fn the_red_hit_spark_variant_is_only_in_the_fury_archive() {
        let Some(image) = image() else { return };
        let found = scan(&image);
        let holders = found
            .get("WO_TRAIL_HITSHIP_RED")
            .unwrap_or_else(|| panic!("WO_TRAIL_HITSHIP_RED: not found in any archive"));
        assert_eq!(
            holders,
            &[oag_hd::archives::DATA06],
            "WO_TRAIL_HITSHIP_RED: expected only in DATA06, found in {holders:?}"
        );
    }
}

/// Wipeout 2048's own effect directory, held to [`oag_raceplay::RACE_EFFECTS`].
///
/// **Chosen, not measured:** which of the wired names a 2048 race plays is
/// whatever its `Data\Particles2048` carries - the names below are the ones it
/// does not, so each is reported by the loader and draws nothing. Where the
/// original's own effect for the same job has a different name (the wreck's
/// `WO_SHIP_EXPLOSION_PLAYER`, `WO_ZONE_SHIP_EXPLOSION`), it stays unwired
/// until its trigger is read.
mod v2048 {
    use oag_raceplay::RACE_EFFECTS;

    const ABSENT: [&str; 9] = [
        "WO_PLASMA_FLASH",
        "WO_SHIP_ENGINEFLARE",
        "WO_LEACHBEAM_ENERGY",
        "WO_SHIP_FXNODE_EXPLO",
        "WO_SHIP_DEATH_SPARKS",
        "WO_BLUE_WELDER",
        "WO_RAIN",
        "WO_RAIN_LENS",
        "WO_SNOW",
    ];

    #[test]
    #[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
    fn the_wired_names_on_2048_are_exactly_the_ones_its_directory_carries() {
        let Some(root) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
            return;
        };
        let archive = oag_assets::psarc::Archive::open_file(&root.join("base/PSP2/data.psarc"))
            .expect("opens");
        let dir = oag_2048::race::EFFECT_DIR;
        let absent: Vec<&str> = RACE_EFFECTS
            .iter()
            .copied()
            .filter(|name| !archive.contains(&oag_fx::psys::effect_path_in(dir, name)))
            .collect();
        assert_eq!(absent, ABSENT);
    }
}
