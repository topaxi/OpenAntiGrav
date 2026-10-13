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
        page.meta.iter().any(|meta| {
            matches!(
                meta.id.as_deref(),
                Some("split_screen" | "split_vertical" | "split_horizontal")
            )
        })
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
