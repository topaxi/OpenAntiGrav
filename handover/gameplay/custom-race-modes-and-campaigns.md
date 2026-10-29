---
status: ready-to-wire
titles: [all]
updated: 2026-10-29
summary: "Custom campaigns v1.0 landed for Pulse/HD/Omega (ADR-0077); open: 2048, grid editor, opponent variants; drag race still design only"
size: XL (20)
difficulty: hard (35)
---

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

## 1d. Multiplayer Time Trial and Speed Lap as ghosts (maintainer's idea, 2026-10-14)

An idea for a mod mode, not built. Local or networked players run a Time Trial or Speed Lap
together:

- Everyone starts on **the same grid spot**, under the normal Time Trial / Speed Lap rules.
- Every other player is drawn **as a ghost**, and there are **no collisions** between players.
- **Time Trial** ranks by total time; **Speed Lap** is won by the fastest single lap over 7 laps.
- The HUD may show the live ranking, and for Speed Lap the current fastest lap.

**Checked against the original (2026-10-14, `h2h-re`, static only):** Pulse PSP's ad-hoc
**Multiplayer Time Trial** (mode 17) is half of this idea already
([`mp-time-trial.md`](../../docs/ghidra/functions/psp-pulse-usa/mp-time-trial.md)): 2 to 8
players, **craft-to-craft collision off** (`MPTimeTrial_Construct` clears the byte
`Collision_DispatchPair` checks), weapons off, the clock and a place counter on the HUD, the
record kept as total race time. It differs from the idea in two ways: the players take
**separate grid slots** (`Race_SpawnGrid`), not one spot, and they are spawned as
**ordinary craft with name tags**, not ghosts (translucency not determined). There is **no
multiplayer Speed Lap** in Pulse. So: Multiplayer Time Trial with collisions off and own grid
slots is a **port**; the shared start spot, ghost drawing and a multiplayer Speed Lap are the
**mod**. The end-of-race order (finish or time) is unread.

**Ported 2026-10-14 (`mp-time-trial-port`): the Time Trial half.** A Time Trial with two or more
people now runs the original's Multiplayer Time Trial rules (the people only, own grid rows,
craft-to-craft collision off, the arcade HUD with the clock and `POS n / m`, a board ranked by
total time, no record written), with no new menu mode: the head count switches the rules
(**chosen**; the rules are measured, [`mp-time-trial.md`](../../docs/ghidra/functions/psp-pulse-usa/mp-time-trial.md#the-port)).
**What stays the mod**: the shared start spot, other players drawn as ghosts, and **Speed Lap
with several people, which the original has no twin for** and which is left as it was (the
split's hatch still fields AI there).

Needs: a per-mode "players are ghosts" rule (collision pairs skipped, the ghost drawable for
another person's craft), one shared spawn pose, and per-person results. It sits on the
extension points above (rules as data) plus the ghost renderer that already exists.

## 2. A custom campaign

**v1.0 landed (2026-10-29) for Pulse, HD/Fury and Omega**:
`docs/formats/custom-campaign.md`, ADR-0077, `schemas/v1/campaign.tosd`,
`oag_mod::campaign` (shape), `oag_game::campaign::custom` (compile),
`assets/mods/examples/example-campaign/` (with a mod pilot, `pilots/rival.toml`), ground truth
`crates/game/tests/custom_campaign_ground_truth.rs`. It is slot-addressed on the
titles' own hex grid, uses existing modes only, and keeps namespaced progress. The
maintainer's decisions from the plan stand: a picker page only when one is installed;
hidden or locked reveal; disc circuits plus `title:circuit` cross-disc; per-event AI
`original`/`ours` with its difficulty; a pilot by name (the mod's own first) or inline.
A campaign that fails, or names a disc not on this machine, is not offered and is logged
only (maintainer, 2026-10-29).

Open after v1.0, in the order the maintainer raised them:

- **2048** (the plan's step 7): a campaign layout of our own drawing 2048's hex art, a
  picker tile on the touch front end, `custom_campaigns = true` on its `Title`.
- **The grid editor** (step 9, its own merge): every slot drawn, confirming a slot picks
  track/mode/craft and saves into it, writing a generated mod through a serializer the
  reader round-trips.
- **Opponent craft variants** in `field.teams` (`{ team, variant }`): `oag_livery::load`
  paints opponent slots by team only today, so this is a livery change too.
- **AI craft overrides** (top speed and other handling for the opponents) and a
  **pilot per slot**: compatible additions to `[grid.event.field]`.
- **A flyer card shipped in the mod** for HD/Omega tiers; only disc cards are named now.
- Not run live yet: a play-through on HD/Omega of a hidden cell appearing after its
  requirement, and a cross-disc event racing (the ground truth plans the cells, it does
  not race them).

The design notes below are from before v1.0:

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

## WebAssembly in two phases (maintainer, 2026-10-12)

**Phase 1: no simulation code.** WebAssembly modules first run only where nothing reaches a race's
state: HUD widgets (first candidate: a mini-map or a lap-delta graph, computed from values the race
already publishes). No cross-engine determinism, NaN rule or conformance test is needed yet, a mod
cannot change a race's outcome, and the host trait and the three engines (below) are proven on
low-stakes code. **Phase 2: the simulation hook**, proven with Pilot Assist as below, **with a second restriction**
(maintainer, 2026-10-12): a mod that touches the simulation (its manifest declares
`simulation = true`; loading a simulation-hook module without it is refused) marks the session
**modded** - no multiplayer (host or join refused, naming the mod), no ghosts or replays recorded
or played back, records kept in a separate modded table, and the HUD/MODS page show MODDED. The
Pilot Assist proof is unaffected (its bit-identical hashes are ground-truth tests, not player
recordings). **Phase 3 (later) lifts that**: each simulation mod's id, version and module hash go
into the replay header and the multiplayer lobby, so a replay plays back only with the identical
module and a lobby admits only identical modules - once the cross-engine conformance test holds.

## Phase 2's first candidate: Pilot Assist on Pulse and Pure (2026-10-12)

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

## WebAssembly extensions design (2026-10-29, `wasm-extensions` branch only)

A design lane (two members) wrote the proposal the phases above call for; it lives on
the local `wasm-extensions` branch, not on main (maintainer's rule). Pages:
`docs/architecture/wasm-extensions/README.md` (capability surface, the `sim` and `hud`
worlds, non-capabilities, packaging, trust, licences),
[hooks.md](../../docs/architecture/wasm-extensions/hooks.md) (one `tick` call per
module per tick between `advance_cannons` and `step_player`; every event's source and
every command's sink at a pinned `file:line`; the maintainer's examples mapped, with
the engine changes each needs),
[determinism-and-state.md](../../docs/architecture/wasm-extensions/determinism-and-state.md)
(NaN, fuel, state blob, hash, replays, lobby, rollback cost), and ADR-0079 (Proposed).
The spike `spikes/wasm-extensions/` (own `[workspace]`, root `Cargo.lock` untouched):
wasmtime 50.0.0 Cranelift and Pulley bit-identical under fuel on one x86-64 host,
including the out-of-fuel trap point; a guest keeping state in its own memory diverges
after a rollback unless the instance is fresh per tick.

Owed before the ADR can be accepted:

- **The same spike on AArch64** (Android) and **a browser run**; the browser needs the
  load-time NaN and fuel rewrite (README open question 1); wasmtime on `wasm32` is
  rejected unless a serious reason shows up (maintainer, 2026-10-29).
- **iOS**: `cargo check --target aarch64-apple-ios` of wasmtime needs Apple's `xcrun`;
  check on a macOS machine, with `signals_based_traps` on and off.
- **A call-depth counter** (stack overflow traps at an engine-dependent depth).
- **The lobby fingerprint** hashes `id@version` only
  (`oag_net::discovery::content_fingerprint`): fold each sim module's SHA-256 in.
- **Example mods in several guest languages** (maintainer, 2026-10-29): the same mod,
  or a close variant (e.g. Feisar faster per speed pad), written once each in **Rust**,
  **C++**, **JavaScript through Javy**, **Python through Pyodide** and **Lua through
  Wasmoon or lua2wasm**, as `mod-examples` entries. Each must load under the design's
  rules: core wasm, the one `oag` import namespace, no WASI, fuel and NaN
  canonicalisation, a host-owned state blob. Expect the ABI to be tested hard by the
  interpreter-based ones: Javy emits WASI preview-1 imports, Pyodide and Wasmoon are
  Emscripten builds that expect a JavaScript host and are tens of MB, and lua2wasm's
  output is unverified. Record per language whether it loads as-is, needs a shim (and
  which), or cannot meet the rules, and the module size and per-tick cost; a language
  that cannot is a finding for the README, not a reason to widen the capabilities.
  The list is a starting set, not a mandate (maintainer, 2026-10-29): pick each
  language's runtime by **feasibility and ease of use** for a mod author (e.g. Wasmoon
  or lua2wasm, whichever fits the ABI with less glue), and say why in its README.

## The first player-facing mod: Kiss Curse (maintainer, 2026-10-29)

The smallest increment that a player feels, ahead of the README's phase 1 and 2: a
craft that touches a wall slows far more than a normal craft.

- **Event**: `wall-contact`, at `Race::count_wall_contact` (`field.rs:545`, from
  `step_player` and `step_opponents`), so AI craft are cursed too.
- **Command**: `scale-thrust{slot, factor}` with `factor` in `[0, 1]` only, written
  into `pending_thrust_scale`, the per-tick slow the Leach Beam already arms
  (`crates/weapons/src/projectile/leach_beam.rs:359`). The guest keeps the curse's
  countdown in its own blob and re-sends the command each tick it runs, so the
  engine needs no timer and no factor above 1.0 (both are the Feisar walk's changes).
- **Host**: a minimal `oag-modhost` behind `extensions`: wasmtime with Cranelift on
  desktop, fuel, NaN canonicalisation, the I/O buffer, a pooled instance per tick, no
  engine built while no mod is loaded; the one `tick` call between `advance_cannons`
  and `step_player`; the blob in the race state and the hash.
- **Mod**: Rust, in `mod-examples`, e.g. thrust x0.25 for 1.5 s after each contact.
- **Proof**: a modded race run twice hashes identically; a vanilla race with the
  feature on matches the feature-off build; the determinism references untouched;
  out of fuel falls back to vanilla deterministically; a ground-truth test that a
  cursed craft's speed after a scrape is below an uncursed one's.
- **Out**: HUD world, the Pilot Assist move, browser, iOS, replays, lobby. The race is
  MODDED and records no replay (phase 2's rule).

## Next Steps

1. Refactor the lap modes into race rules as data, every existing hash unchanged.
2. Mod-declared modes (`modes/*.toml`, `mode.tosd`) built from those rules; the drag race as an
   example mod: a sprint on an existing loop first (finish gate, grid at a sample, HUD, results,
   records, replay header), on the titles the mod declares.
3. WebAssembly phase 1 (ADR first): the host trait and three engines, used by a HUD widget mod
   (mini-map or lap-delta graph); no simulation code.
4. WebAssembly phase 2: the simulation hook, proven by moving Pulse/Pure's Pilot Assist backport
   into a mod with bit-identical replay hashes; then shared by rules and AI controllers.
5. Custom campaigns: v1.0 landed (section 2). Still open: 2048, the grid editor, the
   follow-ups listed there, and one built-in OAG campaign with a drag race event once the
   drag race exists.
   **The examples are a submodule (2026-10-29):** github.com/OpenAntiGrav/mod-examples at
   `assets/mods/examples`, Pulse-only (Hillside borrows Pulse 03_Track art). NIGHT RUN
   races Hillside out of the track-only `oag.hillside` (`file = "<mod id>:<path>"`) and
   bundles Practice Octagon as a **Zone** event (2026-10-29: Zone on a track file landed,
   `docs/formats/track-file.md#zone-mode-on-a-track-file`; its medals are chosen, not
   measured). Mod dependencies: `handover/tooling/a-mod-names-no-dependencies-yet.md`.
   HD/Omega have no disc-backed example (flyers, slots); an HD-only example would
   restore it.
6. Later: open (point-to-point) tracks in the track format, the spline and the AI plan.
