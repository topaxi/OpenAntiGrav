# Custom race modes and custom campaigns: what is needed

2026-10-12, the maintainer's next goal after the mod system landed: (1) a race mode of this
project's own, the example being a **drag race** (start to finish, no laps), and (2) a **custom
campaign** that can contain such a mode. Design only; nothing is built. The decisions already
made are recorded below so the next session does not reopen them.

## Settled already

- **The original campaigns stay vanilla** (their own craft, tracks and original AI). A custom
  campaign - an "original OAG" one or a mod's - is where custom modes, tracks, craft and
  controllers belong.
- **Custom race modes are implemented through mods, potentially WebAssembly** (maintainer,
  2026-10-12). The drag race is the first such mod, not a hard-coded engine mode. Today a mod's
  `menu.toml` may use only the closed `Action`/page vocabulary (`docs/tools/mods.md`, the
  `menu-mods` ADR), and executable mod code (a deterministic WebAssembly module) is only designed,
  in [a-pilot-file-names-its-controller.md](a-pilot-file-names-its-controller.md). So the engine
  work is **extension points**: race rules a mod can declare as data (finish by laps or by
  distance, grid position, weapons, eliminations), and a WebAssembly rules hook for what data
  cannot express, both deterministic and recorded in the replay header.
- **Every TOML file of ours** carries the `toml-schema` header (ADR-0064) with a schema under
  `schemas/v1/`, listed on the generated schema index.
- **AI opponents** exist only for a class a controller covers (`oag_ai::coverage`), and only
  this project's driver flies non-original content.

## 1. A drag race (start to finish, no laps)

Today's modes are `oag_race::Mode` (`TimeTrial`, `SpeedLap`, `Zone`, single race,
`Eliminator`); every one is lap-based, and progress wraps around a **closed** spline
(`crates/race/src/standing.rs`: one lap rule through `wrapped_forward` and `LapGate`). What a
drag race needs:

1. **A finish condition that is a distance, not a lap count**: the race ends for a craft when
   its along-track progress first reaches a finish sample; standings by progress, then by time.
   A new `Mode` variant (or a mode-rules struct the existing modes become instances of - decide
   first, see below) with the lap gate replaced by a finish gate.
2. **Where start and finish are**:
   - On an existing closed circuit: a start sample and a finish sample on the same loop (a
     sprint over part of it). Needs only the finish gate plus spawning the grid at a sample
     other than the lap line.
   - On a truly open (point-to-point) track: the track format and spline code assume a loop
     (`wrapped_forward`, lap length). An open spline needs `oag-trackfile` (a manifest flag,
     minor version of the schema), the spawner, the respawn/rescue code, the AI speed plan
     (ring-based) and the camera/flyby to accept ends. Larger; do it second.
3. **AI**: this project's driver plans speed over a ring (`crates/ai/src/plan.rs`); a sprint on a
   loop works as is, an open track needs the plan to stop at the finish.
4. **Front end**: the mode in RACEBOX/RACE REMIX's mode row (string ids, pointer), its HUD
   (no lap counter; distance to finish or split time), and the results screen.
5. **Replays and records**: the mode and its start/finish in the replay header (ADR-0055) and a
   records key of its own (best time per track and segment).
6. **Medals**: target times are chosen per event (no original authors them), labelled chosen.

Decided (mods, potentially WebAssembly), so the order is: first refactor the built-in lap
modes into **rules as data** (finish = laps or distance, grid sample, weapons, eliminations,
auto-speed) with every existing race hash unchanged; then a mod declares a mode in its own
`modes/*.toml` (`schemas/v1/mode.tosd`) built from those rules - a drag race needs no code at
all; then the WebAssembly rules hook (the ADR the pilot-controller thread calls for, one
`wasmtime` host shared with mod controllers) for modes data cannot express.

## 2. A custom campaign

What the titles' campaigns already give us: Pulse/HD/Omega grids and Pure's list (both launch
events as **campaign cells** through one path, record medals in `[[campaign]]` and apply unlocks;
`crates/game/src/main/session/campaign.rs`, `crates/game/src/campaign/`, the Pure chain in
`crates/ui-screens`). What a custom one needs:

1. **A campaign definition file of our own** (`schemas/v1/campaign.tosd`): id, name, the events in
   order or grouped (tournaments), each event's mode (including a drag race), track (disc
   circuit, `.track.toml`, or a BallisticNG `.wtrk` - quiet for now), speed class, allowed craft
   (title team, or a craft file once that format exists), opponents (count, controller, pilots),
   medal targets, and unlock rules (medals or points opening events).
2. **Where it lives**: inside a mod (`campaigns/*.toml`, a convention folder the mod format
   already reserves room for) and one built-in "OAG" campaign in `oag-extras`.
3. **How it is reached**: a row (e.g. CAMPAIGNS) added by `oag-extras` with `parent`, opening a
   picker of installed custom campaigns, then the list screen Pure's campaign already uses.
4. **Progress**: its own key in the records store, so it never touches the original campaigns'.
5. **Validation**: an unknown mode, track, class or craft is an error naming the campaign and the
   event (the mod format's rule); the MODS page shows it.

## Next Steps

1. Refactor the lap modes into race rules as data, every existing hash unchanged.
2. Mod-declared modes (`modes/*.toml`, `mode.tosd`) built from those rules; the drag race as an
   example mod: a sprint on an existing loop first (finish gate, grid at a sample, HUD, results,
   records, replay header), on every title.
3. The WebAssembly rules hook (ADR first), shared with mod AI controllers.
4. The campaign definition format and its schema, the CAMPAIGNS row and picker, one built-in
   OAG campaign containing a drag race event.
5. Later: open (point-to-point) tracks in the track format, the spline and the AI plan.
