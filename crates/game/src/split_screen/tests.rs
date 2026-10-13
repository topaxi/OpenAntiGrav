use super::*;
use wgpu::DeviceType;

const ALL: [DeviceType; 5] = [
    DeviceType::IntegratedGpu,
    DeviceType::DiscreteGpu,
    DeviceType::VirtualGpu,
    DeviceType::Cpu,
    DeviceType::Other,
];

fn titles() -> [(&'static oag_title::Title, bool); 5] {
    [
        (oag_pulse::TITLE, true),
        (oag_pure::TITLE, true),
        (oag_hd::TITLE, false),
        (oag_2048::TITLE, false),
        (oag_omega::TITLE, false),
    ]
}

#[test]
fn an_integrated_gpu_offers_a_split_for_pulse_and_pure_only() {
    for (title, on_igpu) in titles() {
        assert_eq!(
            offered(title, DeviceType::IntegratedGpu),
            on_igpu,
            "{}",
            title.name
        );
    }
}

#[test]
fn every_other_adapter_offers_every_title_and_a_software_one_is_not_gated() {
    for (title, _) in titles() {
        for device in ALL
            .into_iter()
            .filter(|device| *device != DeviceType::IntegratedGpu)
        {
            assert!(offered(title, device), "{} on {device:?}", title.name);
        }
    }
}

#[test]
fn players_fall_back_to_one_view_only_where_the_split_is_not_offered() {
    assert_eq!(
        players_offered(oag_hd::TITLE, DeviceType::IntegratedGpu, 4),
        1
    );
    assert_eq!(
        players_offered(oag_hd::TITLE, DeviceType::DiscreteGpu, 4),
        4
    );
    assert_eq!(
        players_offered(oag_pulse::TITLE, DeviceType::IntegratedGpu, 4),
        4
    );
    assert_eq!(
        players_offered(oag_omega::TITLE, DeviceType::IntegratedGpu, 1),
        1
    );
}

fn has_split_row(definition: &oag_ui::menu::Definition) -> bool {
    definition.pages.iter().any(|page| {
        page.meta
            .iter()
            .any(|meta| meta.id.as_deref() == Some("split_screen"))
    })
}

fn built(title: &'static oag_title::Title) -> oag_ui::menu::Definition {
    crate::mods::Mods::load()
        .unwrap()
        .build(
            oag_ui::menu::BUILT_IN,
            None,
            Some(oag_ui::menu::For {
                title,
                updated: false,
            }),
        )
        .unwrap()
        .definition
}

#[test]
fn an_integrated_gpu_menu_has_the_split_row_for_pulse_and_pure_alone() {
    for (title, on_igpu) in titles() {
        for device in ALL {
            let mut definition = built(title);
            assert!(has_split_row(&definition), "{} starts with it", title.name);
            let dropped = trim_menu(&mut definition, title, device);
            assert_eq!(dropped, !offered(title, device));
            assert_eq!(has_split_row(&definition), !dropped, "{}", title.name);
            if device == DeviceType::IntegratedGpu {
                assert_eq!(has_split_row(&definition), on_igpu, "{}", title.name);
            }
        }
    }
}

#[test]
fn the_split_tile_on_2048s_grid_goes_with_the_row_on_an_integrated_gpu() {
    use oag_ui::frontend::{ExtraTile, Launch};
    let tiles = || {
        vec![
            ExtraTile {
                label: "RACEBOX".into(),
                launch: Launch::RaceBox,
            },
            ExtraTile {
                label: "SPLIT SCREEN".into(),
                launch: Launch::SplitScreen,
            },
        ]
    };
    let mut igpu = tiles();
    trim_tiles(&mut igpu, oag_2048::TITLE, DeviceType::IntegratedGpu);
    assert_eq!(igpu.len(), 1);
    assert_eq!(igpu[0].launch, Launch::RaceBox);
    let mut discrete = tiles();
    trim_tiles(&mut discrete, oag_2048::TITLE, DeviceType::DiscreteGpu);
    assert_eq!(discrete.len(), 2);
}

fn settings_on_race(definition: &oag_ui::menu::Definition) -> Vec<String> {
    definition
        .pages
        .iter()
        .find(|page| page.id == "race")
        .expect("the RACE page")
        .entries
        .iter()
        .filter_map(|entry| entry.setting().map(str::to_string))
        .collect()
}

const AUTHORED_ROWS: [&str; 2] = ["race.players", "race.split_cut"];

/// HD's race box authors the people and the cut, so its RACE page carries both rows and
/// the generic SPLIT SCREEN row is gone (the authored place wins). Every other title keeps
/// its row and has neither authored one. The pin that moves with this is
/// `an_integrated_gpu_menu_has_the_split_row_for_pulse_and_pure_alone`'s starting point,
/// which builds without this trim and so still starts with the row on every title.
#[test]
fn the_authored_rows_replace_the_generic_row_on_hd_and_nowhere_else() {
    for (title, _) in titles() {
        let mut definition = built(title);
        definition.drop_rows_picked_on_screen(title);
        let authors = title.front_end.is_some_and(|f| f.local_split.is_some());
        assert_eq!(authors, title.name == oag_hd::TITLE.name, "{}", title.name);
        assert_eq!(has_split_row(&definition), !authors, "{}", title.name);
        let settings = settings_on_race(&definition);
        for key in AUTHORED_ROWS {
            assert_eq!(settings.iter().any(|s| s == key), authors, "{key} on {}", title.name);
        }
    }
}

/// HD offers no split on an integrated GPU, and that takes the authored rows with it:
/// otherwise the row would offer a second person the title will not draw.
#[test]
fn an_integrated_gpu_takes_hds_authored_rows_too() {
    let hd = oag_hd::TITLE;
    for device in ALL {
        let mut definition = built(hd);
        definition.drop_rows_picked_on_screen(hd);
        assert!(settings_on_race(&definition).iter().any(|s| s == "race.players"));
        let dropped = trim_menu(&mut definition, hd, device);
        assert_eq!(dropped, device == DeviceType::IntegratedGpu, "{device:?}");
        let settings = settings_on_race(&definition);
        for key in AUTHORED_ROWS {
            assert_eq!(settings.iter().any(|s| s == key), !dropped, "{key} on {device:?}");
        }
    }
}
