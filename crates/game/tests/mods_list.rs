//! The MODS list (`oag_game::mods::expand` on the shipped `mods` page): one row per mod with
//! its switch, the mods that are on first, Square writing the switch, and the detail page a row
//! opens. The rows' mechanics are `oag_ui`'s (`menu/tests/items.rs`).
//!
//! The listings are built here rather than read off a mods directory, so nothing touches disk.

use oag_game::mods::{self, Listing};
use oag_ui::language::StringTable;
use oag_ui::menu::{Definition, Entry, Menu, MenuEvent, Value, ValueSource};

use oag_core::buttons::{Button, Input};

fn listing(id: &str, name: &str, enabled: Option<bool>) -> Listing {
    Listing {
        id: id.to_string(),
        name: name.to_string(),
        version: "1.0.0".to_string(),
        authors: vec!["A. Modder".to_string()],
        license: enabled.map(|_| oag_mod::License::None),
        rows: Vec::new(),
        builtin: id == oag_mod::extras::ID,
        enabled,
        default_enabled: true,
        texture_pack: false,
        error: enabled.is_none().then(|| "not a zip".to_string()),
        problems: Vec::new(),
        publishes: Vec::new(),
        listens: Vec::new(),
    }
}

/// The built-in, an off mod, two on ones, another off one and a candidate that never opened.
fn five() -> Vec<Listing> {
    vec![
        listing(oag_mod::extras::ID, "OpenAntiGrav extras", Some(true)),
        listing("test.off", "Off", Some(false)),
        listing("test.on", "On", Some(true)),
        listing(
            "test.long",
            "A mod with a very long name indeed",
            Some(true),
        ),
        listing("test.also-off", "Also off", Some(false)),
        listing("broken.oagmod", "broken.oagmod", None),
    ]
}

fn model(listing: &[Listing]) -> Menu {
    let strings = StringTable::default();
    let mut definition = Definition::built_in(&strings).unwrap();
    mods::expand(&mut definition, listing, &strings);
    let mut model = Menu::new(definition);
    model.supply(ValueSource::Mods, &mods::choices(listing, &strings));
    model.set_item_toggle_label("TOGGLE");
    model
}

fn press(model: &mut Menu, button: Button) -> Vec<MenuEvent> {
    let mut input = Input::new();
    input.begin_frame(1 << button.index());
    model.update(&mut input)
}

fn ids(model: &Menu) -> Vec<String> {
    model
        .page()
        .meta
        .iter()
        .filter_map(|meta| meta.id.clone())
        .collect()
}

fn focused(model: &Menu) -> String {
    model.page().meta[model.selected()]
        .id
        .clone()
        .unwrap_or_default()
}

fn shown(model: &Menu, id: &str) -> (String, String) {
    let page = model.page();
    let row = page.row_with_id(id).unwrap();
    let entry = &page.entries[row];
    let value = match entry.value() {
        Some(Value::Text(value)) => value,
        other => panic!("{id}: {other:?}"),
    };
    (entry.label().to_string(), value)
}

#[test]
fn the_list_has_a_row_per_mod_with_the_mods_that_are_on_first() {
    let mut model = model(&five());
    assert!(model.open(mods::PAGE));
    assert_eq!(
        ids(&model),
        [
            oag_mod::extras::ID,
            "test.on",
            "test.long",
            "test.off",
            "test.also-off",
            "broken.oagmod"
        ]
    );
    assert_eq!(
        shown(&model, oag_mod::extras::ID),
        ("OpenAntiGrav extras".into(), "ALWAYS ON".into())
    );
    assert_eq!(shown(&model, "test.on"), ("On".into(), "ON".into()));
    assert_eq!(shown(&model, "test.off"), ("Off".into(), "OFF".into()));
    let (long, _) = shown(&model, "test.long");
    assert_eq!(long, "A mod with a very lon...");
    let broken = model.page().row_with_id("broken.oagmod").unwrap();
    assert_eq!(
        model.unavailable(&model.page().entries[broken]),
        Some("not a zip"),
        "its error is the note"
    );
    assert!(matches!(
        model.page().entries.last(),
        Some(Entry::Back { .. })
    ));
}

#[test]
fn square_toggles_the_mod_the_row_names_and_the_switch_keeps_it() {
    let listing = five();
    let mut model = model(&listing);
    assert!(model.open(mods::PAGE));
    model.point_item(mods::PAGE, "test.also-off");
    let events = press(&mut model, Button::Square);
    let [MenuEvent::Changed { setting, value }] = events.as_slice() else {
        panic!("{events:?}");
    };
    let id = mods::item_of(setting).expect("a list row's setting");
    assert_eq!(id, "test.also-off");
    assert_eq!(value, &Value::Text("on".into()));
    let mut switches = oag_game::settings::Mods {
        disabled: ["test.also-off".to_string(), "test.off".to_string()].into(),
        ..Default::default()
    };
    let mod_ = listing.iter().find(|m| m.id == id).unwrap();
    assert!(mods::switch(&mut switches, id, true, mod_.default_enabled));
    assert_eq!(
        switches.disabled.iter().collect::<Vec<_>>(),
        ["test.off"],
        "that mod on, the others left"
    );

    for fixed in [oag_mod::extras::ID, "broken.oagmod"] {
        model.point_item(mods::PAGE, fixed);
        assert!(press(&mut model, Button::Square).is_empty(), "{fixed}");
        assert_eq!(model.reset_offered(), None, "{fixed}");
    }
    assert_eq!(mods::item_of(mods::SELECTED), None);
    assert_eq!(mods::item_of(mods::ENABLED), None);
}

#[test]
fn cross_opens_the_rows_mod_and_back_returns_to_it_or_to_the_mod_the_page_moved_to() {
    let listing = five();
    let strings = StringTable::default();
    let mut model = model(&listing);
    assert!(model.push(mods::PAGE));
    model.point_item(mods::PAGE, "test.long");
    assert_eq!(
        press(&mut model, Button::Cross),
        [MenuEvent::OpenItem("test.long".into())]
    );
    mods::open_item(&mut model, "test.long", &listing, &strings);
    assert_eq!(model.page().id, mods::DETAIL);
    assert_eq!(mods::selection(&model).as_deref(), Some("test.long"));
    let table = mods::table_for(&model, &listing, &strings).unwrap();
    assert!(table.iter().any(|(_, v)| v == "test.long"), "{table:?}");
    press(&mut model, Button::Circle);
    assert_eq!(model.page().id, mods::PAGE);
    assert_eq!(focused(&model), "test.long");

    // The detail page's MOD row steps on: the list's cursor goes with it.
    press(&mut model, Button::Cross);
    mods::open_item(&mut model, "test.long", &listing, &strings);
    model.seed(mods::SELECTED, &Value::Text("test.off".into()));
    model.point_item(mods::PAGE, "test.off");
    press(&mut model, Button::Circle);
    assert_eq!(focused(&model), "test.off");
}

/// The page on a rebuilt tree: the toggle rebuilds the menus, and the order the list was
/// entered with is carried onto them.
#[test]
fn a_rebuild_after_a_toggle_keeps_the_order_and_the_next_entry_sorts_it() {
    let mut listing = five();
    let mut old = model(&listing);
    assert!(old.push(mods::PAGE));
    old.point_item(mods::PAGE, "test.on");
    let before = ids(&old);

    listing[2].enabled = Some(false);
    let orders = old.item_orders();
    let trail = old.trail();
    let mut rebuilt = model(&listing);
    rebuilt.set_item_orders(&orders);
    assert!(rebuilt.restore_trail(&trail));
    assert_eq!(rebuilt.page().id, mods::PAGE);
    assert_eq!(ids(&rebuilt), before, "nothing moves under the cursor");
    assert_eq!(focused(&rebuilt), "test.on");
    assert_eq!(shown(&rebuilt, "test.on").1, "OFF");

    press(&mut rebuilt, Button::Circle);
    assert!(rebuilt.push(mods::PAGE));
    assert_eq!(
        ids(&rebuilt),
        [
            oag_mod::extras::ID,
            "test.long",
            "test.off",
            "test.on",
            "test.also-off",
            "broken.oagmod"
        ],
        "on first, each group in the listing's order"
    );
}
