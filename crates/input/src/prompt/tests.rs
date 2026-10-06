use super::*;
use std::collections::BTreeMap;

fn pad(name: &str, vid: u16, pid: u16) -> PadInfo {
    PadInfo {
        name: name.to_string(),
        vendor_id: Some(vid),
        product_id: Some(pid),
        uuid: None,
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
        Some(PromptFamily::Playstation)
    );
    assert_eq!(
        classify_pad(&pad("x", 0x057e, 0x2009)),
        Some(PromptFamily::Nintendo)
    );
    assert_eq!(
        classify_pad(&pad("x", 0x045e, 0x028e)),
        Some(PromptFamily::Xbox)
    );
    let nameless = |name: &str| PadInfo {
        name: name.to_string(),
        ..PadInfo::default()
    };
    assert_eq!(
        classify_pad(&nameless("PS4 Controller")),
        Some(PromptFamily::Playstation)
    );
    assert_eq!(
        classify_pad(&nameless("Nintendo Switch Pro Controller")),
        Some(PromptFamily::Nintendo)
    );
    assert_eq!(classify_pad(&nameless("8BitDo Something")), None);
}

#[test]
fn the_steam_file_parses_as_sdl_reads_it() {
    let pads = parse_virtual_info(STEAM_INFO);
    assert_eq!(pads.len(), 2);
    assert_eq!(pads[0].slot, 0);
    assert_eq!(pads[0].vendor_id, 0x054c);
    assert_eq!(pads[1].kind, "switchpro");
    assert_eq!(pads[1].family(), Some(PromptFamily::Nintendo));
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
    assert_eq!(classify_pad(&virtual_xinput), Some(PromptFamily::Xbox));
    assert_eq!(
        launch.family_of(&virtual_xinput),
        Some(PromptFamily::Playstation)
    );
    let second = pad("Microsoft X-Box 360 pad 1", 0x045e, 0x028e);
    assert_eq!(launch.family_of(&second), Some(PromptFamily::Nintendo));
}

#[test]
fn a_real_pad_is_not_overruled_by_a_steam_file() {
    let launch = launch(
        &[("SteamVirtualGamepadInfo", "/run/info")],
        &[("/run/info", STEAM_INFO)],
    );
    let real = pad("Controller 1", 0x1234, 0x5678);
    // Steam's file names no pad of this one's slot and is not the answer for
    // a third-party pad; its own ids name no family either, so none.
    assert_eq!(launch.family_of(&real), None);
}

#[test]
fn a_deck_without_the_file_is_xbox_shaped() {
    let launch = launch(&[("SteamDeck", "1"), ("SteamGameId", "9")], &[]);
    assert!(launch.steam_deck && launch.under_steam);
    let deck = pad("Steam Deck Controller", 0x28de, 0x1205);
    assert_eq!(launch.family_of(&deck), Some(PromptFamily::Xbox));
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
    detector.seed_pad(Some(PromptFamily::Xbox));
    assert_eq!(detector.family(PromptStyle::Auto), Some(PromptFamily::Xbox));
    detector.note_key();
    detector.seed_pad(Some(PromptFamily::Nintendo));
    assert_eq!(
        detector.family(PromptStyle::Auto),
        Some(PromptFamily::Keyboard)
    );
    detector.note_pad(Some(PromptFamily::Playstation));
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

fn uuid_of(text: &str) -> [u8; 16] {
    let hex: String = text.chars().filter(|c| *c != '-').collect();
    let mut bytes = [0_u8; 16];
    for (byte, pair) in bytes.iter_mut().zip(hex.as_bytes().chunks(2)) {
        *byte = u8::from_str_radix(std::str::from_utf8(pair).unwrap_or("0"), 16).unwrap_or(0);
    }
    bytes
}

/// The maintainer's own Steam launch log, 2026-10-06, no controller attached.
const KEYCHRON_UUID: &str = "03000000-3434-0000-2102-000011010000";

fn keychron() -> PadInfo {
    PadInfo {
        name: "Keychron Keychron K2 Pro System Control".to_string(),
        vendor_id: Some(0x3434),
        product_id: Some(0x0221),
        uuid: Some(uuid_of(KEYCHRON_UUID)),
    }
}

const SDL_MAPPED: PadCaps = PadCaps {
    sdl_mapped: true,
    face_buttons: 4,
    stick_axes: 2,
    dpad: false,
};

#[test]
fn the_guid_prints_the_way_sdl_and_the_log_spell_it() {
    assert_eq!(uuid_string(Some(uuid_of(KEYCHRON_UUID))), KEYCHRON_UUID);
    assert_eq!(uuid_string(None), "unknown");
}

#[test]
fn a_keyboards_system_control_interface_is_never_a_pad() {
    // The verbatim descriptor: no SDL mapping (gilrs warned "No mapping
    // found"), and whatever buttons and axes the HID report happens to carry.
    let unmapped = PadCaps::default();
    assert!(!is_gamepad(&keychron(), unmapped));
    let busy = PadCaps {
        sdl_mapped: false,
        face_buttons: 4,
        stick_axes: 2,
        dpad: false,
    };
    assert!(!is_gamepad(&keychron(), busy));
    // Even a mapping on it does not make it one.
    assert!(!is_gamepad(&keychron(), SDL_MAPPED));
    let consumer = PadInfo {
        name: "Logitech USB Receiver Consumer Control".to_string(),
        ..PadInfo::default()
    };
    assert!(!is_gamepad(&consumer, busy));
}

#[test]
fn a_dpad_only_pad_without_a_mapping_is_a_pad() {
    // Plenty of controllers have no stick: face buttons and a d-pad are one.
    let retro = pad("Acme Retro Pad", 0x1234, 0x5678);
    let caps = PadCaps {
        sdl_mapped: false,
        face_buttons: 4,
        stick_axes: 0,
        dpad: true,
    };
    assert!(is_gamepad(&retro, caps));
    assert!(is_gamepad(
        &retro,
        PadCaps {
            face_buttons: 2,
            ..caps
        }
    ));
    // A d-pad with fewer than two face buttons is still not enough.
    assert!(!is_gamepad(
        &retro,
        PadCaps {
            face_buttons: 1,
            ..caps
        }
    ));
    // Face buttons alone are not.
    assert!(!is_gamepad(
        &retro,
        PadCaps {
            dpad: false,
            ..caps
        }
    ));
    // The name exclusions still win over a d-pad.
    assert!(!is_gamepad(&keychron(), caps));
}

#[test]
fn known_pads_are_pads_and_their_families_stand() {
    let xbox = pad("Microsoft X-Box 360 pad", 0x045e, 0x028e);
    assert!(is_gamepad(&xbox, SDL_MAPPED));
    assert_eq!(Launch::default().family_of(&xbox), Some(PromptFamily::Xbox));
    let dualsense = pad(
        "Sony Interactive Entertainment DualSense Wireless Controller",
        0x054c,
        0x0ce6,
    );
    assert!(is_gamepad(&dualsense, SDL_MAPPED));
    assert_eq!(
        Launch::default().family_of(&dualsense),
        Some(PromptFamily::Playstation)
    );
    // Its touchpad and motion sensors are separate joystick nodes.
    let touchpad = pad(
        "Sony Interactive Entertainment DualSense Wireless Controller Touchpad",
        0x054c,
        0x0ce6,
    );
    assert!(!is_gamepad(&touchpad, PadCaps::default()));
}

#[test]
fn an_unknown_pad_with_real_axes_is_a_pad_of_no_known_family() {
    let generic = pad("Acme Arcade Stick 9000", 0x1234, 0x5678);
    let caps = PadCaps {
        sdl_mapped: false,
        face_buttons: 4,
        stick_axes: 2,
        dpad: false,
    };
    assert!(is_gamepad(&generic, caps));
    // Buttons alone are not enough, nor is one axis.
    assert!(!is_gamepad(
        &generic,
        PadCaps {
            stick_axes: 1,
            ..caps
        }
    ));
    assert!(!is_gamepad(
        &generic,
        PadCaps {
            stick_axes: 0,
            ..caps
        }
    ));
    assert_eq!(Launch::default().family_of(&generic), None);
    // auto draws the disc's own glyphs for it, not Xbox, and it is not the
    // keyboard either.
    let mut detector = Detector::new();
    detector.note_pad(Launch::default().family_of(&generic));
    assert_eq!(detector.used(), Used::Pad(None));
    assert_eq!(detector.family(PromptStyle::Auto), None);
    detector.note_key();
    assert_eq!(
        detector.family(PromptStyle::Auto),
        Some(PromptFamily::Keyboard)
    );
}

#[test]
fn the_steam_virtual_pad_is_still_xbox_shaped() {
    let virtual_pad = pad("Microsoft X-Box 360 pad 0", 0x28de, 0x11ff);
    assert!(is_gamepad(&virtual_pad, SDL_MAPPED));
    assert_eq!(
        Launch::default().family_of(&virtual_pad),
        Some(PromptFamily::Xbox)
    );
    let valve = pad("Steam Controller", 0x28de, 0x1102);
    assert_eq!(
        Launch::default().family_of(&valve),
        Some(PromptFamily::Xbox)
    );
}
