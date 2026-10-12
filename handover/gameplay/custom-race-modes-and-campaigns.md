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

**A mod supports only the titles it declares** (maintainer, 2026-10-12): the manifest's
`titles = [...]` (already in `oag-mod.toml`) limits where its modes, campaigns and rows appear; a
mode offered on one title need not exist on the others, and the menus never list it there.

Decided (mods, potentially WebAssembly), so the order is: first refactor the built-in lap
modes into **rules as data** (finish = laps or distance, grid sample, weapons, eliminations,
auto-speed) with every existing race hash unchanged; then a mod declares a mode in its own
`modes/*.toml` (`schemas/v1/mode.tosd`) built from those rules - a drag race needs no code at
all; then the WebAssembly rules hook (the ADR the pilot-controller thread calls for, one
`wasmtime` host shared with mod controllers) for modes data cannot express.

## 1b. A second example: an open-world eliminator (battle arena)

The maintainer's idea, "think Mario Kart balloons": an **arena** instead of a circuit, each craft
starts with N lives (balloons), a weapon hit pops one, the last craft with lives left wins (or the
most lives at a time limit). Mostly a custom track plus rules, but the track is the hard part:

1. **An arena track**: an open floor and walls with no lap spline. Everything that assumes a
   spline today must accept none - spawning (grid points authored in the track file instead),
   respawn/rescue (back to an authored point, not the nearest spline sample), standings (no
   progress, so by lives then time), the camera and flyby, the out-of-bounds/reset volumes. The
   own track format gains an "arena" kind (`track.tosd` minor addition) and the Blender add-on a
   way to author spawn points.
2. **Hover over an arbitrary floor**: check how much of `oag-physics`' hover and grounding reads
   the spline frame versus the collision mesh; an arena needs the mesh path alone.
3. **Rules**: lives per craft, a hit costs one, elimination at zero, optional time limit, weapon
   pads respawning - rules as data first (the existing Eliminator mode already counts kills and
   eliminations), WebAssembly only for what data cannot say.
4. **AI**: this project's driver follows a racing line; an arena needs navigation in open space
   (seek the nearest opponent or a weapon pad, avoid walls) - a new controller, which is exactly
   the kind of thing the mod controller hook (WebAssembly) is for, or a built-in arena controller
   of ours if data-only mods should ship it.
5. **HUD**: lives per craft instead of laps; results by survival order.

## 1c. Custom HUDs and HUD modifications (maintainer, 2026-10-12)

The modes above need HUDs of their own (distance to finish, lives per craft), and players will
want to move, hide or add elements. What exists: `oag-hud` holds the layout model, the `Readout` a
race hands it and the `Draw` list `draw_list` builds, with each title's HUD as data from its disc
(`oag_game::hud_overlay` draws it); the asset layers (ADR-0061) already let an override replace a
HUD texture by entry name.

1. **HUD layouts as mod data**: a mod's `hud/*.toml` (a `hud.tosd` schema) that adds, moves, hides
   or restyles elements by stable element id, the same `parent`-style patching the menus use, so
   a small change does not copy a whole layout. Per title, only on the titles the mod declares.
2. **Bindings**: an element shows one `Readout` field (speed, lap, position, lap delta, shield,
   weapon, a mode's own values such as lives or distance to finish); a mode declares the extra
   fields it publishes so its HUD can bind them.
3. **Computed widgets**: anything data cannot express (a lap-delta graph, a mini-map) as a
   WebAssembly widget later. The HUD is not simulation state, so it needs no determinism - a
   simpler host than the rules hook, but the same sandbox (no files, network or clock).
4. **The original HUDs stay as they are**: a HUD mod is opt-in, the vanilla layout is the default
   and the original campaigns keep it.

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

## The first WebAssembly candidate: Pilot Assist on Pulse and Pure (2026-10-12)

The maintainer's suggestion, and the recommended first user of the WebAssembly hook rather than
the arena AI. Pilot Assist is HD's own measured law (`oag_physics::pilot_assist`, fed by
`crates/raceplay/src/pilot_assist.rs`, `docs/physics/pilot-assist.md`), backported to Pulse and
Pure, where it is a non-original addition - what mods are for. HD, 2048 and Omega keep it built in
(original there). Why first: it is small; its shape (the craft's pose against the spline plus the
player's controls in, a correction out) is the observation/controls interface AI controllers need
too; and it has an exact oracle - the WebAssembly version must give **bit-identical replay hashes**
to the built-in Rust one on Pulse and Pure (`pilot_assist_ground_truth.rs`,
`pilot_assist_levels_ground_truth.rs`). It also exercises a mod carrying behaviour (WebAssembly)
and its HUD indicator (data plus the fallback assets) together. Cost: the simulation-hook ADR
(the observation struct, the return, the replay record), a deterministic `wasmtime` host behind a
cargo feature that the simulation may depend on.

**Engine choice (maintainer, 2026-10-12):** wasmtime with Cranelift on Linux, Windows, macOS and
Android; wasmtime's **Pulley** interpreter on iOS (no JIT allowed there; same API and host code);
on the web build the browser's own `WebAssembly` API, called from our Rust through JS glue, the
module compiled asynchronously at race load (never per tick; browsers limit synchronous
compiles on the main thread), per-tick calls copying the observation into the module's memory.
Three engines must give bit-identical results, so: (1) a deterministic subset checked at load -
refuse relaxed SIMD and threads; (2) a NaN rule - a NaN in a module's output is an error naming
the mod (NaN payloads differ between engines; wasmtime's NaN canonicalisation on as well);
(3) one host trait with three backends, the observation and controls defined once; (4) a
conformance test running the same module and inputs on Cranelift, Pulley and a headless browser
(`wasm-bindgen-test`) with identical outputs - Pilot Assist first, also against the built-in Rust
law. The lane confirms Pulley's iOS status and the crates before the ADR.

## Next Steps

1. Refactor the lap modes into race rules as data, every existing hash unchanged.
2. Mod-declared modes (`modes/*.toml`, `mode.tosd`) built from those rules; the drag race as an
   example mod: a sprint on an existing loop first (finish gate, grid at a sample, HUD, results,
   records, replay header), on the titles the mod declares.
3. The WebAssembly simulation hook (ADR first), proven by moving Pulse/Pure's Pilot Assist
   backport into a mod with bit-identical replay hashes; then shared by rules and AI controllers.
4. The campaign definition format and its schema, the CAMPAIGNS row and picker, one built-in
   OAG campaign containing a drag race event.
5. Later: open (point-to-point) tracks in the track format, the spline and the AI plan.
