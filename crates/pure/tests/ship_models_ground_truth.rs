//! Which per-craft models Pure loads, and the one Pulse loads that it does not.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this pins
//!
//! Pure's executable holds its per-craft model paths as five `printf` templates
//! in one contiguous `.rodata` block inside `Ship.cpp`'s own strings, at
//! `0x08a7a5d4`..`0x08a7a61c` on `psp-pure-usa` - read byte for byte, not
//! inferred. **None of the five is a boost plume**, where Pulse carries
//! `%s\%sboost.vex` at `0x08a84ccc`.
//!
//! Three assertions come out of that, and each is a *matrix* rather than a
//! single probe, because a name that misses proves nothing on its own:
//!
//! 1. [`the_five_ship_model_templates_resolve_where_the_definition_says`] -
//!    every one of the eleven `<PI_Team>` locations the disc's own
//!    `Definition.xml` declares, crossed with the five templates, on both
//!    pressings. The misses are as much the finding as the hits: only the eight
//!    core racing teams carry a `Phantom` variant.
//! 2. [`each_model_names_its_own_authoring_path_back`] - the recovered name is
//!    checked against the file's *own* embedded Maya source path
//!    (`Z:/Data/Ships/<Team>/<Leaf>.mb`), so the hash match is corroborated by
//!    something inside the payload rather than by itself.
//! 3. [`pure_ships_no_boost_plume_asset_and_pulse_does`] - the negative, pinned
//!    against Pulse's positive in the same run. Pulse's `shipboost.vex` resolves
//!    and its craft models carry `boost_flare` locator nodes; Pure's resolves
//!    nowhere and no Pure craft model carries such a node.
//!
//! Technique: string sweep over the extracted `BOOT.BIN` plus CRC name-hash
//! matching against the archive directory, and byte-level reads at the pinned
//! addresses. **Ghidra cross-references were never consulted** - the unapplied
//! PSP relocation makes `get_xrefs_to` empty on every PSP database regardless of
//! what references a target, so an empty xref list would have proved nothing.
//!
//! See `docs/formats/pure-status.md` and `crates/pure/src/race.rs`'s `ships`
//! module.

use oag_tables::fexml;

const PURE_USA: &str = "data/images/pure-psp-usa.chd";
const PURE_EU: &str = "data/images/pure-psp-eu.chd";
const PULSE_USA: &str = "data/images/pulse-psp-usa.chd";

/// The eight teams that carry every one of the five templates.
///
/// Read off the disc by [`team_locations`] rather than trusted from here; this
/// list is only what the assertion compares against, and its own docs say why
/// each of the other three is absent from it.
const CORE_TEAMS: &[&str] = &[
    r"Data\Ships\AG_Systems",
    r"Data\Ships\Assegai",
    r"Data\Ships\Auricom",
    r"Data\Ships\Feisar",
    r"Data\Ships\Harimau",
    r"Data\Ships\Piranha",
    r"Data\Ships\Qirex",
    r"Data\Ships\Triakis",
];

/// `None`, with a printed reason, when the disc is not here - unless
/// `OAG_REQUIRE_GAME_DATA=1`, which turns absence into a failure.
fn image(name: &str) -> Option<String> {
    oag_testdata::image(name).map(|path| path.display().to_string())
}

/// Every `<PI_Team>` location the disc's own plugin definition declares.
///
/// Read off the disc rather than listed here, for the same reason
/// `class_table_ground_truth.rs`'s `circuits` helper is: a roster in this
/// repository would be shipped content. It is also strictly better than the
/// Pulse-derived team names other tests here still cross against - those miss
/// `Harimau` and `Medievil`, which are Pure's own and not Pulse's.
fn team_locations(archives: &mut oag_assets::Archives) -> Vec<String> {
    let Ok(blob) = archives.read_name(oag_pure::names::GAME_PLUGIN_DEFINITION) else {
        return Vec::new();
    };
    let Ok(xml) = fexml::text(&blob) else {
        return Vec::new();
    };
    let root = fexml::parse(&xml);
    let mut out = Vec::new();
    fn collect(node: &fexml::Node, out: &mut Vec<String>) {
        if node.name.eq_ignore_ascii_case("PI_Team") {
            for child in &node.children {
                if let Some(location) = child.value("location") {
                    out.push(location.to_owned());
                }
            }
        }
        for child in &node.children {
            collect(child, out);
        }
    }
    collect(&root, &mut out);
    out.sort();
    out.dedup();
    out
}

/// Every one of the five templates, against every declared team, on both discs.
///
/// The hit matrix is the measurement. `Ship.vex` and `Shipwreck.vex` are on all
/// eleven; `VR\Ship.vex` on ten (`Zone_01` has none); `Phantom.vex` and
/// `Phantom_shipwreck.vex` on the eight core racing teams only, and not on
/// `Medievil`, `Zone` or `Zone_01`. Asserting the shape rather than a count is
/// what makes a template that stopped resolving, or one that started resolving
/// somewhere new, fail here rather than pass quietly.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd and pure-psp-eu.chd"]
fn the_five_ship_model_templates_resolve_where_the_definition_says() {
    for source in [PURE_USA, PURE_EU] {
        let Some(source) = image(source) else {
            continue;
        };
        let mut archives =
            oag_assets::Archives::open(&source, oag_pure::TITLE).expect("the archives open");
        let teams = team_locations(&mut archives);
        assert_eq!(
            teams.len(),
            11,
            "{source}: the definition declares eleven <PI_Team> locations"
        );

        for team in &teams {
            for leaf in [oag_pure::race::ships::HULL, oag_pure::race::ships::WRECK] {
                let name = format!(r"{team}\{leaf}.vex");
                assert!(
                    archives.locate(&name).is_some(),
                    "{source}: {name} should resolve - every declared team carries a hull \
                     and a wreck"
                );
            }

            let vr = format!(r"{team}\{}.vex", oag_pure::race::ships::VR_HULL);
            assert_eq!(
                archives.locate(&vr).is_some(),
                team != r"Data\Ships\Zone_01",
                "{source}: {vr} - every declared team but the Zone-mode craft carries a \
                 VR hull"
            );

            let core = CORE_TEAMS.contains(&team.as_str());
            for leaf in [
                oag_pure::race::ships::PHANTOM_HULL,
                oag_pure::race::ships::PHANTOM_WRECK,
            ] {
                let name = format!(r"{team}\{leaf}.vex");
                assert_eq!(
                    archives.locate(&name).is_some(),
                    core,
                    "{source}: {name} - a Phantom-class variant exists for the eight core \
                     racing teams and for nothing else"
                );
            }
        }
    }
}

/// Every model's own payload names the path it was recovered under.
///
/// A `.vex` keeps the exporter's source path as a literal - `Z:/Data/Ships/
/// Feisar/Phantom.mb` inside `Data\Ships\Feisar\Phantom.vex`. That makes the
/// name-hash match self-certifying: the archive answered to a name this project
/// composed, and the blob it handed back spells the same team and the same leaf
/// in its own words. A CRC collision, or a template read wrong, would not.
///
/// `VR\Ship.vex` is included deliberately - it is the one template with a
/// directory in it, so it is the one a mis-split would break.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn each_model_names_its_own_authoring_path_back() {
    let Some(source) = image(PURE_USA) else {
        return;
    };
    let mut archives =
        oag_assets::Archives::open(&source, oag_pure::TITLE).expect("the archives open");

    for team in ["Feisar", "Assegai", "Qirex"] {
        for leaf in [
            oag_pure::race::ships::HULL,
            oag_pure::race::ships::WRECK,
            oag_pure::race::ships::PHANTOM_HULL,
            oag_pure::race::ships::PHANTOM_WRECK,
            oag_pure::race::ships::VR_HULL,
        ] {
            let name = format!(r"Data\Ships\{team}\{leaf}.vex");
            let blob = archives
                .read_name(&name)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let want = format!("Z:/Data/Ships/{team}/{leaf}.mb").replace('\\', "/");
            assert!(
                contains_ascii(&blob, want.as_bytes()),
                "{name} should carry its own authoring path {want}"
            );
        }
    }
}

/// Pure ships no boost-plume asset; Pulse ships one per craft.
///
/// Three axes, and the Pulse half of each is what makes the Pure half a
/// measurement rather than a name that was spelled wrong:
///
/// 1. **The archive.** `<team>\shipboost.vex` resolves on Pulse for every team
///    the same probe reaches, and on Pure for none of the eleven its own
///    definition declares.
/// 2. **The models.** Pulse's craft hulls carry `boost_flare` locator nodes -
///    the anchors `docs/ghidra/functions/psp-pulse-usa/exhaust.md` documents the
///    plume riding. No Pure hull carries a node whose name contains `boost` at
///    all.
/// 3. **The executable** is asserted in `race.rs`'s `ships` docs rather than
///    here, since it needs no archive: Pulse's `%s\%sboost.vex` at `0x08a84ccc`
///    has no Pure counterpart, and the only two `boost`-bearing strings on
///    Pure's whole 3.6 MiB `BOOT.BIN` are `HUD_Perfect Boost!` (`0x08a46398`)
///    and `StartBoostSpeed` (`0x08a79bc8`), neither of them a path.
///
/// What this does **not** assert: that Pure's boost is invisible. Pure carries
/// `Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip` exactly as Pulse does,
/// and a boost that brightens the existing `engine_flare` node in code would
/// leave no asset for any of these three axes to find.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd and pulse-psp-usa.chd"]
fn pure_ships_no_boost_plume_asset_and_pulse_does() {
    let (Some(pure), Some(pulse)) = (image(PURE_USA), image(PULSE_USA)) else {
        return;
    };

    let mut pulse_archives =
        oag_assets::Archives::open(&pulse, oag_pulse::TITLE).expect("Pulse's archives open");
    let pulse_boost = format!(
        r"Data\Ships\{}\{}.vex",
        oag_pulse::race::DEFAULT_TEAM,
        oag_pulse::race::ships::BOOST
    );
    assert!(
        pulse_archives.locate(&pulse_boost).is_some(),
        "{pulse_boost} must resolve, or this test's negative half proves nothing"
    );
    let pulse_hull = format!(
        r"Data\Ships\{}\{}.vex",
        oag_pulse::race::DEFAULT_TEAM,
        oag_pulse::race::ships::HULL
    );
    let blob = pulse_archives.read_name(&pulse_hull).expect("Pulse's hull");
    assert!(
        contains_ascii(&blob, b"boost_flare"),
        "{pulse_hull} carries the plume's anchor nodes - see exhaust.md"
    );

    let mut archives =
        oag_assets::Archives::open(&pure, oag_pure::TITLE).expect("Pure's archives open");
    let teams = team_locations(&mut archives);
    assert_eq!(teams.len(), 11, "the definition declares eleven teams");

    for team in &teams {
        let name = format!(r"{team}\{}.vex", oag_pulse::race::ships::BOOST);
        assert!(
            archives.locate(&name).is_none(),
            "{name} resolves on Pure - if this ever fires, the plume was found and \
             race.rs's `ships` docs are out of date"
        );

        let hull = format!(r"{team}\{}.vex", oag_pure::race::ships::HULL);
        let blob = archives
            .read_name(&hull)
            .unwrap_or_else(|e| panic!("{hull}: {e}"));
        assert!(
            !contains_ascii_ignore_case(&blob, b"boost"),
            "{hull} names a boost node - Pure's hulls carry `engine_flare` and no \
             plume anchor"
        );
    }
}

/// Whether `haystack` contains `needle` as a byte run.
fn contains_ascii(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// As [`contains_ascii`], folding ASCII case on both sides.
fn contains_ascii_ignore_case(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w.eq_ignore_ascii_case(needle))
}
