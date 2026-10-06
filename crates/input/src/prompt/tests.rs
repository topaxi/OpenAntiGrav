use super::*;
use std::collections::BTreeMap;

fn pad(name: &str, vid: u16, pid: u16) -> PadInfo {
    PadInfo {
        name: name.to_string(),
        vendor_id: Some(vid),
        product_id: Some(pid),
    }
}

fn launch(env: &[(&str, &str)], files: &[(&str, &str)]) -> Launch {
    let env: BTreeMap<_, _> = env
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let files: BTreeMap<_, _> = files
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    Launch::read(&|key| env.get(key).cloned(), &|path| {
        files.get(path).cloned()
    })
}

const STEAM_INFO: &str = "[slot 0]\nname=DualSense Wireless Controller\nVID=0x054c\nPID=0x0ce6\ntype=ps5\nhandle=12\n\n[slot 1]\nname=Pro Controller\nVID=0x057e\nPID=0x2009\ntype=switchpro\n";

#[test]
fn a_pad_is_told_apart_by_vendor_and_by_name() {
    assert_eq!(
        classify_pad(&pad("x", 0x054c, 0x09cc)),
        PromptFamily::Playstation
    );
    assert_eq!(
        classify_pad(&pad("x", 0x057e, 0x2009)),
        PromptFamily::Nintendo
    );
    assert_eq!(classify_pad(&pad("x", 0x045e, 0x028e)), PromptFamily::Xbox);
    let nameless = |name: &str| PadInfo {
        name: name.to_string(),
        ..PadInfo::default()
    };
    assert_eq!(
        classify_pad(&nameless("PS4 Controller")),
        PromptFamily::Playstation
    );
    assert_eq!(
        classify_pad(&nameless("Nintendo Switch Pro Controller")),
        PromptFamily::Nintendo
    );
    assert_eq!(
        classify_pad(&nameless("8BitDo Something")),
        PromptFamily::Xbox
    );
}

#[test]
fn the_steam_file_parses_as_sdl_reads_it() {
    let pads = parse_virtual_info(STEAM_INFO);
    assert_eq!(pads.len(), 2);
    assert_eq!(pads[0].slot, 0);
    assert_eq!(pads[0].vendor_id, 0x054c);
    assert_eq!(pads[1].kind, "switchpro");
    assert_eq!(pads[1].family(), PromptFamily::Nintendo);
}

#[test]
fn a_steam_input_pad_is_the_controller_behind_it() {
    let launch = launch(
        &[
            ("SteamAppId", "1"),
            ("SteamVirtualGamepadInfo", "/run/info"),
        ],
        &[("/run/info", STEAM_INFO)],
    );
    let virtual_xinput = pad("Microsoft X-Box 360 pad 0", 0x045e, 0x028e);
    assert_eq!(classify_pad(&virtual_xinput), PromptFamily::Xbox);
    assert_eq!(launch.family_of(&virtual_xinput), PromptFamily::Playstation);
    let second = pad("Microsoft X-Box 360 pad 1", 0x045e, 0x028e);
    assert_eq!(launch.family_of(&second), PromptFamily::Nintendo);
}

#[test]
fn a_real_pad_is_not_overruled_by_a_steam_file() {
    let launch = launch(
        &[("SteamVirtualGamepadInfo", "/run/info")],
        &[("/run/info", STEAM_INFO)],
    );
    let real = pad("Controller 1", 0x1234, 0x5678);
    assert_eq!(launch.family_of(&real), PromptFamily::Xbox);
}

#[test]
fn a_deck_without_the_file_is_xbox_shaped() {
    let launch = launch(&[("SteamDeck", "1"), ("SteamGameId", "9")], &[]);
    assert!(launch.steam_deck && launch.under_steam);
    let deck = pad("Steam Deck Controller", 0x28de, 0x1205);
    assert_eq!(launch.family_of(&deck), PromptFamily::Xbox);
}

#[test]
fn an_unreadable_file_is_no_virtual_pads() {
    let launch = launch(&[("SteamVirtualGamepadInfo", "/missing")], &[]);
    assert!(launch.virtual_pads.is_empty());
}

#[test]
fn auto_follows_the_last_used_device_and_playstation_is_the_disc() {
    let mut detector = Detector::new();
    assert_eq!(detector.family(PromptStyle::Auto), None, "nothing touched");
    detector.seed_pad(PromptFamily::Xbox);
    assert_eq!(detector.family(PromptStyle::Auto), Some(PromptFamily::Xbox));
    detector.note_key();
    detector.seed_pad(PromptFamily::Nintendo);
    assert_eq!(
        detector.family(PromptStyle::Auto),
        Some(PromptFamily::Keyboard)
    );
    detector.note_pad(PromptFamily::Playstation);
    assert_eq!(detector.family(PromptStyle::Auto), None);
    assert_eq!(
        detector.family(PromptStyle::Family(PromptFamily::Nintendo)),
        Some(PromptFamily::Nintendo)
    );
    assert_eq!(detector.family(PromptStyle::Original), None);
}

#[test]
fn every_style_token_round_trips() {
    for family in PromptFamily::ALL {
        let style = PromptStyle::Family(family);
        assert_eq!(PromptStyle::from_name(style.name()), Some(style));
    }
    assert_eq!(PromptStyle::from_name(" AUTO "), Some(PromptStyle::Auto));
    assert_eq!(
        PromptStyle::from_name("original"),
        Some(PromptStyle::Original)
    );
    assert_eq!(PromptStyle::from_name("sega"), None);
}
