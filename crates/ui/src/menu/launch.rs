//! The settings keys a `launch_race` row names its race's inputs by.

/// Where a launch reads each input of the race from: the key of a setting
/// (`race.mode`, `remix.track`, `mod.<id>.<key>`). An input left `None` is the
/// booted title's own, or the engine's default for it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchInputs {
    /// The title whose disc supplies the circuit.
    pub track_title: Option<String>,
    /// The circuit, or a CUSTOM file.
    pub track: Option<String>,
    /// The title whose disc supplies the craft; empty follows the track's.
    pub craft_title: Option<String>,
    /// The craft's team.
    pub team: Option<String>,
    /// The team's variant directory.
    pub variant: Option<String>,
    /// The race mode.
    pub mode: Option<String>,
    /// The speed class.
    pub class: Option<String>,
    /// The AI difficulty.
    pub difficulty: Option<String>,
}

impl LaunchInputs {
    /// The keys RACE REMIX's page holds its picks under, which is what
    /// `action = "launch_remix"` stands for.
    #[must_use]
    pub fn remix() -> Self {
        let key = |name: &str| Some(name.to_string());
        Self {
            track_title: key("remix.track_title"),
            track: key("remix.track"),
            craft_title: key("remix.craft_title"),
            team: key("remix.team"),
            variant: key("remix.variant"),
            mode: key("race.mode"),
            class: key("race.class"),
            difficulty: key("ai.difficulty"),
        }
    }
}
