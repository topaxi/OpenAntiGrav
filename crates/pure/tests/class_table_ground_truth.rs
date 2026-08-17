//! Where Pure's `.vex` class-ID table comes from, asserted against both discs.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What settled the numbering
//!
//! `docs/formats/pure-status.md` left three class IDs decoding exactly on Pure -
//! `0x36b`, `0x36c`, `0x37f` - and which was floor, wall or reset undetermined at
//! confidence 45, because the only argument available was that their object counts
//! looked like Pulse's. Two independent measurements replaced that, and this file
//! is both of them:
//!
//! 1. **Both executables carry the same class-name table, in the same order.**
//!    `PSP_GAME/SYSDIR/BOOT.BIN` on each disc holds a run of NUL-terminated
//!    exporter class names, and the run is identical string for string and index
//!    for index across the two titles. Under the assumption that a class ID is a
//!    table index, that run *is* the renumbering, and it predicts every Pure ID
//!    from one anchor.
//! 2. **The prediction is checked against the assets.** Each predicted class is
//!    looked for in Pure's own track and ship files and has to be there, decoding
//!    as its name says it should.
//!
//! The mapping is not a bare index: three IDs in the numbering have no string in
//! this table, so the run is consecutive with gaps. Both gap positions are fixed
//! by Pulse's *already recovered* constants and then hold on Pure unchanged, which
//! is what [`the_two_titles_share_one_class_name_table`] asserts.
//!
//! See `docs/formats/pure-status.md` and `docs/formats/collision.md`.

use std::path::{Path, PathBuf};

use oag_formats::{collision, track, vex};

const PURE: &str = "data/images/pure-psp-usa.chd";
const PULSE: &str = "data/images/pulse-psp-usa.chd";

/// The executable each disc boots, an unencrypted ELF on both.
const BOOT: &str = "PSP_GAME/SYSDIR/BOOT.BIN";

/// The exporter class names, in the order both executables list them.
///
/// Read out of the binaries rather than typed from memory - see
/// [`the_two_titles_share_one_class_name_table`], which is what checks that this
/// list is what is actually there, on both discs, in this order.
const CLASS_NAMES: &[&str] = &[
    "Floor Collision",
    "Wall Collision",
    "WO Track",
    "Start Position",
    "Speedup Pad",
    "Weapon Pad",
    "Engine Flare",
    "Anim Transform",
    "Dynamic Point Light",
    "Dynamic Shadow Occluder",
    "ParticleSystem",
    "Airbrake",
    "Skycube",
    "Quake",
    "Trail",
    "section",
    "gate",
    "shadow",
    "speaker",
    "Reset Collision",
    "Ship Collision Fx",
    "wospot",
];

/// Which table index each *already recovered* Pulse constant sits at.
///
/// This is the fit's evidence and its constraint at once: 15 constants, each
/// recovered independently and long before this table was found, and every one of
/// them has to land where the run puts it.
fn pulse_anchors() -> Vec<(&'static str, u32)> {
    vec![
        ("Floor Collision", vex::CLASS_FLOOR_COLLISION),
        ("Wall Collision", vex::CLASS_WALL_COLLISION),
        ("WO Track", vex::CLASS_WO_TRACK),
        ("Start Position", vex::CLASS_START_POSITION),
        ("Speedup Pad", vex::CLASS_SPEEDUP_PAD),
        ("Weapon Pad", vex::CLASS_WEAPON_PAD),
        ("Engine Flare", vex::CLASS_ENGINE_FLARE),
        ("Dynamic Point Light", vex::CLASS_DYNAMIC_POINT_LIGHT),
        ("ParticleSystem", vex::CLASS_PARTICLE_SYSTEM),
        ("Airbrake", vex::CLASS_AIRBRAKE),
        ("Skycube", vex::CLASS_SKYCUBE),
        ("Trail", vex::CLASS_TRAIL),
        ("section", vex::CLASS_SECTION),
        ("Reset Collision", vex::CLASS_RESET_COLLISION),
        ("Ship Collision Fx", vex::CLASS_SHIP_COLLISION_FX),
    ]
}

fn workspace(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// `None`, with a printed reason, when the disc is not here - unless
/// `OAG_REQUIRE_GAME_DATA=1`, which turns absence into a failure.
fn image(name: &str) -> Option<String> {
    let path = workspace(name);
    if path.exists() {
        return Some(path.to_string_lossy().into_owned());
    }
    assert!(
        std::env::var("OAG_REQUIRE_GAME_DATA").as_deref() != Ok("1"),
        "{name} is required but absent"
    );
    println!("skipping: {name} not present");
    None
}

/// Reads a plain file off a disc image, by ISO path.
fn read_file(source: &str, path: &str) -> Vec<u8> {
    let mut image = oag_disc::DiscImage::open(source).expect("the image opens");
    image
        .read_file(path)
        .unwrap_or_else(|e| panic!("{source}:{path}: {e}"))
}

/// Every NUL-terminated ASCII string in `data`, starting at the one that begins
/// `first`, for as many as `count` asks for.
fn string_run(data: &[u8], first: &str, count: usize) -> Vec<String> {
    let needle = {
        let mut v = first.as_bytes().to_vec();
        v.push(0);
        v
    };
    let start = data
        .windows(needle.len())
        .position(|w| w == needle.as_slice())
        .unwrap_or_else(|| panic!("{first:?} is not in this executable"));

    let mut out = Vec::new();
    let mut at = start;
    while out.len() < count && at < data.len() {
        let end = match data[at..].iter().position(|&b| b == 0) {
            Some(z) => at + z,
            None => break,
        };
        if end > at
            && let Ok(text) = std::str::from_utf8(&data[at..end])
        {
            out.push(text.to_string());
        }
        at = end + 1;
    }
    out
}

/// The table, and that it is the *same* table on both discs.
///
/// The whole numbering rests on this: if the two executables listed these classes
/// in different orders, an index recovered on one would say nothing about the
/// other and the fit below would be numerology.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd and pulse-psp-usa.chd"]
fn the_two_titles_share_one_class_name_table() {
    let (Some(pulse), Some(pure)) = (image(PULSE), image(PURE)) else {
        return;
    };

    for (title, source) in [("Pulse", &pulse), ("Pure", &pure)] {
        let boot = read_file(source, BOOT);
        let run = string_run(&boot, CLASS_NAMES[0], CLASS_NAMES.len());
        assert_eq!(
            run, CLASS_NAMES,
            "{title}'s class-name run is not the documented one"
        );
    }

    // Pure has neither string at all, on either pressing. That is why `V4`
    // states `mag_floor_collision: None` and `cage_collision: None` as a
    // *finding* - Pure ships no magstrip and no cage - rather than as an
    // unrecovered gap. Pulse has both.
    let pulse_boot = read_file(&pulse, BOOT);
    let pure_boot = read_file(&pure, BOOT);
    for absent in ["Mag Floor Collision", "Cage Collision"] {
        let needle = {
            let mut v = absent.as_bytes().to_vec();
            v.push(0);
            v
        };
        let holds = |data: &[u8]| data.windows(needle.len()).any(|w| w == needle.as_slice());
        assert!(holds(&pulse_boot), "Pulse should name {absent:?}");
        assert!(
            !holds(&pure_boot),
            "Pure names {absent:?}, so it is not the absence V4 records"
        );
    }
}

/// The run is consecutive class IDs with three gaps, and Pulse's 15 recovered
/// constants all land on it.
///
/// The gaps are not free parameters fitted to taste: each is forced by the next
/// anchor after it, and the ID skipped at the first one is
/// [`vex::CLASS_TEXTURE`], a class the table does not name and the numbering
/// still contains. Two of Pulse's anchors set the gaps and the other 13 are
/// then checked against a mapping with nothing left to adjust.
#[test]
fn the_class_ids_are_the_table_index_with_three_gaps() {
    let anchors = pulse_anchors();
    let index_of = |name: &str| {
        CLASS_NAMES
            .iter()
            .position(|n| *n == name)
            .unwrap_or_else(|| panic!("{name:?} is in the table"))
    };

    // **Sorted by table index, not left in the order they were typed.** The fold
    // below accumulates a gap as it walks forward, so an anchor visited out of
    // index order would apply a gap that is not in effect yet and fail for a
    // reason that has nothing to do with the finding. It happened to be typed in
    // ascending order; that is not something a test should depend on.
    let mut anchors = anchors;
    anchors.sort_by_key(|(name, _)| index_of(name));

    let base = vex::CLASS_FLOOR_COLLISION;
    let mut gap = 0i64;
    let mut exact = 0;
    let mut shifts = Vec::new();

    for (name, id) in &anchors {
        let predicted = (base as i64) + index_of(name) as i64 + gap;
        if predicted == i64::from(*id) {
            exact += 1;
        } else {
            let by = i64::from(*id) - predicted;
            assert!(
                by > 0,
                "{name} sits *before* where the run puts it, which no insertion explains"
            );
            gap += by;
            shifts.push((index_of(name), by));
        }
    }

    assert_eq!(
        shifts,
        vec![
            (index_of("Dynamic Point Light"), 1),
            (index_of("Ship Collision Fx"), 2)
        ],
        "the gaps moved, so the fit is not the one the docs record"
    );
    assert_eq!(
        exact,
        anchors.len() - shifts.len(),
        "13 of Pulse's 15 recovered constants must land with nothing adjusted"
    );

    // The ID the first gap skips is `Texture`: present in the numbering, absent
    // from this table. Named here because it is the one gap whose *cause* is
    // known, and because Pure's own `Texture` was measured independently at
    // `0x373` - which is what turns the gap from a fudge into a prediction that
    // came true. See `the_derived_table_is_the_one_oag_formats_states`.
    assert_eq!(
        vex::CLASS_TEXTURE,
        vex::CLASS_ENGINE_FLARE + 2,
        "the skipped id after Anim Transform should be Texture"
    );
}

/// The table `oag-formats` states for version 4 is the one the run derives.
///
/// Anchored on `WO Track`, which `pure-status.md` pinned at `0x36d` on 16 of 16
/// tracks by the `WOtd` magic - measured before this table was found and with no
/// knowledge of it.
///
/// **`mesh` and `transform` are deliberately not checked here.** They are not
/// forgotten: `0x11e` and `0x6d` sit far outside this run's `0x36b..=0x383` span
/// because they belong to a *separate* registration table whose names are not in
/// this string run at all, and neither was recovered by index - each was pinned by
/// an exact property of its own payload (see `pure-status.md`'s table). Nothing
/// here predicts them, and a reader should not expect it to.
#[test]
fn the_derived_table_is_the_one_oag_formats_states() {
    let index_of = |name: &str| {
        CLASS_NAMES
            .iter()
            .position(|n| *n == name)
            .expect("in the table")
    };
    let v4 = vex::classes::V4;
    let anchor = v4.wo_track.expect("WO Track is recovered for version 4");
    let base = anchor - index_of("WO Track") as u32;

    // The same gap structure Pulse's anchors forced, applied unchanged.
    let id = |name: &str| {
        let i = index_of(name);
        let gap = u32::from(i >= index_of("Dynamic Point Light"))
            + 2 * u32::from(i >= index_of("Ship Collision Fx"));
        base + i as u32 + gap
    };

    assert_eq!(v4.floor_collision, Some(id("Floor Collision")));
    assert_eq!(v4.wall_collision, Some(id("Wall Collision")));
    assert_eq!(v4.reset_collision, Some(id("Reset Collision")));
    assert_eq!(v4.start_position, Some(id("Start Position")));
    assert_eq!(v4.speedup_pad, Some(id("Speedup Pad")));
    assert_eq!(v4.weapon_pad, Some(id("Weapon Pad")));
    assert_eq!(v4.engine_flare, Some(id("Engine Flare")));
    assert_eq!(v4.skycube, Some(id("Skycube")));
    assert_eq!(v4.section, Some(id("section")));
    assert_eq!(v4.ship_collision_fx, Some(id("Ship Collision Fx")));

    // The independent one. `Texture` is the ID the first gap skips, so the run
    // predicts `0x373` for it - and `0x373` is what 156 of Pure's 156 version-4
    // files measured on their own, by closing `sum(clut_size + texel_size)`
    // against each file's declared texture-block length. Two unrelated methods
    // agreeing on one number is the strongest single line in this file.
    assert_eq!(
        v4.texture,
        Some(base + index_of("Anim Transform") as u32 + 1),
        "the run's skipped id must be the texture class measured off the assets"
    );
    assert_eq!(v4.texture, Some(0x373));
}

/// Every class the run predicts is in Pure's own files, decoding as its name says.
///
/// The prediction could be arithmetically perfect and still name nothing real, so
/// this is the half that looks at the assets. Each assertion is a different kind
/// of check rather than the same node count three times.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn the_predicted_classes_are_in_pures_own_files() {
    let Some(source) = image(PURE) else { return };
    let mut archives = oag_pure::open(&source).expect("Pure's disc opens as Pure");

    let track_name = oag_pure::race::DEFAULT_TRACK;
    let blob = archives
        .read_name(track_name)
        .unwrap_or_else(|e| panic!("{track_name}: {e}"));
    let nodes = vex::nodes(&blob).expect("the node tree walks");
    let v4 = vex::classes::V4;
    assert_eq!(
        vex::classes_of(&blob).expect("a class table"),
        v4,
        "the default circuit should be a version-4 file"
    );

    let count =
        |class: Option<u32>| class.map_or(0, |c| nodes.iter().filter(|n| n.class_id == c).count());

    // Collision: all three classes present, and every payload closing to its own
    // declared length - `collision::from_vex` refuses a node that does not, so
    // reaching a non-empty result at all is the check.
    let collision = collision::from_vex(&blob).expect("the collision nodes decode");
    assert_eq!(
        collision.len(),
        3,
        "the circuit should carry one node of each of Pure's three collision classes"
    );
    for kind in [
        collision::SurfaceKind::Floor,
        collision::SurfaceKind::Wall,
        collision::SurfaceKind::Reset,
    ] {
        assert!(
            collision.iter().any(|node| node.kind == kind),
            "no {kind:?} node, so {:?} is not the id it decodes under",
            kind.class_id(v4)
        );
        assert!(
            !collision
                .iter()
                .filter(|node| node.kind == kind)
                .all(|node| node.geometry.meshes.is_empty()),
            "{kind:?} decoded to no objects"
        );
    }

    // `Start Position`: exactly one, and its payload decodes to a slot whose
    // forward axis is a unit vector. A wrong class would either find nothing or
    // decode to nonsense here.
    assert_eq!(count(v4.start_position), 1, "one authored grid node");
    let slot = nodes
        .iter()
        .find(|n| Some(n.class_id) == v4.start_position)
        .and_then(|n| track::start_position(&blob[n.payload()], vex::byte_order(&blob)))
        .expect("the Start Position payload decodes");
    let forward = slot.forward;
    let length =
        (forward[0] * forward[0] + forward[1] * forward[1] + forward[2] * forward[2]).sqrt();
    assert!(
        (length - 1.0).abs() < 1e-3,
        "the grid slot's forward is not a unit vector: {forward:?} has length {length}"
    );

    // The pads: geometry *and* trigger volumes decode off the same nodes by two
    // separate paths, which is the cross-check `docs/formats/pads.md` values on
    // Pulse. Both non-empty, and one volume per node.
    for (label, class) in [
        ("Speedup Pad", v4.speedup_pad),
        ("Weapon Pad", v4.weapon_pad),
    ] {
        let nodes_of = count(class);
        assert!(nodes_of > 0, "{label}: no nodes under the predicted class");
        let volumes = oag_formats::pads::volumes(&blob, &nodes, class.expect("checked"));
        assert_eq!(
            volumes.len(),
            nodes_of,
            "{label}: {nodes_of} node(s) but {} trigger volume(s)",
            volumes.len()
        );
    }

    assert!(count(v4.section) > 0, "no visibility sections");
    assert!(count(v4.skycube) > 0, "no sky");

    // The ship's locators, out of a different file under the same table. The
    // engine flare is the sharpest of these: the original authors exactly one,
    // centred on the hull's x axis and at its tail, which a wrong class cannot
    // fake.
    let ship = oag_pulse::race::ships::entry_name(oag_pure::race::DEFAULT_TEAM, "Ship");
    let ship_blob = archives
        .read_name(&ship)
        .unwrap_or_else(|e| panic!("{ship}: {e}"));
    let ship_nodes = vex::nodes(&ship_blob).expect("the ship's node tree walks");
    let flares =
        vex::class_world_transforms(&ship_blob, &ship_nodes, v4.engine_flare.expect("recovered"));
    assert_eq!(flares.len(), 1, "exactly one Engine Flare locator");
    let m = flares[0];
    assert!(
        m[12].abs() < 0.05,
        "the nozzle should be centred on x, and is at {}",
        m[12]
    );
    assert!(
        m[14] < 0.0,
        "the nozzle should be at the tail (-Z in model space), and is at {}",
        m[14]
    );

    assert!(
        !vex::class_world_transforms(
            &ship_blob,
            &ship_nodes,
            v4.ship_collision_fx.expect("recovered"),
        )
        .is_empty(),
        "no Ship Collision Fx locators"
    );
}
