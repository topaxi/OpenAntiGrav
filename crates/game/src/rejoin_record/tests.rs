use super::*;

fn rejoin() -> Rejoin {
    Rejoin {
        addr: "race.example.org:40123".into(),
        cert_hash: [7; 32],
        room: 3,
        team: "AG Systems".into(),
        name: "pilot".into(),
        token: "token-1".into(),
    }
}

fn scratch(name: &str) -> oag_testdata::scratch::TempDir {
    oag_testdata::scratch::dir(&format!("rejoin-record-{name}")).child("rejoin.toml")
}

#[test]
fn a_fresh_seat_round_trips() {
    let path = scratch("round-trip");
    store_to(&path, "Wipeout Pulse", &rejoin(), 1_000).unwrap();
    assert_eq!(
        load_from(&path, "Wipeout Pulse", 1_000 + HOLD_SECONDS - 1),
        Some(rejoin())
    );
    assert!(
        !path.with_extension("toml.tmp").exists(),
        "the temp file is renamed away"
    );
}

#[test]
fn the_file_carries_its_schema_header() {
    let path = scratch("header");
    store_to(&path, "Wipeout Pulse", &rejoin(), 5).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("schemas/v1/rejoin.tosd"), "{text}");
    let table: toml::Table = text.parse().unwrap();
    let header: oag_schema::Header = table[oag_schema::TABLE].clone().try_into().unwrap();
    oag_schema::check(Some(&header), KIND, 1).unwrap();
}

#[test]
fn an_expired_seat_is_discarded_without_an_offer() {
    let path = scratch("expired");
    store_to(&path, "Wipeout Pulse", &rejoin(), 1_000).unwrap();
    assert_eq!(
        load_from(&path, "Wipeout Pulse", 1_000 + HOLD_SECONDS),
        None
    );
    assert!(!path.exists(), "and the record is deleted");
}

#[test]
fn another_titles_seat_is_not_offered() {
    let path = scratch("title");
    store_to(&path, "Wipeout HD", &rejoin(), 1_000).unwrap();
    assert_eq!(load_from(&path, "Wipeout Pulse", 1_001), None);
    assert!(!path.exists());
}

#[test]
fn a_missing_foreign_or_broken_file_is_no_seat() {
    let path = scratch("broken");
    assert_eq!(load_from(&path, "Wipeout Pulse", 1), None);
    std::fs::write(&path, "not [ toml").unwrap();
    assert_eq!(load_from(&path, "Wipeout Pulse", 1), None);
    std::fs::write(
        &path,
        "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/pilot.tosd\", version = \"1.0.0\" }\n",
    )
    .unwrap();
    assert_eq!(load_from(&path, "Wipeout Pulse", 1), None);
}

#[test]
fn the_shipped_fixture_loads() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rejoin.toml");
    let path = scratch("fixture");
    std::fs::copy(fixture, &path).unwrap();
    let seat = load_from(&path, "Wipeout Pulse", 1_700_000_000).expect("a fresh seat");
    assert_eq!(seat.token, "token-1");
    assert_eq!(seat.room, 7);
}

#[test]
fn clearing_a_missing_record_is_quiet() {
    let path = scratch("clear");
    clear_at(&path);
    store_to(&path, "Wipeout Pulse", &rejoin(), 1).unwrap();
    clear_at(&path);
    assert!(!path.exists());
}

#[test]
fn a_server_that_names_no_title_fits_any_title() {
    let path = scratch("untitled");
    store_to(&path, "", &rejoin(), 1_000).unwrap();
    assert_eq!(load_from(&path, "Wipeout Pulse", 1_001), Some(rejoin()));
}
