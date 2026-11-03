//! The kind byte of a command record.

/// The kind byte of a command record: one table of the codes, so the record's kind is
/// named once and a new family is one row here.
///
/// [`crate::Command::FireModEffect`] shares [`Self::FireEffect`]'s code, told apart by the argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    /// `scale-thrust`, code 1.
    ScaleThrust = 1,
    /// `shield-delta`, code 2.
    ShieldDelta = 2,
    /// `give-pickup`, code 3.
    GivePickup = 3,
    /// `play-cue`, code 5.
    PlayCue = 5,
    /// `fire-effect`, code 6.
    FireEffect = 6,
    /// `clear-pickup`, code 8.
    ClearPickup = 8,
    /// `pickup-roll`, code 9.
    PickupRoll = 9,
    /// `set-weather`, code 10.
    SetWeather = 10,
    /// `set-sun`, code 11.
    SetSun = 11,
    /// `grade-sky`, code 12.
    GradeSky = 12,
    /// `scale-pad`, code 13.
    ScalePad = 13,
    /// `scale-slot-pads`, code 14.
    ScaleSlotPads = 14,
    /// `hud-tint`, code 16.
    HudTint = 16,
    /// `hud-shake`, code 17.
    HudShake = 17,
    /// `hud-bar-extra`, code 18.
    HudBarExtra = 18,
    /// `hud-text`, code 19.
    HudText = 19,
    /// `hud-icon`, code 20.
    HudIcon = 20,
    /// `prefer-route`, code 21.
    PreferRoute = 21,
    /// `pickup-look`, code 22.
    PickupLook = 22,
    /// `emit`, code 23.
    Emit = 23,
    /// `give-shield`, code 24.
    GiveShield = 24,
    /// `hold-shield`, code 25.
    HoldShield = 25,
    /// `clear-shield`, code 26.
    ClearShield = 26,
    /// `transit`, code 27.
    Transit = 27,
    /// `scale-speed`, code 28.
    ScaleSpeed = 28,
    /// `probe-clearance`, code 29.
    ProbeClearance = 29,
}

impl CommandKind {
    /// Every kind, in code order.
    pub const ALL: [Self; 26] = [
        Self::ScaleThrust,
        Self::ShieldDelta,
        Self::GivePickup,
        Self::PlayCue,
        Self::FireEffect,
        Self::ClearPickup,
        Self::PickupRoll,
        Self::SetWeather,
        Self::SetSun,
        Self::GradeSky,
        Self::ScalePad,
        Self::ScaleSlotPads,
        Self::HudTint,
        Self::HudShake,
        Self::HudBarExtra,
        Self::HudText,
        Self::HudIcon,
        Self::PreferRoute,
        Self::PickupLook,
        Self::Emit,
        Self::GiveShield,
        Self::HoldShield,
        Self::ClearShield,
        Self::Transit,
        Self::ScaleSpeed,
        Self::ProbeClearance,
    ];

    /// The kind byte.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// The kind a byte names, or `None` for a code no command has.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.code() == code)
    }

    /// The name the README and the load report use.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ScaleThrust => "scale-thrust",
            Self::ShieldDelta => "shield-delta",
            Self::GivePickup => "give-pickup",
            Self::PlayCue => "play-cue",
            Self::FireEffect => "fire-effect",
            Self::ClearPickup => "clear-pickup",
            Self::PickupRoll => "pickup-roll",
            Self::SetWeather => "set-weather",
            Self::SetSun => "set-sun",
            Self::GradeSky => "grade-sky",
            Self::ScalePad => "scale-pad",
            Self::ScaleSlotPads => "scale-slot-pads",
            Self::HudTint => "hud-tint",
            Self::HudShake => "hud-shake",
            Self::HudBarExtra => "hud-bar-extra",
            Self::HudText => "hud-text",
            Self::HudIcon => "hud-icon",
            Self::PreferRoute => "prefer-route",
            Self::PickupLook => "pickup-look",
            Self::Emit => "emit",
            Self::GiveShield => "give-shield",
            Self::HoldShield => "hold-shield",
            Self::ClearShield => "clear-shield",
            Self::Transit => "transit",
            Self::ScaleSpeed => "scale-speed",
            Self::ProbeClearance => "probe-clearance",
        }
    }

    /// Written with slot 0 and naming none: the command is the race's, not a craft's.
    #[must_use]
    pub const fn race_wide(self) -> bool {
        matches!(
            self,
            Self::PickupRoll | Self::SetWeather | Self::SetSun | Self::GradeSky | Self::ScalePad
        )
    }
}
