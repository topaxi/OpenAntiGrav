//! Validates the [`weapons`](oag_tables::weapons) decoder against Pure's own
//! table, `Data\XML\weaponstats.xml`, on both pressings.
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
//! `weapons_ground_truth.rs` pins the decoder against Pulse's two tables. This
//! is the Pure half, and it exists because Pure's file is a different dialect
//! - `docs/formats/weapon-stats.md`'s "Pure dialect" section - and for a year
//! the decoder handled that dialect by *losing* things: the whole table until
//! 2026-08-26, the Bomb until 2026-09-15, the Disruptor until the same day.
//! Every claim below is a shape the file has and Pulse's does not, so a
//! regression toward Pulse's shape fails here and nowhere else.
//!
//! Values are never asserted, per ADR-0006. Presence and structure only.

use oag_tables::weapons::{self, DisruptorEffectKind, Weapon};

const IMAGES: [&str; 2] = [
    "data/images/pure-psp-usa.chd",
    "data/images/pure-psp-eu.chd",
];

/// Pure's one weapon table off each pressing that is present.
fn tables() -> Vec<(&'static str, weapons::WeaponStats)> {
    let mut out = Vec::new();
    for image in IMAGES {
        let Some(path) = oag_testdata::image(image) else {
            continue;
        };
        let mut archives =
            oag_pure::open(path.to_str().expect("image path is utf-8")).expect("open as Pure");
        let entry = oag_pure::TITLE.weapons.race;
        let blob = archives
            .read_name(entry)
            .unwrap_or_else(|e| panic!("{image}: {entry} did not resolve: {e}"));
        let stats = weapons::from_blob(&blob).unwrap_or_else(|e| panic!("{image}: {e}"));
        out.push((image, stats));
    }
    out
}

/// Ten weapons, sharing nine with Pulse: no Cannon, LeachBeam, Repulser or
/// Shuriken, and a Disruptor. Nothing is skipped any more - the fuse-less
/// Bomb used to be.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_authors_ten_weapons_and_the_decoder_skips_none_of_them() {
    for (image, stats) in tables() {
        let authored: Vec<Weapon> = stats.absorb.iter().map(|(w, _)| *w).collect();
        println!(
            "{image}: {} weapon(s) with absorb: {authored:?}",
            authored.len()
        );
        assert_eq!(authored.len(), 10, "{image}");
        for pulse_only in [
            Weapon::Cannon,
            Weapon::LeachBeam,
            Weapon::Repulser,
            Weapon::Shuriken,
        ] {
            assert!(
                !authored.contains(&pulse_only),
                "{image}: Pure authors {pulse_only:?}, a Pulse weapon"
            );
        }
        assert!(
            authored.contains(&Weapon::Disruptor),
            "{image}: no Disruptor"
        );
        assert!(
            stats.skipped.is_empty(),
            "{image}: the decoder skipped {:?} - a dialect it cannot read",
            stats.skipped
        );
    }
}

/// The Disruptor decodes with every one of the ten effects the original's
/// parser reads present, and the two `<Stats>` attributes.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pures_disruptor_decodes_with_all_ten_parsed_effects() {
    for (image, stats) in tables() {
        let disruptor = stats
            .disruptor()
            .unwrap_or_else(|| panic!("{image}: no Disruptor decoded"));
        assert!(disruptor.absorb > 0.0 && disruptor.speed > 0.0, "{image}");
        for kind in DisruptorEffectKind::ALL {
            let effect = disruptor
                .effect(kind)
                .unwrap_or_else(|| panic!("{image}: {kind:?} is not authored"));
            assert!(effect.time > 0.0, "{image}: {kind:?} has no positive time");
            let (wants_amount, wants_percent) = match kind {
                DisruptorEffectKind::AutopilotSlow | DisruptorEffectKind::AutopilotFast => {
                    (false, true)
                }
                DisruptorEffectKind::Drunk
                | DisruptorEffectKind::RubberShip
                | DisruptorEffectKind::DrunkCamera => (true, false),
                _ => (false, false),
            };
            assert_eq!(effect.amount.is_some(), wants_amount, "{image}: {kind:?}");
            assert_eq!(
                effect.speed_percent.is_some(),
                wants_percent,
                "{image}: {kind:?}"
            );
        }
        // The per-class speed is monotone up the ladder, by the code's own
        // `+80` a rung.
        assert!(disruptor.speed_for_class(4) > disruptor.speed_for_class(0));
        println!(
            "{image}: Disruptor decoded, {} effect(s)",
            disruptor.effects.iter().filter(|e| e.is_some()).count()
        );
    }
}

/// Pure's Bomb has no fuse and the Mine beside it does.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pures_bomb_has_no_fuse_and_its_mine_does() {
    for (image, stats) in tables() {
        let bomb = stats
            .bomb()
            .unwrap_or_else(|| panic!("{image}: no Bomb decoded"));
        let mine = stats
            .mine()
            .unwrap_or_else(|| panic!("{image}: no Mine decoded"));
        assert_eq!(
            bomb.timetodie, None,
            "{image}: Pure's Bomb has grown a fuse"
        );
        assert!(mine.timetodie > 0.0, "{image}: Pure's Mine lost its fuse");
        assert!(
            bomb.trigger_radius > 0.0 && bomb.blastradius > bomb.trigger_radius,
            "{image}: bomb trigger {} against blast {}",
            bomb.trigger_radius,
            bomb.blastradius
        );
        println!("{image}: Bomb fuse {:?}, Mine fuse present", bomb.timetodie);
    }
}

/// Five `<Pickupodds>` blocks, `Vector` among them, every one weighting all
/// ten weapons with all four columns.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_authors_five_pickup_tables_of_ten_weapons() {
    for (image, stats) in tables() {
        let classes: Vec<&str> = stats.pickups.iter().map(|t| t.class.as_str()).collect();
        println!("{image}: pickup classes {classes:?}");
        assert_eq!(classes.len(), 5, "{image}");
        assert!(
            classes.iter().any(|c| c.eq_ignore_ascii_case("Vector")),
            "{image}: no Vector table"
        );
        for table in &stats.pickups {
            assert_eq!(table.odds.len(), 10, "{image}: {}", table.class);
            assert!(
                table.get(Weapon::Disruptor).is_some(),
                "{image}: {} weights no Disruptor",
                table.class
            );
            assert!(
                table.get(Weapon::Shuriken).is_none(),
                "{image}: {} weights a Shuriken",
                table.class
            );
        }
    }
}
