//! Our own rumble patterns (`assets/fallback/Data/XML/Rumble/`) read, and cover
//! every pattern the event map names.

use oag_input::rumble::map::{ROWS, UNFIRED};
use oag_tables::rumble::Library;

const DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/fallback/Data/XML/Rumble"
);

fn library() -> Library {
    let mut library = Library::new();
    for entry in std::fs::read_dir(DIR).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        let pattern =
            oag_mod::rumble::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
        library.insert(&name, pattern);
    }
    library
}

#[test]
fn every_file_reads() {
    assert!(library().len() >= 20);
}

#[test]
fn every_row_of_the_map_has_a_pattern() {
    let library = library();
    for row in ROWS {
        assert!(library.index_of(row.pattern).is_some(), "{}", row.pattern);
    }
}

#[test]
fn no_pattern_is_authored_for_a_name_nothing_fires() {
    let library = library();
    for (name, _) in UNFIRED {
        assert!(library.index_of(name).is_none(), "{name}");
    }
}

#[test]
fn the_large_motor_carries_each_pattern() {
    for (name, p) in library().iter() {
        let peak = p.large.iter().copied().max().unwrap_or(0);
        assert!(peak >= 90, "{name} peaks at {peak}");
        assert!(p.large.len() <= 64, "{name}");
    }
}
