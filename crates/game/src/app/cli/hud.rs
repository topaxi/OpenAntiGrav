//! Forcing the HUD into a state for a capture.

/// Forcing the HUD into a state for a capture.
#[derive(clap::Args, Debug)]
pub(crate) struct HudArgs {
    /// With a posed craft (`--pose-from` or `--pose`): the HUD's race state as the original's frame showed it,
    /// `PLACE,LAP,LAP_SECONDS,RACE_SECONDS[,SPEED_KMH[,SHIELD_PERCENT[,BEST_LAP_SECONDS[,ZONE[,ZONE_SCORE[,ZONE_SECONDS]]]]]]` (lap 1-based, the clocks in seconds; the last three are Zone mode's counters, written into the race's own state so the grade follows), read out of the
    /// emulator's RAM at the same stop as the pose. A posed capture runs no ticks, so without this
    /// the HUD reads the grid's place and zero clocks.
    ///
    /// **A capture aid, so it needs `--screenshot`** and every `--hud-*` pin below follows it. The
    /// pin overwrites the readout every frame and writes its Zone counters into the race's own
    /// state, which a race a person plays, a network race (`--connect`, `--online`) or a driven
    /// session (`--drive`) must never have.
    #[arg(
        long,
        value_name = "PLACE,LAP,LAP_S,RACE_S[,KMH[,SHIELD_PCT[,BEST_S[,ZONE[,SCORE[,ZONE_S]]]]]]",
        requires = "screenshot",
        conflicts_with_all = ["connect", "online", "drive", "drive_file"]
    )]
    pub(crate) hud_state: Option<String>,

    /// With `--hud-state`: the original's exhaust boost timer over its length (`0` to `1`; Pulse PSP
    /// `*(craft+0x78)+0xb8 / +0xb4`), which the HUD's multi-copy boost draw and the speed bar's glow stamp
    /// run on. A posed capture runs no ticks, so its flare has no timer of its own.
    #[arg(long, value_name = "PHASE", requires = "hud_state")]
    pub(crate) hud_boost_phase: Option<f32>,

    /// With `--hud-state`: the seconds the player's running Autopilot pickup has left (Pulse PSP `record+0x148`),
    /// which the HUD prints under the pickup's icon (`5.00`). Drawn on the HUD alone; the race's own timer is
    /// untouched, so the craft is not flown by the driver. `--give` cannot stand in: a posed capture runs no tick for it,
    /// and a held pickup shows no countdown until it runs.
    #[arg(long, value_name = "SECONDS", requires = "hud_state")]
    pub(crate) hud_autopilot: Option<f32>,

    /// With `--hud-state`: the pickup the player holds (a weapon's `type`, such as `Turbo`; Pulse PSP `record+0x1bc`),
    /// whose backdrop and icon the HUD draws, settled and with no countdown. Drawn on the HUD alone, so the craft
    /// holds nothing. A posed capture runs no tick, and `--give` acts inside one.
    #[arg(long, value_name = "TYPE", requires = "hud_state")]
    pub(crate) hud_pickup: Option<String>,

    /// With `--hud-state`: the shield bar's post-hit flash as the original's HUD object held it, `TIMER,BLINK` in seconds
    /// (Pulse PSP `hud+0x11c` and `hud+0x1dc`). While the timer is above zero the bar draws forced red whatever the
    /// pool. Drawn on the HUD alone: a posed capture runs no tick, so no drop of the pool arms the timer.
    #[arg(long, value_name = "TIMER,BLINK", requires = "hud_state")]
    pub(crate) hud_shield_flash: Option<String>,

    /// With `--hud-state`: holds the now-playing marquee on this song title (Pulse draws one; a title without
    /// one draws nothing). A capture runs no music, so no song starts and the bar and scrolling text never appear.
    /// **A capture aid**: the display is stepped `--hud-song-ticks` times and frozen there.
    #[arg(long, value_name = "TITLE", requires = "hud_state")]
    pub(crate) hud_song: Option<String>,

    /// With `--hud-song`: the artist shown ahead of the title.
    #[arg(long, value_name = "ARTIST", requires = "hud_song")]
    pub(crate) hud_song_artist: Option<String>,

    /// With `--hud-song`: marquee updates (60 per second) into the song's animation. The bar and text fade in over
    /// the first few, then the text scrolls. Default 60, past the fade-in, where the bar has stopped climbing.
    #[arg(long, value_name = "TICKS", requires = "hud_song")]
    pub(crate) hud_song_ticks: Option<u32>,

    /// With `--hud-state`: draws the Head to Head bar on the HUD alone, `GAP,lead` or `GAP,trail` (the gap in the
    /// race's arc-length units, the number printed before the `m`; whether the player is in first place). The race's
    /// mode and standings are untouched.
    #[arg(long, value_name = "GAP,lead|trail", requires = "hud_state")]
    pub(crate) hud_head_to_head: Option<String>,
}
