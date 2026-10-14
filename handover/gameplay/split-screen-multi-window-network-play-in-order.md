---
categories: [rendering, frontend]
---

# Split screen, multi-window and network play: one shared prerequisite, then three efforts in dependency order

2026-09-16. Not an RE finding - a planning thread, opened on request, laying
out how split screen, multiple OS windows and network multiplayer relate to
each other and to the engine as it stands today, in the order that minimizes
wasted work. No code changed. No ADR exists yet for any of this; whichever
design is actually chosen for the shared prerequisite below is ADR-shaped and
should become one before implementation starts, the same way [ADR-0003](../../docs/architecture/adr/0003-no-ecs.md)
and [ADR-0007](../../docs/architecture/adr/0007-fixed-timestep-vs-original.md)
already pin the decisions this plan leans on.

**Decided, on request, and stated up front so the rest of this thread reads
consistently: the network protocol is ours, not a port.** Unlike every other
subsystem this project reimplements, network play is not observed from the
original and is not trying to be compatible with it - see the "no source to
observe" point in section 3 for why there is nothing to port from even if
compatibility were wanted. It is designed state-of-the-art from scratch:
server-side authority (the server, not any client, is the only source of
truth, specifically to close off client-side cheating) with client-side
prediction for responsiveness, reconciled by replaying inputs against
corrected server state when a prediction turns out wrong. Full shape in
section 3. Split screen, multi-window and network play all target **up to 8
players** - the existing `MAX_SHIPS = 8` grid cap, so a full-player race
needs zero AI-filled slots and a networked race can never exceed what a
local race already handles.

**2026-09-16, revised same day**: an external review pass caught that the
first version of this thread named the wrong tick function throughout.
`World::tick` does not exist. The real 60 Hz step is `Race::tick`
(`crates/raceplay/src/tick.rs`), living in **`oag-game`**, the composition
root, not in `oag-gameplay`. It advances `RaceSim`
(`crates/raceplay/src/sim.rs`), which bundles `World` together with
`CollisionWorld`, `Spline`, `ai_order: Vec<u32>`, `ai_tuning`,
`weapon_pad_refresh_left` and a per-tick `cues: Vec<CueEvent>` audio-output
accumulator (`self.sim.cues`, [ADR-0018](../../docs/architecture/adr/0018-audio-mixer-architecture.md)'s
"cues are a per-tick output, never `World` state") - `World` alone is not the
simulation, and `RaceSim::state_hash` (`crates/raceplay/src/hash.rs`) hashes
the extras precisely because of that. Every "`World`"/"`World::tick`"
reference below that actually meant the whole tick step has been corrected
to `RaceSim`/`Race::tick`; see the "Corrected: what actually has to move"
callout in section 3 for what changes as a result. `oag_gameplay::World`
itself is still exactly as described - a plain, fixed-array,
`memcpy`-shaped struct per ADR-0003 - it's just not the whole story on its
own.

None of the three is scoped as a roadmap item on its own. `docs/overview/roadmap.md`
carries a single unchecked `- [ ] Networking` line under **M8 - Beyond Pulse**,
the last milestone, gated on single-player completeness across every title.
Split screen and multi-window aren't named in the roadmap at all - the only
existing trace of split screen is two frontend threads noting the PS2 menu
XML already authors split-screen widgets (`Team SelectionSplit`, `Track
CreationSplit`) that this build currently strips to single-player
(`handover/frontend/pulses-race-box-is-read-both-pressings.md`). This thread
does not change that prioritization; it exists so that whenever any of the
three is picked up, it starts from an accurate picture instead of rediscovering
the same constraints.

## Why one prerequisite gates all three

The engine today is **hard single-player**, not "single-player by default":

- `World` (`crates/gameplay/src/world.rs:203`) already carries
  `ships: [Ship; MAX_SHIPS]` with `MAX_SHIPS = 8` - room for a full grid - but
  `race: RaceState` is a *single* field. Its own doc comment says why:
  "Single-ship for now: the modes that need it - time trial, speed lap, Zone -
  have one ship on the track, and per-opponent timing arrives with the grid
  rather than before it." Only `ships[0]` is ever the timed, human-controlled
  ship; `crates/game/src/main/race_stage.rs` reads that index directly for
  every HUD and lap-timing field. **Not everything per-ship is missing,
  though**: `ShipState::standing: oag_race::Standing` (lap, circuit place,
  finished) is already tracked "every craft, the player included" - it's
  specifically `World::race` (the clock, Zone counters, best lap) that's
  singular. Widening `race` to per-player reuses `standing`'s existing shape
  rather than inventing per-ship tracking from nothing.
- `InputSnapshot` (`crates/gameplay/src/input.rs:253`) is one struct, no
  player index, doc-commented as "the whole of what the simulation is allowed
  to know about input" - by design, a replay records exactly this and nothing
  else. `Race::tick` takes exactly one per call.
- `oag-input`'s `Pad` explicitly **merges every attached gamepad into one
  logical stream** rather than assigning devices to players:
  "Pads are merged rather than assigned to players: there is one ship, and a
  Deck with a pad plugged into its dock should steer from either"
  (`crates/input/src/pad.rs:366`). There is no `GamepadId` -> player-slot
  mapping anywhere, even as a stub.
- `App` (`crates/game/src/main/app.rs:95`) is documented as "one of these per
  run, not one per window" - the composition root assumes exactly one winit
  window for the process's whole life.

All three features - split screen, multi-window, and network play - need the
simulation to think in terms of **N players feeding N inputs into one
`RaceSim` per tick**, instead of one. That's a single, shared, foundational
change:

1. `RaceState` becomes per-player (array or `[RaceState; MAX_PLAYERS]`
   alongside the existing `[Ship; MAX_SHIPS]`). **`MAX_PLAYERS = MAX_SHIPS =
   8`**, decided on request rather than left open: split screen, multi-window
   and network play all target up to 8 players, so there's no case where the
   player cap needs to be smaller than the grid it fills. A slot not driven
   by a human is still AI-driven exactly as today, so this needs an explicit
   notion of "which grid slots are locally/remotely human" alongside the
   per-player `RaceState`, not instead of it.
2. `Race::tick` takes N `InputSnapshot`s (or a `[InputSnapshot; MAX_PLAYERS]`)
   instead of one, applied to N ship slots instead of `ships[0]` alone.
3. `oag-input` gains a device -> player-slot assignment (which `GamepadId`
   or keyboard drives which slot), replacing the current merge-everything
   `Pad`.

`RaceState` and `InputSnapshot` are `oag-gameplay` types, and the device
assignment is `oag-input`'s - no rule in `docs/architecture/workspace-layout.md`
is threatened by widening either. But `Race::tick` itself, the function that
actually consumes N inputs per tick, lives in `oag-game`, so this
prerequisite touches the composition root too, not only the two gameplay-side
crates - worth knowing going in, since it means "land the prerequisite" is
partly an `oag-game` change, not a self-contained `oag-gameplay`/`oag-input`
one. It's still the one piece of work on the critical path for *every* one
of the three features below, plus the already-placeholdered `oag-replay`
(M7, input recording/playback, per `workspace-layout.md`'s "crates that do
not exist yet" table). Worth landing as its own change before any of split
screen, multi-window or networking starts, rather than growing it ad hoc
inside the first feature that needs it.

`ADR-0003` (no ECS) is the reason the `World` portion of this is tractable
at all: `World` is a plain struct with fixed-size arrays, chosen specifically
so "snapshotting the whole world is one memcpy-shaped operation" for replays
and golden tests. Widening it to carry N `RaceState`s is a shape change
within that same design, not a departure from it - though the ADR's own
"revisit if" clause ("a later title's entity model does not fit fixed
arrays") is worth rereading once the per-player shape is nailed down, in
case N independently-timed players turns out to want something ADR-0003
didn't anticipate. **`RaceSim` as a whole is not memcpy-shaped, though** -
`ai_order: Vec<u32>` and `cues: Vec<CueEvent>` are heap-allocated - which
matters most for network reconciliation; see the corrected callout in
section 3.

## 1. Split screen - lowest complexity, do this first

Same process, same machine, one `RaceSim`, one fixed 60 Hz tick, up to 8
viewports in **one** window. This is the cheapest of the three because it changes
nothing about process/window lifecycle (`App` stays "one per run") and
nothing about transport - it's purely: multiple local inputs in, multiple
local views out, same tick.

What's missing beyond the shared prerequisite:

- **Viewport/multi-camera support in `oag-render`.** Today there is, by its
  own doc comment, "one track-ribbon builder and one camera convention in
  the repository" (`crates/render/src/lib.rs:4`) - no `Viewport` struct, no
  split-region compositing. This is new surface area, not a variant of
  something that exists. `oag-render` owning no window
  (`crates/render/src/lib.rs:12`, permanent by design) is actually favorable
  here: the surface/window count stays 1, and split screen is purely "render
  N times into N sub-rects of the one surface," which doesn't touch the
  window-ownership boundary at all.
- ~~**Per-player HUD** in `oag-ui`, laid out per-viewport rather than
  full-screen.~~ **Done, 2026-10-13** (see below).
- **Per-player pad assignment**, covered by the shared prerequisite above.
- **A layout that scales to 8**, not just the original's 2/4-way splits -
  viewport count and arrangement (2-up, 4-up, up to 8-up) becomes a variable
  the viewport/camera-region support above needs to handle, not a fixed
  quadrant layout.

**2026-10-13: the viewport half landed**, `--players N` (1 to 8) on a direct
`--race`, written up in [split-screen.md](../../docs/rendering/split-screen.md):
`oag_render::viewport` lays out 1 to 8 views, `oag_raceplay` keeps a view-side chase
camera per extra view (`Race::set_viewpoint`) and `Scene::render_viewports` draws one
scene per view, each in its own encoder and submit with only the first clearing.
Slots `0..N` are marked `Controller::Local`; view `i` follows slot `i`. One view is
byte-identical to before. Layout, the shared far-chase rig, no shake on extra views,
no bloom/blur/jitter under a split and the hidden HUD are all **chosen, not
measured**. **2026-10-13, the per-player HUD landed**
([split-screen.md](../../docs/rendering/split-screen.md#hud)): one `Readout` per
viewed slot (`Race::readout_for`), the shield bars' state per slot, one HUD per view
drawn in one list, a per-view reticle off `sight_target(slot)`, and HD's own
two-player split layouts for a two-view race. **Still open from this list**:
per-team camera rigs, per-view Pulse bloom (`oag_post::bloom` ignores the viewport
origin), the 8-up frame budget (measured, `split-render-cost` below: 9.4 ms of CPU on the workstation, Deck unmeasured). ~~The results/end-race flow (still slot 0's, ends
every view)~~ **2026-10-13, `split-screen-end-race`: done** (below, and
[split-end-race.md](../../docs/ui/split-end-race.md)). **2026-10-13, `hd-split-black-view`: HD's and Omega's post chains
checked and fixed.** On HD and 2048 a split drew only its last view (the encode pass
clears the frame), and on Omega and HD the last view was graded from the first view's
pixels; the chain now runs once over the whole area after the last view
([split-screen.md](../../docs/rendering/split-screen.md#drawing)). **Open from it**
(all chosen, not measured): one shared exposure and glow for all views, HD's boost
zoom streak and Omega's distortion offsets are not drawn under a split, and a
race that borrows HD's zoom ring (`Scene::run_boost_zoom`) still runs it per view
from the primary's pulse (read from the code, not captured).
**2026-10-13, `split-render-cost`: a split is CPU-bound on a discrete GPU, and the first cut is in.**
Measured on the RX 7800 XT (RADV, release, 1280x720; the tables and how to rerun them are in
[split-screen.md](../../docs/rendering/split-screen.md#cost)): Pulse at eight views cost
19 ms of CPU a frame and 1.1 ms of GPU scene passes, so the GPU was never the limit there.
The Raphael iGPU proxy flips it at one and two views (GPU-bound, 4 to 5 ms of scene passes). The shadow gather, both shadow maps, the scenery
clocks and the advert cards were redone per view though they do not depend on the camera;
the first view now does them alone (`Layer::Next` skips them) and eight views cost 9.4 ms.
Pictures are byte-identical (29 stills). **Open from it**: the Steam Deck number, which
decides whether the next lever is a per-view draw budget (CPU) or render scale (GPU); steps
in the doc, the maintainer runs them. `encoder.finish` looks proportional to the draw count (the lever for per-view LOD and draw
distance, see `handover/rendering/graphics-settings-resolve-from-intent-and-context.md`),
and per-view uniform slices are the prerequisite for recording views on several threads.
**2026-10-13, `shadow-gather-per-tick`: the gather is per tick now, and `mapped` follows each view.**
The hull gather is taken at each tick's poses and lerped between ticks (once per tick, not per
rendered frame): 47 of 49 stills `cmp` equal to main, the lerp within 0.053 world units at p99 and
0.237 at worst of the live gather on a real Zone lap, a 240 Hz window saves 1.5 ms of CPU at one
view and 1.25 to 1.6 ms up to eight, a 60 Hz window neither gains nor loses
([split-screen.md](../../docs/rendering/split-screen.md#cost)). The `mapped` depth map is redrawn
per view around that view's craft (view two's shadows are there where main's had none): about
0.1 ms of CPU a view on the discrete card and **1 to 1.6 ms of iGPU time a view**. **Open from it**:
the floor rays and the track caster, both done in `shadow-floor-rays` (next entry); the bench
needs `OAG_RENDER_BENCH_FRAMES_PER_TICK` set to mean a refresh rate.
**2026-10-13, `shadow-floor-rays`: the floor rays ask once per point, the `mapped` track is culled to each map's box.**
`conform_to_floor` remembers each `(position, normal)` it has asked the road about for the one
hull: the gather is bit-identical (496,800 vertices over 300 ticks hash equal, 26 stills `cmp`
equal) and 2.9 times cheaper, 1,196 -> 417 us a tick on the eight-craft grid, 1.0 ms of CPU a frame
at a 60 Hz window on the RX 7800 XT at every view count. The `mapped` depth map records only the
track draws whose bounding sphere touches that map's box (Pulse 465 of 1,737, Pure 123 of
1,179): pictures identical, the iGPU's pipelined time down 0.3 to 0.7 ms at one view and about 2
ms at eight views (18.45 -> 16.55 on Pulse, so eight views now hold 60 Hz there). No simulation
crate changed. **Open from it**: a cheaper per-ray query is a semantics change (`floor_above`
sees only the first four hits in collider and triangle order); culling the gather to craft some
view can see needs the two-tick cache to keep every craft's gather; the Steam Deck numbers.
([split-screen.md](../../docs/rendering/split-screen.md#cost))
**2026-10-13, `pad-join-assignment`: the pad-to-slot UI is done.**
`oag_input::pad::Assignment` gained `Seating::Seated` (an unassigned pad takes the
lowest free human slot, a lost pad frees its slot, a pad past the humans drives
nothing), `--players N` seats the keyboard and first pad on slot 0 and the rest in
connection order, and a SPLIT SCREEN row under RACEBOX (oag-extras) opens a "press a
button to join" screen (`oag_ui_screens::join`, `oag_input::lobby`) whose START goes
on to Track and Ship Select with the seated count. All **chosen, not measured**; the
PS2's authored flow is a camera choice (`FE_RACE_CAM_SS`) and a per-player team
pick, not a device join. See [split-screen.md](../../docs/rendering/split-screen.md).
A local slot with no device on it (a pad unplugged mid-race, or fewer pads than `--players`) flies as the AI (`split_screen::follow_devices`), because a race now ends only when every person-flown slot has finished. **Open from it**: two real pads on real hardware (the rules are unit-tested over stand-in
ids, the keyboard path runs end to end; since `per-player-ship-pick` two virtual uinput
pads also walked the join screen, Track and Ship Select and a race in an Xvfb).
**2026-10-13, `per-player-ship-pick`: each seat picks its own craft.** The PS2 disc's
authored `Team SelectionSplit` (both players on one page, top and bottom, READY once
each confirms) is followed on PS2 Pulse with two seats; HD authors the players taking
`Team Selection Player 1` then `Player 2` in turn, which every other title and any
count above two get (chosen). Each seat is driven by its own device and a pick reaches
slot `i` (`Options::seat_picks`; `--seat-team` headless). Written up in
[split-screen.md](../../docs/rendering/split-screen.md#ship-select-per-seat). **Still
open**: ~~HD's front end shows no SPLIT SCREEN row~~ (it always had one, revealed under
RACEBOX; walked 2026-10-13, see the entry below), stick navigation on a seat's screen, a variant family's
duplicate check on HD, and where Omega authors its Ship Select screens.
**2026-10-13, `split-screen-end-race`: the end of the race is every person's.**
A person's view holds after their own line (lap and clocks held, a `FINISHED` line, the
driver flying their craft); when the last person finishes one view is drawn
(`Race::views_drawn`) under end screens that name every person: Pulse's own
`PopulateMultiplayerTable` (read, network-gated in the original, ours for a split) with
`PLAYER 1`/`PLAYER 2` and a highlight per person, HD's grid with the same names, the
built-in table's `P1`/`P2` for Pure and 2048 (whose touch pages summarise one racer). A
split banks no loyalty, builds no `Rewards`, plays no results voice; restart keeps the
seats and quit unseats (already the case). `--autopilot` now flies every local seat so
a headless 2-person race reaches its end. **Found, and wired the same day (below): the PS2 pressing authors
`EndRace Results Split` and `EndRace Rewards Split`** (two stacked per-player lap tables
and reward panels, `Player_0`/`Player_1`, two people only; its executable names them
beside `Split ScreenV/H` and `Kill Game Multiplayer`), see
[split-end-race.md](../../docs/ui/split-end-race.md).
**2026-10-13, `pulse-split-pages`: Pulse's two-person screens, both pressings.** `EndRace
Results Split`, `EndRace Rewards Split` and `Team SelectionSplit` are wired for Pulse on
the PS2 (the disc's own) and the PSP (the PS2's design, **borrowed**, built from the PSP's
own assets and labelled so in the loader report; `FrontEnd::borrows_ps2_split_screens`,
Pulse alone). The geometry bug is fixed (the grid reads each file's own rows). With two people
each banks their own team's loyalty and `Rewards Split` shows it (chosen). Written up in
[split-end-race.md](../../docs/ui/split-end-race.md) and
[selection-screens.md](../../docs/ui/selection-screens.md). **Still open from it**: the PS2
executable's entry law for these screens (not in a Ghidra project here), the PS2's highlight
placement (chosen), a live walk on the PS2 pressing and with real pads, the second person's
speedup-pad column (the race keeps the tally for the first craft), HD's podium for a split,
per-person records and ghosts, and a still of the shared Ship Select with both crafts.
**2026-10-13, `split-hud-layout-toml`: the split HUD is data.** A title's split-screen
layout is `schemas/v1/split-hud.tosd` files (`oag_hud::split_hud`): override, then the
disc's authored layout (HD and, newly, Omega: ported, and a two-view frame looked at), then ours through
the fallback layer (`SplitHud_{Pulse,Pure,HD}.toml`, chosen, not measured, Pulse's one
file for both pressings), then the scaled whole-screen layout; one rule per view shape,
so three, five and seven views mix. Each view also has a scissor now. See
[split-screen.md](../../docs/rendering/split-screen.md#hud) and
[asset-layers.md](../../docs/architecture/asset-layers.md#a-split-screen-hud-layout-file).
**2026-10-13, `split-hud-vertical`: vertical columns.** A two-view vertical cut reads a `tall`
rule (`views = [2]`, `aspect = [0.0, 2.4]`) in `SplitHud_Pulse.toml` and `SplitHud_Pure.toml`
(Pulse gains the Eliminator's kill column and the Pilot Assist indicator, Head2Head's bar is named and not seen drawing; Pure's
weapon grid is raised clear of the bottom edge) and 2048 has its own `SplitHud_2048.toml`, both
cuts and every count, mirroring HD's arrangement (2048's horizontal pair changed with it,
deliberately). HD's authored `vert_` layouts still answer HD. 2048's file checked against
Omega: Omega's HUD vocabulary is HD's, so `SplitHud_HD.toml` applies there, not wired.
PS2 Pulse's split HUD, which floated at 76% of the view because the grid's cells are 2.0 by
1.607 pixels and `hud_views` assumed square ones, is fixed (per-axis units; PSP, Pure, HD and
2048 byte-identical, PS2 splits moved on purpose). ~~Pure's weapon grid
is still half clipped in rows, Pulse's strip kill column overlaps the speed bars~~ (both fixed
2026-10-14, `split-hud-fixes`, as data in the two `SplitHud_*.toml`; Pure's grid on a 32:9
column too, `pure-tall-32x9`: a `tall-wide` rule from 1.4 up, `-0.14`, chosen, not measured).
~~A split Head2Head fields seven opponents and draws the Single Race readout, so its gap bar
never draws~~ **done 2026-10-14 (`split-head2head`)**: exactly two people and no AI (three or more
refused by name), each view's gap against the other person, both cuts and multi-window, Pulse's
bar in the place group; Pure and HD draw their place (no authored readout); Tournament is
campaign-only. **Still open from it**: Omega's
`Zone_HUDvert` and its non-authored cases, and a real display.
**Still open before it**: Time Trial and the Eliminator on Pure and HD and
HD's Eliminator column are claimed by name and not looked at, the PS2's text reads smaller
than the PSP's in the same file, and nobody has judged the layouts on a real display.

Split screen is also the one feature with existing authored data to check
against: the PS2 build's menu XML already has `Team SelectionSplit`/`Track
CreationSplit` screens and a `MP_Screen.xml` slideshow that this project's
picker currently strips (`picker::strip_player_suffix`,
`handover/frontend/pulses-race-box-is-read-both-pressings.md:108`). Worth
reading that thread before building the front-end leg, since the original's
screen flow for enabling split screen is already partly RE'd.

## 2. Multi-window - same simulation, different rendering/app-lifecycle risk

**First cut landed 2026-10-14 (lane `multi-window`), `--windows N` and `--window-seats`, desktop
only.** Design, verification and the pacing command are in `docs/rendering/multi-window.md`,
[ADR-0067](../../docs/architecture/adr/0067-a-multi-window-race-is-one-device-and-one-submission-per-window.md)
(the device, the submission, the lifecycle) and
[ADR-0068](../../docs/architecture/adr/0068-a-window-holds-a-list-of-seats.md) (a window holds a list
of seats). What is built: `Gpu` split into the device and an `Output` (window and surface); a `Screen`
per further window with its own `Framebuffer`; **a window holds a list of seats** (`Race::windows`,
`WindowViews { seats, cut }`): one seat is the whole-screen view and HUD (`Scene::render_view`,
`Layer::Window`), several are cut up inside the window exactly as a split screen (`render_window_views`
over `render_slots`, the split-HUD rules picked by the window's own count and cut); one encoder and one
submission per window; `--windows N [--players P]` deals people evenly, `--window-seats 4,2v` spells
the seats and the two-seat cut per window; window 1 keeps the front end, the pause menu and the pointer;
closing a window hands every seat it held to the AI (`Assignment::unseat` and the existing
`follow_devices`); focus is "away" only when no window has it; further windows present with
`Mailbox`/`Immediate`; the HD/2048/Omega integrated-GPU rule applies. Single-window and split frames are
byte-identical to `main` (12 of 12 `--screenshot` pairs, re-run after the seat lists).
**The layout screen landed 2026-10-14 (lane `window-layout-picker`)**: after the join screen's START
on a desktop build, WINDOW LAYOUT chooses the windows (1 to the head count, 8 at most), the seats per
window, a two-seat window's cut, each window's monitor and windowed or fullscreen, with a preview of
boxes per window; pad, keyboard and pointer; remembered in `[race] window_seats`/`window_monitors`
and re-validated against the monitors present; skipped on wasm, Android and iOS (see
`docs/rendering/multi-window.md`, "The layout screen").
**The flyby plays on every window (2026-10-14, lane `window-flyby`)**: each window that holds a seat
draws the player's flyby camera full window until it ends, then its own seats
(`Race::window_drawn`, view-side only; window 1 and the single-window split are byte-identical);
the layout screen's START row no longer shows the head count (a button shows no value). The end
screens per window were read, not exercised (no finish reachable headless or under Xvfb); nothing
looked broken. **Still open from it** (all in the doc's
"Open"): a keyboard per window; the pause overlay and
the end screens on every window; `--window-monitor` and remembered placement; per-window motion blur,
FSR 3.1 history and dynamic resolution; the camera-independent work once per frame across windows;
**pacing on real displays** (cannot be measured under Xvfb: the
`RUST_LOG=warn,oag=info oag-game --race --windows 2 --no-audio` command and how to read it are in the
doc); monitor hot-plug.

Same process, same machine, one `RaceSim`, one fixed 60 Hz tick, up to 8
viewports in **one** window. This is the cheapest of the three because it changes
nothing about process/window lifecycle (`App` stays "one per run") and
nothing about transport - it's purely: multiple local inputs in, multiple
local views out, same tick.

What's missing beyond the shared prerequisite:

- **Viewport/multi-camera support in `oag-render`.** Today there is, by its
  own doc comment, "one track-ribbon builder and one camera convention in
  the repository" (`crates/render/src/lib.rs:4`) - no `Viewport` struct, no
  split-region compositing. This is new surface area, not a variant of
  something that exists. `oag-render` owning no window
  (`crates/render/src/lib.rs:12`, permanent by design) is actually favorable
  here: the surface/window count stays 1, and split screen is purely "render
  N times into N sub-rects of the one surface," which doesn't touch the
  window-ownership boundary at all.
- ~~**Per-player HUD** in `oag-ui`, laid out per-viewport rather than
  full-screen.~~ **Done, 2026-10-13** (see below).
- **Per-player pad assignment**, covered by the shared prerequisite above.
- **A layout that scales to 8**, not just the original's 2/4-way splits -
  viewport count and arrangement (2-up, 4-up, up to 8-up) becomes a variable
  the viewport/camera-region support above needs to handle, not a fixed
  quadrant layout.

**2026-10-13: the viewport half landed**, `--players N` (1 to 8) on a direct
`--race`, written up in [split-screen.md](../../docs/rendering/split-screen.md):
`oag_render::viewport` lays out 1 to 8 views, `oag_raceplay` keeps a view-side chase
camera per extra view (`Race::set_viewpoint`) and `Scene::render_viewports` draws one
scene per view, each in its own encoder and submit with only the first clearing.
Slots `0..N` are marked `Controller::Local`; view `i` follows slot `i`. One view is
byte-identical to before. Layout, the shared far-chase rig, no shake on extra views,
no bloom/blur/jitter under a split and the hidden HUD are all **chosen, not
measured**. **2026-10-13, the per-player HUD landed**
([split-screen.md](../../docs/rendering/split-screen.md#hud)): one `Readout` per
viewed slot (`Race::readout_for`), the shield bars' state per slot, one HUD per view
drawn in one list, a per-view reticle off `sight_target(slot)`, and HD's own
two-player split layouts for a two-view race. **Still open from this list**:
per-team camera rigs, per-view Pulse bloom (`oag_post::bloom` ignores the viewport
origin), the 8-up frame budget (measured, `split-render-cost` below: 9.4 ms of CPU on the workstation, Deck unmeasured). ~~The results/end-race flow (still slot 0's, ends
every view)~~ **2026-10-13, `split-screen-end-race`: done** (below, and
[split-end-race.md](../../docs/ui/split-end-race.md)). **2026-10-13, `hd-split-black-view`: HD's and Omega's post chains
checked and fixed.** On HD and 2048 a split drew only its last view (the encode pass
clears the frame), and on Omega and HD the last view was graded from the first view's
pixels; the chain now runs once over the whole area after the last view
([split-screen.md](../../docs/rendering/split-screen.md#drawing)). **Open from it**
(all chosen, not measured): one shared exposure and glow for all views, HD's boost
zoom streak and Omega's distortion offsets are not drawn under a split, and a
race that borrows HD's zoom ring (`Scene::run_boost_zoom`) still runs it per view
from the primary's pulse (read from the code, not captured).
**2026-10-13, `split-render-cost`: a split is CPU-bound on a discrete GPU, and the first cut is in.**
Measured on the RX 7800 XT (RADV, release, 1280x720; the tables and how to rerun them are in
[split-screen.md](../../docs/rendering/split-screen.md#cost)): Pulse at eight views cost
19 ms of CPU a frame and 1.1 ms of GPU scene passes, so the GPU was never the limit there.
The Raphael iGPU proxy flips it at one and two views (GPU-bound, 4 to 5 ms of scene passes). The shadow gather, both shadow maps, the scenery
clocks and the advert cards were redone per view though they do not depend on the camera;
the first view now does them alone (`Layer::Next` skips them) and eight views cost 9.4 ms.
Pictures are byte-identical (29 stills). **Open from it**: the Steam Deck number, which
decides whether the next lever is a per-view draw budget (CPU) or render scale (GPU); steps
in the doc, the maintainer runs them. `encoder.finish` looks proportional to the draw count (the lever for per-view LOD and draw
distance, see `handover/rendering/graphics-settings-resolve-from-intent-and-context.md`),
and per-view uniform slices are the prerequisite for recording views on several threads.
**2026-10-13, `shadow-gather-per-tick`: the gather is per tick now, and `mapped` follows each view.**
The hull gather is taken at each tick's poses and lerped between ticks (once per tick, not per
rendered frame): 47 of 49 stills `cmp` equal to main, the lerp within 0.053 world units at p99 and
0.237 at worst of the live gather on a real Zone lap, a 240 Hz window saves 1.5 ms of CPU at one
view and 1.25 to 1.6 ms up to eight, a 60 Hz window neither gains nor loses
([split-screen.md](../../docs/rendering/split-screen.md#cost)). The `mapped` depth map is redrawn
per view around that view's craft (view two's shadows are there where main's had none): about
0.1 ms of CPU a view on the discrete card and **1 to 1.6 ms of iGPU time a view**. **Open from it**:
the floor rays and the track caster, both done in `shadow-floor-rays` (next entry); the bench
needs `OAG_RENDER_BENCH_FRAMES_PER_TICK` set to mean a refresh rate.
**2026-10-13, `shadow-floor-rays`: the floor rays ask once per point, the `mapped` track is culled to each map's box.**
`conform_to_floor` remembers each `(position, normal)` it has asked the road about for the one
hull: the gather is bit-identical (496,800 vertices over 300 ticks hash equal, 26 stills `cmp`
equal) and 2.9 times cheaper, 1,196 -> 417 us a tick on the eight-craft grid, 1.0 ms of CPU a frame
at a 60 Hz window on the RX 7800 XT at every view count. The `mapped` depth map records only the
track draws whose bounding sphere touches that map's box (Pulse 465 of 1,737, Pure 123 of
1,179): pictures identical, the iGPU's pipelined time down 0.3 to 0.7 ms at one view and about 2
ms at eight views (18.45 -> 16.55 on Pulse, so eight views now hold 60 Hz there). No simulation
crate changed. **Open from it**: a cheaper per-ray query is a semantics change (`floor_above`
sees only the first four hits in collider and triangle order); culling the gather to craft some
view can see needs the two-tick cache to keep every craft's gather; the Steam Deck numbers.
([split-screen.md](../../docs/rendering/split-screen.md#cost))
**2026-10-13, `pad-join-assignment`: the pad-to-slot UI is done.**
`oag_input::pad::Assignment` gained `Seating::Seated` (an unassigned pad takes the
lowest free human slot, a lost pad frees its slot, a pad past the humans drives
nothing), `--players N` seats the keyboard and first pad on slot 0 and the rest in
connection order, and a SPLIT SCREEN row under RACEBOX (oag-extras) opens a "press a
button to join" screen (`oag_ui_screens::join`, `oag_input::lobby`) whose START goes
on to Track and Ship Select with the seated count. All **chosen, not measured**; the
PS2's authored flow is a camera choice (`FE_RACE_CAM_SS`) and a per-player team
pick, not a device join. See [split-screen.md](../../docs/rendering/split-screen.md).
A local slot with no device on it (a pad unplugged mid-race, or fewer pads than `--players`) flies as the AI (`split_screen::follow_devices`), because a race now ends only when every person-flown slot has finished. **Open from it**: two real pads on real hardware (the rules are unit-tested over stand-in
ids, the keyboard path runs end to end; since `per-player-ship-pick` two virtual uinput
pads also walked the join screen, Track and Ship Select and a race in an Xvfb).
**2026-10-13, `per-player-ship-pick`: each seat picks its own craft.** The PS2 disc's
authored `Team SelectionSplit` (both players on one page, top and bottom, READY once
each confirms) is followed on PS2 Pulse with two seats; HD authors the players taking
`Team Selection Player 1` then `Player 2` in turn, which every other title and any
count above two get (chosen). Each seat is driven by its own device and a pick reaches
slot `i` (`Options::seat_picks`; `--seat-team` headless). Written up in
[split-screen.md](../../docs/rendering/split-screen.md#ship-select-per-seat). **Still
open**: ~~HD's front end shows no SPLIT SCREEN row~~ (it always had one, revealed under
RACEBOX; walked 2026-10-13, see the entry below), stick navigation on a seat's screen, a variant family's
duplicate check on HD, and where Omega authors its Ship Select screens.
**2026-10-13, `split-screen-end-race`: the end of the race is every person's.**
A person's view holds after their own line (lap and clocks held, a `FINISHED` line, the
driver flying their craft); when the last person finishes one view is drawn
(`Race::views_drawn`) under end screens that name every person: Pulse's own
`PopulateMultiplayerTable` (read, network-gated in the original, ours for a split) with
`PLAYER 1`/`PLAYER 2` and a highlight per person, HD's grid with the same names, the
built-in table's `P1`/`P2` for Pure and 2048 (whose touch pages summarise one racer). A
split banks no loyalty, builds no `Rewards`, plays no results voice; restart keeps the
seats and quit unseats (already the case). `--autopilot` now flies every local seat so
a headless 2-person race reaches its end. **Found, and wired the same day (below): the PS2 pressing authors
`EndRace Results Split` and `EndRace Rewards Split`** (two stacked per-player lap tables
and reward panels, `Player_0`/`Player_1`, two people only; its executable names them
beside `Split ScreenV/H` and `Kill Game Multiplayer`), see
[split-end-race.md](../../docs/ui/split-end-race.md).
**2026-10-13, `pulse-split-pages`: Pulse's two-person screens, both pressings.** `EndRace
Results Split`, `EndRace Rewards Split` and `Team SelectionSplit` are wired for Pulse on
the PS2 (the disc's own) and the PSP (the PS2's design, **borrowed**, built from the PSP's
own assets and labelled so in the loader report; `FrontEnd::borrows_ps2_split_screens`,
Pulse alone). The geometry bug is fixed (the grid reads each file's own rows). With two people
each banks their own team's loyalty and `Rewards Split` shows it (chosen). Written up in
[split-end-race.md](../../docs/ui/split-end-race.md) and
[selection-screens.md](../../docs/ui/selection-screens.md). **Still open from it**: the PS2
executable's entry law for these screens (not in a Ghidra project here), the PS2's highlight
placement (chosen), a live walk on the PS2 pressing and with real pads, the second person's
speedup-pad column (the race keeps the tally for the first craft), HD's podium for a split,
per-person records and ghosts, and a still of the shared Ship Select with both crafts.
**2026-10-13, `split-hud-layout-toml`: the split HUD is data.** A title's split-screen
layout is `schemas/v1/split-hud.tosd` files (`oag_hud::split_hud`): override, then the
disc's authored layout (HD and, newly, Omega: ported, and a two-view frame looked at), then ours through
the fallback layer (`SplitHud_{Pulse,Pure,HD}.toml`, chosen, not measured, Pulse's one
file for both pressings), then the scaled whole-screen layout; one rule per view shape,
so three, five and seven views mix. Each view also has a scissor now. See
[split-screen.md](../../docs/rendering/split-screen.md#hud) and
[asset-layers.md](../../docs/architecture/asset-layers.md#a-split-screen-hud-layout-file).
**2026-10-13, `split-hud-vertical`: vertical columns.** A two-view vertical cut reads a `tall`
rule (`views = [2]`, `aspect = [0.0, 2.4]`) in `SplitHud_Pulse.toml` and `SplitHud_Pure.toml`
(Pulse gains the Eliminator's kill column and the Pilot Assist indicator, Head2Head's bar is named and not seen drawing; Pure's
weapon grid is raised clear of the bottom edge) and 2048 has its own `SplitHud_2048.toml`, both
cuts and every count, mirroring HD's arrangement (2048's horizontal pair changed with it,
deliberately). HD's authored `vert_` layouts still answer HD. 2048's file checked against
Omega: Omega's HUD vocabulary is HD's, so `SplitHud_HD.toml` applies there, not wired.
PS2 Pulse's split HUD, which floated at 76% of the view because the grid's cells are 2.0 by
1.607 pixels and `hud_views` assumed square ones, is fixed (per-axis units; PSP, Pure, HD and
2048 byte-identical, PS2 splits moved on purpose). ~~Pure's weapon grid
is still half clipped in rows, Pulse's strip kill column overlaps the speed bars~~ (both fixed
2026-10-14, `split-hud-fixes`, as data in the two `SplitHud_*.toml`; Pure's grid on a 32:9
column too, `pure-tall-32x9`: a `tall-wide` rule from 1.4 up, `-0.14`, chosen, not measured).
~~A split Head2Head fields seven opponents and draws the Single Race readout, so its gap bar
never draws~~ **done 2026-10-14 (`split-head2head`)**: exactly two people and no AI (three or more
refused by name), each view's gap against the other person, both cuts and multi-window, Pulse's
bar in the place group; Pure and HD draw their place (no authored readout); Tournament is
campaign-only. **Still open from it**: Omega's
`Zone_HUDvert` and its non-authored cases, and a real display.
**Still open before it**: Time Trial and the Eliminator on Pure and HD and
HD's Eliminator column are claimed by name and not looked at, the PS2's text reads smaller
than the PSP's in the same file, and nobody has judged the layouts on a real display.

Split screen is also the one feature with existing authored data to check
against: the PS2 build's menu XML already has `Team SelectionSplit`/`Track
CreationSplit` screens and a `MP_Screen.xml` slideshow that this project's
picker currently strips (`picker::strip_player_suffix`,
`handover/frontend/pulses-race-box-is-read-both-pressings.md:108`). Worth
reading that thread before building the front-end leg, since the original's
screen flow for enabling split screen is already partly RE'd.

## 2. Multi-window - same simulation, different rendering/app-lifecycle risk

**First cut landed 2026-10-14 (lane `multi-window`), `--windows N`, desktop only.** Design,
verification and the pacing command are in `docs/rendering/multi-window.md` and
[ADR-0067](../../docs/architecture/adr/0067-a-multi-window-race-is-one-device-and-one-submission-per-window.md).
What is built: `Gpu` split into the device and an `Output` (window and surface); a `Seat` per further
window with its own `Framebuffer`; every window runs the whole single-window chain, one encoder and
one submission each (`Scene::render_view`, `Layer::Window`, `Race::windowed`); window 1 keeps the front
end, the pause menu and the pointer; a seat window closing hands its slot to the AI
(`Assignment::unseat` and the existing `follow_devices`); focus is "away" only when no window has it;
seat windows present with `Mailbox`/`Immediate`; the HD/2048/Omega integrated-GPU rule applies.
Single-window and split frames are byte-identical to `main` (12 of 12 `--screenshot` pairs).
**Still open from it** (all in the doc's "Open"): ~~a menu row for the window count~~ (the layout screen, below in section 2's newer block); a keyboard per
window; the pause overlay and the end screens on every window; `--window-monitor` and remembered
placement; per-window motion blur, FSR 3.1 history and dynamic resolution; the camera-independent
work once per frame across windows; **pacing on real displays** (cannot be measured under Xvfb: the
`RUST_LOG=warn,oag=info oag-game --race --windows 2 --no-audio` command and how to read it are in the
doc); monitor hot-plug.

Same process, same machine, one `RaceSim`, one tick - but up to 8 *OS windows*
instead of up to 8 viewports in one window (each player's view full-screen in
their own window, e.g. for a multi-monitor local setup). Simulation-side,
this needs nothing split screen doesn't already need - it reuses the shared
prerequisite and, if built second, split screen's per-player input/HUD work
directly.

The complexity here is entirely on the app/render plumbing, and it's a
different shape of work than split screen's, not a strict superset:

- `App`'s "one of these per run, not one per window" assumption
  (`crates/game/src/main/app.rs:95`) has to go. winit's `ApplicationHandler`
  needs to route resize/redraw/input events per-window, and `Session`
  (currently singular) needs to become one-per-window or gain an explicit
  window index.
- Each window needs its own wgpu surface; `oag-render`'s device/queue can be
  shared across them (it owns no window and never will,
  `crates/render/src/lib.rs:12`), but the surface-per-window bookkeeping
  and per-window resize handling is new.
- No new viewport/camera-region concept is needed here (each window draws
  one full-frame camera) - which is why, camera-wise, this is *simpler* than
  split screen even though it's listed second: the ordering is driven by
  `App`'s lifecycle assumption being the harder thing to unwind, not by
  render complexity.

Split screen and multi-window could in principle be built in parallel once
the shared prerequisite lands, since they touch different parts of
`oag-render`/`oag-game` (viewport compositing vs. multi-surface window
lifecycle). Doing split screen first is recommended anyway: it ships local
multiplayer sooner, with zero risk to `App`'s existing single-window
lifecycle, and multi-window can then reuse its per-player input/HUD work
wholesale.

## 3. Network multiplayer - highest complexity, ours by design, and the one with no source to observe

Different processes, possibly different machines, up to 8 players. **This is
original engineering, not a port**: the design is server-authoritative with
client-side prediction and replay-based reconciliation, chosen on request
rather than derived from the original, because there is no original to
derive it from (see below). This is qualitatively harder than the first two,
for reasons beyond "more code":

- **The protocol, stated explicitly since it's the one piece of this thread
  that's a design decision rather than an architecture survey:**
  - **The server is the sole authority.** It runs the canonical `RaceSim`,
    accepts only `InputSnapshot`s from clients (never client-reported
    state), and ticks the simulation itself. A client cannot report "I am at
    this position" or "I hit that ship" and have it believed - the server
    derives both from its own tick. **Validation needs to be more than bounds
    checking, though**: `Input` (`crates/gameplay/src/input.rs:157`) tracks
    `pressed`/`released` as *edges*, derived internally from a held-mask
    history (`pressed = held & !held_last`) - a client sending its own
    precomputed edge bits could claim a press that never transitioned
    through held state. The server has to re-derive edges from a held-state
    stream it advances itself, the same way `Input::advance` already does
    locally, never trust a client's `pressed`/`released` bits directly. Also
    worth being honest about scope: server authority bounds *state* cheats
    (position, damage, "I won") - it does nothing against input-level cheats
    like frame-perfect timing or a client that withholds inputs to see
    others' positions first. Different problem, not solved by this design.
  - **Each client predicts its own ship locally**, running the same
    deterministic `Race::tick` ahead of the server's acknowledged state
    using its own just-captured input, so the local player's controls feel
    immediate rather than waiting a round trip. This is the same tick
    function every other mode already uses - prediction is "run the sim
    client-side too," not a second implementation of it.
  - **Reconciliation is a snapshot-and-replay, not a smooth correction.**
    When an authoritative snapshot arrives from the server (tagged with the
    input sequence number it was computed from), the client restores its
    `RaceSim` to that snapshot and **replays its own buffered local inputs**
    since that point forward, exactly as recorded, to arrive back at a
    corrected present. If the server's result matches what the client had
    already predicted, this is invisible; if not, the client's view snaps to
    the corrected trajectory. **Corrected: what actually has to move isn't
    the memcpy-shaped `World` alone, it's the whole `RaceSim`** - which
    carries a `Vec` (`ai_order`) to clone on restore, cheap but not literally
    a memcpy, and a `cues: Vec<CueEvent>` audio-output accumulator
    ([ADR-0018](../../docs/architecture/adr/0018-audio-mixer-architecture.md))
    that a reconciliation replay must not let reach the audio layer a second
    time - a tick that already played its cues once must not play them again
    just because it got re-run to reconcile a misprediction. `InputSnapshot`
    and the already-placeholdered `oag-replay` for M7 are still the right
    two primitives to build on; the correction is about what gets
    snapshotted, not whether snapshot-and-replay is the right shape.
  - **Remote players are not predicted the same way** - the working plan is
    interpolation/extrapolation between authoritative server updates, since a
    client has no early knowledge of another player's next input. **Flagged
    as a real disagreement worth resolving before the ADR, not settled
    here**: interpolation/extrapolation is the standard compromise in
    genres where players rarely alter each other's motion frame to frame. A
    racing pack does not have that property - ship-ship contact, and
    Leech/Quake/Disruptor hits, change a craft's motion on contact, every
    tick, and with the local ship predicted ahead and remote ships shown
    behind, a contact resolves against a remote ship that's stale by close
    to a full round trip, which is exactly when the interpolated guess is
    most likely to be wrong. Given the sim is already deterministic and
    fixed-step (ADR-0007), rollback with predicted remote inputs
    (repeat-last-known-input until a real one arrives, GGPO-shaped) is worth
    weighing against interpolation specifically for this game's tighter
    inter-ship coupling before the protocol ADR is written - server
    authority can still sit on top of either choice.
- **It needs a transport that doesn't exist as a concept anywhere in the
  codebase yet.** `docs/architecture/workspace-layout.md` lists `oag-net`
  only as a not-yet-created placeholder for M8, with the explicit warning
  that "a crate created before its shape is understood tends to get the
  wrong shape" - and the same doc's own precedent (`oag-weapons`, planned
  but never created because the work fit existing crates) means `oag-net`
  shouldn't be scaffolded until the transport and message shape (tick rate
  vs. send rate, reliable-vs-unreliable channels, snapshot delta encoding)
  is actually decided.
- **Determinism is what makes client-side prediction trustworthy**, and
  it's exactly what `ADR-0007` bought deliberately: the engine keeps a fixed
  60 Hz timestep and diverges from the original's variable-`dt` simulation
  specifically so that "replays, golden tests and cross-platform comparison
  all still work." The same property that makes a golden test reproducible
  is what makes a client's predicted tick and the server's authoritative
  tick agree often enough that reconciliation is the exception, not the
  norm - a nondeterministic sim would make every server correction visible
  as a snap, every tick.
- **`oag-core`'s existing state hash** (`crates/core/src/hash.rs`, used today
  by CI's cross-platform `determinism` job) is directly reusable to confirm
  a client and server agree on state after reconciliation, and to flag a
  genuine desync (a client running a different build, a platform-specific
  float divergence `ADR-0002`/`ADR-0007` don't fully rule out) rather than
  silently drifting.
- **There is no original implementation to reverse-engineer this from, and
  none is wanted.** `docs/networking/README.md` puts confidence at 90 that
  the PSP Pulse build ships no `pspnet` stack and no `libhttp` at all (three
  PRX modules against Pure's 26, which has the full network stack) - meaning
  Pulse likely never had online play to observe in the first place. That's a
  real break from this project's usual "observe -> hypothesize -> verify ->
  document -> implement" methodology, and it's intentional here rather than
  a gap: the protocol above is designed fresh, not ported, so there's
  nothing to verify it against except itself (tests, and eventually live
  play). `docs/networking/README.md`'s existing scope (ad-hoc PSP local
  mode, game sharing via `GSHARE/SHARE.BIN`, DLC) is a **different, narrower
  question** - recreating what the original's local/ad-hoc mode did, if
  anything - and is not what this section is planning; that scope stays
  wherever `docs/networking/README.md` already put it, unaffected by this
  thread's server-authoritative online design.

Network play only strictly needs the shared prerequisite (N inputs, up to 8
players, one `RaceSim`) - it doesn't require split screen or multi-window to
exist first, since a networked race could run one player per machine, one
window each, no local split at all. But it benefits from whichever local
multi-player work already forced the `World`/`InputSnapshot`/input-mapping
generalization to be real and tested against actual multi-player input,
rather than reasoning about it in the abstract for the first time under
network latency.

**Corrected: the server side is not already free.** The first version of
this thread claimed the headless server "already supports [running the sim
without a renderer] without change" via `oag-gameplay`'s render-free design.
That's wrong - `Race::tick`/`RaceSim` live in `oag-game`, and per
`workspace-layout.md`'s dependency rule 2, "no crate may depend on
`oag-game`," so a server can't simply be some other crate importing
`oag_raceplay`. Two real options, not one free one:
1. A second `[[bin]]` target inside the `oag-game` crate itself (its
   `Cargo.toml` already has `[lib] oag_game` plus `[[bin]] oag-game` - "a
   thin `[[bin]]` over `[lib]` so boot logic is testable headlessly," per
   `CLAUDE.md`'s crate table). A headless-server binary in the same crate
   reuses `Race`/`RaceSim`/`tick` through the lib directly, with no new
   dependency edge and no rule violation - this is the cheap path.
2. Hoisting `RaceSim` and `Race::tick` out of `oag-game` into a
   gameplay-side crate, so anything (a server, `oag-trace`, a future
   `oag-net`) can depend on it directly. Architecturally cleaner, but real
   work, and not obviously worth it just for the server - `oag-trace`
   already hit this exact wall and chose to duplicate the logic it needed
   rather than depend on `oag-game`
   (`crates/trace/src/replay.rs`'s `locate`, whose own doc comment states
   "`oag-trace` cannot depend on `oag-game`... so the resampling is
   duplicated crate to crate on purpose"), which is a working precedent for
   option 1's same-crate reuse being preferred over a wider refactor.

## 3a. Remote craft: interpolate or roll back (2026-10-13)

**Status: MEASURE. Not decided.** (2026-10-14: M2 and M3 measured on the
desktop, S24 and M1/M4 still open.) Written up from a design discussion between
the maintainer and Claude. The options, outcomes, evidence and survey below are
the record; the recommendation at the end is Claude's and is meant to be
argued with. The network-protocol ADR waits on the measurements. The permanent
summary is `docs/networking/README.md`.

### The options

Only the remote craft differ; your own craft is predicted either way.

- **A. Interpolate.** Draw other craft slightly in the past, blended between
  authoritative snapshots. Your craft runs in the present, theirs in the past.
- **B. Roll back.** Simulate every craft in the present. Guess each remote
  craft's input (repeat the last known one); when the authoritative state
  differs, restore the saved `RaceSim` to the last confirmed tick and replay.
- **C. Mix** (rollback near you, interpolation far away). Ruled out: the sim is
  one `World`, and a craft in the past cannot collide with one in the present,
  so the two timelines would need a seam inside the sim.
- **D. Split by object class** (craft rolled back, weapons as server-confirmed
  events). Not evaluated. Raised as a question for the challenge below.

### One scenario, both options

Round trip 100 ms is 6 ticks at 60 Hz. A craft covers "a few units per tick" even
at Phantom class (`docs/gameplay/lap-counting.md`), so a remote craft drawn 6
ticks stale is roughly 10 to 20 units off (an estimate, not a measurement).

- **Ship contact.** A: you see no contact, the server resolves a bump anyway and
  shoves your craft by something you cannot see (or you see a clean pass the
  server scored as a hit). B: the contact happens on your screen at about the
  right moment; if the guess was wrong the rival jumps a small distance or a bump
  is undone.
- **A missile.** A: it spawns when its snapshot arrives and is drawn behind where
  it is; homing runs against stale positions; shots at a displayed target need
  FPS-style hit rewinding. B: the fire input arrives about 6 ticks late, so the
  replay spawns the missile mid-flight and it pops into view ahead of the
  launcher; you can dodge it on screen and be hit after a rollback.

### Outcomes

| | A. Interpolate | B. Roll back |
| --- | --- | --- |
| Remote craft look | Smooth, but behind | In the present, occasional small snaps |
| Contact fairness | Worst: contact you cannot see | Mostly agrees with the server |
| Artefacts | Phantom pushes, rubber-banding at contact | Missile pop-in, undone dodges, small teleports |
| Jitter and loss | Tolerant: hold or extrapolate | Window grows with delay, needs a cap |
| Bandwidth | Server sends state (8 craft plus projectiles) at some rate | Inputs only, a few bytes per craft per tick |
| Client CPU | Cheap, no replay | Replay of up to N ticks times 8 craft per correction |
| Determinism | Tolerates drift, snapshots correct it | Inputs-only needs bit-exact sims |
| Mod mismatch | Remote craft still draw; own prediction suffers | A mismatch mispredicts constantly, so the fingerprint must match to join |
| Code to build | Snapshot format, interpolation buffer, hit rewinding | Sim-only tick path, snapshot budget, replay window cap |

Where each breaks. **A** breaks on contact, which a Wipeout pack has constantly;
it is the better fit for high-jitter links and weak devices. **B** has three
specific risks, all unmeasured: (1) an analog stick changes almost every tick,
so "repeat the last input" is rarely exact and a strict policy would roll back on
nearly every server packet, about 60 rollbacks a second of about 6 ticks each, so
roughly 7 times the sim cost (an expectation from the input type); (2) the
project targets phones (`oag-mobile`), where a replay budget that is fine on a
desktop may not be; (3) the repo gaps already listed under Open: a replay
re-fires view and audio effects, `RaceSim` clones the track, `TickClock` caps
catch-up at 8 ticks, and (closed 2026-10-13) a remote slot's input reaching no craft, which
`crates/raceplay/src/tests/reconcile.rs` now pins the other way.

### Evidence

**Rocket League, primary source.** Cone, "It IS Rocket Science!", GDC 2018
(slides at `https://media.gdcvault.com/gdc2018/presentations/Cone_Jared_It_Is_Rocket.pdf`,
GDC Vault 1024972), read as `pdftotext` output. The animations and the spoken talk
are not covered, so a diagram-only or spoken point is missing.

| The slides say | What it means here |
| --- | --- |
| Challenges: "Input delay is not an option", "Server can't wait for client input", "100% server authoritative"; fixed tick 120 Hz | Same constraints as ours except weapons. My earlier "1 to 3 ticks of input delay" conflicts with this and is dropped from the recommendation |
| Hitting a moving object: "Client predicts his vehicle / Server authoritative ball / Client interpolates ball / Client's vehicle and the ball exist in two different timelines" | They tried option A for the ball and it failed for hits. The strongest evidence against A for contact |
| FPS-style lag compensation shown with a 200 ms and a 10 ms client; the closing slide says "No server-side lag compensation" | Rewinding hitboxes was weighed and not shipped |
| Input buffer: client sends one input per physics frame; the server buffers them, "No need to pause for input", "Eliminates some cheats (speed, jitter)", "Increases average latency"; empty buffer runs physics on the previous input; upstream throttle (server tells the client to run extra or fewer frames); downstream throttle (server consumes 0, 1 or 2 inputs a frame, "minor desyncs") | A server-side jitter buffer with throttles is part of the design. It replaces my earlier "server stamps inputs and drops late ones" |
| Prediction and correction: record input and frame number, run physics, record history; server returns frame number and physics state; compare with history; "Large difference requires correction"; "Revert all physics actors to that frame", "Run multiple physics frames to catch up" | The machinery we already have (snapshot, restore, replay), with a threshold, applied to all actors |
| "Predict Everything": "Works well with ball (predictable)", "Not as well with cars (unpredictable)", "Expensive corrections: 200ms ping, 120hz = 24 correction frames". Results: "No input delay", "High-ping clients don't ruin game", "Reliably hit moving objects", "100% server authority" | Remote cars are the weak case, and cars are our whole game. At 60 Hz, 200 ms is 12 frames. Not stated in the text: what input a remote car is assumed to hold |

Limits of the analogy. Rocket League has one ball, a few cars, no weapons and
simple bodies; we have 8 craft, projectiles and a track collision soup. The 60 Hz
server send rate is secondhand (a Steam forum post relaying a Psyonix developer,
disputed in the same thread).

**Papers**, from search excerpts only (see `docs/networking/README.md` for the
list): Mauve et al. 2004 (local lag is deliberate input delay, timewarp is
rollback; the choice against dead reckoning is application-dependent), Fiedler's
networked-physics series (lockstep needs exact determinism and every frame waits
for all players), Pantel and Wolf (two racing games handled delay-induced
collisions poorly; stale opponent positions without compensation).

**Other racing games.** One search each; no technical description found for any.

| Game | What the sources say | Quality |
| --- | --- | --- |
| iRacing | Architecture undocumented in what I found. A forum user believes it shows little warping; one article describes long-running "phantom collisions" | Forum belief and one article |
| Forza Horizon 5 | A report says contact can happen on one screen and not the other, which points to per-client views that disagree. Forza Motorsport: only server-side progression | Anecdotal |
| Mario Kart 8 Deluxe | Press attributes disconnects to peer-to-peer; players reported inconsistent hits and repeated items; a patch fixed an item bug under unstable connections | Press attribution |
| Gran Turismo 7 | Always online, which Polyphony said is meant to curb cheaters. Nothing on collision sync; one guide claims a peer-to-peer hybrid, uncorroborated | Low |
| Codemasters F1, ACC, Trackmania, Wipeout HD | F1 older games were host-based (a hosting vendor's claim); ACC ships a standalone dedicated server; Trackmania matchmaking has no car collisions (sync unconfirmed); Wipeout HD nothing | Low |

The pattern: every complaint found is about contact (phantom collisions, ramming,
inconsistent hits), consistent with stale remote cars (option A) but not proof of
what any of those games do.

### Measure (the gate)

1. **M1, remote-input predictability.** Replay recorded human (or AI) input as the
   remote craft and count corrections and position error of last-input prediction
   over windows of 3, 6 and 12 ticks (50, 100 and 200 ms), under both "every
   packet" and a threshold policy. A remote slot's input reaches its craft since 2026-10-13.
2. **M2, replay cost per tick.** Time one sim-only `Race` tick for 8 craft plus
   weapons on the desktop and on the Galaxy S24. Budget is rollbacks per second
   times window ticks times that cost. Runs on the existing loopback in
   `crates/raceplay/src/tests/reconcile.rs`.
   **Desktop done 2026-10-14**, S24 open: 60 us per sim-only tick against 65 us
   for `Race::tick` (8 craft, 12 Pulse circuits, `dev` profile, loaded
   machine); a 6 tick rollback including the restore is 0.42 ms, 2.5 percent of
   a frame. Method, spread and load in `docs/networking/README.md`,
   "Reconciliation cost".
3. **M3, snapshot cost.** `RaceSim` clone with and without the immutable track
   half, in milliseconds and bytes.
   **Desktop done 2026-10-14:** 63 us and 1.7 MiB whole, of which the track half
   is 47 us and 0.74 MiB. A snapshot costs as much as a tick, so a per-tick
   snapshot doubles sim CPU and the split is worth taking only for a design that
   does that; whole-struct clone stands until then. S24 open.
4. **M4, what a correction looks like.** A scripted contact scene with the
   renderer-side smoothing, judged by eye.

M2 and M3 were roughly half a day on the existing loopback (desktop half done
2026-10-14). M1 and M4 waited on the
remote-slot input path, which landed 2026-10-13 (`Race::tick` steps every
person-flown slot), so both are unblocked.
Proposed pass condition, number chosen arbitrarily and open to challenge: a
100 ms round trip (6 ticks) costs at most 25 percent of a 16.7 ms frame on the S24.

### Recommendation (Claude's, to be challenged)

1. Option B: predict every craft in one present timeline and replay only on a
   large difference from the server's state, as Rocket League does. Periodic server
   snapshots are the safety net.
2. The server buffers each client's input, consumes one per tick, throttles the
   client up or down, and repeats the previous input on an empty buffer.
3. No client input delay by default. If M1 shows rollback storms on bad links, an
   opt-in local lag for those links only.
4. Interpolation only as the fallback for a stalled link or a craft beyond the
   window cap. Correction smoothing in the renderer only.

Why: Rocket League tried the interpolated-object approach for hits and shipped
predict-everything; it also weighed and dropped lag compensation. Contact is the
core of our game. This reuses the snapshot, restore and replay machinery that
exists.

Assumptions to attack: the sim ticks cheaply enough (M2); remote stick input is
smooth enough for last-input prediction (M1); 8 craft plus weapons are not much
more chaotic than Rocket League's scene; bit-exact determinism holds on every
client including modded rules; a phone can afford the replay.

Weakest points. Rocket League itself says its predict-everything is weak for
cars, and a pack of 7 remote craft is that case at scale. It has no weapons, so
missile pop-in and retroactive hits (dodge on screen, hit after the rollback) are
untouched by its evidence. Our bodies and collision are heavier. The sim-only tick
path is not built.

What would change my mind: M1 shows many corrections a second even with a
threshold; M2 puts the 6-tick replay over the budget on the S24; playtests find
weapon pop-in unacceptable. Then A for remote craft plus a cheap contact
prediction, or D (weapons as server-confirmed events) on top of B.

Questions for the challenge: is client input delay really off the table at 60 Hz
with heavier craft? Is 25 percent of a frame the right budget? Should weapons be
server-confirmed events from the start (D)? Should the correction threshold be in
world units or in effect on lap or contact outcome?

## 3b. Transport: WebTransport for the server and the web build (2026-10-13)

**Status: leaning, not decided.** Raised by the maintainer: support WebTransport
on the server and in the web build, "or even all builds". Not measured, and the
crate facts below come from search excerpts, so check them against docs.rs before
choosing.

**Decided, 2026-10-13 (maintainer): accepting self-signed certificates on a LAN is
acceptable.** A native client may trust a LAN host's self-signed certificate
(pinning or trust on first use). A browser has no click-through for WebTransport
(from memory, verify), so a web client still needs the host to advertise the
certificate hash for `serverCertificateHashes`, for example in the join code or
LAN discovery. A public dedicated server still uses a normal certificate.

**Why it comes up.** The web build (`docs/tools/web.md`) is `wasm32` in a browser
tab. A browser cannot send raw UDP, WebSocket is TCP (a lost packet stalls every
packet behind it, which hurts 60 Hz input streams), and WebRTC data channels need
signalling and ICE. WebTransport gives unreliable datagrams plus reliable streams
over QUIC, which is exactly the two channel kinds the protocol needs: inputs and
fast state on the unreliable one with redundancy, events and snapshots on the
reliable one.

**Browser support (search excerpts, not caniuse or MDN).** Chrome and Edge from
97, Firefox from 114, Safari 26.4 in March 2026 with WebTransport on by default.
Sources call it Baseline since about March 2026 but "newly available", not
"widely available". One source warns that engines disagree about what a server
must advertise (a legacy `:protocol = "webtransport"` token versus the newer draft
value), and that devtools do not show its traffic yet.

**Constraints that shape the design.**
1. It needs TLS. A public dedicated server gets a normal certificate. A LAN
   listen server has none, so browsers must pin it with `serverCertificateHashes`
   (from memory: SHA-256 of a self-signed ECDSA P-256 certificate, validity under
   14 days; verify). Native clients can pin or skip verification. How a LAN host
   mints the certificate and advertises its hash to web clients is the part to
   prove first; native clients accepting it is decided (above).
2. A browser tab cannot accept inbound connections, so a web build can be a
   client but never a listen server. A LAN game with a browser player needs a
   native or dedicated host.
3. UDP-blocked networks fail QUIC. A WebSocket fallback would cost the
   head-of-line behaviour; decide whether to offer one at all.
4. A listen server then has to run an HTTP/3 stack inside a game client.

**Rust side (search excerpts only).** `wtransport` is a pure-Rust WebTransport
crate; its old 0.1.x docs needed Chrome flags for a self-signed certificate and a
newer `SelfSignedIdentityBuilder` was reported but not verified. The
`web-transport-quinn` family lists certificate fingerprints among its features,
and `aeronet_wt_wasm` was found as a browser-side client crate. None of this was
read in the crates' own docs.

**The choice, "which builds".**
- **WebTransport everywhere.** One wire protocol and one stack on every client and
  the server. Cost: the certificate dance and an HTTP/3 layer even for native LAN
  play.
- **WebTransport for web and the dedicated server, plain QUIC or UDP for native.**
  Native skips the HTTP/3 and certificate layer. Cost: two stacks, and the server
  speaks both anyway.
- **A transport trait with the two channel kinds, WebTransport implemented first.**
  Keeps the other two open and costs little.

Claude's lean, to be challenged: the trait with WebTransport first, and "all
builds" as the target if the LAN certificate story proves workable, since one wire
protocol is worth a lot while the protocol is still being designed. Prove first,
in order: a native server and a native client over WebTransport with a pinned
self-signed certificate; the same server from the wasm build. Roughly a day each,
a guess.

## Recommended order

1. ~~**Shared prerequisite**: per-player `RaceState`, N-input `Race::tick`,
   device-to-player mapping in `oag-input`. Touches `oag-game` as well as
   the two gameplay-side crates (see the corrected "Why one prerequisite
   gates all three" section). Blocks all three below and `oag-replay` (M7).~~
   **Done, 2026-09-16.** All three landed, plus the headless-server binary
   section 3 asked for, in four commits:
   - `World::race` is `[RaceState; MAX_PLAYERS]` with `MAX_PLAYERS = MAX_SHIPS
     = 8`, and `World::controllers: [Controller; MAX_PLAYERS]` beside it -
     `Ai`, `Local` or `Remote`, both hashed. `World::SINGLE_PLAYER` is the one
     `Local` in slot 0 the engine already had hard-coded, and
     `human_slots`/`primary_slot`/`primary_race`/`mode`/`laps_target` are what
     replaced the literal `0`s. `crates/gameplay/tests/determinism.rs`'s
     constants moved, isolated the documented way - see that file's own
     2026-09-16 history entry.
   - `Race::tick` takes `oag_gameplay::PlayerInputs`, a fixed
     `[InputSnapshot; MAX_PLAYERS]`. The player slot is threaded into
     `flown_for_the_player`, `autopilot_controls`, `grant_free_turbo`,
     `spend_pickup`, `tick_autopilot` and `lost_off_the_track`;
     `advance_cannons` asks `World::controllers` instead of `slot == 0`.
     ~~**The player step still runs once** and returns one `Evaluated`~~ -
     superseded 2026-10-13: it runs once per person-flown slot and returns
     `Evaluations` (see Open, "Every human slot's input reaches its own
     craft"). N cameras and N HUDs are still step 2's work.
   - `oag_input::pad::Assignment` maps `GamepadId` and the keyboard to slots;
     `Pad::poll_players` and `Controls::player_snapshots` read per slot, and
     `Controls`' button state is one `Input` per slot because an edge is per
     pilot. Default: every device on slot 0, which is the old merged stream.
   - `oag-headless-sim`, a `[[bin]]` of `oag-raceplay` (it was `oag-game`'s until the race left it) over
     `Setup::headless`, ticks with no renderer and prints its state hash -
     section 3's option 1, taken. `just headless-sim` runs it; its claims are
     also in `crates/raceplay/src/tests/headless.rs` so the gate holds them.

   Behaviour is unchanged throughout: a single-player race still reads exactly
   `ships[0]`. ~~No ADR was written~~ - **done, 2026-09-16**:
   [ADR-0052](../../docs/architecture/adr/0052-world-and-race-tick-widen-to-n-players.md)
   records it, written from what shipped rather than from a plan, same as this
   thread's own doc comments on `World::race`, `World::controllers`,
   `Controller` and `PlayerInputs` already did informally.
2. **Split screen**: viewport/camera-region support in `oag-render`,
   per-player HUD in `oag-ui`. Lowest risk - no window-lifecycle changes.
   **Viewports, cameras and the per-player HUD done, 2026-10-13** (see section 1);
   pad assignment and front-end entry remain.
3. **Multi-window**: unwind `App`'s one-window-per-run assumption, per-window
   wgpu surfaces. Can run in parallel with (2) once (1) lands; ordered after
   split screen here only because it's more invasive to `oag-game`'s current
   structure.
4. **Network multiplayer**: server-authoritative with client-side prediction
   and replay-based reconciliation, up to 8 players, decided per section 3 -
   original design, not a port. ~~Before a transport or `oag-net` exists,
   prove the shape cheaply with an **in-process loopback client/server**:
   two `RaceSim`s in one test binary, inputs passed directly (no socket),
   asserting the client's post-reconciliation `state_hash` matches the
   server's.~~ **Done, 2026-09-21.** `RaceSim` derives `Clone`,
   `Race::snapshot_sim`/`Race::restore_sim` are the pair
   (`crates/raceplay/src/reconcile.rs`), and
   `crates/raceplay/src/tests/reconcile.rs` is the loopback: a client
   predicts wrong for forty ticks, is handed the server's authoritative
   snapshot and the true inputs, and lands on the server's `state_hash`
   exactly. It asserts the divergence *before* reconciling, so it cannot pass
   vacuously as a restatement of determinism, and it covers the
   already-correct case as a no-op. No transport, no message format and no
   `oag-net` came with it, deliberately. **The transport and message-format
   decision (needed before `oag-net` is created) is now the thing worth
   making** - subject to the remote-player question below, which this did not
   settle.

## Open

- ~~Whether `RaceState` should become a fixed `[RaceState; MAX_PLAYERS]` or
  gain an explicit human/AI-slot distinction on top of the existing
  `[Ship; MAX_SHIPS]` grid - not decided here, needs a design pass against
  how AI opponents and human players currently share the same `ships` array.~~
  **Done, 2026-09-16. Both, not either.** `race` is the fixed array, indexed
  by grid slot so `race[i]` and `ships[i]` are the same racer by construction,
  *and* `controllers` is the human/AI distinction beside it. Collapsing them
  would have made "has a `RaceState`" mean "is human", which destroys the one
  thing the array is for - ranking eight timed craft against each other.
  `RaceState::mode`/`laps_target` are replicated eight times with only slot
  0's authoritative, read through `World::mode`/`World::laps_target`; that
  redundancy is recorded on the field rather than hoisted out, because
  splitting `RaceState` is a change to `oag-race`'s own recovered type.
- ~~An ADR for the per-player shape is still owed.~~ **Done, 2026-09-16** -
  [ADR-0052](../../docs/architecture/adr/0052-world-and-race-tick-widen-to-n-players.md).
  **Still owed, and distinct**: the network-protocol ADR (server-authority,
  client prediction, reconciliation) section 3 describes - that one still has
  a real open design question in it (remote-player prediction, below) and
  shouldn't be written until that's resolved.
- ~~**A reconciliation replay re-fires every view and audio side effect, not
  just the cues.**~~ **Landed 2026-10-14** (lane `reconcile-quiet-tick`):
  `Race::tick_sim_only` (`crates/raceplay/src/reconcile.rs`) is the replay step.
  It is a mode on the one `Race::tick` body (`Race::quiet`, read through
  `draws()`) and not a second entry point, because the view writes sit inside
  the sim's helpers (shield flash, pad plume, respawn trail, run statistics) and
  a second body would be a 700-line copy. The cues, announcements and class
  announcements are dropped by truncating the three queues to their entry
  length. Pinned by `tests::reconcile::a_quiet_replay_reaches_the_hash_of_a_normal_one`
  (two people, every tick, every slot), `a_quiet_replay_leaves_the_view_alone`
  (the whole `RaceView` as text, unchanged, against a control that moves),
  `a_quiet_tick_enters_the_pad_without_the_plume_or_the_cue`,
  and the disc-backed `quiet_replay_ground_truth` (eight craft, weapons in every
  hand, a real circuit). The loopback's three replays now go through it.
  **Still open from it:**
  - ~~**The sim reads the reticle, so "hash always equal" and "camera untouched"
    cannot both hold.**~~ **Landed 2026-10-14** (lane `sim-side-lock`). The lock a
    Missile or LeachBeam fires at is `RaceSim::locks`, advanced from the world
    with no camera (`crates/raceplay/src/lock.rs`) and hashed; the reticle follows
    it. `update_sight` is gone from the sim-only tick and the quiet-replay hash
    tests hold with no exception. See `docs/networking/README.md`, "The lock is
    simulation state". **Still open from it:** opponents have no lock (their shot
    takes the window's candidate with no hold), so an AI missile is guided at
    once; routing them through the hold is an AI behaviour change that moves AI
    race hashes and was left.
  - **A recording does not hear a replayed tick.** `record_tick` is skipped, so
    a client recording a race keeps the inputs it first predicted.
  - **`restore_sim` restores undrained cues too.** `sim.cues` is part of the
    clone; a client that drained cues after the snapshot tick would hear them
    again once the snapshot is put back.
  - `capture_results` and `wreck_ended_tick` still run in a sim-only tick,
    because the caller reads them back.
- ~~**A remote slot's input still reaches no craft and no hash, so nothing can
  mispredict one yet.**~~ **Landed 2026-10-13** (lane `remote-slot-input`).
  `Race::tick` runs the player step for every slot `World::person_flown` names
  (a `Local` or `Remote` controller with a craft on the grid, plus the primary
  slot always) in `crates/raceplay/src/player_step.rs`, and `step_opponents`
  skips those slots. It returns `Evaluations`: a `[Option<Evaluated>; MAX_PLAYERS]`
  with `get(slot)`, `primary()` and a `Deref` to the primary's, so a
  single-player caller reads `.wall` as before. Pinned positively by
  `tests::reconcile::a_remote_slots_input_reaches_its_own_craft` (two races
  identical but for slot 1's stick: hash differs, slot 1's heading turns, slot
  0's craft does not move) and
  `a_mispredicted_remote_slot_reconciles_to_the_servers_hash` (the loopback
  with the wrong guess on slot 1). The old pin passed vacuously: its slot 1 was
  never made active, so a stepped slot 1 would have been skipped too; the new
  fixture activates it and sets it 60 units to the side so `resolve_craft_pairs`
  does not mask the stick. **Sections 3a's M1 and M4 are unblocked**: a remote
  slot's input now reaches its craft. ADR-0052's "the player step still runs
  once" consequence no longer holds; the decision text stays as written (ADRs
  are immutable) and a superseding note is the maintainer's call.
- **What is per slot now, and what stays on the primary (audit of every
  `primary_slot()` use, 2026-10-13).** Simulation, now per person-flown slot:
  controls, autopilot (the operator's `--autopilot` flag flies the primary only),
  pickup spend, pads, the force law, the off-track/airborne/reset rescue with its
  own `last_on_track[slot]`, `race[slot]` rules (laps, Zone counter and
  recharge, free turbo, Eliminator lap refill, `eliminate`), the Eliminator
  respawn delay and sample, `tick_wreck_voice`'s "is a player" test, the
  finish-line driver hand-over (`update_standings`), the Autopilot pickup's
  `locate_player_driver(slot)`, and the Eliminator's kill-target finish.
  Moved to per person-flown slot afterwards (`per-slot-sim-leftovers`,
  2026-10-13; every one is **chosen, not measured** where it says so, the
  original having one player). The LeachBeam's lock is `Race::sight_target(slot)`,
  a simulation quantity computed from the world for whichever slot fires, with
  the reticle reading the primary's; a second person has no reticle, so no
  Pulse hold on the lock (chosen). The fire arms of `spend_pickup` still wrote
  a literal `0`: the Rocket, Plasma (and its cue), Shuriken, Quake and Repulser
  now credit the slot that fired, and the Missile's reticle hold follows
  `primary_slot()` rather than slot 0. Weapon pads: the `human` odds column and
  the slot's own place now go to every person-flown slot (they went to slot 0),
  and the pad refresh countdown runs on the primary slot, the first one
  stepped. AI: the rubber band, the position-keyed tables and the Pure zone
  multiplier read the best-placed person (`Race::rubber_band_reference`, lowest
  slot on a tie, chosen); the original field's round robin is every active craft
  nobody flies; pad seeking skips every person-flown slot; the weapon AI's
  `player_gap` is the nearest person ahead, else the first person's gap
  (chosen); the speed plan's reference hull tries the people's craft first, and
  `build_player_speed_plan` builds the primary's (one plan per hull already
  serves every person's autopilot, `adapt_hull_plans` walks every slot).
  Results: `Race::finished` is every person-flown slot's own `race[slot].finished`
  (line crossing and wreck are per slot, the Eliminator's kill target ends all
  persons together), `runs_on_after_the_line` needs every person across the
  line, and the board marks every person's row as the player's
  (`scoreboard::Craft::person`; `Board::player()` is the best-placed one). Slot
  0 handed to the AI with a person elsewhere gets a driver seeded from
  `Driver::for_slot(race_seed, MAX_SHIPS)` and located on the line on the first
  tick (`Race::seed_vacated_driver`; lazily, because `driver.index` is hashed).
  **Unanswered, with addresses:** how the original ends a two-player race.
  `Race_FinishAllCrafts` (`0x08824e10`), reached only through `FUN_0882461c`
  from `FUN_088dfb90` in the `GriefReport_*` screen code, is the one multiplayer
  finish path found (`race-finish.md`), and it never ran live in Single Race or
  Time Trial, so `Race::finished` ("every person is done") is chosen, not
  measured. The AI's single player is `AiManager_SnapshotRacers` (`0x088352c0`).
  **Player-visible now:** `--players N` (`oag_game::split_screen`) marks every
  viewed slot `Controller::Local`, so a split-screen race reaches the end-race
  screens only once the last person has finished, the first finisher's craft
  flying on under the driver meanwhile.
  **Still open:** `oag_game::ai_choice` still declines the original driver for
  more than one human (`Reason::MultipleHumans`) - the sim sites above are ready
  for it, but `use_original_ai`'s grid numbers assume the one person is slot 0
  (opponent grid position `slot + 8 - count`), which a person in another slot
  would shift; nobody has measured the original's field with two players, so
  lifting the decline is a decision, not a fix. `Board.tick` is still "the
  player's own last crossing" (now the tick the last person finished). ~~The
  end-race screens read `Board::player()` for one row~~ - they name every person
  now (2026-10-13, `split-screen-end-race`).
  View, stays primary for split screen's lane: `zoom.rs` boost pulse,
  `destroy_camera.rs`, `craft_flash.rs` (shake cancel, reset flash),
  `countdown.rs` cue slot, `hit_sparks.rs` shake, `intro_camera.rs` skip button,
  `access.rs::player_slot` and everything reading through it (telemetry,
  camera, finish camera, split gap, scenery fx, lap messages), and the
  `ships[0]` reads at the end of `tick.rs` (camera target, boost FOV kick,
  flaps, `shielded`, contact cue slot) and in `weapons/reticle.rs` and
  `effects.rs`. `RaceSim::last_on_track` became a row per slot; its hash writes
  the primary's value where the scalar was and then each other person-flown
  slot's, so a one-person race hashes byte-for-byte as before.
- **Cloning `RaceSim` copies the track along with the race.** The collision
  soup (`Vec<TriangleSoup>`, hundreds of thousands of vertices on a real
  circuit), the spline, the racing line, `ai_order`, both pad-volume lists
  and the weapon table are all immutable for a race's whole life -
  `sim.collision` is built at `Race::start` and no tick writes it - and a
  per-tick snapshot pays for all of them. Whole-struct `Clone` was chosen
  anyway and the reason is on the struct: `state_hash` is not a
  snapshot-completeness oracle, so a hand-written snapshot type that dropped
  one of the fields *outside* the hash would pass a hash-equality test and
  diverge a race thousands of ticks later. Splitting the read-only half out
  is the obvious optimization; **it should be taken against a measured
  reconcile budget, which does not exist yet**, not on principle.
- `TickClock::advance` caps catch-up at `max_ticks_per_step: 8` and
  **discards** the surplus (`crates/core/src/tick.rs`). A reconciliation
  burst is longer than that whenever the round trip exceeds 133 ms. The
  loopback test loops directly and never touches the clock, so this is not a
  bug today - it is a trap for the first client that drives a replay through
  `TickClock` instead. **2026-10-14: checked against the quiet path, it does
  not bite and `oag-core` stays as it is.** A replay is a count of ticks owed
  (the ticks between the snapshot and now), not elapsed time, so it is a plain
  loop of `tick_sim_only` calls and never an `advance`. Going through the clock
  would be wrong twice: it would cap at 8 and discard the rest, and its
  `tick` counter counts ticks it granted, so replayed ticks would be counted a
  second time. Even at the cap the replay is cheap (8 ticks is about 0.5 ms
  measured, `docs/networking/README.md`), so the cap is a correctness
  hazard and not a CPU one. The live tick after the replay is what goes through
  the clock.
- Transport choice for `oag-net` (framing, reliable-vs-unreliable channel
  split for inputs vs. snapshots, send rate relative to the fixed 60 Hz
  tick) - not decided, needed before the crate is created. Section 3b has the
  WebTransport option for the server and the web build (leaning, not decided).
- Reconciliation tuning: how many ticks of local input the client buffers
  for replay, how snapshots are diffed/compressed for the wire, and how a
  genuine desync (caught by the state hash) is surfaced to the player - none
  of this is decided, all of it sits inside the server-authoritative/
  client-prediction shape section 3 already commits to.
- **Trust model and server roles, decided 2026-10-13** in a design discussion
  and written into `docs/networking/README.md`. Online is server-authoritative.
  A client-hosted game exists on LAN only and is trusted; cheating there is a
  social problem. It is one server role with two deployments, a headless
  dedicated binary and a listen server inside a client; the second `[[bin]]`
  in `oag-game` above serves both. No client anti-cheat: an oracle bot or
  assist, including a screen-reading one with a faked input device, is out of
  band and accepted. Mods may change anything, so the server advertises a
  rules and content fingerprint that the client shows on join, and mod code
  from a lobby is never auto-run. A malicious server is out of scope. Still
  owed: the network-protocol ADR.
- Whether the original's ad-hoc/game-sharing scope
  (`docs/networking/README.md`) is worth recreating separately from this
  online design, purely as its own RE-and-reimplement exercise - not a
  blocker for anything here, since the two are explicitly independent per
  section 3.
- ~~**`docs/networking/README.md`'s "Pulse likely has no online play"
  conclusion rests on absent PRX files on disc alone, and that may not be
  the whole story.**~~ **Answered 2026-09-21, and the suspicion was right:
  the conclusion is wrong.** No new RE was needed - the answer was already
  in the repository. `docs/ghidra/functions/psp-pulse-usa/imports.md`
  (confidence 99, read from `BOOT.BIN`'s own import table rather than from
  the disc's PRX list) has `sceNetInet` 18 of 18 resolved, `sceNetAdhoc` 7
  of 7, `sceNetApctl` 6 of 6, `sceNetAdhocctl` and `sceNetResolver` 5 each,
  `sceNet` 4, `sceHttp` 5 of 7, `sceNetAdhocMatching` and `sceSsl` 2 each,
  `sceWlanDrv` 1, plus the `sceNp`/`sceNpAuth`/`sceNpService` trio
  (unresolved names only because NP uses a non-SHA-1 NID scheme). Pulse
  links the full network stack and PSN. The PRX-absence inference broke
  exactly where this entry guessed it would: a module loaded from firmware
  leaves no file on the disc.
  **One caveat, and it does not change the reading**: `imports.md` is
  UCUS-98712 (USA) while the PRX rows in `docs/psp/pulse-disc-layout.md` are
  UCES00465 (EU). `just resolve-imports` against the already-imported
  `psp-pulse-eu` would close it cheaply.
  **This does not touch section 3's design**, which is ours by decision
  rather than for want of something to observe - reconfirmed by the user
  2026-09-21. It does mean `docs/networking/README.md`'s confidence-90 "there
  is no `pspnet` stack and no `libhttp`" is stale and contradicted by a
  confidence-99 page in the same tree, and it means the ad-hoc/game-sharing
  scope two entries up has a real subsystem behind it rather than a
  suspected one. **Correcting that page is its own change and is not on this
  thread's critical path.** **Done, 2026-10-13**: `docs/networking/README.md`
  is rewritten, the stale claim replaced by the import-table evidence.
- **8-way split screen is 8 full scene renders per frame**, not a cost this
  thread has estimated. Worth a frame-budget pass once the viewport work in
  section 1 exists, before committing to 8-up as a shipped configuration
  rather than a tested ceiling.
- **Zone, Speed Lap and (per the shared-prerequisite section) every mode
  that currently reads `World::race` as a single field is single-ship by
  the original's own design**, not just by this codebase's current
  limitation - `World::race`'s doc comment names Time Trial, Speed Lap and
  Zone specifically as one-ship-on-track modes. Whether any of those modes
  gets a genuine multi-player treatment (a multi-player Zone, say) or stays
  single-ship even after the shared prerequisite lands is a design question
  the prerequisite work doesn't answer by itself.
- Snapshot/delta sizing for the wire (8 ships plus projectiles plus
  `RaceSim`'s non-`World` extras) is unestimated - relevant to the
  transport decision above but not computed here.
- **2026-10-13, `split-entry-hd-2048-omega`: the split entry on HD, Omega and 2048, and
  the integrated-GPU rule.** The SPLIT SCREEN row was already on HD and Omega (revealed
  under RACEBOX); 2048 had no way in, so it gained a third extra tile. Walked windowed with
  two virtual pads on all three: join, the title's own screens, a two-view race (HD to
  `Ship Select - Player 2`). The join screen was unreadable on HD (names drawn over the
  device column) and invisible on Omega (white text on a light page); it now uses HD's list
  blocks there. `Title::split_on_integrated_gpu` (Pulse, Pure) drops the row and tile on an
  integrated GPU and sends a windowed `--players N` to one view. Written up in
  [split-screen.md](../../docs/rendering/split-screen.md#where-the-entry-is-per-title-2026-10-13-split-entry-hd-2048-omega).
  **Open from it**: ~~Omega's racebox has HD's two lists and they are not wired~~ (done, see
  `split-entry-mirror-hd` below); a real integrated adapter was not walked; ~~Omega has no Ship
  Select in a split~~ (Omega does author `Team Selection Player 1`/`2`: see below).
- **2026-10-13, `split-entry-mirror-hd`: Omega authors HD's rows, 2048 borrows them.** Omega's real
  `racebox_definition.xml` (`data09`) has `NumberOfPlayers` and `SplitScreen`, so `oag_omega` names it
  (`oag_title::LocalSplit::Authored`) and Omega's RACE page carries HD's two rows with the generic row
  gone; its `vert_splitscreen_*` HUD files (`data00`) are wired for the vertical cut. 2048 has no race box:
  `LocalSplit::Borrowed` in `oag_2048` holds HD's lists (chosen, not measured, labelled borrowed, the
  words from 2048's own table, a disc-backed test holds them equal to HD's and Omega's), the rows sit
  on the RACE page behind the RACEBOX tile, and the third `GameModeChoice` tile (`Launch::SplitScreen`,
  `trim_tiles`) is removed. Walked windowed on an own Xvfb with two pads on both: rows, 2 players,
  vertical, join, a left and right two-view race (Omega also through Track Select). Written up in
  [split-screen.md](../../docs/rendering/split-screen.md#where-the-entry-is-per-title-2026-10-13-split-entry-hd-2048-omega).
  **Open from it**:
  - ~~**Omega's Ship Select is wired and bare**~~ **Done, 2026-10-14 (`omega-ship-select`)**: Omega's HD-era
    teams carry `TEAM_VARIANTS` (three open models each, off the teams definition), the bars carry each
    model's ratings and the `SHIP MODEL` frame draws the PS4 race hull at HD's pose (**chosen, not measured**,
    `FrontEnd::ship_preview_pose`). Walked for one person and each seat. ~~The five 2048-era teams are not
    offered on the screen~~ **Done 2026-10-14 (`omega-2048-era-teams`)**: all five are offered (19 teams), the
    roster's `VariantJoin` reaches `Team::hex_cells`/`variant_stats`, each shows its four `Ships\<Team>\<n>` craft
    as open cells (four `IsVR` models padlocked), the ratings off each model's `<FE>`, the logo off
    `Ships\<Team>\FE\Logo`, and a pick starts a race on that craft (walked to a race start). Still open there: the
    team-level `<Unlock purchase>`/`grid` rules are unread on every team (all offered), AI opponents are
    still drawn from the HD-era teams only (chosen), `ShipDesc` and the hexagonal window's stills are not drawn;
    ~~the bars' two halves are one
    grey on Omega's page~~ **done 2026-10-14 (`omega-colour-template`)**: Omega's boot copies `WOHD_Colours` over the
    base colour block (`BootProfile::colour_template`), so `HD_Grey` is `0xFF010001` and the bar fill shows;
    the authored `ShipDesc`, livery hint and `PadlockText` are not drawn.
  - **2048 seats pick no ship**: no per-player pair is authored (`Team_Definition.xml` is one touch screen).
  - **HD guest join not used**: `FE_GUEST_JOIN` ("PLAYER 2 PRESS START TO JOIN" on HD, "PRESS OPTIONS TO
    JOIN" on Omega) names a different button than our lobby's join press (confirm), so showing it on the
    join screen would misdirect; left as our hint line.
  - **`FE_OFF` on the greyed cut row, and making it unfocusable**: a greyed row is selectable and inert
    on every page of the menu tree by design (`oag_ui::menu::pointer`/`nav`), so drawing the authored text
    and skipping focus is a menu-wide change, left open.
  - 2048's windowed placeholder skin clips the left glyphs of values wider than about nine characters
    (`HORIZONTAL`, the TEAM value `Feisar2048\3`); the same clip is on `main` without this lane's
    change, headless captures are not clipped.
- **2026-10-13, `hd-authored-split-setup`: HD's own split setup is wired.** HD authors the local
  entry in RACEBOX itself (`NumberOfPlayers` 1 or 2, `SplitScreen` horizontal or vertical,
  [hd-frontend.md](../../docs/formats/hd-frontend.md#racebox-authors-local-split-screen-a-players-list-and-a-cut)):
  the RACE page carries both rows as authored (after AI DIFFICULTY, the cut greyed at one person),
  and they replace our generic SPLIT SCREEN row on HD. START with two people opens the join screen
  for exactly two seats (START refused until both are taken, chosen), then HD's Track Select and
  per-seat Ship Select, then the race. `RB_VERT` cuts left and right (`oag_render::viewport::Cut`,
  `Race::cut`/`view_rects` the one rect source for scene and HUD) and composes HD's `vert_`
  `splitscreen_hud` layouts; `--split-cut vertical` does it from the command line. The padlock
  beside `NumberOfPlayers` is a trial-build lock (`g_FuryContentFlags`, confidence 72,
  [racebox-padlocks.md](../../docs/ghidra/functions/ps3-hdfury-eu/racebox-padlocks.md)), not
  progression, so two players are unlocked. The lists' `intvalue`s are preview-image indices, not
  unlock ids (the earlier reading was wrong). Walked windowed with two virtual pads: race box ->
  2 players -> vertical -> join -> Track Select -> Ship Select P1/P2 -> a two-column race. **Open
  from it**: the authored `FE_OFF` text over a greyed cut row is not drawn; HD has no menu route to
  three or more people (the authored maximum is two; `--players N` still cuts up to eight); the
  vertical cut on titles that author no choice (`--split-cut`) was not judged by eye; Omega ships
  the `vert_` files but its race box authors no cut to choose them by; the trial build's other
  padlocks (which modes, classes and weapon states it locks) are unenumerated.

## Next Steps

- ~~Land the shared prerequisite (per-player `RaceState`, N-input
  `World::tick`, per-player device mapping in `oag-input`) as its own
  change, before starting split screen.~~ **Done, 2026-09-16** - see
  "Recommended order" step 1 for what landed, and note that the function is
  `Race::tick`, not `World::tick`, which this line still had wrong. Split
  screen is unblocked; its first move is the viewport work in section 1.
- Read `handover/frontend/pulses-race-box-is-read-both-pressings.md` for the
  existing split-screen menu-XML findings before building split screen's
  front-end leg.
- Resolve the remote-player prediction question (interpolation vs.
  rollback-with-predicted-inputs, see section 3 and the "Open" entry above)
  before writing the network-protocol ADR - unlike the now-written
  [ADR-0052](../../docs/architecture/adr/0052-world-and-race-tick-widen-to-n-players.md),
  this one still has a real open design question in it, not just a write-up
  owed. Not before M8 opens per the roadmap, but the decision itself doesn't
  need to wait for that milestone to start.
- Decide the transport (framing, channel split, send rate) as the next open
  question once that ADR is written. **The loopback proof that gated this is
  done** (2026-09-21, see "Recommended order" step 4), so the only thing
  still ahead of the transport decision is the remote-player question above.
- ~~Give a reconciliation replay a tick path that does not re-fire the camera,
  the particles and the sound cues.~~ **Done 2026-10-14**, `Race::tick_sim_only`;
  see the Open entry. What it leaves: a recording that does not hear a replay.
- Before writing that ADR, resolve the remote-player prediction question
  flagged in section 3 (interpolation vs. rollback-with-predicted-inputs) -
  it changes the protocol shape, not just a tuning parameter.
  **2026-10-13: status is MEASURE, not decided.** Section 3a has the options,
  their outcomes, the Rocket League slides, the racing-game survey, the four
  measurements and Claude's recommendation, which is open to challenge. The ADR
  waits on the numbers.
- Run section 3a's measurements: ~~M2 (replay cost per tick) and M3 (snapshot
  cost) on the desktop~~ **done 2026-10-14**. **Open: M2 and M3 on the Galaxy
  S24** (build the sweep `reconcile_cost_sweep` into the APK's test path or run
  `oag-headless-sim` there and time `tick` against a quiet tick), then M1 and M4,
  which the remote-slot input path unblocked.
- ~~Check `BOOT.BIN`'s import table for `sceUtilityLoadNetModule`/
  `sceNetInet*`/`sceNetAdhoc*` NIDs to confirm or correct
  `docs/networking/README.md`'s "no online play" reading before citing it
  further.~~ **Done, 2026-09-21** - see the Open entry above.
  `docs/ghidra/functions/psp-pulse-usa/imports.md` already had the answer and
  it corrects the reading: the full stack is imported. The rewrite of
  `docs/networking/README.md` landed 2026-10-13.

## From the HANDOVER.md index (moved 2026-09-25)

planning thread, not an RE finding, revised once already after external review caught a wrong function name. ~~The engine is hard single-player today: `World::race` is one `RaceState` (not per-ship, despite `ships: [Ship; MAX_SHIPS=8]` already existing), `InputSnapshot` is one struct with no player index, `oag-input`'s `Pad` merges every gamepad into one stream by design~~ - **the shared prerequisite landed 2026-09-16**, all of it: `World::race` is `[RaceState; MAX_PLAYERS=MAX_SHIPS=8]` with a `controllers: [Controller; 8]` (`Ai`/`Local`/`Remote`) beside it and both hashed; `Race::tick` takes `oag_gameplay::PlayerInputs`, a fixed `[InputSnapshot; 8]`; `oag_input::pad::Assignment` maps each `GamepadId` and the keyboard to a slot, with `Controls` keeping one `Input` per slot because an edge is per pilot; and `oag-headless-sim`, a second `[[bin]]` over `Setup::headless`, ticks with no renderer at all (`just headless-sim`). Behaviour is unchanged - `World::SINGLE_PLAYER` and the default assignment put one person in slot 0 and every device with them, so a single-player race reads exactly `ships[0]` as before; `crates/gameplay/tests/determinism.rs`'s constants moved once, isolated the documented way. **Still open from it**: ~~the ADR this thread asked for was never written~~ - **done, 2026-09-16**, [ADR-0052](../../docs/architecture/adr/0052-world-and-race-tick-widen-to-n-players.md) (the reasoning already lived in doc comments on `World::race`, `Controller` and `PlayerInputs`; the ADR is the record catching up). The *network-protocol* ADR (server authority/prediction/reconciliation) is separately still owed and has a real open design question in it, not just a write-up. The player step still runs **once** for `World::primary_slot` returning one `Evaluated` - N cameras and N HUDs are split screen's own work. `App` is still documented "one of these per run, not one per window". All three features target **up to 8 players** (matching `MAX_SHIPS`); the prerequisite also unblocked the already-placeholdered `oag-replay` (M7). Recommended order after that: **split screen** first (one window, up to 8 viewports - needs new viewport/camera support in `oag-render`, which has "one camera convention" today, but zero `App`/window-lifecycle risk; PS2's menu XML already authors the split-screen screens this build currently strips, see `../frontend/pulses-race-box-is-read-both-pressings.md`), then **multi-window** (up to 8 OS windows, reuses split screen's per-player input/HUD work, but has to unwind `App`'s one-window assumption - different risk, not a superset), then **network multiplayer** last and by far the largest effort, decided as **ours, not a port**: server-authoritative (server is sole source of truth, closing off state-level cheating), client-side prediction (each client runs the same deterministic tick locally, ahead of server acknowledgment), reconciled by restoring an authoritative snapshot and replaying buffered local inputs - leaning on `ADR-0003`'s snapshot design and `ADR-0007`'s fixed-60Hz determinism, with `oag-core`'s existing state hash reusable for desync detection. ~~The server side needs its own headless path (a second `[[bin]]` in the `oag-game` crate reusing `Race`/`RaceSim` directly is the cheap option; nothing may depend on `oag-game` per rule 2, so a separate server crate can't just import it).~~ **Done, 2026-09-16** - `oag-headless-sim` is that binary, with no networking in it and none wanted until the loopback client/server test justifies a transport. **No original implementation to build this against**: `../../docs/networking/README.md` puts confidence at 90 that Pulse's PSP build ships no `pspnet`/`libhttp` stack at all - though that reading rests on absent PRX files alone and hasn't been checked against `BOOT.BIN`'s own import table for `sceUtilityLoadNetModule` (a firmware-loaded API that wouldn't need an on-disc PRX), flagged as unverified in the thread. Open: ~~per-player `RaceState` shape~~ (**decided 2026-09-16**: the fixed array *and* the human/AI marker, not either), ~~the unwritten ADR~~ (**done**, ADR-0052, but the separate network-protocol ADR is still owed), transport choice for `oag-net` (not created until decided), whether remote (non-local) players should be interpolated or rollback-predicted given how tightly pack racing couples ship motion tick to tick (flagged as a real design disagreement, not settled - this is what the network-protocol ADR is waiting on), and 8-way split screen's render cost is unestimated

## 2026-10-13, `split-entry-align`: Pulse and Pure entry follows the PS2

Pulse (both pressings) and Pure show SPLIT SCREEN VERTICAL and HORIZONTAL after the
campaign row; the RACEBOX SPLIT SCREEN row is gone there (see `docs/rendering/split-screen.md`,
"The Pulse and Pure entry"). The row's choice feeds `Session::split_cut`, so
VERTICAL draws left and right (merged with `hd-authored-split-setup`). **Open**: (1) the PS2's
Tournament race type and the `TournamentLoad` dialog are not built, and Head to Head is CLI-only
(`--mode head_to_head`, two people and no AI since `split-head2head`; the RACE page's mode row has
no entry for it, so the two-person cap is not yet enforced at the join screen); (2) the
RACE page's default MODE is whatever the profile holds, the PS2's split page opens on Arcade;
(3) on the PS2 pressing in another language the rows read the project's translation, not the
disc's own `FE_RACE_CAM_SS` text.

## 2026-10-13, `split-grid-positions`: where the people start

Two people start on slots 8 and 7 (person 1 the back), the AI on 1 to 6, as PS2 Pulse and HD/Fury
both spawn them; done in `oag_gameplay::rank_from_back` / `Setup::people`, evidence in
`docs/ghidra/functions/ps3-hdfury-eu/grid-local-players.md` and
`docs/ghidra/functions/ps2-pulse-eu/grid-local-players.md`, summary in `docs/rendering/split-screen.md`.

**Open:**
- Confidence is 85 (decompile of two binaries that agree). A live check wants a second virtual
  pad on RPCS3 (`scripts/rpcs3_pad.py` drives one) or PCSX2: join two players, read both crafts'
  poses at the countdown and compare with the rows (HD race manager at `0x0095ae78`, ships at
  `+0xe8`, slot array at `+0x2d78`).
- Three to eight people are chosen, not measured (the next row forward each).
- **Fixed 2026-10-14 (`split-head2head`, maintainer's report): the grid's head count was the first
  window's view count.** `load` read `Options::views` (one window's HUD need) for `Setup::people`,
  so `--window-seats 1,1` counted one person and started the second at the front as an opponent
  (and `2,2` two). `Options::people` now carries the seats across every window (`prepare.rs` from
  `--window-seats`/`--players`, `Session::launch` from the join seats; `0` means one per view, so
  one window is unchanged) and the log reads `people on the grid: 1:8 2:7 ...`. Pinned by
  `crates/game/tests/window_seats_grid_ground_truth.rs` (`1,1`, `2,2`, `3,1`, `--players 4` and `2`).
- What sets the ship count `g_GameState[0]` for a split race is unread: the rule assumes it stays eight.
- A launch whose `--players` is clamped after the load (an integrated GPU) keeps the people's rows.

## 2026-10-14, `split-head2head`: Head to Head with people is two craft

Two seated people fly exactly two craft and no AI (`Mode::field_is_fixed`; the second person
takes the one opponent's slot), and three or more are refused by name from the CLI
(`Mode::max_people`, `--players`/`--window-seats`). **Chosen, not measured**: nothing in the RE
docs reads the original's two-player Head to Head (mode `0xf` shares the HUD swap, no grid-size
row). Each person's HUD reads the gap to the other: Pulse's `SplitHud_Pulse.toml` carries the
bar in the place group in every rule, so both cuts and each window of `--window-seats 1,1`
draw `1st`/`2nd`/`+- NNm` from that person's slot; the end screens already name both seats
and give a winner. A split's place is ranked over the craft in the race.

**Open:**
- The RACE page's MODE row has no Head to Head entry (`Mode::ALL` omits it), so a player cannot
  reach it with people but from the CLI; the cap belongs at the join screen or START once a row
  exists (`h2h-port-titles`).
- Pure, HD, Omega, 2048 draw no gap readout: none authors one. HD has no Head2Head cell, Pure's
  and 2048's are unread (status.md `?`); they draw the title's place, `1 / 2` and `2 / 2`.
- The top and bottom cut's `strip-head-to-head` rule (place group at 0.8) is small: the gap
  label reads about 11 px at 1280x720. A larger one needs the shield and speed group moved off
  the right edge in that rule; not tried. The columns (`tall`) are fine at 1.6.
- The opponent's team and grid slot for one person are unchanged (extrapolated, head2head.md).
