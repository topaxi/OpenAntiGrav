//! The view call: a second call of a module whose `view` rows are off their defaults, made
//! only so what the module draws can follow this device's picks
//! (`docs/architecture/wasm-extensions/hooks.md`, "Picks: sim and view").
//!
//! The simulation call (every tick, every device) reads a `view` row's default. The view call
//! reads the real pick, runs on a copy of the blob the simulation call started from, and
//! keeps only its outputs: its blob, its other commands, its fuel and its faults are dropped,
//! so a pick that is not in the race's identity has no way to reach what is.

use oag_ext_abi::Command;

/// Whether `command` only changes what is drawn: written to a field cleared every tick that
/// no simulated state, hash or reply reads. Exhaustive on purpose: a new command is classed
/// here before it compiles.
pub(super) fn is_output(command: &Command) -> bool {
    match command {
        Command::FireEffect { .. }
        | Command::FireModEffect { .. }
        | Command::ScreenFill { .. }
        | Command::FovOffset { .. }
        | Command::DrawBeam { .. }
        | Command::SetShaderParam { .. }
        | Command::PortalView { .. }
        | Command::HudTint { .. }
        | Command::HudShake { .. }
        | Command::HudBarExtra { .. }
        | Command::HudText { .. }
        | Command::HudIcon { .. }
        | Command::RaiseWarning { .. }
        | Command::PickupLook { .. } => true,
        Command::ScaleThrust { .. }
        | Command::ShieldDelta { .. }
        | Command::GivePickup { .. }
        | Command::ClearPickup { .. }
        | Command::PickupRoll { .. }
        | Command::ScalePad { .. }
        | Command::ScaleSlotPads { .. }
        | Command::PlayCue { .. }
        | Command::SetWeather { .. }
        | Command::SetSun { .. }
        | Command::GradeSky { .. }
        | Command::GiveShield { .. }
        | Command::HoldShield { .. }
        | Command::ClearShield { .. }
        | Command::PreferRoute { .. }
        | Command::Emit { .. }
        | Command::Transit { .. }
        | Command::ScaleSpeed { .. }
        | Command::FireModEffectAhead { .. }
        | Command::PlayModSound { .. }
        | Command::ProbeClearance { .. } => false,
    }
}

/// The commands a call returned, split into those that act on the race and those that are drawn.
pub(super) fn split(commands: Vec<Command>) -> (Vec<Command>, Vec<Command>) {
    commands
        .into_iter()
        .partition(|command| !is_output(command))
}
