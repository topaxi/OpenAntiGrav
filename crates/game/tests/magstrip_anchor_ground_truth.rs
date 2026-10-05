//! The `arc_anchor_point` locator of every HD and Omega hull, read off a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(magstrip_anchor_ground_truth)'
//! ```
//!
//! `MagstripWake_Construct` resolves a node called `arc_anchor_point` against the
//! ship and warns `Ship has no arc_anchor_point locator` when it is missing, so
//! the claim a wiring member needs is that **every** hull of both titles carries
//! exactly one, and where it sits in the hull's model space. The claims here are
//! relational (ADR-0006): one node per file, a decoded matrix, and the anchor
//! sits on the hull's centreline (`x` within 0.01 of zero).

use oag_title::Title;

const TEAMS: &[&str] = &[
    "ag_systems",
    "assegai",
    "auricom",
    "egx",
    "feisar",
    "goteki",
    "harimau",
    "icaras",
    "mirage",
    "piranha",
    "qirex",
    "triakis",
    "detonator",
    "zone",
];

/// Every spelling of a hull directory: the base, and the two DLC variants.
fn hull_dirs() -> Vec<String> {
    TEAMS
        .iter()
        .flat_map(|t| [t.to_string(), format!("{t}_c1"), format!("{t}_n1")])
        .collect()
}

/// One report row per copy of `Locators.vex` that `spell` names, each carrying
/// the `arc_anchor_point` translation.
fn census(archives: &mut oag_assets::Archives, spell: &dyn Fn(&str) -> String) -> Vec<String> {
    let mut rows = Vec::new();
    for dir in hull_dirs() {
        let name = spell(&dir);
        for (at, blob) in archives.read_every_name(&name) {
            let nodes = oag_vex::vex::nodes(&blob).expect("the locators file parses");
            let anchors: Vec<_> = nodes
                .iter()
                .filter(|n| n.name.as_deref() == Some("arc_anchor_point"))
                .collect();
            assert_eq!(anchors.len(), 1, "{at} {name}: one anchor per hull");
            let class = anchors[0].class_id;
            let found = oag_vex::vex::matrix::named_class_world_transforms(&blob, &nodes, class);
            let (_, m) = found
                .iter()
                .find(|(n, _)| *n == Some("arc_anchor_point"))
                .expect("the anchor decodes as a matrix");
            assert!(
                m[12].abs() < 0.01,
                "{at} {name}: the anchor sits off the hull's centreline: {}",
                m[12]
            );
            rows.push(format!(
                "{at} {dir} class={class} t=({:.4}, {:.4}, {:.4})",
                m[12], m[13], m[14]
            ));
        }
    }
    rows
}

fn report(label: &str, rows: &[String], title: &'static Title) {
    println!("{label} ({}): {} copies", title.name, rows.len());
    for row in rows {
        println!("  {row}");
    }
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_hd_hull_carries_one_arc_anchor_point() {
    let Some(path) = oag_testdata::exact("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_hd::TITLE)
        .expect("the archives open");
    let rows = census(&mut archives, &|d| format!("Data/Ships/{d}/Locators.vex"));
    report("hd", &rows, oag_hd::TITLE);
    assert!(
        rows.len() >= 38,
        "every shipped hull was read: {}",
        rows.len()
    );
}

#[test]
#[ignore = "needs the Omega PS4 packages extracted under data/extracted/ps4"]
fn every_omega_hull_carries_one_arc_anchor_point() {
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&source.display().to_string()).expect("open omega");
    let rows = census(&mut archives, &|d| {
        format!("Data/art/published/hdships/{d}/Locators.vex")
    });
    report("omega", &rows, oag_omega::TITLE);
    assert!(
        rows.len() >= 38,
        "every shipped hull was read: {}",
        rows.len()
    );
}
