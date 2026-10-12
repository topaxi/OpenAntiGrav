# A pilot file names its controller: ours, the original, or a mod's

2026-10-12, the maintainer's idea: the AI pilot files
(`assets/ai/example-pilot.toml`, loaded from `~/.config/oag/pilots/` by
`oag_raceplay::pilots`, edited in OPTIONS > AI PILOTS) could also say **which
controller** flies the craft. **Design only; nothing is built.**

## Today

- A pilot file tunes **this project's** driver only: ranges per axis (`line_bias`,
  `lean`, `wander`, `inside`, ...), unknown keys refused by name, anything missing
  from `balanced`.
- The original driver (`oag_ai::original`, recovered on Pulse, HD, 2048 and Omega;
  Pure's throttle only) is chosen by **game mode**: campaign races fly it, everything
  else flies ours (`RaceDefaults::original_opponent`, decided 2026-10-11 as "never a
  setting"). A `controller` key would make the original reachable outside the
  campaign, which is the change this note proposes.

## Proposal

```toml
controller = "ours"        # default: this project's driver, every axis below applies
# controller = "original"  # the title's recovered driver; the axes do not apply,
#                          # only `skill` (the original's own skill scale) does
# controller = "mod:<name>" # a mod's controller, see below
```

- `"original"` on a title whose driver is not recovered (Pure today, any foreign
  track's host) is an error naming the title, not a silent fall back to ours.
- The original campaigns stay vanilla (maintainer, 2026-10-12): they fly the original
  whatever a pilot file says. A **custom campaign** (this project's own, or a mod's) is
  where a campaign may name any controller, craft or track.
- A pilot that sets axes for a controller that ignores them is an error naming the
  key, as a misspelled key already is.

## A mod's controller: the entry point

The container a mod travels in exists since 2026-10-12 (`oag-mod`: a folder, `.zip` or `.oagmod`, the
`oag-mod.toml` manifest, `docs/tools/mods.md`), with `pilots/` reserved beside `menu.toml` and
`strings/`. Only menu rows are read from it today; a pilot file or a controller module in a
mod's `pilots/` is not, and the entry point below is still to be built.

The constraint is determinism: replays are inputs plus a hash a second
([ADR-0055](../../docs/architecture/adr/0055-replays-are-inputs-and-a-ghost-is-poses.md)),
and multiplayer will need the same lockstep. So a mod controller must be a pure
function of what it is given, and bit-identical on every machine.

- **WebAssembly** is the candidate: sandboxed, no clock, no I/O unless granted, and
  `wasmtime` can be configured for deterministic floats (NaN canonicalisation, no
  relaxed SIMD). A module exports one `controls(observation) -> controls` per tick.
- The **observation** is a fixed, versioned struct: own pose, velocity, shield, the
  track's spline samples ahead, the AI line and corridor, other craft's poses. The
  **controls** are `InputSnapshot`'s fields. Both are this project's own types, so
  the ABI is ours to version.
- A Lua or scripting host is the alternative; it is easier to write and harder to
  keep deterministic. Not recommended.
- `oag-ai` stays dependency-light: the host lives above it (in `oag-raceplay`), and
  `oag-ai` sees a `Controller` trait the three implement.

## Open questions (the maintainer's)

- May a pilot fly `"original"` in race box, remix and multiplayer, reversing
  "never a setting" outside the campaign?
- Mod controllers in multiplayer: allowed only when every peer has the same module
  (hash in the lobby), or local races only?
- WebAssembly as the mod ABI, yes or no.

## Next Steps

1. A `Controller` trait over `oag_ai`'s driver and `oag_ai::original`, behaviour
   unchanged (a refactor with identical hashes).
2. `controller = "ours" | "original"` in the pilot file and the AI PILOTS screen.
3. An ADR for the mod ABI, then a `wasmtime` host behind a cargo feature, with one
   example module that reproduces `balanced` closely enough to race.

## Custom campaigns (idea, 2026-10-12)

The maintainer's framing: original campaigns stay vanilla; a custom campaign (an
"original OAG" campaign this project authors, or a mod's) gets the freedom. That
implies a campaign definition of this project's own (events, tracks including
foreign and track-file ones, craft including craft files, each event's controller
and pilots, medal targets, unlocks), read the way the titles' campaign tables are
read and shown by screens of the shape `oag_ui_screens::pure_chain` builds. Not designed yet.
