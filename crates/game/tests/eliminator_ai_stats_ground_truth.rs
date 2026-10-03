//! The `<EliminatorAIStats>` and `<AllWeapons>` rows of every title's
//! `WeaponAIstats.xml`, read off a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(eliminator_ai_stats_ground_truth)'
//! ```
//!
//! The census is every copy of the table in every archive a title's
//! [`oag_assets::Archives`] opens (patches and DLC included), under every
//! spelling the title names or ships. Each copy must parse - the parser
//! refuses a short row by name, so a parse is a decode - and the claims are
//! relational, never a literal (ADR-0006): the row exists where the title is
//! HD-lineage and not on Pulse or Pure, a copy is byte-identical to its
//! siblings, and each tier triple is monotonic.

use oag_tables::weapons::ai::{self, EliminatorAiStats};
use oag_title::Title;

/// Every copy of `names` in every archive of `source`, as `(where, blob)`.
fn copies(source: &str, title: &'static Title, names: &[&str]) -> Vec<(String, Vec<u8>)> {
    let mut archives = oag_assets::Archives::open(source, title).expect("open the title");
    names
        .iter()
        .flat_map(|name| {
            archives
                .read_every_name(name)
                .into_iter()
                .map(|(at, blob)| (format!("{at} {name}"), blob))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn assert_shape(e: &EliminatorAiStats) {
    assert!(e.flip_scale[0] < e.flip_scale[1] && e.flip_scale[1] < e.flip_scale[2]);
    assert!(e.use_scale[0] < e.use_scale[1] && e.use_scale[1] < e.use_scale[2]);
    assert!(e.score_scale[0] < e.score_scale[1] && e.score_scale[1] < e.score_scale[2]);
    assert!(
        e.absorb_scale[0] > e.absorb_scale[1] && e.absorb_scale[1] > e.absorb_scale[2],
        "absorb is the one scale that falls with difficulty: {:?}",
        e.absorb_scale
    );
    assert!(e.infront_flip > e.normal_flip);
}

/// Parses every copy and returns, per copy, whether it authors the two extra
/// rows. A copy with the Eliminator row must decode it completely (the parser
/// refuses a short row by name) and match its siblings that carry it.
fn check(found: &[(String, Vec<u8>)], count: usize) -> Vec<Option<EliminatorAiStats>> {
    assert_eq!(found.len(), count, "census: {:?}", labels(found));
    let mut first: Option<EliminatorAiStats> = None;
    let mut rows = Vec::new();
    for (label, blob) in found {
        let stats = ai::from_blob(blob).unwrap_or_else(|e| panic!("{label}: {e}"));
        assert!(stats.all_weapons().is_some(), "{label}: no AllWeapons row");
        let e = stats.eliminator();
        if let Some(e) = e {
            assert_shape(&e);
            assert_eq!(*first.get_or_insert(e), e, "{label} differs from its sibling");
        }
        println!("{label}: eliminator row {e:?}");
        rows.push(e);
    }
    rows
}

fn labels(found: &[(String, Vec<u8>)]) -> Vec<&str> {
    found.iter().map(|(l, _)| l.as_str()).collect()
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_ships_two_copies() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let found = copies(
        &image.display().to_string(),
        &oag_hd::TITLE,
        &[r"Data\XML\WeaponAIstats.xml"],
    );
    let rows = check(&found, 2);
    assert_eq!(rows.iter().flatten().count(), 1, "{rows:?}");
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn two_thousand_forty_eight_ships_the_same_row_under_both_spellings() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let source = path.display().to_string();
    let found = copies(
        &source,
        &oag_2048::TITLE,
        &[
            r"Data\XML\WeaponAIStats.xml",
            r"Data\XML\WeaponAIStats2048.xml",
        ],
    );
    let rows = check(&found, 2);
    assert_eq!(rows.iter().flatten().count(), 2, "{rows:?}");
    assert_eq!(found[0].1, found[1].1, "the two spellings differ");

    let hd = oag_testdata::image("hdfury-ps3-eu-dec.iso").map(|image| {
        copies(
            &image.display().to_string(),
            &oag_hd::TITLE,
            &[r"Data\XML\WeaponAIstats.xml"],
        )
    });
    if let Some(hd) = hd {
        let hd_rows = check(&hd, 2);
        let with = hd_rows.iter().position(Option::is_some).expect("a copy");
        assert_eq!(hd_rows[with], rows[0], "HD's row and 2048's differ");
    }
}

#[test]
#[ignore = "needs the extracted Omega packages in data/extracted/ps4/"]
fn omega_ships_the_same_row() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let found = copies(
        &path.display().to_string(),
        &oag_omega::TITLE,
        &[
            r"Data\XML\WeaponAIstats.xml",
            r"Data\XML\WeaponAIStats2048.xml",
        ],
    );
    println!("omega census: {:?}", labels(&found));
    assert!(!found.is_empty(), "Omega ships no weapon-AI table");
    let rows = check(&found, found.len());
    assert!(rows.iter().any(Option::is_some), "no Omega copy has the row");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_and_pure_author_neither_row() {
    for (image, title) in [
        ("pulse-psp-usa.chd", &oag_pulse::TITLE),
        ("pure-psp-usa.chd", &oag_pure::TITLE),
    ] {
        let Some(path) = oag_testdata::image(image) else {
            continue;
        };
        let found = copies(
            &path.display().to_string(),
            title,
            &[r"Data\XML\WeaponAIstats.xml"],
        );
        assert!(!found.is_empty(), "{image}: no table");
        for (label, blob) in &found {
            let stats = ai::from_blob(blob).unwrap_or_else(|e| panic!("{label}: {e}"));
            assert!(stats.eliminator().is_none(), "{label}");
            assert!(stats.all_weapons().is_none(), "{label}");
        }
    }
}
