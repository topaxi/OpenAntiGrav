---
status: ready-to-wire
titles: [all]
updated: 2026-10-21
summary: "Route Cue::Message and is_player by the listening seat for two local seats on one machine"
difficulty: easy (30)
---

# The Autopilot's "disengaging" call plays for every seat

Reported by the maintainer on 2026-10-21. Landed on the `autopilot-seat-cue` branch for a network client; local seats left on purpose.

## Open

- **Which path leaked.** Reproduced in `crates/raceplay/src/tests/autopilot.rs`: a network client (every person `Controller::Remote`, `set_seat`) raised `Cue::Disengaging` for another person's slot and sounded the held `~AUTOPILOT` for it. Split screen in one window and one window per seat share one `Audio` and one `Race`, so they play every local person's call by design now (`Race::heard_here`); the maintainer confirmed on 2026-10-21 that the report was a network game, the path fixed; the shared-output rule for local seats is chosen, not measured, and confirmed by the maintainer (2026-10-21): on one machine every person hears every call; routing audio per display (one window per seat on separate outputs) is not wanted without real demand.
- **Other dry lines raised per slot are not routed yet.** `Cue::Message` is raised as slot 0 for any person's board; `CueEvent::is_player` is `slot == 0`, so a client seated at 1 plays its own `CraftUnlessPlayer` cues positionally. The engine loop's `slot != 0` is the same shape. Not speech; untouched.
- The original's rule: HD's `Autopilot_Update` has no seat test (`docs/ghidra/functions/ps3-hdfury-eu/autopilot.md`, "Who hears the disengaging call"); the flags in its play helper are unread.

## Next Steps

1. Route `Cue::Message` and `is_player` by the listening seat.
