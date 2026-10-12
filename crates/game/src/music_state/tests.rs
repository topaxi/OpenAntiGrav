use super::*;

fn place(index: usize, second: u32) -> RaceMusicState {
    RaceMusicState {
        index,
        second,
        tracks: 16,
        track_seconds: 213.4,
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-music-state-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("music.toml")
}

#[test]
fn a_place_round_trips_per_title() {
    let path = scratch("round-trip");
    store_to(&path, "Wipeout Pulse", &place(3, 45)).unwrap();
    store_to(&path, "Wipeout HD", &place(7, 120)).unwrap();
    store_to(&path, "Wipeout Pulse", &place(4, 9)).unwrap();

    assert_eq!(load_from(&path, "Wipeout Pulse"), Some(place(4, 9)));
    assert_eq!(load_from(&path, "Wipeout HD"), Some(place(7, 120)));
    assert_eq!(load_from(&path, "Wipeout Pure"), None);
    assert!(
        !path.with_extension("toml.tmp").exists(),
        "the temp file is renamed away"
    );
}

#[test]
fn the_file_carries_its_schema_header() {
    let path = scratch("header");
    store_to(&path, "Wipeout Pulse", &place(1, 2)).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("schemas/v1/music-state.tosd"), "{text}");
    let table: toml::Table = text.parse().unwrap();
    let header: oag_schema::Header = table[oag_schema::TABLE].clone().try_into().unwrap();
    oag_schema::check(Some(&header), KIND, 1).unwrap();
}

#[test]
fn a_missing_foreign_or_broken_file_is_no_saved_place() {
    let path = scratch("broken");
    assert_eq!(load_from(&path, "Wipeout Pulse"), None);
    std::fs::write(&path, "not [ toml").unwrap();
    assert_eq!(load_from(&path, "Wipeout Pulse"), None);
    std::fs::write(
        &path,
        "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/pilot.tosd\", version = \"1.0.0\" }\n",
    )
    .unwrap();
    assert_eq!(load_from(&path, "Wipeout Pulse"), None);
    std::fs::write(&path, "[[playlist]]\ntitle = \"Wipeout Pulse\"\n").unwrap();
    assert_eq!(
        load_from(&path, "Wipeout Pulse"),
        None,
        "no header, refused by name"
    );
}

#[test]
fn the_shipped_example_reads() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/music-state.toml");
    assert_eq!(load_from(&path, "Wipeout Pulse"), Some(place(3, 45)));
}
