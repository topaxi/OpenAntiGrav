//! The settings keys a `launch_race` row names its race's inputs by.

/// Where a launch reads each input of the race from: the key of a setting
/// (`race.mode`, `mod.<id>.<key>`). An input left `None` is the
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
