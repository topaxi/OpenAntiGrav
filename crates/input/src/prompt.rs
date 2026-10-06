//! Which family of button glyphs a prompt should draw: the device the player
//! is actually holding, or the one they set by hand.
//!
//! **Detection is plain functions over plain data**, so a test feeds it a
//! device descriptor, an environment map and a file's text and no device or
//! Steam client has to exist. [`Detector`] is the only state: which kind of
//! device was last used, and the family of the pad last used.
//!
//! # How a Steam launch is told apart from the real pad
//!
//! Under Steam Input a pad arrives as a *virtual* XInput controller, so its
//! own name and VID/PID say Xbox whatever is in the player's hands. Steam
//! writes the truth to a file and names it in `SteamVirtualGamepadInfo`;
//! SDL's own reader (`SDL_steam_virtual_gamepad.c`) parses `[slot N]`
//! sections with `name=`, `VID=`, `PID=` and `type=` keys, `type` being an
//! `SDL_GamepadType` string (`ps5`, `switchpro`, `xboxone`, `steam`...). That
//! is [`parse_virtual_info`]. No Steamworks SDK is needed. `SteamDeck=1` is
//! the fallback for the Deck's own controls. See `docs/ui/button-prompts.md`.

use std::collections::BTreeMap;

/// The family a prompt's glyphs are drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PromptFamily {
    /// Cross, circle, square, triangle: the originals' own.
    Playstation,
    /// A, B, X, Y.
    Xbox,
    /// A, B, X, Y, with A and B on the other side of the pad.
    Nintendo,
    /// The key actually bound.
    Keyboard,
}

impl PromptFamily {
    /// Every family, in the order the config names them.
    pub const ALL: [Self; 4] = [
        Self::Playstation,
        Self::Xbox,
        Self::Nintendo,
        Self::Keyboard,
    ];

    /// The config token.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Playstation => "playstation",
            Self::Xbox => "xbox",
            Self::Nintendo => "nintendo",
            Self::Keyboard => "keyboard",
        }
    }
}

/// `[controls] prompt_style`: what a prompt draws, as the player set it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PromptStyle {
    /// Follow the last-used device. See [`Detector::family`].
    #[default]
    Auto,
    /// The disc's own glyphs, whatever device is in use.
    Original,
    /// A family forced by hand.
    Family(PromptFamily),
}

impl PromptStyle {
    /// The config token.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Original => "original",
            Self::Family(family) => family.name(),
        }
    }

    /// A config token, or `None` for one this build does not know.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim().to_ascii_lowercase();
        match name.as_str() {
            "auto" => Some(Self::Auto),
            "original" => Some(Self::Original),
            other => PromptFamily::ALL
                .into_iter()
                .find(|family| family.name() == other)
                .map(Self::Family),
        }
    }
}

/// What a pad says about itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PadInfo {
    /// The OS's name for it.
    pub name: String,
    /// USB vendor id, where the backend knows it.
    pub vendor_id: Option<u16>,
    /// USB product id, where the backend knows it.
    pub product_id: Option<u16>,
    /// The backend's 16-byte device id (SDL's joystick GUID), for the log.
    pub uuid: Option<[u8; 16]>,
}

/// SDL's textual GUID for a device id, `unknown` without one.
#[must_use]
pub fn uuid_string(uuid: Option<[u8; 16]>) -> String {
    let Some(bytes) = uuid else {
        return "unknown".to_string();
    };
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// What a device can do, as far as the backend can tell: the half of
/// "is this a gamepad" the name cannot answer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PadCaps {
    /// The backend holds an SDL controller mapping for this device.
    pub sdl_mapped: bool,
    /// How many of south, east, west and north the device really has.
    pub face_buttons: u8,
    /// How many of the left stick's two axes the device really has.
    pub stick_axes: u8,
    /// The device has a d-pad: all four direction buttons, or a hat / axis
    /// pair for both directions. The PSP had no stick, so this stands in for
    /// one.
    pub dpad: bool,
}

/// Interfaces a keyboard, mouse or pad's sensors expose that an OS lists as
/// a joystick but that are never a pad in the hand. A Keychron keyboard's
/// consumer-control HID interface is one: `Keychron Keychron K2 Pro System
/// Control`.
const NOT_A_PAD: [&str; 7] = [
    "system control",
    "consumer control",
    "keyboard",
    "mouse",
    "touchpad",
    "motion sensors",
    "power button",
];

/// Whether a device is a gamepad a player could be holding.
///
/// A device whose name is a known non-pad interface never is. Otherwise it is
/// when the backend has an SDL mapping for it, or it really has face buttons
/// and either both axes of a stick or a d-pad.
#[must_use]
pub fn is_gamepad(info: &PadInfo, caps: PadCaps) -> bool {
    let name = info.name.to_ascii_lowercase();
    if NOT_A_PAD.iter().any(|needle| name.contains(needle)) {
        return false;
    }
    caps.sdl_mapped || (caps.face_buttons >= 2 && (caps.stick_axes >= 2 || caps.dpad))
}

const VENDOR_SONY: u16 = 0x054c;
const VENDOR_MICROSOFT: u16 = 0x045e;
const VENDOR_NINTENDO: u16 = 0x057e;
const VENDOR_VALVE: u16 = 0x28de;

/// The family a pad's own name and VID/PID say, `None` for a pad nothing
/// here recognises: a prompt then draws the disc's own glyphs rather than
/// guessing a family. Valve's own pads (the Steam Controller, the Deck, Steam
/// Input's virtual pad) are Xbox-shaped.
#[must_use]
pub fn classify_pad(info: &PadInfo) -> Option<PromptFamily> {
    match info.vendor_id {
        Some(VENDOR_SONY) => return Some(PromptFamily::Playstation),
        Some(VENDOR_NINTENDO) => return Some(PromptFamily::Nintendo),
        Some(VENDOR_MICROSOFT | VENDOR_VALVE) => return Some(PromptFamily::Xbox),
        _ => {}
    }
    let name = info.name.to_ascii_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|needle| name.contains(needle));
    if has(&[
        "dualshock",
        "dualsense",
        "playstation",
        "ps3",
        "ps4",
        "ps5",
        "wireless controller",
    ]) {
        Some(PromptFamily::Playstation)
    } else if has(&["nintendo", "switch", "joy-con", "joycon", "pro controller"]) {
        Some(PromptFamily::Nintendo)
    } else if has(&["xbox", "x-box", "xinput"]) {
        Some(PromptFamily::Xbox)
    } else {
        None
    }
}

/// The family an `SDL_GamepadType` string stands for, `None` for `unknown`.
///
/// `steam` (the Steam Controller and the Deck) is Xbox-shaped.
#[must_use]
pub fn family_of_sdl_type(kind: &str) -> Option<PromptFamily> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "ps3" | "ps4" | "ps5" => Some(PromptFamily::Playstation),
        "switchpro" | "joyconleft" | "joyconright" | "joyconpair" | "gamecube" => {
            Some(PromptFamily::Nintendo)
        }
        "standard" | "xbox360" | "xboxone" | "steam" => Some(PromptFamily::Xbox),
        _ => None,
    }
}

/// One `[slot N]` of the file `SteamVirtualGamepadInfo` names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VirtualPad {
    /// The slot number: the virtual pad's index.
    pub slot: u32,
    /// The real controller's name.
    pub name: String,
    /// The real controller's VID.
    pub vendor_id: u16,
    /// The real controller's PID.
    pub product_id: u16,
    /// The `type=` string, verbatim.
    pub kind: String,
}

impl VirtualPad {
    /// The family of the real controller behind this slot, `None` when
    /// neither names one, from its
    /// `type=` first and its name and VID/PID second.
    #[must_use]
    pub fn family(&self) -> Option<PromptFamily> {
        family_of_sdl_type(&self.kind).or_else(|| {
            classify_pad(&PadInfo {
                name: self.name.clone(),
                vendor_id: Some(self.vendor_id),
                product_id: Some(self.product_id),
                uuid: None,
            })
        })
    }
}

fn number(text: &str) -> u16 {
    let text = text.trim();
    text.strip_prefix("0x")
        .map_or_else(|| text.parse(), |hex| u16::from_str_radix(hex, 16))
        .unwrap_or(0)
}

/// Parses the file `SteamVirtualGamepadInfo` names, the way SDL does:
/// `[slot N]` headers, then `key=value` lines. Unknown keys and junk lines
/// are skipped.
#[must_use]
pub fn parse_virtual_info(text: &str) -> Vec<VirtualPad> {
    let mut pads: Vec<VirtualPad> = Vec::new();
    for line in text.lines().map(str::trim) {
        if let Some(slot) = line
            .strip_prefix("[slot ")
            .and_then(|rest| rest.strip_suffix(']'))
            .and_then(|slot| slot.trim().parse().ok())
        {
            pads.push(VirtualPad {
                slot,
                ..VirtualPad::default()
            });
        } else if let (Some(pad), Some((key, value))) = (pads.last_mut(), line.split_once('=')) {
            match key {
                "name" => pad.name = value.to_string(),
                "VID" => pad.vendor_id = number(value),
                "PID" => pad.product_id = number(value),
                "type" => pad.kind = value.to_string(),
                _ => {}
            }
        }
    }
    pads
}

/// What the process was launched into, read once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Launch {
    /// `SteamDeck=1`: Steam sets it for a game it starts on a Deck.
    pub steam_deck: bool,
    /// `SteamAppId` or `SteamGameId` is set: Steam started this process.
    pub under_steam: bool,
    /// The real controllers behind Steam Input's virtual ones.
    pub virtual_pads: Vec<VirtualPad>,
}

impl Launch {
    /// Reads the launch environment through `env` and, for the Steam
    /// controller file, `read_file`. Both injected so a test needs neither a
    /// process environment nor a file.
    #[must_use]
    pub fn read(
        env: &dyn Fn(&str) -> Option<String>,
        read_file: &dyn Fn(&str) -> Option<String>,
    ) -> Self {
        let set = |key: &str| env(key).is_some_and(|value| !value.is_empty());
        Self {
            steam_deck: env("SteamDeck").is_some_and(|value| value == "1"),
            under_steam: set("SteamAppId") || set("SteamGameId"),
            virtual_pads: env("SteamVirtualGamepadInfo")
                .filter(|path| !path.is_empty())
                .and_then(|path| read_file(&path))
                .map(|text| parse_virtual_info(&text))
                .unwrap_or_default(),
        }
    }

    /// The process environment and the real file system.
    #[must_use]
    pub fn from_process() -> Self {
        Self::read(&|key| std::env::var(key).ok(), &|path| {
            std::fs::read_to_string(path).ok()
        })
    }

    /// The family of `pad`, knowing what Steam said about it, `None` when
    /// nothing says.
    ///
    /// A pad Steam Input virtualised carries the Steam slot in its name
    /// (`... pad 0`); that slot's real type wins. Without a slot match a
    /// single virtual pad is the answer; several that agree are too. A Deck
    /// with no file is Xbox-shaped.
    #[must_use]
    pub fn family_of(&self, pad: &PadInfo) -> Option<PromptFamily> {
        let steam_made =
            pad.vendor_id == Some(VENDOR_VALVE) || pad.vendor_id == Some(VENDOR_MICROSOFT);
        let slot = pad
            .name
            .rsplit(|c: char| !c.is_ascii_digit())
            .next()
            .and_then(|digits| digits.parse::<u32>().ok());
        let by_slot = slot
            .filter(|_| steam_made)
            .and_then(|slot| self.virtual_pads.iter().find(|v| v.slot == slot));
        if let Some(virtual_pad) = by_slot {
            return virtual_pad.family();
        }
        let mut families: BTreeMap<Option<PromptFamily>, usize> = BTreeMap::new();
        for virtual_pad in &self.virtual_pads {
            *families.entry(virtual_pad.family()).or_default() += 1;
        }
        if families.len() == 1 && (self.under_steam || steam_made) {
            return families.into_keys().next().flatten();
        }
        if self.steam_deck && steam_made {
            return Some(PromptFamily::Xbox);
        }
        classify_pad(pad)
    }
}

/// Which kind of device the player touched last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Used {
    /// Nothing yet.
    Nothing,
    /// A key.
    Keyboard,
    /// A real pad, of this family when one is known.
    Pad(Option<PromptFamily>),
}

/// Tracks the last-used device and answers which glyph family follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detector {
    used: Used,
}

impl Default for Detector {
    fn default() -> Self {
        Self::new()
    }
}

impl Detector {
    /// Nothing used yet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            used: Used::Nothing,
        }
    }

    /// A key went down.
    pub fn note_key(&mut self) {
        self.used = Used::Keyboard;
    }

    /// A real pad of `family` (`None` when unknown) was pressed or pushed
    /// past its dead zone. A device that is not a pad never reaches this.
    pub fn note_pad(&mut self, family: Option<PromptFamily>) {
        self.used = Used::Pad(family);
    }

    /// A pad is attached and nothing has been touched yet: its family
    /// stands until something is.
    pub fn seed_pad(&mut self, family: Option<PromptFamily>) {
        if self.used == Used::Nothing {
            self.used = Used::Pad(family);
        }
    }

    /// The last-used device.
    #[must_use]
    pub const fn used(&self) -> Used {
        self.used
    }

    /// The family `style` draws now, or `None` for the disc's own glyphs.
    ///
    /// `Auto` follows the device: a PlayStation pad is the disc's own
    /// glyphs, so it answers `None` too, as does a pad of unknown family,
    /// and a run nothing has touched answers `None` so a headless capture
    /// draws what it always drew.
    #[must_use]
    pub fn family(&self, style: PromptStyle) -> Option<PromptFamily> {
        match style {
            PromptStyle::Original => None,
            PromptStyle::Family(family) => Some(family),
            PromptStyle::Auto => match self.used {
                Used::Nothing | Used::Pad(None | Some(PromptFamily::Playstation)) => None,
                Used::Keyboard => Some(PromptFamily::Keyboard),
                Used::Pad(Some(family)) => Some(family),
            },
        }
    }
}

#[cfg(test)]
mod tests;
