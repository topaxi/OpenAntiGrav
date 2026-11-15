//! The `MUSIC ORDER` row: shipped once, on the AUDIO page, for every title.

use oag_ui::menu::*;

fn built_in() -> Definition {
    Definition::built_in(&oag_ui::language::StringTable::default())
        .expect("the built-in menu must parse")
}

/// `MUSIC ORDER` is one row on the AUDIO page on every title (Pulse's disc authors the option,
/// the maintainer wants the shuffle for every soundtrack), offers exactly the orders the sound
/// crate plays, opens on the saved one, and the saved one survives the settings file.
#[test]
fn the_music_order_row_is_on_the_audio_page_for_every_title_and_round_trips() {
    use oag_sound::MusicOrder;

    for title in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_hd::TITLE,
        oag_omega::TITLE,
        oag_2048::TITLE,
    ] {
        let definition = built_in();
        let rows: Vec<(&str, &Entry)> = definition
            .pages
            .iter()
            .flat_map(|page| {
                page.entries
                    .iter()
                    .map(move |entry| (page.id.as_str(), entry))
            })
            .filter(|(_, entry)| entry.setting() == Some("audio.music_order"))
            .collect();
        assert_eq!(rows.len(), 1, "{}: one setting, one row", title.name);
        assert_eq!(rows[0].0, "audio");
    }

    let definition = built_in();
    let offered: Vec<String> = definition
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .find_map(|entry| match entry {
            Entry::Choice { values, .. } if entry.setting() == Some("audio.music_order") => {
                Some(values.iter().map(|v| v.value.clone()).collect())
            }
            _ => None,
        })
        .expect("the row");
    let names: Vec<String> = MusicOrder::ALL
        .iter()
        .map(|order| order.name().to_string())
        .collect();
    assert_eq!(
        offered, names,
        "the row and `MusicOrder::ALL` must be one list"
    );

    for order in MusicOrder::ALL {
        let mut settings = oag_game::settings::Settings::default();
        settings.audio.music_order = order;
        let seeded = oag_game::settings::menu_seeds(
            &settings,
            oag_mesh::mesh_render::Anisotropy::default(),
            oag_pulse::TITLE,
            oag_disc::Platform::Psp,
        )
        .into_iter()
        .find(|(setting, _)| *setting == "audio.music_order")
        .expect("the row is seeded")
        .1;
        assert_eq!(seeded, Value::Text(order.name().to_string()));

        let written = toml::to_string(&settings).expect("serialise");
        let loaded: oag_game::settings::Settings = toml::from_str(&written).expect("parse");
        assert_eq!(loaded.audio.music_order, order, "persists");
    }
    let older: oag_game::settings::Settings =
        toml::from_str("[audio]\nmusic_volume = 50").expect("parse");
    assert_eq!(
        older.audio.music_order,
        MusicOrder::Sequential,
        "sequential by default"
    );
}
