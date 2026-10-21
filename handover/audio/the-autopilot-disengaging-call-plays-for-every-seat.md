# The Autopilot's "disengaging" call plays for every seat

Reported by the maintainer on 2026-10-21, not investigated in a lane yet.

## Open

- **In a multiplayer race, player 1 hears the Autopilot's `disengaging` announcer when player 2's
  Autopilot runs out.** The call is raised per slot: `Field::tick_autopilot`
  (`crates/raceplay/src/field.rs`) pushes `Cue::Disengaging` for any person-flown slot when its
  remainder crosses one second (`Autopilot_Update`, `docs/ghidra/functions/psp-pulse-usa/autopilot.md`).
  Nothing on the playback side appears to check which seat a voice line belongs to before playing it,
  so a cue for slot 1 reaches the mix player 1 listens to. Which multiplayer path the maintainer heard
  it on (split screen in one window, one window per seat, or network play) is not recorded; check all
  three.
- **The held `~AUTOPILOT` voice is the other half and may be wrong the other way.**
  `oag_sound::sfx` opens it off `Race::autopilot_is_active` (`crates/raceplay/src/effects.rs`), which
  reads `self.ship()`, one craft, so a second seat's Autopilot may never sound its held voice.
- The original's rule is not read: Pulse and Pure have no split screen, and HD's split-screen announcer
  routing is unread. Until it is, the call should be heard by the seat it concerns (its window or
  client); for one shared output it is a choice, labelled chosen, not measured.

## Next Steps

1. Reproduce: a two-seat race with an Autopilot pickup on seat 2; record which output plays the call.
2. Route speech cues (`Cue::Disengaging`, the shield callout) by the cue's slot to the seat that owns it,
   and drive the held `~AUTOPILOT` voice per seat; a test asserts seat 1 hears no call for seat 2.
3. Read HD's split-screen announcer path (EBOOT, the `disengaging` cue's caller) for the original's rule.
