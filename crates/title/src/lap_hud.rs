//! What a title's HUD does at a lap crossing: the message lines it raises and
//! the split gap it shows.
//!
//! Every title in the Studio Liverpool lineage has the same two routines, a
//! message drain (`Hud_UpdateMessages` on Pulse) and a time-gap update
//! (`Hud_UpdateTimeDiff`), and each reads them off a flag word and a few status
//! bytes that its lap code writes. They are the same *shape* and not the same
//! *text*: Pulse and HD key their phrases `IG_HUD_*` and tint a line green or
//! red, Pure keys them `HUD_<English>` and draws every line white, and HD
//! splits a lap into thirds where Pulse and Pure gap at the line alone. This
//! type carries those differences as data, each measured in the title's own
//! executable (`docs/ghidra/functions/<binary>/hud-message-triggers.md`), so
//! the engine asks the title instead of branching on its name.
//!
//! A title with no value here raises no lap line and runs no gap. That is an
//! honest absence, not Pulse's answer borrowed.

/// How a message line is tinted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageTint {
    /// Green (`0x30ff30`) for a good line, red (`0xff3030`) for a bad one, the
    /// outline's alpha following the text's. Pulse's `FUN_0881b52c` third
    /// argument and HD's `0x91d20`.
    GoodBad,
    /// White (`0xffffff`) whatever the line says, the text's alpha alone fading
    /// and the outline left as the layout authors it: Pure's `FUN_0881f2ec`
    /// writes one colour word per slot per tick and has no good/bad argument.
    White,
}

/// How the held split gap is worded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapText {
    /// `"- 2.8"` when the player is ahead and `"+ 2.8"` when behind, built from
    /// literals in the executable (`hud+0xb8`/`+0xbc` on Pulse, `+0xcc`/`+0xd0`
    /// on HD), with `TimeDiffIcon` tinted alongside.
    Signed,
    /// Two language-table phrases carrying a `%.1f`, Pure's `HUD_You are %.1f
    /// secs ahead` and `... behind`. Pure authors no `TimeDiffIcon`.
    Phrases {
        /// Table id of the phrase for a player who crossed first.
        ahead: &'static str,
        /// Table id of the phrase for a player who crossed second.
        behind: &'static str,
    },
}

/// The split gap: the lap line's table of first-over times and the display
/// built on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitGapRule {
    /// How the held gap reads.
    pub text: GapText,
    /// Whether a held "player ahead" gap ends as soon as the player is no
    /// longer first. **Pulse only**: `Hud_UpdateTimeDiff` re-reads
    /// `PLAYER_HUD+0x10` into the hold flag every tick. HD's and Pure's
    /// routines have no such test and hold for the full 3.5 s.
    pub green_hold_needs_first_place: bool,
    /// Whether Time Trial and Speed Lap keep the count-up running with no other
    /// craft live. **HD only** (and Omega, read): its display routine skips the
    /// live-craft count in those two modes (while the profile byte
    /// `0x009384e1` is clear, the ordinary race); Pulse and Pure need two. It can
    /// only bite once a stamp has been made, which on HD needs
    /// [`Self::stamps_need_two_live`]'s second craft, so a solo Time Trial shows
    /// nothing and this matters after the other craft is gone.
    pub count_up_alone_in_timed_modes: bool,
    /// Whether the table is left unwritten while only one craft is live. **HD
    /// only**: `Craft_RecordSplit` (`0x000d06e0`) opens with `if
    /// Race_CountLiveCrafts(race_manager, 0) == 1 return`, so a lone craft never
    /// stamps a split and no gap or count-up can start. Pulse's and Pure's lap
    /// functions stamp unconditionally and test the live count only when drawing.
    pub stamps_need_two_live: bool,
    /// Gaps raised per lap: `1` is the lap line alone; HD's `3` adds the two
    /// splits at a third and two thirds of the lap (`1/3` and `2/3` of the
    /// spline progress, `0x3eaaaaab`/`0x3f2aaaab` in `Craft_UpdateLapProgress`'s
    /// HD counterpart).
    pub splits_per_lap: u32,
    /// How many crossing indices the executable's table holds before a craft
    /// past them raises nothing: Pulse `21`, Pure `6`.
    pub table_crossings: u32,
}

/// What a title's HUD does at a lap crossing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LapHud {
    /// Table id of the line a lap with no wall hit raises.
    pub perfect_lap: &'static str,
    /// Table id of the line a lap that beats the stored record raises.
    pub new_lap_record: &'static str,
    /// Table id of the line the start of the last lap raises.
    pub final_lap: &'static str,
    /// Whether Time Trial raises the new-lap-record line. **HD does not**,
    /// while the profile byte `0x009384e1` is clear (an ordinary race): its
    /// `0x91d20` skips the line in mode 5 behind that byte.
    pub record_line_in_time_trial: bool,
    /// How every line is tinted.
    pub tint: MessageTint,
    /// The split gap beside the lines.
    pub gap: SplitGapRule,
}
