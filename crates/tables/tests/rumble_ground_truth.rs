//! The rumble patterns HD (disc and PSN), 2048 and Omega ship: 69 each, one
//! [`Pattern`] each, and the same 69 on all four.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. The check is [`census`], called from
//! `ai_stats_lineage_ground_truth::assert_same_as_pulse` so each archive is
//! opened once for both families; it reads only the `rumble` entries.
//!
//! "The same 69" is one digest committed here and asserted on every archive:
//! four archives with one digest are four archives with equal patterns, and no
//! pattern value is written in this file. The layout and the census numbers are
//! on `docs/formats/rumble-xml.md`.

use oag_assets::psarc::Archive;
use oag_tables::rumble::{self, Class, Kind, Library, Pattern, Priority};

/// FNV-1a over every pattern in name order (lower-cased name, every field, both
/// tracks), so a changed or missing pattern changes it.
const DIGEST: u64 = 8_308_538_667_236_026_183;

fn fnv(hash: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *hash ^= u64::from(b);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

fn digest(library: &Library) -> u64 {
    let mut rows: Vec<(&str, &Pattern)> = library.iter().collect();
    rows.sort_by_key(|(name, _)| *name);
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for (name, p) in rows {
        fnv(&mut hash, name.as_bytes());
        let kind = match p.kind {
            Kind::OneShot => 0,
            Kind::Looping => 1,
        };
        let class = p.class.map_or(0, |c| 1 + c as u8);
        let priority = p.priority.map_or(0, |c| 1 + c as u8);
        fnv(
            &mut hash,
            &[0, kind, class, priority, u8::from(p.positional.is_some())],
        );
        fnv(&mut hash, &p.loop_start.to_le_bytes());
        fnv(&mut hash, &p.loop_end.to_le_bytes());
        if let Some(f) = p.positional {
            fnv(&mut hash, &f.inner.to_bits().to_le_bytes());
            fnv(&mut hash, &f.outer.to_bits().to_le_bytes());
        }
        fnv(&mut hash, &(p.large.len() as u32).to_le_bytes());
        fnv(&mut hash, &p.large);
        fnv(&mut hash, &(p.small.len() as u32).to_le_bytes());
        fnv(&mut hash, &p.small);
    }
    hash
}

/// Reads every rumble pattern of `archive` and asserts the census.
pub fn census(label: &str, archive: &mut Archive) {
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| rumble::pattern_name(p).is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("xml")))
        .cloned()
        .collect();
    let mut library = Library::new();
    for path in &paths {
        let (name, _) = rumble::pattern_name(path).expect("filtered above");
        let blob = archive
            .read_path(path)
            .unwrap_or_else(|e| panic!("{label} {path}: {e}"));
        let pattern =
            rumble::parse(&blob).unwrap_or_else(|e| panic!("{label} {path}: refused: {e}"));
        assert!(
            library.insert(name, pattern).is_none(),
            "{label} {path}: twice"
        );
    }

    let all: Vec<&Pattern> = library.iter().map(|(_, p)| p).collect();
    let count = |f: &dyn Fn(&Pattern) -> bool| all.iter().filter(|p| f(p)).count();
    assert_eq!((paths.len(), library.len()), (69, 69), "{label}: patterns");
    assert_eq!(count(&|p| p.kind == Kind::OneShot), 57, "{label}: OneShot");
    assert_eq!(count(&|p| p.kind == Kind::Looping), 12, "{label}: Looping");
    assert_eq!(count(&|p| p.positional.is_some()), 7, "{label}: Positional");
    assert_eq!(
        count(&|p| p.class == Some(Class::Game)),
        62,
        "{label}: Game"
    );
    assert_eq!(count(&|p| p.class == Some(Class::Ui)), 3, "{label}: UI");
    assert_eq!(count(&|p| p.class.is_none()), 4, "{label}: no Class");
    assert_eq!(
        count(&|p| p.priority == Some(Priority::High)),
        25,
        "{label}: High"
    );
    assert_eq!(
        count(&|p| p.priority == Some(Priority::Medium)),
        25,
        "{label}: Medium"
    );
    assert_eq!(
        count(&|p| p.priority == Some(Priority::Low)),
        15,
        "{label}: Low"
    );
    assert_eq!(count(&|p| p.priority.is_none()), 4, "{label}: no Priority");
    assert_eq!(
        count(&|p| p.large.len() != p.small.len()),
        24,
        "{label}: unequal tracks"
    );
    assert!(
        all.iter().all(|p| p.small.iter().all(|&b| b <= 1)),
        "{label}: small > 1"
    );
    let ones: usize = all
        .iter()
        .map(|p| p.small.iter().filter(|&&b| b == 1).count())
        .sum();
    let zeros: usize = all
        .iter()
        .map(|p| p.small.iter().filter(|&&b| b == 0).count())
        .sum();
    assert_eq!((ones, zeros), (476, 1085), "{label}: small-motor frames");
    assert!(
        all.iter().all(|p| p.frames() <= 106 && p.frames() >= 1),
        "{label}: frames"
    );
    assert_eq!(
        digest(&library),
        DIGEST,
        "{label}: not the same 69 as the others"
    );
}
