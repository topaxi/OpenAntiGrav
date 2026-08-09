//! Enumerates Pure's `handlingstats.xml` schema in one pass and diffs it
//! against Pulse's, off both real discs.
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
//! `seam_ground_truth::pures_handlingstats_is_read_up_to_its_next_schema_difference`
//! found Pure's schema differences **one at a time**, each visible only once the
//! one before it had been handled: five rungs, then three absent attributes, then
//! no `<FE>`, then no `<pitch>`. Four rounds of the same discovery, and the last
//! two were not on `docs/formats/pure-status.md` at all.
//!
//! The reason that page could not predict them is worth stating, because it is a
//! property of surveys rather than a property of Pure: its per-layer confidences
//! came from **counting** - nodes walked, sizes closed, files identified - and a
//! count cannot see an element that is simply not there. Absence needs a
//! **comparison**, which is what this file is.
//!
//! So this walks every shipped file on both discs and compares the *sets* of
//! element and attribute names, which is the only form of the question that can
//! return "Pulse has one of these and Pure does not". It asserts the whole
//! symmetric difference at once, so the next schema difference is a failure here
//! naming what moved, rather than a panic somewhere downstream naming one field.
//!
//! # Names only
//!
//! Per `docs/architecture/adr/0006-no-copyrighted-content.md` nothing here reads
//! or reports an attribute *value*. Element and attribute names are a description
//! of the format; the numbers beside them are the game's design data.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use oag_formats::fexml::{self, Node};

/// Pure's disc. Serial `UCUS-98612`.
const PURE_IMAGE: &str = "data/images/pure-psp-usa.chd";
/// Pulse's, for the other half of every diff. Serial `UCUS-98712`.
const PULSE_IMAGE: &str = "data/images/pulse-psp-usa.chd";

/// `None`, with a printed reason, when a disc is not here - unless
/// `OAG_REQUIRE_GAME_DATA=1`, which turns absence into a failure.
fn image(relative: &str) -> Option<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    if path.exists() {
        return Some(path.to_string_lossy().into_owned());
    }
    assert!(
        std::env::var("OAG_REQUIRE_GAME_DATA").as_deref() != Ok("1"),
        "{relative} is required but absent"
    );
    println!("skipping: {relative} not present");
    None
}

/// Both discs, or nothing: a one-sided diff is not a diff.
fn both_images() -> Option<(String, String)> {
    Some((image(PURE_IMAGE)?, image(PULSE_IMAGE)?))
}

/// Every `Data\Ships\<dir>` the disc's own plugin definition names.
///
/// Read off the disc rather than hard-coded, because a hard-coded roster is the
/// thing this whole file exists to stop: a list written from memory cannot
/// surface a ship directory nobody thought of, and `Zone_01` is exactly that
/// case. `oag_formats::handling::TEAMS` is Pulse's list and does not apply here.
fn ship_directories(archives: &mut oag_assets::Archives, definition: &str) -> Vec<String> {
    let blob = archives
        .read_name(definition)
        .unwrap_or_else(|e| panic!("{definition}: {e}"));
    let expanded = text(&blob);
    let root = fexml::parse(&expanded);

    let mut out = BTreeSet::new();
    collect_ship_locations(&root, &mut out);
    out.into_iter().collect()
}

/// Walks for any `location` attribute naming a directory under `Data\Ships`.
///
/// Deliberately not keyed on the enclosing element name: Pure spells it
/// `<PI_Team><Values location=.../></PI_Team>` and there is no guarantee Pulse
/// nests it identically, so matching on the attribute keeps the walk title-blind.
fn collect_ship_locations(node: &Node, out: &mut BTreeSet<String>) {
    const PREFIX: &str = r"Data\Ships\";
    if let Some(location) = node.value("location")
        && let Some(rest) = location.strip_prefix(PREFIX)
        // Pulse also names per-ship `.dat` files under the same directories; only
        // the directory itself carries a `handlingstats.xml`.
        && !rest.contains('\\')
        && !rest.is_empty()
    {
        out.insert(rest.to_string());
    }
    for child in &node.children {
        collect_ship_locations(child, out);
    }
}

/// One document's element set: every element path under `<Handling>`, each
/// mapped to the union of the attribute names seen on it.
type Schema = BTreeMap<String, BTreeSet<String>>;

fn walk(node: &Node, path: &str, out: &mut Schema) {
    let here = format!("{path}/{}", node.name);
    let attrs = out.entry(here.clone()).or_default();
    for (name, _) in &node.attrs {
        attrs.insert(name.clone());
    }
    for child in &node.children {
        walk(child, &here, out);
    }
}

/// The `<Handling>` element, wherever the document root put it.
fn handling_root(node: &Node) -> Option<&Node> {
    if node.name.eq_ignore_ascii_case("Handling") {
        return Some(node);
    }
    node.children.iter().find_map(handling_root)
}

/// Expands a blob if it is shortened, and reads it as text if it is not.
fn text(blob: &[u8]) -> String {
    if fexml::is_fexml(blob) {
        fexml::expand(blob).expect("expands")
    } else {
        String::from_utf8_lossy(blob).into_owned()
    }
}

/// Folds one blob into an accumulating schema, rooted at `<Handling>`.
///
/// Case is normalised because element lookup in the decoder is case-insensitive
/// and the two releases disagree: PS2 Pulse spells `<Pitch>` where PSP Pulse
/// spells `<pitch>`. Comparing raw spellings would report that as a schema
/// difference, which it is not.
fn fold(blob: &[u8], out: &mut Schema) {
    let expanded = text(blob);
    let parsed = fexml::parse(&expanded);
    let root = handling_root(&parsed).expect("a <Handling> element");
    let mut one = Schema::new();
    walk(root, "", &mut one);
    for (path, attrs) in one {
        out.entry(path.to_ascii_lowercase())
            .or_default()
            .extend(attrs.into_iter().map(|a| a.to_ascii_lowercase()));
    }
}

/// Reads every ship directory's `handlingstats.xml` off one disc and folds them
/// into a single schema.
fn ship_schema(
    source: &str,
    open: fn(&str) -> oag_assets::Result<oag_assets::Archives>,
    definition: &str,
) -> (Schema, usize) {
    let mut archives = open(source).expect("the disc opens as its own title");
    let dirs = ship_directories(&mut archives, definition);
    assert!(!dirs.is_empty(), "{source}: no ship directories named");

    let mut schema = Schema::new();
    let mut read = 0;
    for dir in &dirs {
        let entry = format!(r"Data\Ships\{dir}\handlingstats.xml");
        // Not every named directory has to carry one - the claim worth making is
        // about the files that exist, and a directory without the file is itself
        // reportable rather than fatal.
        let Ok(blob) = archives.read_name(&entry) else {
            println!("{source}: {dir} names no handlingstats.xml");
            continue;
        };
        fold(&blob, &mut schema);
        read += 1;
    }
    println!(
        "{source}: {read} of {} ship directories carry one",
        dirs.len()
    );
    (schema, read)
}

/// Renders a schema as sorted `path[attr attr]` lines, for a failure message
/// that says what the disc holds rather than that two maps differ.
fn render(schema: &Schema) -> String {
    schema
        .iter()
        .map(|(path, attrs)| {
            format!(
                "{path} [{}]",
                attrs.iter().cloned().collect::<Vec<_>>().join(" ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The whole schema difference, asserted in one place.
///
/// Every entry is `path -> attributes present on one disc and not the other`; an
/// empty attribute set means the whole element is absent. The pairs are what
/// `docs/formats/pure-status.md` records, and the reason to assert the *exact*
/// map rather than "Pure is missing at least these" is that the weaker form is
/// what let four differences arrive one at a time.
fn expected_pulse_only() -> Vec<(String, Vec<String>)> {
    // In path order, which is the order `difference` walks a `BTreeMap` in.
    owned(vec![
        ("/handling/stats/class/airbrake", &["sideshift"]),
        // The live blocker. Feeds `oag_physics::Pitch`, so it stays required
        // until ADR-0009 item 2's gate opens - see `seam_ground_truth`.
        (
            "/handling/stats/class/pitch",
            &[
                "antigrav_height_adjust",
                "pitch_air",
                "pitch_damping",
                "pitch_ground",
            ],
        ),
        // Presentation: the ship-select bars. `Stats::fe` is an `Option` for this.
        (
            "/handling/stats/fe",
            &["handling", "shield", "speed", "thrust"],
        ),
        (
            "/handling/stats/misc",
            &["easyshield", "weight_distribution"],
        ),
    ])
}

/// The literal tables above, in the owned form [`difference`] produces.
fn owned(rows: Vec<(&str, &[&str])>) -> Vec<(String, Vec<String>)> {
    rows.into_iter()
        .map(|(path, attrs)| {
            (
                path.to_string(),
                attrs.iter().map(|a| (*a).to_string()).collect(),
            )
        })
        .collect()
}

/// Elements Pure authors that Pulse does not.
///
/// One entry, and it is a *shape* difference rather than a field: Pure's zone-mode
/// pseudo-team authors its parameter blocks directly on `<Stats>` with no
/// `<Class>` wrapper at all. See
/// [`pures_zone_mode_file_authors_its_blocks_outside_any_class`].
fn expected_pure_only() -> Vec<(String, Vec<String>)> {
    owned(vec![
        (
            "/handling/stats/airbrake",
            &["amount", "drag", "falloff", "gain", "turn"],
        ),
        (
            "/handling/stats/antigrav",
            &["grip_air", "grip_ground", "rebound", "ride_height"],
        ),
        ("/handling/stats/brakes", &["amount", "falloff", "gain"]),
        (
            "/handling/stats/engine",
            &["accelcap", "amount", "falloff", "gain", "turbo"],
        ),
        (
            "/handling/stats/physical",
            &["flight_gravity", "mass", "normal_gravity", "track_gravity"],
        ),
        ("/handling/stats/turning", &["amount", "falloff", "gain"]),
    ])
}

/// The same schema off Pure's **other pressing**.
///
/// The cheapest confidence available on this page, per
/// [the rubric](../../../docs/reverse-engineering/confidence-rubric.md): a second
/// binary is priced above a second reading of the first, and both discs were
/// already here. If the two pressings ever disagree, every claim above is about
/// one pressing rather than about the title, which is a distinction worth having
/// a test make rather than an assumption.
///
/// Values are not compared - only the element and attribute names - so this stays
/// inside `docs/architecture/adr/0006-no-copyrighted-content.md` the same way the
/// rest of the file does.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd and pure-psp-eu.chd"]
fn both_pure_pressings_carry_the_identical_schema() {
    let Some(usa) = image(PURE_IMAGE) else { return };
    let Some(eu) = image("data/images/pure-psp-eu.chd") else {
        return;
    };

    let (usa_schema, usa_files) = ship_schema(
        &usa,
        oag_pure::open,
        oag_pure::names::GAME_PLUGIN_DEFINITION,
    );
    let (eu_schema, eu_files) =
        ship_schema(&eu, oag_pure::open, oag_pure::names::GAME_PLUGIN_DEFINITION);

    assert_eq!(
        usa_files, eu_files,
        "the two pressings ship a different roster"
    );
    assert_eq!(
        render(&usa_schema),
        render(&eu_schema),
        "the two pressings ship a different handling schema"
    );
}

/// The one-pass diff this file exists for.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd and pulse-psp-usa.chd"]
fn pures_handling_schema_differs_from_pulses_in_exactly_these_ways() {
    let Some((pure, pulse)) = both_images() else {
        return;
    };

    let (pure_schema, pure_files) = ship_schema(
        &pure,
        oag_pure::open,
        oag_pure::names::GAME_PLUGIN_DEFINITION,
    );
    let (pulse_schema, pulse_files) = ship_schema(
        &pulse,
        oag_pulse::open,
        oag_pulse::names::GAME_PLUGIN_DEFINITION,
    );

    println!("pure:\n{}", render(&pure_schema));
    println!("pulse:\n{}", render(&pulse_schema));

    assert_eq!(
        pulse_files,
        oag_formats::handling::TEAMS.len(),
        "Pulse's roster is eight teams, and `handling::TEAMS` is the list this \
         walk should have rediscovered"
    );
    assert!(
        pure_files > pulse_files,
        "Pure was measured with more ship directories than Pulse, not fewer"
    );

    let missing = difference(&pulse_schema, &pure_schema);
    let extra = difference(&pure_schema, &pulse_schema);

    assert_eq!(
        missing,
        expected_pulse_only(),
        "what Pulse authors and Pure does not has moved"
    );
    assert_eq!(
        extra,
        expected_pure_only(),
        "what Pure authors and Pulse does not has moved"
    );
}

/// Everything in `left` that `right` does not have, as `path -> attributes`.
///
/// An element absent from `right` entirely yields all of its attributes, which is
/// why an empty attribute list never appears in the expectations above: a path
/// only reaches the result because something on it is missing.
fn difference(left: &Schema, right: &Schema) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for (path, attrs) in left {
        let theirs = right.get(path);
        let only: Vec<String> = attrs
            .iter()
            .filter(|a| theirs.is_none_or(|t| !t.contains(*a)))
            .cloned()
            .collect();
        if !only.is_empty() {
            out.push((path.clone(), only));
        }
    }
    out
}

/// The finding no survey of Pulse could have predicted: **one of Pure's files
/// has no `<Class>` blocks at all**.
///
/// `Data\Ships\Zone_01\handlingstats.xml` - `<Stats team="ZoneMode">` - authors
/// `<Engine>`, `<Brakes>`, `<Turning>`, `<Airbrake>`, `<Antigrav>` and
/// `<Physical>` directly on `<Stats>`, with no speed-class ladder over them, and
/// its `<Airbrake>` and `<Antigrav>` are shorter than the ones inside a `<Class>`.
///
/// This is the "a table read from data collapses two failures into one" shape in
/// a new place. The decoder's `classes()` iterates `<Class>` children, so on this
/// file it iterates nothing and the six blocks are **discarded with no
/// diagnostic** - a document with a whole parameter set in it reads as a document
/// with none. That is not a bug in this file's terms, because nothing has yet
/// decided what a classless ladder means, but a silent zero is the wrong shape
/// for the answer and this test is where the decision gets recorded when it is
/// made.
///
/// Pinned as it is, not as it should be, for the same reason
/// `seam_ground_truth` pins the `<pitch>` blocker: it fails the day someone
/// changes it, and makes them come back here and say what they decided.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn pures_zone_mode_file_authors_its_blocks_outside_any_class() {
    use oag_formats::handling;

    let Some(source) = image(PURE_IMAGE) else {
        return;
    };
    let mut archives = oag_pure::open(&source).expect("Pure's disc opens");

    let entry = oag_pure::names::handling_stats("Zone_01");
    let blob = archives
        .read_name(&entry)
        .unwrap_or_else(|e| panic!("{entry}: {e}"));

    let mut schema = Schema::new();
    fold(&blob, &mut schema);
    assert!(
        !schema.contains_key("/handling/stats/class"),
        "Zone_01 grew a <Class> block: {}",
        render(&schema)
    );
    for block in [
        "engine", "brakes", "turning", "airbrake", "antigrav", "physical",
    ] {
        assert!(
            schema.contains_key(&format!("/handling/stats/{block}")),
            "Zone_01 no longer authors <{block}> on <Stats>: {}",
            render(&schema)
        );
    }

    // And what the decoder currently makes of it: a clean parse that keeps the
    // cameras and the hull and throws the six blocks away.
    let stats = handling::from_blob(&blob).expect(
        "this file has no <pitch> to be blocked on, because it has no <Class> at \
         all - if it now fails, say why here",
    );
    assert!(
        stats.classes.is_empty(),
        "a classless file yielded {} class blocks",
        stats.classes.len()
    );
    assert!(
        !stats.has_pulse_class_ladder(),
        "an empty ladder must not read as Pulse's four"
    );
    assert!(
        stats.class(handling::SpeedClass::Venom).is_none(),
        "there is no rung to find"
    );
}

/// The second file with the same root element, which the per-team walk above
/// does not reach: `Data\XML\HandlingStats.xml`.
///
/// Pure ships it, it carries `<Global>`, and it **parses today** - which is worth
/// pinning precisely because nothing predicted that either. Its schema differs
/// from Pulse's in two places, neither of which the decoder reads:
///
/// - Pulse authors `<StartBoost>`; Pure has no such element.
/// - Pulse's `<WeaponPad>` carries `elimination_refresh_time` beside
///   `refresh_time`; Pure's carries only `refresh_time`. Pure has no elimination
///   mode, so this is the same "Pulse-era addition" direction as every other
///   difference here.
///
/// Both discs author five `<GlobalClass>` blocks with `VECTOR` first, which is
/// the property `handling::global_classes`'s skip depends on.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd and pulse-psp-usa.chd"]
fn the_global_handling_file_is_on_both_discs_and_differs_in_two_places() {
    use oag_formats::handling;

    let Some((pure, pulse)) = both_images() else {
        return;
    };

    let mut pure_schema = Schema::new();
    let mut pulse_schema = Schema::new();
    let mut pure_blob = Vec::new();
    for (source, open, schema, keep) in [
        (
            &pure,
            oag_pure::open as fn(&str) -> oag_assets::Result<oag_assets::Archives>,
            &mut pure_schema,
            true,
        ),
        (
            &pulse,
            oag_pulse::open as fn(&str) -> oag_assets::Result<oag_assets::Archives>,
            &mut pulse_schema,
            false,
        ),
    ] {
        let mut archives = open(source).expect("the disc opens as its own title");
        let blob = archives
            .read_name(handling::GLOBAL_ENTRY)
            .unwrap_or_else(|e| panic!("{source}: {} : {e}", handling::GLOBAL_ENTRY));
        fold(&blob, schema);
        if keep {
            pure_blob = blob;
        }
    }

    println!("pure global:\n{}", render(&pure_schema));
    println!("pulse global:\n{}", render(&pulse_schema));

    assert_eq!(
        difference(&pulse_schema, &pure_schema),
        vec![
            (
                "/handling/global/globalclass/weaponpad".to_string(),
                vec!["elimination_refresh_time".to_string()]
            ),
            (
                "/handling/global/startboost".to_string(),
                vec![
                    "boostmul".to_string(),
                    "normalmul".to_string(),
                    "overallduration".to_string(),
                    "stallend".to_string(),
                    "stallmul".to_string(),
                    "windowend".to_string(),
                    "windowstart".to_string(),
                ]
            ),
        ],
        "the global file's Pulse-only elements have moved"
    );
    assert!(
        difference(&pure_schema, &pulse_schema).is_empty(),
        "Pure's global file grew something Pulse has not: {:?}",
        difference(&pure_schema, &pulse_schema)
    );

    // The decoder reads neither of the two differing elements, so Pure's copy
    // goes through unchanged. This is the one part of the handling format that
    // needed no schema work at all for a second title.
    let global = handling::global_from_blob(&pure_blob)
        .expect("Pure's global file parses")
        .expect("and carries <Global>");
    for class in handling::SpeedClass::ALL {
        assert!(
            global.speedup_pads(class).time > 0.0,
            "{class}: a boost with no duration never applies"
        );
        assert!(
            global.gravity_mul(class).airborne > 0.0,
            "{class}: a non-positive gravity scale is not a ship"
        );
    }
}
