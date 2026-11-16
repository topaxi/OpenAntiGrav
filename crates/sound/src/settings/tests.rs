use oag_audio::{SpatialChoice, SpatialMode};

use super::*;

#[test]
fn a_manual_choice_is_remembered_for_its_own_output_only() {
    let mut s = Settings::default();
    assert_eq!(s.spatial_audio, SpatialChoice::Auto, "auto is the default");
    s.choose_spatial(Some("usb-codec"), SpatialChoice::Headphones);
    assert_eq!(s.spatial_audio, SpatialChoice::Headphones);

    s.recall_spatial("hdmi");
    assert_eq!(
        s.spatial_audio,
        SpatialChoice::Auto,
        "another output is not affected"
    );
    s.recall_spatial("usb-codec");
    assert_eq!(s.spatial_audio, SpatialChoice::Headphones);
}

#[test]
fn picking_auto_forgets_the_output() {
    let mut s = Settings::default();
    s.choose_spatial(Some("a"), SpatialChoice::Original);
    s.choose_spatial(Some("a"), SpatialChoice::Auto);
    assert!(s.spatial_devices.is_empty());
    s.recall_spatial("a");
    assert_eq!(s.spatial_audio, SpatialChoice::Auto);
}

#[test]
fn a_pick_with_no_output_id_is_not_remembered() {
    let mut s = Settings::default();
    s.choose_spatial(None, SpatialChoice::RearCue);
    assert_eq!(s.spatial_audio, SpatialChoice::RearCue);
    assert!(s.spatial_devices.is_empty());
}

#[test]
fn the_memory_is_capped_and_drops_the_oldest() {
    let mut s = Settings::default();
    for i in 0..SPATIAL_DEVICES_MAX + 3 {
        s.choose_spatial(Some(&format!("d{i}")), SpatialChoice::Original);
    }
    assert_eq!(s.spatial_devices.len(), SPATIAL_DEVICES_MAX);
    assert_eq!(s.spatial_devices[0].device, "d3");
    // Re-picking moves an output to the newest end instead of duplicating it.
    s.choose_spatial(Some("d3"), SpatialChoice::RearCue);
    assert_eq!(s.spatial_devices.len(), SPATIAL_DEVICES_MAX);
    assert_eq!(
        s.spatial_devices.last().map(|e| e.mode),
        Some(SpatialMode::RearCue)
    );
}
