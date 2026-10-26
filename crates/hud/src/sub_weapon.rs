//! `SubWeapon`: the line of text under the held pickup's icon.
//!
//! One `<Text name="SubWeapon">` that every pickup layout of every title authors
//! inside the pickup cluster (Pulse at `(240, 45)`, scale `0.6`; HD in the
//! `PickupParent` group; 2048's at `(0, 63)`), and that the player's own status
//! update writes **every frame**. It builds one string and hands it to the
//! widget, in this order:
//!
//! 1. the Autopilot is running: `"%1.2f"` of its remaining seconds;
//! 2. the held weapon is the Cannon: `"%d"` of its rounds left, over whatever
//!    step 1 wrote;
//! 3. otherwise the string stays empty, which draws nothing.
//!
//! So the Cannon's count wins the line and the Autopilot's countdown shows
//! otherwise - a craft cannot be holding the Cannon with an Autopilot running
//! and show both. Read in each title's executable, and watched on Pulse in a
//! capture of the original (the `5.00` under the Autopilot's icon):
//! `PlayerStatus_Update` (`0x0883b3b8`) in Pulse, the same function at `0x000cb928` in HD,
//! `FUN_811b1ee2` in 2048 v1.04, `FUN_012e5470` in Omega. Pure carries only the
//! first step (`0x08922340`): it has no Cannon.
//! `docs/ghidra/functions/psp-pulse-usa/sub-weapon.md`.

/// What the `SubWeapon` line says this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SubWeapon {
    /// The Cannon's rounds left, `"%d"`.
    Rounds(u32),
    /// The Autopilot's seconds left, `"%1.2f"`.
    Autopilot(f32),
}

impl SubWeapon {
    /// The text the widget draws.
    #[must_use]
    pub fn text(self) -> String {
        match self {
            Self::Rounds(rounds) => rounds.to_string(),
            Self::Autopilot(seconds) => format!("{seconds:.2}"),
        }
    }
}
