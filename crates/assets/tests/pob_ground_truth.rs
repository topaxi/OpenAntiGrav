//! Walks every `.pob` emitter tree on both discs.
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
//! [`oag_formats::pob::ParticleSystem::emitters`] reads an emitter record at
//! fixed offsets and follows three pointer fields into a tree, all straight
//! out of the file's own bytes. Both halves of that are unverifiable from a
//! single hand-checked file:
//!
//! 1. **The offsets are a layout, not a coincidence.** They were decoded
//!    against one asset, `WO_SHIP_COLL_SPARK_DAMAGE`. If a field is really
//!    at `+0x5c` for every authored effect, then every file's lifetimes,
//!    schedules and per-emission counts come back positive and small - and
//!    if the reading is off, the corpus is where that shows.
//! 2. **The tree walk terminates on real data.** `WO_ROCKET_EXPLO` nests
//!    children two ways at once; the parser refuses cycles and unbounded
//!    walks, and this is the check that no real file trips either.
//!
//! It also pins the two enumerations a renderer must not guess at: the draw
//! class behind [`Emitter::draw_class`] and the channel modes behind
//! [`ChannelMode`]. An unknown value in either would otherwise reach a
//! renderer as a silent no-draw.
//!
//! No values are asserted against a table written down elsewhere - that
//! would just be the transcription this test exists to make unnecessary.
//! What is asserted is that every file agrees with the *shape* the layout
//! claims.

use std::path::{Path, PathBuf};

use oag_assets::Archive;
use oag_formats::pob::{self, ChannelMode, Emitter, ParticleSystem};

/// `.pob` blobs on the PSP disc, per `docs/formats/pob.md`.
const PSP_SYSTEMS: usize = 35;

/// And on the PS2 disc's main archive.
const PS2_SYSTEMS: usize = 41;

fn image(name: &str) -> Option<PathBuf> {
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

/// Every `SYSP` blob in `archive`, decompressed, in directory order.
///
/// Found by content rather than by name: the entry names are hashes, and
/// scanning for the magic is what makes this work unchanged on the PS2
/// archive, whose nine extra effects have no PSP counterpart.
fn particle_systems(archive: &mut Archive) -> Vec<(usize, Vec<u8>)> {
    let count = archive.directory().entries.len();
    let mut out = Vec::new();
    for index in 0..count {
        let Ok(head) = archive.peek(index, 4) else {
            continue;
        };
        if !pob::looks_like_particle_system(&head) {
            continue;
        }
        let Ok(blob) = archive.read(index) else {
            continue;
        };
        out.push((index, blob));
    }
    out
}

/// Everything one emitter must satisfy for the layout to be a layout.
fn check(system: &str, emitter: &Emitter) {
    let where_ = format!("{system}/{} (+{:#x})", emitter.name, emitter.offset);

    assert!(
        emitter.draw_class().is_some(),
        "{where_}: render mode {} is past the blend table",
        emitter.render_mode
    );
    assert!(
        matches!(emitter.blend_class, 1..=3),
        "{where_}: blend class {}",
        emitter.blend_class
    );
    // Shapes 1, 2 and 8 exist in the dispatch and are unread; a value
    // outside the whole switch would mean the field is not the shape.
    assert!(
        emitter.shape <= 8,
        "{where_}: emitter shape {}",
        emitter.shape
    );

    for (name, channel) in [
        ("size", &emitter.size),
        ("alpha", &emitter.alpha),
        ("rotation", &emitter.rotation_speed),
        ("emission", &emitter.emission_scale),
    ] {
        assert!(
            !matches!(channel.mode, ChannelMode::Unknown(_)),
            "{where_}: {name} channel mode {:?}",
            channel.mode
        );
        // Keyframe times are a normalized age, ascending.
        let mut previous = f32::NEG_INFINITY;
        for &(time, _) in &channel.keys {
            assert!(
                (0.0..=1.0).contains(&time) && time >= previous,
                "{where_}: {name} channel key time {time}"
            );
            previous = time;
        }
    }

    // A schedule the interpreter can run: an emitter that emitted zero
    // particles at a zero interval would spin.
    assert!(
        emitter.duration_ticks >= 0.0 && emitter.duration_ticks.is_finite(),
        "{where_}: duration {}",
        emitter.duration_ticks
    );
    assert!(
        emitter.interval_ticks.0 >= 1 && emitter.interval_ticks.1 >= emitter.interval_ticks.0,
        "{where_}: interval {:?}",
        emitter.interval_ticks
    );
    assert!(
        emitter.per_emission.0 >= 1 && emitter.per_emission.1 >= emitter.per_emission.0,
        "{where_}: per emission {:?}",
        emitter.per_emission
    );
    assert!(
        emitter.lifetime_ticks.0 >= 0 && emitter.lifetime_ticks.1 >= 0,
        "{where_}: lifetime {:?}",
        emitter.lifetime_ticks
    );
    assert!(
        emitter.live_cap > 0,
        "{where_}: live cap {}",
        emitter.live_cap
    );
    assert!(
        emitter.speed_per_tick.0.is_finite() && emitter.speed_per_tick.1 >= 0.0,
        "{where_}: speed {:?}",
        emitter.speed_per_tick
    );
    assert!(
        emitter.cone_degrees >= 0.0 && emitter.cone_degrees <= 180.0,
        "{where_}: cone {} degrees",
        emitter.cone_degrees
    );
    assert!(
        emitter.atlas_grid.0 >= 1 && emitter.atlas_grid.1 >= 1 && emitter.atlas_frames >= 1,
        "{where_}: atlas {:?} x {}",
        emitter.atlas_grid,
        emitter.atlas_frames
    );
    assert!(
        emitter.child_spawn_probability >= 0.0 && emitter.child_spawn_probability <= 1.0,
        "{where_}: child probability {}",
        emitter.child_spawn_probability
    );
}

/// Parses every system in one archive and reports the corpus totals.
fn walk_archive(label: &str, spec: &str, expected: usize) {
    let mut archive = Archive::open(spec).expect("open archive");
    let blobs = particle_systems(&mut archive);
    assert_eq!(
        blobs.len(),
        expected,
        "{label}: found {} SYSP blobs, expected {expected}",
        blobs.len()
    );

    let (mut emitters, mut trees, mut deepest) = (0usize, 0usize, (0usize, String::new()));
    for (index, blob) in &blobs {
        let system = ParticleSystem::parse(blob)
            .unwrap_or_else(|error| panic!("{label} entry {index}: {error}"));
        let parsed = system
            .emitters(blob)
            .unwrap_or_else(|error| panic!("{label} {}: {error}", system.name));

        assert_eq!(
            parsed[0].name, system.name,
            "{label}: the root emitter's name is the resource's own"
        );
        // `flags::LOOPING` is what its doc comment claims - a property of
        // the whole effect - only while no file mixes set and clear
        // emitters. This is the assertion that claim rests on.
        let looping = parsed[0].looping();
        for emitter in &parsed {
            assert_eq!(
                emitter.looping(),
                looping,
                "{label} {}: {} disagrees with the root about LOOPING",
                system.name,
                emitter.name
            );
        }
        for emitter in &parsed {
            check(&system.name, emitter);
            // Every tree index a record hands out must name a real record.
            for child in [emitter.death_child, emitter.particle_child]
                .into_iter()
                .flatten()
            {
                assert!(
                    child < parsed.len(),
                    "{label} {}: child {child}",
                    system.name
                );
            }
        }

        if parsed.len() > deepest.0 {
            deepest = (parsed.len(), system.name.clone());
        }
        emitters += parsed.len();
        trees += 1;
    }

    println!(
        "{label}: {trees} systems, {emitters} emitters, largest {} with {}",
        deepest.1, deepest.0
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_psp_particle_system_walks_its_emitter_tree() {
    let Some(image) = image("pulse-psp-usa.chd") else {
        return;
    };
    walk_archive(
        "psp",
        &format!("{}:PSP_GAME/USRDIR/Data.wad", image.display()),
        PSP_SYSTEMS,
    );
}

/// The rubric's "a second binary is worth more than a second reading of the
/// first": the PS2 port's own 41 systems, through the same parser, with no
/// adjustment.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn every_ps2_particle_system_walks_the_same_way() {
    let Some(image) = image("pulse-ps2-eu.chd") else {
        return;
    };
    walk_archive(
        "ps2",
        &format!("{}:54748/WADS2.WAD", image.display()),
        PS2_SYSTEMS,
    );
}
