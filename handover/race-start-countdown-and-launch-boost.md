# Race start: the countdown state machine, the launch reaction boost, and whether Zone shows a different countdown

Three questions, tracked as one grid across the four titles this project reads. None is
answered yet - this file opens the thread rather than closing any cell of it.

|                                  | Pure | Pulse | HD/Fury | 2048 |
| -------------------------------- | ---- | ----- | ------- | ---- |
| Countdown state machine + timing | open | open  | open    | open |
| Launch reaction speedboost       | open | open  | open    | open |
| Zone/view display variants       | open | open  | open    | open |

**Countdown state machine + timing**: what drives `ReadyText`/`GoText`/`CountdownTime`
and the `<Mode3D>` models (`Pulse_Ready_Go`, `Cockpit_321GO`) from race load to green -
durations, the false-start stall `race-modes.md:470-473` already names ("silently kills
the engine if you hold thrust before the lights") but has never traced, and what state
holds it.

**Launch reaction speedboost**: whether landing the accelerate press at the right instant
relative to the lights grants a speed boost - the mechanism is unknown going in: a
timestamp on first thrust press compared to green, or throttle held through the
countdown with a false-start penalty (or both are the same mechanism read two ways). Two
different searches, not one - see the Open section.

**Zone/view display variants**: the user's memory is that HD/Fury's Zone races show a
different countdown display/render than a circuit race; unsure about the other three
titles. Two independent axes may both be true and must not be collapsed into one: a
**mode** axis (the `321Go_*.vex` family already carries `StartFinish`/`Zone`/
`HD_Zone_Battle`/`hd_detonator` variants per
[a-circuits-billboard-slots-are-a-9-entry.md](a-circuits-billboard-slots-are-a-9-entry.md)),
and a **view** axis (`hud.md:622` already names two separate Pulse countdown models,
`Pulse_Ready_Go` for the chase/external HUD and `Cockpit_321GO` for the cockpit view -
independent of mode).

## Open

- Everything. This file was opened to hold the grid above before the first cell closes,
  per this project's own "a durable partial beats an undurable whole" practice.
- HD/Fury's `TrackStartup_Load`-adjacent vocabulary (`Countdown`, `StartLight`,
  `RaceStart`, `Grid`, `321`) has not been swept for named functions/strings yet - the
  cheapest lead, since `ps3-hdfury-eu`'s EBOOT carries real C++ symbol names.
- Pulse's `ready`/`321_GO`/`go` announcer cue set
  (`docs/ghidra/functions/psp-pulse-usa/zone-mode.md:252`) is unxrefed - whatever plays
  `321_GO` is very likely the countdown tick handler, and the `Zone_UpdateRacing`/
  `Zone_UpdateResults` family it sits beside implies a countdown sibling by naming
  pattern.
- `Ship_SetState`'s full case list (this repo only names case 5, destroyed, per
  `crates/race/src/state.rs`'s doc comment) is unenumerated - the false-start stall is
  likely one of its cases, and a boost grant is plausibly adjacent to it.
- The billboard thread's own top open item - the mode-descriptor pointer replacing
  `Num==7`'s mesh, traced to one of the four `321Go_*.vex` shapes - doubles as this
  thread's Zone-display-variant question. Chasing it answers both.
- 2048 (`vita-2048-*`) and Pure (`psp-pure-*`) are completely unstarted here. Per
  [2048-engine-lineage-hd-fury.md] (this session's own memory, not yet a repo doc),
  2048 shares HD/Fury's engine lineage, so HD's vocabulary is worth trying against it
  before falling back to Pure/Pulse-style stripped-binary hunting.

## Next Steps

1. Sweep `ps3-hdfury-eu` for `Countdown`/`StartLight`/`RaceStart`/`Grid`/`321` in
   function names and strings.
2. Reuse whatever vocabulary that finds against `vita-2048-eu-v104` (or `-usa-base`) by
   name/string search before hunting `FUN_*` addresses cold.
3. In `psp-pulse-usa`, xref whatever plays the `321_GO` announcer cue and read outward
   from there; separately, list functions near `Zone_UpdateRacing`/`Zone_UpdateResults`.
4. Enumerate `Ship_SetState`'s cases (whichever title has it named/decompiled first) to
   find the false-start stall and look for an adjacent boost grant.
5. Finish tracing the mode-descriptor pointer from
   [a-circuits-billboard-slots-are-a-9-entry.md](a-circuits-billboard-slots-are-a-9-entry.md)'s
   open item 1 to a `321Go_*.vex` shape - settles the mode-display axis.
6. Before publishing any "no, X doesn't differ" negative for the Zone/view question, get
   a live screenshot first - the billboard thread's own stated lesson, from a mistake
   made on this exact asset family.
