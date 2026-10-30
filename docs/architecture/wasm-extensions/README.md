# WebAssembly extensions: who may extend what, through which surface, and what stays closed

**Status: a design, with its first slice built** (2026-10-29): Wall Penalty, below. The decision record is
[ADR-0079](../adr/0079-extensions-are-sandboxed-webassembly-guests-behind-two-worlds.md),
status *Proposed*; the `frontend` world (mod screens) is
[ADR-0080](../adr/0080-a-wasm-mod-may-own-a-screen-and-save-a-small-blob.md), also
*Proposed*. This page is the shape of the API: the extension kinds, how
each is declared, the interface sketch (the `sim`, `hud` and `frontend` worlds,
their events and commands), the explicit non-capabilities, limits, trust, targets, licences,
and the maintainer's examples walked end to end. The engine half is beside it:
[hooks.md](hooks.md) (where each event is raised and each command lands, at its
`file:line`, and where the one guest call per tick sits) and
[determinism-and-state.md](determinism-and-state.md) (floats, fuel, the state
blob, the hash, replays, network play, web and iOS, the spike's measurements).
Where this page and those two disagree, they own the engine facts and this page
is to be corrected.

## What is built: Wall Penalty (2026-10-29)

The smallest slice a player feels, ahead of the rollout's phases: the example mod
`oag.wall-penalty` (`assets/mods/examples/wall-penalty/`, Rust, 627 bytes of wasm) makes a
craft that touches a wall keep a quarter of its thrust for 1.5 s, renewed each tick it
stays on the wall. All behind the `extensions` cargo feature (`oag-game` turns on
`oag-raceplay/extensions`, which links `oag-modhost`); off, nothing of wasmtime is
compiled and a race offered a module names it as left out in its load report.

| Piece | Where | What it is |
| --- | --- | --- |
| The host | `crates/modhost` | Section 4.1's configuration minus Pulley and the load-time rewrite (desktop Cranelift only); one process-wide `Engine`, built at the first compile and counted (`engines_created`); a fresh pooled instance per tick; fuel per tick `FUEL_PER_TICK` = 200,000, **chosen**; a trap or a refused reply is a `Fault` |
| The ABI | `crates/ext-abi` (`oag_ext_abi`) | Core wasm: exports `memory`, `oag_abi` (the major and the layout revision, below), `oag_io` (a 4,096-byte buffer), `tick`, optionally `oag_state_len` and `oag_track` (the [track region](#and-track-properties-2026-10-30)); **no imports at all** (the `oag` namespace is empty in ABI 1, so `rng-next` and `log` are not offered yet). The buffer's fixed little-endian layout is on that crate's page, the one definition the host and the guest SDK both compile against (it moved out of `oag-modhost` with the SDK, below); it carries the README's records only as far as the examples need: the tick, the active-slot mask, the events, the blob, the reply, the craft records |
| Event | `wall-contact{slot}` | Raised in `Race::count_wall_contact` for a person's craft and an AI craft alike, **on every tick the craft touches a wall** (`contacts > floor_contacts`, the test `wall_contact_ticks` counts), at most once per craft per tick; delivered the next tick (the one-tick latency) |
| Command | `scale-thrust{slot, factor}` | No `ticks` field and no host timer: the guest re-sends it each tick it wants it. `factor` finite and in `[0, 1]` for Wall Penalty (`[0, 2]` since Energy Turbo, below); anything else, a slot with no craft, or two for one slot is a fault, never a clamp. Below 1.0 **multiplied** into `Ship::pending_thrust_scale`, so a Leach Beam drain armed the tick before is kept |
| Kind | `[extension.rules]` in `oag-mod.toml` | A rules module, because section 2.1 lets a rules module touch any slot and a craft module only slots flying its hull: a curse on every craft would otherwise be one craft row per team. Section 1.1 names a rules module from a mode file; `modes/*.toml` does not exist yet, so until it does a mod's rules module runs in every race the mod is on for, on its titles, **except the original campaigns** (section 1.2) **and network races** (the host's race runs vanilla until phase 3; the lobby's fingerprint leaves the repository's examples out) |
| State | `RaceSim::extensions` | Each module's blob (the manifest's `state_bytes`, at most 1,024), its fault tick, and the pending wall-contact bits, cloned with a snapshot and hashed last in `RaceSim::state_hash`; a race with no module hashes nothing there |
| No replay | `Race::start_recording` | A modded race records none (section 4.4, phase 2's rule) |
| Overlay | `oag_present::perf::mods` | The `Dev` overlay's `mods` block: per module fuel last tick, peak, share of the budget, call wall time, and the fault tick; a per-tick output timed in `oag-raceplay`, never hashed |

Proof: `oag_raceplay`'s `tests::extensions` (a vanilla race's 600 tick hashes fold
to one constant in both builds and build no engine; a modded race hashes the same
twice; a runaway module faults on the same tick twice while the race and the other
module go on), `oag_modhost`'s unit tests, and the disc-backed
`wall_penalty_ground_truth` (Pulse PSP EU, `16_Track`, Venom, throttle held: a second after
the first wall contact the cursed craft is at 50.7 km/h against 115.6 km/h, two
seconds after 15.6 against 81.7; in a Single Race an AI craft's first contact
leaves it at 255.3 km/h a second later against 410.7 km/h). `just check-extensions` lints the feature and runs
the host's and `oag-raceplay`'s unit tests with it on.

### And Energy Turbo (2026-10-29)

The second example, `oag.energy-turbo` (`assets/mods/examples/energy-turbo/`, 3,040
bytes): a pickup of its own, which the weapon pads hand out in place of **every second
Turbo** (the other half stay the title's Turbo), drawn as the title's Turbo icon on a translucent red backdrop (`[pickup]` in its manifest:
`icon = { title = "turbo" }`, `colour = { rgba = "E0202080" }`; the title's own fade-in, overshoot,
size pulse, hold and fade-out play on it). Hold fire to boost (thrust x1.9) for as long as the pool lasts, paid at
0.8 % of the pool a tick (a full pool lasts about 2 s) and stopping at 5 % so the drain
alone never wrecks; the title's wall sparks burst at the nozzle three times a second. An AI
craft holds no button: its driver asks to fire wherever the Turbo's own AI gate would
fire a Turbo (a straight, nobody close ahead, `Driver::affords_boost`), and the mod
turns that into a 1 s burst followed by a 2 s rest. Absorb it and half of what it cost comes back. While it is
carried a speed pad gives 15 % of the pool instead of a boost (a separate income, not
part of the absorb's half). All numbers are the mod's constants, **chosen**. What it
added to the engine and the ABI, each its own commit with the vanilla fold unchanged:

| Piece | Kind | What it is |
| --- | --- | --- |
| Craft records | observation | One 16-byte record per slot (`oag_ext_abi::Craft`): the held pickup (its place in `Weapon::ALL` plus one, 255 for the mod's own), flags (person-flown, fire button held, racing past GO), shield, its maximum, speed. Copies; the held input is the `InputSnapshot` the tick already has |
| `scale-thrust` up to 2.0 | command | Above 1.0 through a new `Environment::extension_thrust` (`oag-physics`, applied to the engine's thrust after the Leach Beam's scale; `1.0` skips the multiply, so no vanilla bit moves). Below 1.0 still the Leach Beam's path |
| `shield-delta{slot, amount}` | command | A loss through `oag_physics::damage::apply_weapon` at its full amount, so the destroyed edge, the Shield pickup's immunity and the critical crossing are a hit's (a drained pool wrecks: proven); a gain through `damage::add`'s clamp. `|amount|` at most the slot's maximum; refused where the mode takes no damage |
| The mod's own pickup | engine | `oag_tables::weapons::Weapon::Extension` (7.5): a fieldless arm outside `Weapon::ALL`, so `Option<Weapon>` stays one byte and the World snapshot does not grow; hashed as 255, which no vanilla race writes. The engine never fires or absorbs it: a person's press (`Race::spend_pickup`) and the AI's fire (`Race::spend_opponent_pickup`, through the Turbo's gate; the AI never absorbs it) raise **`pickup-fire{slot}`** / **`pickup-absorbed{slot}`**; an absorb clears it and pays no pool. The HUD draws the look the mod declares for it: its manifest's `[pickup]`, or this tick's `pickup-look` (below). A mod that hands one out with no `[pickup]` is faulted for the race on its first `give-pickup` or `pickup-roll`, naming the missing table |
| `give-pickup{slot}` / `clear-pickup{slot}` | command | Puts the mod's pickup in an empty slot / takes it out of a slot holding it (a fault otherwise). A race without weapons gives nothing. The slot's last-granted weapon is left alone, so the title's repeat rule is unchanged |
| `pickup-roll{weapon, every}` | command | Race-wide (sent on slot 0): every `every`-th time a weapon pad (`Race::test_weapon_pads`) draws that weapon, it hands out the mod's pickup instead, with the drawn weapon still the slot's last grant. The count is hashed; re-sending the same rule keeps it. The first module to send any of the three pickup commands owns the kind for the race; another one faults |
| `scale-pad{slot, 0}` and **`speed-pad{slot, pad}`** | command, event | The turbo holds its slot's speed pads at 0 while carried, so a pad gives no push, plume or pad sound, and pays the pool on the `speed-pad` the entry raises ([track properties](#and-track-properties-2026-10-30)) |
| `play-cue{slot, cue}` | output | One of six of the engine's cues (`oag_ext_abi::Cue`, matched to the engine's in `oag_raceplay::extensions`), pushed beside the tick's own |
| `fire-effect{slot, effect}` | output | The title's own `CollisionSpark` or `HitSpark` `.pob`, at the hull's back end, drawn by the view half; a title without it draws nothing |
| `fire-effect{slot, own index}` | output | The sending mod's own particle effect (`[effects]` of its manifest, `docs/formats/mod-effect.md`), arg bit 15 set, at the hull's back end; one that did not load draws nothing |
| `hud-tint{slot, element, rgba}` | output | Recolours a HUD element of that slot. `Pickup` is **not** tintable any more: a `hud-tint` of it is a malformed record (`pickup-look` colours the pickup). Elements are title-agnostic (`Speed`, `Thrust`, `Energy`, `Pickup`, `Position`, `Lap`, `Time`) and `oag_hud::ext_draws::element_of` maps each to the title's widgets; an element the title has no widget for draws nothing |
| `pickup-look{slot, icon, colour}` | output | Command 22. The look of that slot's held mod pickup: `icon` is `{title: weapon}` (the title's own icon sprite for the weapon; a title with none draws none) or `{mod: name}` (a PNG of the mod's `[icons]`), `colour` is `{title: weapon}` (the title's own backdrop row for the weapon: Pulse's recovered table, HD's and Omega's own) or `{rgba}` (a colour of the mod's own). Re-sent each tick it should hold; with none sent the manifest's `[pickup]` is the look, and the last look is kept while the pickup fades out after use. The title's fade-in, overshoot, size pulse, hold and fade-out (Pulse, HD, Omega) play on any look. **Per title:** Pulse, HD and Omega draw both colour kinds; **Pure has no backdrop widget, so both colour kinds draw nothing there** (its icon is the title's model for the weapon, or nothing for a mod icon); 2048 has no backdrop colour table, so its backdrop stays as authored and a `rgba` or title colour draws nothing there (its icon goes through the UV table, a mod icon is fitted inside `PickupBgFrame`). A mod icon is fitted, aspect kept, in the slot the title's weapon icons share (**chosen, not measured**). An output: drawn, never hashed |
| `hud-shake{slot, element, amplitude}` | output | Jitters the element's widgets by up to `amplitude` HUD pixels (at most 16) this tick. The offset is an integer hash of tick, slot and element, never zero while `amplitude` is above 0 (chosen, not measured) |
| `hud-bar-extra{slot, element, extra, rgba}` | output | A bar shows `extra` times its value on top: it is rescaled so value x (1 + extra) fills it, the base share keeps the bar's colour and the added share (`extra / (1 + extra)` of the fill, 47 % at 0.9) blends from the bar's colour into `rgba`. Each element means the same on every title: `Thrust` is the thrust bar and draws nothing on a title without one (Pulse); `Speed` is the speed bar |
| `hud-text{slot, anchor, x, y, text, rgba, size}` | output | Text in the title's HUD font at one of nine anchors of the HUD layout grid, offset `x`, `y` HUD pixels (12-bit signed), `size` in quarters of the title's own text height. The string is at most 48 bytes, carried in the reply's string pool |
| `hud-icon{slot, anchor, x, y, sprite, rgba, size}` | output | The title's HUD sprite of that name (`TimeDiffIcon`, `TurboIcon`) at an anchor, `rgba` its modulating colour; a name the layout lacks draws nothing |

The HUD outputs are re-sent each tick they should hold, are per slot (in split screen
each lands in the view of the slot it names, since each view draws its own slot's
HUD), and sit inside the title's safe area because the safe area scales the whole HUD
frame after the items are placed. A slot may send at most 32 `hud-text` and `hud-icon`
items per tick; the 33rd faults the module naming the cap. Text may be a literal or a key
(`hud_string`) of the sending mod's own `strings/<language>.toml`, the files its menu rows use, shown in the
game's language and else English's, and nothing if neither has the key; an icon may be a
title sprite by name or one of the mod's own `[icons]` PNGs (`hud_mod_icon`), read and
decoded with the race load (all IO off the render thread), packed in the HUD's sheet,
drawn at its own pixel size times the placement's. A PNG that is missing or does not
decode draws nothing and the load report names it. The flag is bit 15 of the command's
`u16`.

**The string pool.** A command record is 12 bytes, so a text or sprite name does not fit
in one. The reply section carries a 768-byte pool after the commands (`POOL_AT`): entries
of a length byte then UTF-8, named by their offset in the command's `u16` argument. The
placement (anchor in 4 bits, size in 4, x and y 12 each) is packed in the record's
`f32` word and the colour in the `u32`. One record per item keeps the per-slot cap
independent of the 96-command reply cap (`MAX_COMMANDS`).

Outputs are never `RaceSim` state: a test runs a module sending all of them against one
sending none and the state hashes match every tick.

**Timing and edge cases.** The call is at phase C, after the people's pickups are
spent (phase 2) and before any step. A `pickup-roll` sent at tick N's call applies from
N+1's pads; a `give-pickup` lands at N's call, so a fire pressed on N+1 is already the
mod's. A speed pad entered on tick N itself boosts as ever, because the module's
`scale-pad{slot, 0}` starts at N+1; the module pays energy only for a `speed-pad` entered
on a tick it held the slot at 0 (a bit in its record). An absorb on tick N reaches the
module the same tick, so the pads are already the title's for N.

**Proof** (`energy_turbo_ground_truth`, Pulse PSP EU, `16_Track`, Single Race with an
empty grid, this project's driver flying, the pool at half):
- Fire held from tick 360: 619.8 km/h at tick 420 against 479.5 carrying it unfired,
  the pool 47.50 to 4.94 (spent 42.56, the floor reached by then).
- The absorb gives back 21.28, exactly half.
- Carried over the first pad: no boost (505.4 against 623.6 km/h 0.5 s on) and +14.25
  energy.
- A craft without the turbo flies the vanilla race tick for tick.
- Two runs hash the same every tick.
- A full field over 180 s: the pads handed out 4 Turbos and 3 energy turbos (every
  second), and AI craft boosted on theirs for 487 ticks.

**Rules modules per race**: both examples are rules modules and run together. A race
runs up to four (`MAX_SIM_MODS`, chosen), not section 1.2's one. The two compose:
Wall Penalty scales below 1.0 and the turbo above it.

Not built from this page yet: the HUD MODDED tag and the results screen naming a
faulted mod (4.2), the modded records table, `race-start`/`race-end`, the
observation's craft records and spline samples, every other event and command,
the other five kinds, the HUD and frontend worlds, Pulley, the browser and the
load-time rewrite, and `oag-server` linking the host (rule 6 forbids `wasmtime` to
it until phase 3).

### And Hillside's rain (2026-10-30)

The third example mod, and the first **track module**: `oag.hillside`
(`assets/mods/examples/hillside/guest/`, Rust on `oag-ext`, 1,161 bytes of wasm) turns
Hillside's final lap rainy. The first craft of any kind, a person's or the AI's, to enter
its final lap starts a 3 s ramp (180 ticks): the host title's own rain begins, the sun's
lens flare fades out and the sky greys. Every earlier lap sends nothing at all, so it is
the vanilla frame, pixel for pixel.

Four things landed with it; ABI 1's revision is **3** (the lap fields were revision 2, the
commands revision 3, each with its own fold):

- **The lap fields.** The craft record's two zero bytes (offsets +2, +3) are `lap` (1-based,
  saturating at 255) and `laps_target` (0 when the race has no target), filled from each
  ship's `Standing` and `World::laps_target`. SDK: `Craft::lap()`, `laps_target()`. A craft is on
  its final lap when `laps_target() > 0 && lap() == laps_target()`; after it finishes `lap` is
  `target + 1`.
- **`[extension.track]`**: `module = "..."` and `track = "tracks/x.track.toml"` (both in the
  mod; `simulation = true` as for a rules module). `oag_raceplay::extensions::prepare` loads it
  **only** when the race's `Options::foreign_track` is that file (canonical path equality, so
  another mod's track with the same id never matches, and a disc circuit never runs it); a mod
  that is not a folder has no path to match and its track module never loads. This is
  built differently from section 1.1's sketch (a `track.extension` key in the track
  manifest): the binding lives in the mod's manifest and names the track file. The load report
  says "track module loaded". The session is MODDED as for a rules module: open, an
  outputs-only module perhaps should not mark a race modded (it changes nothing a replay
  needs).
- **Three race-wide output commands**, sent on slot 0 like `pickup-roll`, re-sent every tick,
  held by `Extensions` for the view half, cleared each tick, **never hashed** (the unit test
  `output_commands_reach_the_cues_effects_and_hud_and_leave_the_hash_alone` runs them and
  compares hashes with a module that sends none). Each value is finite in `[0, 1]` or the
  module faults; two modules sending one: the later in id order wins (chosen):
  - `set-weather{kind, strength}` (`Reply::set_weather(WeatherKind::Rain, s)`): the host title's
    authored rain. On Pulse the weather `Setup` of Fort Gale (`Title::weather`'s `rain_circuit`,
    `14_Track`): its `<Weather>`, `WO_RAIN` and the `WO_RAIN_LENS` lens, loaded at race load when
    a track module is present, with **no cover anchors**, so the whole track is open and
    Hillside's covered bend rains too. A title with no rain circuit, or one whose
    `trackstartup.xml` authors no `<Weather>`, says so in the loader report and nothing draws.
    `strength` scales the drops' emission (the authored per-emission counts, rounded), the
    wet road and the lens drops' target count, and the mist overlay's alpha.
    **Forced enhanced:** while this weather runs the enhanced effects (splashes, the wet road,
    craft spray, the rising mist) are on whatever the WEATHER setting says
    (`Race::enhanced_weather`); the setting keeps governing a disc circuit's authored weather.
    Idle at strength 0: it neither runs nor draws, and keeps its pool and clock.
  - `set-sun{intensity}` (`set_sun`): scales the lens flare's ring alpha at the draw
    (`scene/lens_flare.rs`), 0 off; untouched at 1.0. **On a track file only the flare fades**:
    the sun's disc is part of the borrowed `03_Track` Skycube, not a separate draw, and the sun
    as a light (the shadow direction and the hull rig) is untouched.
  - `grade-sky{grey}` (`grade_sky`): one per-draw uniform (`Uniforms::sky_grey`, a padding
    float until now), written for the sky draw alone and branched on `> 0.0` in `shade.wesl`, so
    a vanilla pixel does not move: the authored colour is mixed toward an overcast grey (its
    luma times 0.45 plus `(0.16, 0.18, 0.21)`: darker, desaturated, slightly cool; chosen, not
    measured). The sky stays Hillside's borrowed Skycube; the track format is unchanged.

Measured 2026-10-30, Pulse PSP EU, headless `oag-game --race --mode single_race --opponents
--track-file` at 960x544. The player is idle: its craft drifts to the first bend's wall and
sits there from about tick 4,500, so the camera is the same in every frame from tick 6,000
on. The AI leader enters its final lap between ticks 8,400 and 8,560 (the ramp is mid-way at
8,560 and done by about 8,700; the HUD's "current" clock restarts at the first line crossing
and is not the tick). Sky box `x 300..420, y 0..55` (clear sky left of the sun):

| Frame | Sky saturation | Sky luminance | Flare rings | Rain |
| --- | --- | --- | --- | --- |
| Module on, lap 1 (tick 6,000) | 0.201 | 0.711 | present | none |
| Same tick, module off, lap 1 | 0.201 | 0.711 | present | none |
| Module on, tick 8,560 (the ramp running) | 0.126 | 0.660 | fading | drops, lens drops |
| Module on, final lap (tick 9,000) | 0.088 | 0.537 | gone | drops, lens drops, wet road |
| Module off, tick 9,000 (same camera) | 0.202 | 0.753 | present | none |

The module-on and module-off frames at ticks 300, 6,000 and 8,400 (the last one before the
final lap) are **byte-identical PNGs** (0 differing pixels; module off against module off is
identical too, so the comparison has no noise); `oag-game`'s
`hillside_rain_frame_ground_truth` asserts the tick 6,000 pair equal and the tick 9,000 pair's
sky saturation, luminance and flare patch down (a mutation check: dropping the flare's alpha
scale fails it). `hillside_rain_ground_truth` (disc-backed) counts it: a two-lap race at tick 600 holds
no output and no drop; a one-lap race at tick 900 holds 32 drops in the air (the original
holds 32), 49 on the glass, a wet road of 0.26 and 32 drop segments for the splashes, with
`Sky { rain: Some(1.0), sun: 0.0, grey: 1.0 }`; a disc circuit never loads the module.

Fuel (`tests::extension_examples`, a full grid with one craft on its final lap): 2,363 on the
tick it starts, 292 after, against the 200,000 budget.

### And track properties (2026-10-30)

A mod reads arbitrary **author-defined properties** off a track's pads, its triggers and
the track itself, and hears which pad or trigger a craft entered. The engine gives no
property a meaning: what `boost` or `gate` does is the mod's. The authoring side is the
[track file](../../formats/track-file.md#properties-and-triggers-for-simulation-mods)
(`mod_*` custom properties in Blender). ABI 1 is edited in place (nothing is released): the
revision stays 3 and the layout fold is re-recorded.

**The track region.** A second buffer, read-only, no imports: a guest exporting
`oag_track() -> u32` has the host write the race's track there before each call (the
same bytes every tick, built once at load, never hashed; `oag_ext_abi::track`). Entities
in a fixed order: the track, each speed pad, each weapon pad, each trigger, each spline path
and each route ([Checkpoint Circuit](#and-checkpoint-circuit-2026-10-30)), so speed pad
`i` is entity `1 + i`. Each is 96 bytes (kind, index, first property, property count,
local-to-world 4x4, box minimum and maximum) and its properties are `(key, f32)` pairs in
a table after the entities. **The key** is the FNV-1a 32 hash of the name's UTF-8 bytes,
computed at compile time by the SDK's `prop!("boost")`; two names of one entity hashing
alike are refused at authoring and at load. **Caps** (chosen): 256 entities, 64 triggers,
1,024 properties, so the region is 32,780 bytes in the guest (zeroed `.bss`, no wasm
bytes). A disc circuit exposes its pads and the track with no properties. What was
counted: Pulse PSP's 40 track files carry 544 speed and 218 weapon pads in all (19 a
file on average), HD's Talon's Junction 18 speed pads; no per-circuit maximum was taken
and 2048 and Omega were not counted. A track past a cap, a disc circuit included, leaves
every module out (Wall Penalty too), named in the load report. SDK: `Observation::track()`, then `speed_pad(i)`, `weapon_pad(i)`, `trigger(i)`,
`whole()`, each `prop(prop!("name")) -> Option<f32>`, `to_world()`, `bounds()`.

| Piece | Kind | What it is |
| --- | --- | --- |
| `speed-pad{slot, pad}` | event | Every entry into a speed pad (`Race::test_speedup_pads`' edge), whatever its factor |
| `weapon-pad{slot, pad}` | event | Every entry into a weapon pad (`Race::test_weapon_pads`' edge), cooling down or not, granting or not |
| `trigger-enter{slot, trigger}`, `trigger-exit{slot, trigger}` | event | A trigger box against the craft's pad sweep (the four points `Race::pad_sweep` tests a pad with), once a tick for each craft that steps, only while a module runs; inside is any point in the box. Exits come before enters |
| `scale-pad{pad, factor}` | command | Race-wide (slot 0), held until replaced, hashed. `pad` below the track's speed pads, else a fault; the later module in id order wins |
| `scale-pad{slot, factor}` | command | This tick (re-sent each tick it holds), hashed. Two modules multiply, bounded at 2 |

Both factors are in `[0, 2]`; a pad pushes the slot by the pad's factor times the slot's
(so up to 4). **What a factor scales (chosen, not measured):** the pad's push alone, as the
length of the direction the pad hands the physics (`Environment::pad_hit` reads an
unnormalised direction as a scaled boost), so `oag-physics` is untouched; the jump lift
added to it when the pilot pitches up is not scaled. At a product of 0 the pad gives no
push at all (no `pad_hit`), and the entry pays no plume, pad sound, lap boost count or Zone
score; above 0 they are the pad's as ever. Exactly 1.0 without a module, so a vanilla race
hashes as before (`a_vanilla_race_hashes_the_same_with_the_feature_on_or_off_and_builds_no_engine`).

Events carry the pad or trigger index in the record's bytes 2..4 (zero for the others), in
slot order, each slot's wall contact, fire, absorb, speed pad, weapon pad, trigger exits and
trigger enters; past 32 in one call the rest are dropped.

**Proof.** `crates/ext/src/tests.rs` (a guest reads `prop!("boost")` as 1.5 off pad 1 and
none off pad 0, and hears all four events); `oag_modhost`'s region and `scale-pad` checks;
`crates/raceplay/src/tests/extensions.rs` (`triggers::`: a craft drives through a trigger,
one enter and one exit, and the guest reads its property out of the region);
`track_props_ground_truth` (Pulse PSP EU, Hillside with its speed pads widened 20 units
either side so this project's driver crosses them, Venom, throttle held):

- `boost = 2.0` authored on pad 0, read by a WAT module that sends `scale-pad{0, boost}`:
  entered at tick 1,242 the push is 2.0 against 1.0 vanilla, the same race up to it;
  720.9 km/h 30 ticks later against 618.0; pad 2, entered at tick 1,777, pushes 1.0.
- `scale-pad{slot 0, 0}` every tick: over 2,400 ticks no push arms, no plume, no pad
  sound (vanilla: 2 pushes, 3 pad sounds), and the module heard 3 `speed-pad` events for
  the 3 pads entered.

### And the SDK (2026-10-29)

Both examples wrote ABI 1 by hand: copied offsets, `unsafe` byte plumbing, a state
blob serialised field by field, commands as positional tuples, magic codes, and the
host's rules (an empty slot, two `scale-thrust` for one craft, a shield change past
the pool) left for a race-time fault. Two crates replace that, and both examples are
rewritten on them with **no `unsafe`, no offsets and no ABI codes**:

| Piece | Where | What it is |
| --- | --- | --- |
| The layout | `crates/ext-abi` (`oag-ext-abi`) | `no_std`, zero dependencies, `#![forbid(unsafe_code)]`. Offsets, sizes, caps, flag bits, and the records `Craft`, `Event`, `Command` with their encode and decode; the shared vocabulary `Slot`, `Weapon` (the 14 codes in `Weapon::ALL`'s order), `Pickup` (`None`, `Weapon`, `Ours`), `Cue`, `Effect`, `Rgba`. `oag-modhost` depends on it and keeps only the checks that need the observation (the slot holds a craft, the shield maximum, the pickup held, one of a kind per slot); `oag-raceplay` maps the vocabulary to the engine's with exhaustive `match`es, so either side growing is a compile error |
| The revision | `oag_ext_abi::REVISION` | Bumped by hand on any edit to the layout; a unit test folds every offset, size, code and one encoding of each record into a committed `(REVISION, fold)` pair, so a layout edit without a bump fails the gate. `oag_abi()` returns `(ABI << 8) \| REVISION`; the host refuses another revision **by name** ("built for ABI 1 rev 2, this host is ABI 1 rev 1"), and a value below 256 as a module "without a layout revision". Revision 1 is the layout the pre-SDK examples were built against, so the old `.wasm` files are refused |
| The state length | `oag_state_len` | An optional export; when present it must equal the manifest's `state_bytes` (refused by name otherwise). `state_bytes` stays required in the manifest and its schema |
| The guest SDK | `crates/ext` (`oag-ext`) | `no_std`, no `alloc`, depends on `oag-ext-abi` only; no engine crate may depend on it (`just check-deps`, rule 2). `Observation` (`tick()`, `crafts()` over occupied slots, `craft(slot)`, `events()` skipping unknown kinds); `Craft`, a view whose accessors (`is_person`, `fire_held`, `is_racing`, `shield`, `shield_max`, `speed_kmh`, `pickup`) read their field when asked; `Reply` with one method per command, taking a `&Craft` where the slot must hold one and returning `Err(Refused)` where the host would fault; the count written by the glue; `blob!` for the state (a plain-data struct, field order, little-endian, no padding, a `const` assert against `MAX_STATE_BYTES`); the `Extension` trait carrying the fresh-instance and one-tick-late rules; `export!` emitting `oag_abi`, `oag_io`, `oag_track`, `oag_state_len`, `tick` and the panic handler on wasm32 only; `Observation::track()` and `prop!` read the track region. Its `test` feature's `test::Race` runs an extension natively through the host's own writer and decoder |
| The examples | `mod-examples`, branch `ext-sdk` | Wall Penalty 87 lines (its blob `[u16; 8]`), Energy Turbo 307 (its record kept at 12 bytes with a spare `u16`), each with native tests under `cargo test`. The dependency is a `path` into the engine checkout the submodule sits in; a stand-alone clone needs a git dependency pinned to the revision, which waits until the branch is public |

Measured on the rebuilt modules (`opt-level = "s"`, LTO, stripped). Fuel is from
`oag_raceplay`'s `tests::extension_examples`: a full grid of eight racing crafts
carrying the mod's pickup, slot 0 a person holding fire, a loud tick (every craft
touching a wall, firing and crossing a pad), then a quiet one:

| Module | Bytes before | Bytes after | Fuel, loud tick | Fuel, quiet tick |
| --- | --- | --- | --- | --- |
| Wall Penalty | 627 | 638 | 1,968 to 2,586 | 1,207 to 834 |
| Energy Turbo | 1,774 | 2,395 | 14,267 to 11,722 | 10,027 to 8,432 |

A first cut that decoded every occupied slot up front cost Wall Penalty 2.25 times its
hand-written fuel on the quiet tick; the craft view is why it now costs less.

Proof: `oag_ext_abi`'s unit tests (the fold, record round trips, the malformed
cases), `oag_modhost`'s (another revision and a module without one refused, both
named; an `oag_state_len` disagreeing with the manifest refused), `oag_ext`'s
(the blob's byte layout against Energy Turbo's hand-written one, the reply's
refusals, an extension run natively), the examples' own `cargo test`, and
`tests::extension_examples`, which compiles each committed example `.wasm` in
`just check-extensions`: a module built against another revision fails there by
name. `wall_penalty_ground_truth` and `energy_turbo_ground_truth` pass unchanged
against the rebuilt modules, and the vanilla fold did not move.

Not built yet: an `oag-ext check` tool, a generated C header and a C++ example
built from it, a template mod, and the git dependency.

### Breakable armour from the Shield (2026-10-30)

A mod turns the title's Shield into armour with generic pieces; the armour itself is a
test fixture (`crates/raceplay/src/tests/extension_armor.rs`, a WAT guest, and
`crates/ext/src/tests/armor.rs`, the same logic through `oag_ext::test::Race`), never an
example mod.

| Piece | Kind | What it is |
| --- | --- | --- |
| `hit{slot, amount, weapon}` | event (code 9) | Weapons hit a craft, **a hit its Shield absorbed included**. `amount` is the sum of what the tick's hits asked of the pool (`oag_physics::damage::Shield::asked`: the weapon's damage after the weapons-off scale, whether or not the Shield swallowed it); `weapon` is the first of them. One event per craft per tick. The weapon code sits in the event's index field and the `f32` in bytes 4-7 (`EVENT_VALUE`) |
| craft flag `shielded` | observation | Bit 8 of the craft record's flags (`CRAFT_SHIELDED`): the Shield is running, so a hit this tick is absorbed. Without it a guest could not tell an absorbed `hit` from a landed one |
| `give-shield{slot}` | command (24) | Starts the title's Shield for its authored `<Shield time>`, as firing a held Shield pickup does (`Race::spend_pickup`), including its rule that one already running is not refreshed. A race without weapons, or a title with no Shield table, gives nothing |
| `hold-shield{slot}` | command (25) | Floors a running Shield's timer at two ticks, so the step leaves it running. Re-sent each tick it should hold; never starts one. Once the module stops, the Shield ends within two ticks |
| `clear-shield{slot}` | command (26) | Zeroes the timer at the call, before the step |

**`give-shield`, not a generalised `give-pickup`.** The Shield is a timed state, not a
held pickup: `give-pickup` has no argument and one owner per race, and a held Shield
would need the player's fire press. `give-shield` reuses the fire arm's rule and table.

**Timing.** Every hit source runs after the call (`projectile::step`, `advance_quake`,
`advance_leach_beam`, `advance_repulsers`), so a hit in step N reaches the call at N+1,
which runs before step N+1. A `clear-shield` sent there protects nothing in step N+1:
the first hit after the call that sent it lands on the energy. A second weapon hitting in
the same step N as the breaking one is still absorbed (the Shield is up for the whole
step); a module cannot close that, and the armour fixture does not claim it. Proven by
`a_hit_on_a_held_shield_takes_nothing_and_reaches_the_module_one_call_later` and
`the_hit_that_breaks_the_armour_costs_nothing_and_the_next_one_lands` with real Rocket
impacts, and each test fails when the hit buffer or the hold is dropped.

**What raises `hit` (a limit).** Rocket, Missile, Plasma, Shuriken, Mine and Bomb blasts,
Cannon rounds, the Repulser and a Leach Beam drain that reaches `apply_weapon`. The Quake
wave and the Leach Beam link skip a craft under a running Shield before `Ship_Damage`
(the Quake's `lifted && !shielded` test, the link's break on the target's Shield), as the
original's does: a skipped craft is not hit, so they raise nothing against a running
Shield and raise `hit` when they land. `shield-delta` goes through `apply_weapon` but is not a weapon and raises
none. Slowdown and the Disruptor are not damage and raise none.

**Chosen, not measured:** the two-tick floor of `hold-shield` and the one-event-per-tick
merge. No original holds a Shield. **The Eliminator's respawn Shield** (`eliminator.rs`)
uses the same timer, so `hold-shield` and `clear-shield` act on it too.

**State.** The hit buffer (amount and first weapon per slot) is `Extensions` state,
hashed after the `slots.is_empty()` early return, so a race with no module hashes as
before (`VANILLA_FOLD`). The `ShipState` hash is untouched. The ABI changed in place: no
revision bump, the layout fold in `ext-abi` re-recorded.

**Titles.** The engine pieces name no title, and the Shield time is read from the race's
own weapon table. They were exercised only on the synthetic Pulse fixture of the
`oag-raceplay` tests; no disc-backed race on Pulse, HD, 2048 or Omega was run, and no
census of those titles' Shield tables was taken for this change.


### And Checkpoint Circuit (2026-10-30)

The fourth example, `oag.checkpoint-circuit` (`assets/mods/examples/checkpoint-circuit/`, a
track module of 9,046 bytes on `oag-ext`, and its own track file built by
`checkpoint_circuit.py` in Blender): the example of triggers, track properties, HUD text from a
mod's own strings, and the AI route hook. Each craft has **20 s** to its next checkpoint gate
(`oag/triggers/checkpoint <n>`, `mod_checkpoint = n`, 1 to 4 in lap order); reaching it resets
the clock; past zero it loses **1/240** of its maximum energy a tick (`shield-delta`) until it
reaches the gate or is wrecked. The pit lane (the trigger `mod_pit = 1`) gives **1/300** of
the maximum a tick, never past it. An AI craft below **40 %** asks for the pit route. All four
numbers are the guest's constants, **chosen, not measured**. A person's HUD shows the clock
under `CPC_CHECKPOINT` (red in the last 3 s; past zero the energy bar turns red and shakes) and
`CPC_RECHARGING` in the pit, both from the mod's `strings/english.toml`.

The pit lane is **a real fork in the spline**: three paths and two junctions, the pattern
Pulse's own forked circuits use (`05_Track`, `07_Track`: the primary and the alternate leave
one junction and meet at the next, sharing their first and last point bit for bit). So the
course builds it as a route (`oag_race::course::Route`), our AI plans it like any other route
(its speed plan verified clean on the first load, so the coin offers it) and drives it with the
player's physics. What it added, each with the vanilla fold unchanged:

| Piece | Kind | What it is |
| --- | --- | --- |
| Path properties | track file | `mod_*` on a Blender spline object, written as the glb spline node's `oag_props` extra (one object); `oag_trackfile::Track::path_props`, one list per path or none. A file without any writes nothing new |
| Paths and routes in the region | observation | Entity kinds 5 (`Path`) and 6 (`Route`) after the triggers; their counts at `PATHS_AT` (6) and `ROUTES_AT` (8), so the entities start at 12. A path's box bounds its points, a route's its samples (world space, the identity placement). A route carries the properties of the paths it drives, the first path's value kept where two set one name, so a guest finds the pit lane's route by `route(r).prop(prop!("pit"))`. SDK: `Track::paths()`, `routes()`, `path(i)`, `route(i)` |
| `prefer-route{slot, route}` | command (21) | Re-sent each tick it should hold; `route` below the region's route count (the course's numbering, `Course::routes`), one per slot per tick, else a fault. Held in `Extensions` for the tick and **hashed** after the no-module early return. SDK: `Reply::prefer_route(&craft, route)`. Code 22 is `pickup-look`, 23 is `emit` ([below](#and-events-between-mods-2026-10-30)) |

**Where it lands.** `Race::steer_branching` (`crates/raceplay/src/routes.rs`), `Ai_ChooseBranch`'s
states 0 to 2: the coin is thrown exactly as before on entering the pre-fork path
(`oag_ai::branch::choose`, the driver's own seed and visit count), and where the driver moves
onto a route's line, within `SWITCH_SAMPLES` of the split, a preference held that tick wins over
the coin's draw if the route leaves that fork and its speed plan verified. So the race's random
streams never move, a preference dropped before the switch leaves the coin's draw, and a person
steering ignores it (it moves the driver only; under Autopilot it moves the Autopilot's). The
hook picks a way; the AI drives it with the same physics as every other craft. `RouteLine`
keeps its course index, because `route_lines` skips a route it cannot match onto the spline.

**Proof** (`checkpoint_circuit_ground_truth`, Pulse PSP EU, Venom, Single Race on the track
file, the committed module):
- Alone on the grid and never flown, the clock runs out at tick 1,471 (1,200 ticks after GO),
  then the craft loses 0.396 of its 95 a tick, every tick, and is wrecked at tick 1,710 (239
  draining ticks); the vanilla craft keeps 95 throughout.
- Flown round by this project's driver it reaches the gates in order 1, 2, 3, 4, 1, 2 with
  468 to 526 ticks to spare, each resetting the clock to 1,199, and its energy equals the
  vanilla race's bit for bit every tick of 5,000.
- In a full field with weapons off, AI slot 1 started at 39 % is in the pit trigger from tick
  1,681 to 2,083 and goes from 42.90 to 110.00 of 110, gaining 1/300 a tick and never past it.
- At full energy every AI craft's route and coin draws equal the vanilla race's every tick of
  2,400 (six of seven take the pit lane by their coin there); slot 7, whose coin keeps it on the
  main line, takes the pit route at tick 840 when started at 41.0 of 105, its draws unchanged.
  Dropping the hook (`take = branching.pending`) fails this test.
- Two runs hash the same every tick.
- Two full-field races that differ only in a module holding `prefer-route` on a slot whose
  coin already takes the pit lane agree on every craft's position and route every tick of
  1,500 and hash differently from the first call: the held preference is state. Dropping
  `Preferred::hash_into` fails it at tick 0.

Fuel (`tests::extension_examples`, a full grid): 14,443 on the loud tick, 10,499 after.


### And events between mods (2026-10-30)

Mods, and tracks authored in Blender, send each other events over declared channels. ABI 1 was
edited in place (no revision bump; the layout fold re-recorded in its own commit, every example
module rebuilt), so **events are 16 bytes now** and every module built before it is refused only by
its stale reading of them: rebuild against the engine you run.

| Piece | Kind | What it is |
| --- | --- | --- |
| `emit{slot, channel, value}` | command (23) | `channel!("<mod id>/<name>")`, the compile-time FNV-1a 32 hash a property's name has. The mod must publish the channel and a module may send **4** a tick, else a fault by name. SDK: `Reply::emit`, `Refused::Emits` |
| `mod{slot, from, channel, value}` | event (10) | Every module hears it on the **next** tick, sorted, ahead of the engine's events; the queue is **hashed**. `from` is the sender mod id's key, or `TRACK_SENDER` |
| a trigger's `oag_emit` | track file | A craft entering it sends the channel, no module needed; any module hears it from `TRACK_SENDER` |
| an animation's `oag_on` | track file | The art animation holds its first key until the channel fires, then plays from there. Render only |
| `[channels] publishes, listens` | `oag-mod.toml` | Namespaced names, refused by name when malformed or foreign; the MODS page shows SENDS, HEARS and **NO ONE SENDS** |
| `[channels] publishes` | `.track.toml` | What its triggers may send |

The caps (**4** a module a tick, **16** `mod` events a tick) are **chosen, not measured**.
[hooks.md](hooks.md#events-between-mods-emit-and-mod) has where it lands.

**Proof.** `tests/extension_channels.rs` (hand-written guests): an emit on tick 3 reaches the
listener with slot, value, sender and channel intact on tick 4 and once only; the result is the same
with the emitter before or after the listener (swapped mod ids); two emitters' events arrive in
channel-key order whichever sent first; a race that differs from another only in a queued value
hashes differently at the end of the emitting tick (dropping `hash_mail` fails it); a race with no
module hashes as the same race without the feature, tick for tick; a fifth emit and an undeclared
channel each fault the module naming the rule, with the four delivered or none queued; a trigger
reaches a listening module from `TRACK_SENDER` and, with no module, still raises its channel
(`Race::channel_started`) and leaves every hash as it was. `oag-ext`'s own pair of guests
(`Race::hear`) proves the SDK end. The render side's clock
(`foreign::art::tests`, `an_animation_that_starts_on_a_channel_holds_until_it_fires_then_plays_from_there`)
holds the first key before the channel fires and plays from it after, a node no channel starts keeps
reading the world clock, and `write_node_anims_started` writes a channel-driven model every frame
(a table that depends on more than the clock). Checkpoint Circuit announces each gate on
`oag.checkpoint-circuit/checkpoint` (`checkpoint_circuit_ground_truth`: never faulted, the channel
fired). **Screenshots** (headless, `--race --track-file channels.track.toml --autopilot` with a fixed
camera at the gate, built by `roundtrip.py channels`: the loop with its airship starting on a
gate's channel): at tick 200 the airship rests at its first key, far left; it holds there until the
first craft crosses the gate (one of the field, not necessarily the person); at ticks 340 and
400 it has flown on along its loop.

**Later: event urgency levels (a note, not built).** Today every event is delivered on the next
tick, which is what keeps the result independent of module order. A later revision might name
levels per channel: *next tick* (the default, as built), *immediate in the mod* (a module's own
event seen at once in the same call; it needs no host), *async* (only a fixed delay in ticks, never
a wall-clock one), *lazy* (only the latest value per channel and slot is kept, for a gauge that
need not queue) and a *priority* that decides what is dropped past the cap, in place of today's
sort order. Each stays deterministic by the same rule: delivery order and drops are a function of
the queued events alone, never of which module ran first.

## Keeping the mod author skill current

A change to the ABI, the guest SDK, `oag-mod.toml` or a mod-facing format updates
`assets/mods/examples/.claude/skills/oag-mod-author/SKILL.md` in the same change.

## What is already decided, and not reopened here

Quoted from the handover thread on custom race modes and the ADRs it cites; the
page builds on them rather than re-deriving them.

- **Custom race modes are implemented through mods, potentially WebAssembly**
  (maintainer, 2026-10-12). Rules a mod can say as data come first (`modes/*.toml`);
  the WebAssembly hook is for what data cannot express.
- **Two phases, then a third.** Phase 1 runs modules only where nothing reaches a
  race's state (HUD widgets, and the frontend world's mod screens, section 2.3). Phase 2 opens the simulation hook; a mod that touches
  the simulation declares `simulation = true` and marks the session **MODDED**: no
  multiplayer, no ghosts or replays recorded or played back, records in a separate
  modded table, MODDED on the HUD and the MODS page. Phase 3 lifts that once module
  hashes travel in the replay header and the lobby and the cross-engine conformance
  test holds.
- **Engine choice**: wasmtime with Cranelift on Linux, Windows, macOS and Android;
  wasmtime's Pulley interpreter on iOS (no JIT); the browser's own `WebAssembly` on
  the web build. Three engines must give bit-identical results: a deterministic
  subset checked at load (no relaxed SIMD, no threads), a NaN rule, one host trait
  with three backends, a conformance test.
- **Phase 2's first user is Pilot Assist** on Pulse and Pure, with bit-identical
  replay hashes against the built-in Rust law as the oracle.
- **The original campaigns stay vanilla.** A custom campaign
  ([ADR-0077](../adr/0077-a-custom-campaign-compiles-to-the-titles-own-grid.md)) is
  where custom modes, tracks, craft and controllers belong.
- **A mod supports only the titles it declares** (`titles = [...]` in `oag-mod.toml`).
- **A track file carries no trigger events in 1.x**; behaviour that reacts to the
  race is a mod behaviour, the WebAssembly rules host
  ([ADR-0076](../adr/0076-a-track-file-carries-its-art-as-an-optional-section.md)
  decision 11).
- **Mods may change anything, simulation and race modes included**; the server
  advertises its content fingerprint and the client shows it on join; mod code
  arriving through a lobby is never auto-run (`docs/networking/README.md`,
  decision 6). Online play is server-authoritative; a LAN host is trusted
  (decisions 2 and 3).
- **Modern saves**: progress persists automatically, no save slots.
- **Pre-release**: a wire or format change edits v1 in place; no migration paths.

The maintainer's ask this page answers (2026-10-29, verbatim intent): a wasmtime
extension design where custom crafts, custom tracks and mods define their own
behaviour, inspired by 2048's prototype craft with individual specialties (Feisar
gets faster on each speed pad); custom tracks that trigger or add physics (spring
jumps, a gravity switch to the underside), custom track rules (A to B instead of a
circuit), custom weapons, custom or extended HUDs. Output: where the engine is
hooked, what is exposed, what is not.

## 1. Who extends what

Six extension kinds, three worlds. A **world** is the set of functions a guest
module exports and the host functions it may import. The **sim world** is
deterministic and is called once per tick inside the race tick; the **HUD world**
runs on the render side and can reach nothing the simulation hashes; the
**frontend world** runs a mod's own screen in the menus (section 2.3), reaches no
race and no hash, and can hand back one small data blob.

| Kind | Bound to | World | Declared in | Runs when |
| --- | --- | --- | --- | --- |
| **Craft** | a team identity (`oag_title::team::TeamIdentity::identity`, the shared lower-case id, `crates/title/src/team.rs:18-27`), optionally a variant (2048's `feisar2048\4` is the prototype, `oag_2048::race::SHIP_TYPES`) | sim | `oag-mod.toml` `[[extension.craft]]` | every grid slot flying that hull, a person's or the AI's alike |
| **Track** | one track file ([ADR-0063](../adr/0063-the-own-track-format-is-glb-plus-a-toml-manifest.md)) | sim | the track manifest, `track.extension` (a new optional key of `schemas/v1/track.tosd`); **built instead as** `oag-mod.toml`'s `[extension.track]` (`module` and `track`), see "And Hillside's rain" | a race on that track |
| **Rules / race mode** | a mode the mod declares (`modes/*.toml`, the rules-as-data step the thread puts first) | sim | the mode file, `extension = "..."` | a race in that mode |
| **Weapon** | a pickup id the mod declares (a custom arm of `oag_tables::weapons::Weapon`, an engine change) | sim | `oag-mod.toml` `[[extension.weapon]]` | a race with weapons on, on the titles the mod is for |
| **HUD** | a widget box in a HUD layout patch (`hud/*.toml`, the thread's section 1c) | hud | `oag-mod.toml` `[[extension.hud]]` | the HUD of a race the mod is on for |
| **Screen** | a menu row of the mod's (`menu.toml`, action `open-mod-screen`, [ADR-0080](../adr/0080-a-wasm-mod-may-own-a-screen-and-save-a-small-blob.md)) | frontend | `oag-mod.toml` `[[extension.screen]]` | the player opens the row; never during a race |

A **pilot controller** (a module flying an AI slot, `controller = "mod:<name>"` in
a pilot file) is the pilot-controller thread's ADR; it would share this host, this
observation and these limits, and is not one of the five kinds here.

### 1.1 Declaring an extension

A mod is a folder, `.zip` or `.oagmod` with `oag-mod.toml` at its root
([ADR-0066](../adr/0066-the-menu-is-the-disc-plus-mods.md) decision 6,
`crates/mod/src/manifest.rs`, `schemas/v1/mod.tosd`). The manifest gains one
`[extension]` table. **This edits `v1/mod.tosd` in place** (pre-release, no
migration): every key below is optional, so an existing manifest stays valid.

```toml
toml-schema = { location = "https://oag.topaxi.com/mods/schemas/v1/mod.tosd", version = "1.0.0" }

[mod]
id = "topaxi.prototypes"
name = "Prototype craft"
version = "0.1.0"
authors = ["A. Modder"]
license = "MIT"
description = "Each team's craft gets one specialty."
titles = ["wipeout-pulse", "wipeout-2048"]
oag = ">=0.1"

[extension]
simulation = true                  # required when any sim-world module is listed;
                                   # a sim module without it is refused by name
api = 1                            # the ABI major the modules were built against
memory_pages = 16                  # the guest's maximum linear memory, 64 KiB pages (ceiling 256)
state_bytes = 256                  # the host-owned state blob (section 2.1), capped by the host

[[extension.craft]]
team = "feisar"                    # TeamIdentity::identity
module = "ext/feisar.wasm"         # a file in the mod, core wasm, sim world
# variant = "prototype"            # optional: one of a title's craft variants only
# titles = ["wipeout-2048"]        # optional: narrows [mod] titles further

[[extension.weapon]]
id = "emp"                         # the pickup id this mod adds
name_string_id = "WEAPON_EMP"      # strings/<language>.toml
icon = "ext/emp.png"               # the HUD icon, the mod's own file
module = "ext/emp.wasm"            # sim world

[[extension.hud]]
id = "lap-delta"
module = "ext/lap_delta.wasm"      # hud world
layout = "hud/lap-delta.toml"      # the widget box, per title layout (section 2.2)

[[extension.screen]]
id = "tune"                        # a mod menu row names this: action = "open-mod-screen", screen = "tune"
module = "ext/tune.wasm"           # frontend world (section 2.3)
title_string_id = "TUNE_TITLE"     # strings/<language>.toml
data_bytes = 1024                  # the most the screen may save (cap 4096, section 2.3)
```

A track-bound module is named by the **track manifest**, not by `oag-mod.toml`,
because a track travels on its own (another mod may race it:
`file = "oag.hillside:tracks/hillside.track.toml"`, `docs/tools/mods.md`):

```toml
[track]
id = "spring-loop"
title = "Spring Loop"
glb = "spring-loop.glb"
extension = "spring-loop.wasm"     # beside the manifest; sim world; optional
```

`track.extension` is a minor addition to `schemas/v1/track.tosd` (an optional key;
a reader of today reports it as unread, ADR-0063 decision 3). The volumes such a
module reacts to are a new optional glb node kind, `oag/volumes/<name>`, the same
box shape as a pad (`_OAG_TO_WORLD`, `_OAG_MIN`, `_OAG_MAX`), tested in the pad
sweep after the weapon pads: a generalisation of the pad test, not a second
trigger system ([hooks.md](hooks.md#event-sources), `volume-entered`).

A rules module is named by the mode file the thread's step 2 introduces
(`modes/*.toml`, `schemas/v1/mode.tosd`, not written yet):

```toml
[mode]
id = "a-to-b"
name_string_id = "MODE_A_TO_B"
finish = "distance"                # rules as data: laps | distance | mod
extension = "ext/a_to_b.wasm"      # what data cannot say; optional
```

### 1.2 Load order and conflicts

- **Which mods take part** is the MODS page's business and no other: the search
  path (`oag_mod::discover::mods_search_path`, `crates/mod/src/discover.rs:20`)
  filtered by the disabled set
  ([ADR-0078](../adr/0078-a-mod-is-turned-off-by-id-live-and-the-built-in-never.md);
  `oag_game::mods::Mods::player`, `crates/game/src/mods.rs:456`). Extensions are read
  through that one gate, so a mod that is off contributes none.
- **Modules are called in the order their ids sort** (not search-path order, which
  differs between machines and would desync a lobby); each sees the same observation
  and never another's commands ([hooks.md](hooks.md#the-shape-one-call-per-tick-events-in-commands-out)).
- **A module is compiled at race load, never per tick**, on the load thread the race
  already builds on (`docs/architecture/race-load-transition.md`); the browser
  compiles asynchronously (section 5). The spike compiled a 1.6 KB guest in about
  2 ms ([determinism-and-state.md](determinism-and-state.md#the-spike)).
- **One craft module per hull per race.** Two enabled mods binding the same team
  identity on the same title: the first by id wins, the second is left out **and
  named in the load report** (`mod B: craft extension for "feisar" already provided
  by mod A`). Never stacked: two modules each scaling thrust is a result nobody
  authored.
- **One track module per track file, one rules module per race**; the mode names
  its own. A track module and a rules module may both run in a race (a spring-jump
  track in an A-to-B mode). A track's volumes answer only to the track's own module.
- **Weapon modules stack by pickup id**; a repeated id is the craft rule again.
- **HUD modules stack**; each owns its box, and two widgets on one box draw in id
  order, the later over the earlier.
- **Commands from several modules in one tick** are applied in id order, each
  checked on its own; a command outside its bound is a fault (section 4.2), never a
  silent clamp.
- **A module that fails to compile or validate** leaves its mod out of the race
  with the reason (ADR-0066 decision 8: a broken mod never stops the game). The race
  runs vanilla and says so.
- **The original campaigns load no extension**, whatever is installed
  (settled: the campaigns stay vanilla). Race box, Race Remix, custom campaigns and
  multiplayer (phase 3) do.

## 2. The interface sketch

**The wire ABI is core WebAssembly**: one I/O buffer in the guest's memory, three
exports, one import namespace `oag`. The component model does not run natively in
browsers and three engines must run the same bytes
([determinism-and-state.md](determinism-and-state.md#web-and-ios)). The WIT below
is **documentation of the records and signatures**, written in the component-model
interface language because it is the clearest way to say what crosses the
boundary; the host writes each record into the buffer in a fixed little-endian
layout and reads the reply back.

**One call per tick, events in, commands out**
([hooks.md](hooks.md#the-shape-one-call-per-tick-events-in-commands-out)). The
engine does not call into a guest at every interesting point; it records events
where they happen and calls each sim module once per tick at one fixed phase, C,
after the people's pickups are spent and before any craft steps
(`Race::tick_body`, between `crates/raceplay/src/tick.rs:186` and `:193`;
[hooks.md](hooks.md#the-tick-with-the-call-the-events-and-the-commands-in-it)).
A command lands in the same tick's force law and race rules. **Events raised after
C** (pads, volumes, walls, hits, laps, respawns, eliminations, an AI craft's fire)
reach the guest in the **next** tick's observation: a one-tick latency, identical
on every machine, stated rather than hidden. A law that needs same-tick reaction
is a measured original's and belongs in the engine, not in a mod.

**State is the host's.** A guest runs on a pooled instance that is fresh every
tick, so nothing in its own memory survives a tick boundary; what does is the
module's fixed-size state blob, passed in with the observation and returned with
the commands, kept in the race state and hashed with it
([determinism-and-state.md](determinism-and-state.md#mod-state-is-a-fixed-size-blob-in-the-race-state)).

### 2.1 The sim world

```wit
package oag:ext@1.0.0;

interface types {
    record vec3 { x: f32, y: f32, z: f32 }
    record quat { x: f32, y: f32, z: f32, w: f32 }

    enum controller { local, remote, ai }          // oag_gameplay::Controller
    enum mode { time-trial, speed-lap, zone, single-race, eliminator, custom }
    enum pad-kind { speedup, weapon }
    enum weapon { rocket, missile, quake, cannon, turbo, shield, autopilot,
                  plasma, bomb, mine, leach-beam, repulser, shuriken, disruptor,
                  custom }                          // oag_tables::weapons::Weapon + a custom arm

    /// What a person is holding this tick: oag_gameplay::InputSnapshot
    /// (crates/gameplay/src/input.rs:28), as the tick received it.
    /// Person-flown slots only; an AI slot reads all-zero.
    record input {
        held: u32, pressed: u32, released: u32,
        stick-x: f32, stick-y: f32,
        airbrake-left: f32, airbrake-right: f32,
    }

    /// One grid slot, read-only: copies of fields the renderer already reads.
    record craft {
        slot: u8,
        active: bool,
        controller: controller,
        team: string,                   // TeamIdentity::identity
        variant: option<u8>,
        position: vec3,                 // oag_physics::Body
        orientation: quat,
        linear-velocity: vec3,
        angular-velocity: vec3,
        speed-kmh: f32,
        shield: f32, shield-max: f32,   // ShipState::shield, Dimensions::shield
        craft-state: u8,                // oag_physics::damage::CraftState
        thrust: f32,                    // ShipState::thrust
        grounded: f32,                  // ShipState::grounded (0..1)
        time-airborne: f32,
        pad-timer: f32,                 // ShipState::pad_timer
        turbo-timer: f32,
        mag-lock-blend: f32,            // ShipState::mag_lock_blend (0..1)
        lap: u32, laps-target: option<u32>,
        progress: option<f32>,          // RaceState::progress, along the course
        place: u32,
        finished: bool,
        pickup: option<weapon>,
        pickup-custom: option<string>,  // the id when pickup == custom
        input: input,
    }

    /// A spline sample near a craft, a fixed count ahead of each person-flown
    /// slot, for an assist or a controller.
    record sample { index: u32, position: vec3, tangent: vec3, down: vec3,
                    half-width: f32, racing-line: f32 }

    /// This tick's events, in engine order (hooks.md, "Event sources").
    variant event {
        pad-crossed(pad-crossed),       // the entry edge, raceplay pads.rs:113 and :258
        volume-entered(volume-event),   // a track file's oag/volumes/<name> (engine change)
        volume-left(volume-event),
        wall-contact(slot-event),       // per contact edge
        weapon-fired(weapon-use),       // a person's this tick, an AI craft's a tick later
        weapon-hit(weapon-hit),         // damage already applied
        lap-completed(lap-completed),
        respawned(slot-event),          // raceplay respawn.rs:280
        reset(slot-event),              // raceplay respawn.rs:322
        eliminated(slot-event),
    }
    record pad-crossed { slot: u8, kind: pad-kind, index: u32 }
    record volume-event { slot: u8, volume: u32, name: string }
    record slot-event { slot: u8 }
    record weapon-use { slot: u8, weapon: weapon, custom: option<string> }
    record weapon-hit { victim: u8, source: u8, weapon: weapon, custom: option<string> }
    record lap-completed { slot: u8, lap: u32, lap-ticks: u32 }

    /// What a tick shows the guest. Built by the host once per tick and
    /// handed to every sim module in turn.
    record observation {
        tick: u64,
        race-ticks: u64,                // oag_race::race_clock_ticks
        mode: mode,
        mode-id: option<string>,        // a mod mode's id when mode == custom
        speed-class: string,
        ship-count: u8,
        course-length: f32,
        crafts: list<craft>,            // MAX_SHIPS entries, inactive ones flagged
        ahead: list<list<sample>>,      // per person-flown slot, empty for the rest
        events: list<event>,            // raised since the last call, capped
        state: list<u8>,                // this module's blob, state_bytes long
    }

    /// What a guest may ask for. Every command is checked by the host
    /// (section 2.1, "commands"); an out-of-range one is a fault.
    variant command {
        scale-thrust(scale-thrust),
        impulse(impulse),
        hold-mag-lock(hold-ticks),
        shield-delta(shield-delta),
        give-pickup(give-pickup),
        clear-pickup(u8),
        assist(assist),
        set-finished(u8),
        set-standing-key(standing-key),
        publish(publish),
        message(message),
    }
    record scale-thrust { slot: u8, factor: f32, ticks: u32 }
    record impulse { slot: u8, delta-velocity: vec3 }
    record hold-ticks { slot: u8, ticks: u32 }
    record shield-delta { slot: u8, amount: f32 }
    record give-pickup { slot: u8, weapon: weapon, custom: option<string> }
    /// What Race::pilot_assist_input computes today: the correction the
    /// built-in law would feed Environment::pilot_assist (the phase 2 proof).
    record assist { slot: u8, steer: f32, pitch: f32, lateral: f32 }
    record standing-key { slot: u8, key: f32 }
    record publish { key: string, value: value }
    variant value { number(f32), integer(s32), text(string) }
    record message { slot: u8, string-id: string }

    /// The guest's answer: its commands and the blob to keep. A module that
    /// has nothing to say returns the blob it was given.
    record reply { commands: list<command>, state: list<u8> }

    record setup {
        title: string,                  // the title slug
        track-id: string,
        mode: mode, mode-id: option<string>,
        speed-class: string,
        laps-target: option<u32>,
        ship-count: u8,
        seed: u64,
        crafts: list<craft>,            // the grid as placed, tick 0
        volumes: list<string>,          // the track's volume names, by index
        state: list<u8>,                // zeroed
        tune: list<u8>,                 // this mod's saved blob for this title (section 2.3), or empty
    }
    record result { finished: list<u8>, places: list<u8>, state: list<u8> }
}

/// The host functions a sim guest may import: the whole `oag` namespace.
interface sim-host {
    /// The next value of this module's own stream: an oag_core::Rng
    /// (xoshiro128**, crates/core/src/rng.rs) seeded from the race seed and the
    /// mod's id, stored beside the blob and hashed with it. A module that
    /// draws nothing never moves World::rng, which AI and pickup rolls read.
    rng-next: func() -> u32;
    /// A log line, rate-limited by the host and tagged with the mod id (proposed).
    log: func(level: u8, text: string);
}

world sim {
    import sim-host;
    use types.{setup, observation, reply, result};

    /// Once, after the grid is placed (Race::start) and before tick 0. Sees
    /// the grid; cannot move it. Returns the initial blob.
    export race-start: func(setup: setup) -> list<u8>;
    /// Once per tick at phase C.
    export tick: func(obs: observation) -> reply;
    /// Once, when the race ends for everyone. Nothing persists past it in v1
    /// (open question 9).
    export race-end: func(result: result);
}
```

**Commands: what each writes, and its bound.** Where each lands and what the host
refuses there is [hooks.md](hooks.md#command-sinks); this table is the API's side.
Every command names a slot the module may touch (a craft module: slots flying its
hull; a track module: any slot, on its own volumes; a rules module: any slot; a
weapon module: the firing slot and the slots its use names). Numbers marked
*proposed* are starting points, chosen, not measured.

| Command | Writes | Bound | Engine change needed |
| --- | --- | --- | --- |
| `scale-thrust{slot, factor, ticks}` | the slot's engine output for `ticks` ticks, through `Ship::pending_thrust_scale` (`crates/gameplay/src/world.rs:204`, consumed by the step at `player_step.rs:231`) into `Environment::thrust_scale` (`crates/physics/src/forces.rs:292`) | `factor` in `[0.25, 2.0]`, `ticks` in `[1, 3600]` (proposed); two live scales on one slot multiply and the product is bounded again | a host-side timer per slot (the field is re-armed every tick by the Leach Beam), and the force law accepting a factor above 1.0 there; a true top-speed scale is one more `Environment` field defaulting to the identity |
| `impulse{slot, delta-velocity}` | `Body::linear_velocity`, at the point between a craft's controls and its step where `oag_physics::rail::apply` already writes a body (`player_step.rs:256`, `crates/physics/src/rail.rs:17`). **Not** through `ShipState::pending_impulse`, which also arms the collision stun | magnitude at most the class's top speed (proposed); one per slot per tick | the velocity write point |
| `hold-mag-lock{slot, ticks}` | the maglock's engage input for the slot (`crates/physics/src/maglock.rs`), so `mag_lock_blend` ramps in as on a magstrip; holds only where a magnetic floor is under the craft | `ticks` in `[1, 600]` (proposed) | none beyond the input |
| `shield-delta{slot, amount}` | `ShipState::shield` through the damage path (`crates/physics/src/damage.rs`), so the shield flash, the absorb window and the wreck threshold behave as a hit does | `amount` in `[-shield-max, +shield-max]`; never above `shield-max` | none |
| `give-pickup{slot, weapon}` / `clear-pickup{slot}` | `Ship::pickup` (`world.rs:114`, `oag_weapons::pickup::Held`); `custom` requires an id this mod declares | refused in a mode with weapons off (`oag_race::Mode`) | a custom arm of `Weapon` for a mod weapon (section 7.5) |
| `assist{slot, ...}` | `Environment::pilot_assist` for a person's slot, before the step: what `Race::pilot_assist_input` (`crates/raceplay/src/pilot_assist.rs`, called at `player_step.rs:219`) computes today | each term in the built-in law's range; a slot the race did not grant an assist to is refused | the command's sink (the phase 2 proof has no other) |
| `set-finished{slot}` | `Standing::finish_tick` for every craft and `RaceState::finished` for a person's slot, at phases 8 and 9 of the same tick; a mode that finishes by mod passes `laps_target: None` to both | once per slot per race; never un-finished | none: `laps_target: None` already means "never ends on its own" (`crates/race/src/state.rs:154`) |
| `set-standing-key{slot, key}` | a per-slot key the ordering `oag_race::places` sorts before progress and time (`standing.rs:322`) | finite `f32` | `places` takes standings and the course only today |
| `publish{key, value}` | an **output**, not state: the module's HUD channel, queued beside `self.sim.cues` and drained by the view half, so a sim-only replay drops it and it never enters the hash | at most 32 keys per mod, keys at most 32 bytes, text at most 64 bytes (proposed) | the channel |
| `message{slot, string-id}` | an output likewise: the HUD message board of a person-flown slot (`self.view.huds[slot].messages`) | one per slot per tick; the id must exist in the mod's strings | the queue |

A reply is capped (64 commands per module per tick, proposed; at most one of a
kind per slot); past the cap the module faults. A non-finite `f32` anywhere in a
reply is a fault ([determinism-and-state.md](determinism-and-state.md#floating-point)).
The events list is capped too (a fixed-size per-tick buffer in the race state,
cleared at C; an overflow drops the newest and is counted, deterministically).

### 2.2 The HUD world

```wit
package oag:ext@1.0.0;

interface hud-types {
    record rect { x: f32, y: f32, w: f32, h: f32 }
    record rgba { r: f32, g: f32, b: f32, a: f32 }
    enum font { hud, hud-small, default }          // oag_hud::Font
    enum align { left, centre, right }
    variant value { number(f32), integer(s32), text(string) }

    /// A read-only copy of oag_hud::Readout (crates/hud/src/lib.rs:555),
    /// the values the title's own HUD draws, plus this mod's published keys.
    record readout {
        speed-kmh: f32,
        shield: f32, shield-max: f32,
        lap: u32, laps: u32,
        place: u32, ships: u32,
        race-ticks: u64, lap-ticks: u64,
        best-lap-ticks: option<u32>,
        lap-splits: list<option<u32>>,
        wrong-way: bool,
        zone: u32, score: s32,
        pickup: option<string>,
        thrust-percent: f32,
        mode: string,
        published: list<tuple<string, value>>,
    }

    /// What a widget draws: a subset of oag_ui::frontend::Draw
    /// (crates/ui/src/frontend/draw.rs: Fill, Sprite, Text), clipped to the box.
    variant draw { fill(fill), sprite(sprite), text(text) }
    record fill { rect: rect, color: rgba }
    record sprite { texture: u32, rect: rect, uv: rect, color: rgba }
    record text { font: font, text: string, x: f32, y: f32, align: align, scale: f32, color: rgba }
}

interface hud-host {
    /// A string from the mod's strings/<language>.toml, or the id itself.
    string: func(id: string) -> string;
    /// A texture handle: the mod's own file named in its [[extension.hud]] row,
    /// or an entry of the title's HUD sheet by name. Registered at load.
    texture: func(name: string) -> option<u32>;
    log: func(level: u8, text: string);
}

world hud {
    import hud-host;
    use hud-types.{readout, rect, draw};

    /// Once per frame the HUD draws, for the view the widget is on. `box` is
    /// the widget's rectangle in the layout's own grid, the space
    /// oag_hud::Sprite::rect is in (crates/hud/src/widget.rs:107-118), inside the
    /// safe area. `alpha` is the frame's interpolation fraction; `dt` the frame's
    /// wall-clock step, for animation only: nothing here reaches the simulation.
    export frame: func(readout: readout, box: rect, alpha: f32, dt: f32) -> list<draw>;
}
```

The host appends a widget's draws to the `oag_hud::Frame` lists the pass already
paints (`crates/hud/src/draw.rs:695-713`), after `draw_list` builds the title's own
(`HudOverlay::draw`, `crates/game/src/hud_overlay.rs:335`), scissored to `box` the
way a split-screen view's span is (`Span::clip`, `draw.rs:718`). The widget's box
comes from the mod's HUD layout patch, a `hud/*.toml` that adds, moves or hides
elements by stable id (the thread's section 1c; `hud.tosd` not written yet) and may
declare a `[[widget]]` with an `id`, a `rect` per title layout and the module that
draws it. On a title the mod is not for, the widget does not exist. The draw list is
capped (64 draws per widget per frame, proposed); video, backdrops, model draws and
anything outside the safe area are refused.

### 2.3 The frontend world

A **mod screen** is a page the mod owns, reached from a mod menu row, so that a
mod can have an editor, a picker or a settings page of its own (a craft tune
editor, a livery chooser, a custom-campaign grid editor). It is the third world
because it differs from both others: it needs input events like no HUD widget
does, it runs while no race exists, and it may leave something behind (a blob).
It is **UI side only**: no determinism, no state in the hash, no `World`, and it
never marks a session MODDED (that stays the sim module's `simulation = true`,
section 2.4).

**The row.** [ADR-0066](../adr/0066-the-menu-is-the-disc-plus-mods.md) decision 4
allows a mod row to be a `submenu` or an `action` "of the closed vocabulary". That
stays closed: ADR-0080 adds exactly **one** variant to
`oag_ui::menu::vocabulary::Action` (`crates/ui/src/menu/vocabulary.rs:13`, whose doc
says a new action is added there and handled in the composition root),
`open-mod-screen`, and the row names the screen: `screen = "tune"`. A row naming a
screen the mod does not declare is a load error naming the mod and key, as ADR-0066
decision 4 already makes an unknown action. A mod still adds no page, setting or
row to a disc's own menus, and a disc screen never gains behaviour a mod invented
(section 3).

**Widgets: host widgets first, plus a capped canvas.** Recommended, and chosen
over a raw `Draw` list alone. The existing screens all have the shape *`Input` and
`Pointer` in, `Outcome` and `Draw` out* (`TagEntry::update` and `::pointer`,
`crates/ui-screens/src/tag_entry.rs:361`, `:423`; the picker's `Event` with its
navigation sound, `crates/ui-screens/src/picker.rs:147-175`), and every new screen
must ship with mouse and touch in the same change. A guest that returned only
draws would have to re-implement focus, hit-testing, scrolling, the drag threshold
and the click sounds, and each mod would get them differently wrong on a pad or a
finger. So the guest returns a **view**, a small retained description, and the host
runs it:

- **Host widgets**: `list` (rows with an optional value, scrolled by the pointer's
  `scroll` and `drag`), `slider` (a bounded `f32` with step), `toggle`, `choice`
  (the existing option-row idiom), `text-entry` (the `TagEntry` grid over a given
  alphabet, `crates/ui-screens/src/tag_entry.rs:224`), and `button`. The host owns
  focus, the pad and the pointer (`oag_ui::pointer::Pointer`, `crates/ui/src/pointer.rs:46`:
  `clicked`, `back`, `scroll`, `drag`, and a touch decided on lift), the navigation
  sounds and the skin of the title it runs on, so a mod screen looks like the title
  it is in and a pad-only or touch-only player is served the same.
- **A canvas**: a capped list of the HUD world's three draws (`fill`, `sprite`,
  `text`; a subset of `oag_ui::frontend::Draw`, `crates/ui/src/frontend/draw.rs:28`),
  inside the screen box, for a preview a widget cannot say (a craft's stats bars).
  **Display only**: a canvas has no hit regions; interaction is a host widget's, so
  pointer and pad parity is not the author's to get right. Video, backdrops and
  model draws are refused, as in the HUD world.

**Events in, a view out.** Three exports; the instance lives from the screen's
open to its close (nothing here is rolled back, so, unlike the sim world, guest
memory may hold the screen's working state; it is dropped at close and is never
hashed):

```wit
interface frontend-types {
    use hud-types.{rect, rgba, draw};            // the canvas draws are the HUD world's
    enum button { up, down, left, right, accept, back, aux }   // oag_gameplay::input::Button, the menu subset
    variant widget {
        list(list-view),   slider(slider-view),   toggle(toggle-view),
        choice(choice-view), text-entry(text-view), button(button-view),
    }
    record list-view   { id: string, rows: list<row>, selected: u32 }
    record row         { label: string, value: option<string> }      // string ids resolved by the host
    record slider-view { id: string, label: string, min: f32, max: f32, step: f32, value: f32 }
    record toggle-view { id: string, label: string, on: bool }
    record choice-view { id: string, label: string, options: list<string>, selected: u32 }
    record text-view   { id: string, label: string, alphabet: string, max-len: u32, text: string }
    record button-view { id: string, label: string, enabled: bool }

    /// What the screen shows. The host lays the widgets out top to bottom in
    /// `box`, in order; the canvas draws sit behind them.
    record view { widgets: list<widget>, canvas: list<draw>, focus: option<string> }

    /// What the player did, already interpreted by the host's widgets.
    variant event {
        changed-list(tuple<string, u32>),        // widget id, selected row
        changed-slider(tuple<string, f32>),
        changed-toggle(tuple<string, bool>),
        changed-choice(tuple<string, u32>),
        changed-text(tuple<string, string>),
        pressed(string),                         // a button id
        back,                                    // the player asked to leave
    }

    /// Replies to an event: a new view, and optionally the blob to keep.
    record reply { view: view, save: option<list<u8>>, close: bool }

    record open-context {
        title: string,                           // the title id the screen runs on
        language: string,
        data: list<u8>,                          // this mod's saved blob for this title, or empty
        box: rect,                               // the screen box, in the layout's grid, inside the safe area
        teams: list<team-info>,                  // the title's teams (identity, string id), read-only
    }
    record team-info { identity: string, name-string-id: string }
}

interface frontend-host {
    /// A string from the mod's strings/<language>.toml, or the id itself.
    string: func(id: string) -> string;
    /// A texture handle: the mod's own file named in its [[extension.screen]] row.
    texture: func(name: string) -> option<u32>;
    log: func(level: u8, text: string);
}

world frontend {
    import frontend-host;
    use frontend-types.{open-context, view, event, reply};
    export open:  func(ctx: open-context) -> view;
    export event: func(ev: event) -> reply;
}
```

The host re-asks the guest only on an event, never per frame, so a screen costs
nothing while the player looks at it; animation is the host widgets' own (focus,
scroll). A canvas that must move waits for open question 14.

**Limits.** The same fuel rewrite and memory ceiling as the other worlds
([section 4.1](#41-engine-configuration-and-the-load-time-rewrite)), a budget per
call; whether this world uses fuel or a wall-clock deadline follows open question 2
for the HUD world, and the recommendation is **fuel here too**: the rewrite exists
for the sim world anyway, a deadline would make the same screen behave differently
on a Deck and a phone, and the cost is none. Caps (proposed): 64 widgets and 256
canvas draws a view, a string of at most 128 bytes, a list of at most 256 rows, the
blob at most 4 KiB. **One answer for every failure**: a trap, a refused value or an
oversize blob closes the screen, discards that reply (nothing is saved), and names
the mod and the reason in the load report and as a one-line notice on the menu row's
page. The player is back in the menu; no race and no other save is touched.

**Compiling, off the render thread.** A screen's module is compiled when its row is
first opened, not at startup: on a load thread (`oag-thread`) on native, and
asynchronously through the browser's `WebAssembly` on the web, as a race's modules
are (section 5). The screen shows a loading state until `open` returns; a module that
will not validate leaves the screen unopened, named in the load report, like any mod
row that does not fit (ADR-0066 decision 8). Guest calls themselves are fast (a
view is a few hundred bytes), and are made on the UI thread between frames; I/O, the
blob save included, is off the render thread (maintainer, 2026-10-27).

**Text.** A label is a string id the mod ships in its own `strings/<language>.toml`
(ADR-0066's consequence: "a mod that asks for a string id must ship it or use a
project id"), resolved by `frontend-host.string` and by the host for widget labels.
The file follows `assets/ui/strings/english.toml`'s convention: `ID = { text = "...",
human = false }`, `human = true` only when a human wrote or approved it; another
language may lag by naming the gap in an `[untranslated]` table, and a machine
translation may fill one with `human = false`. Mod-shipped literal text with no id is
refused: a screen without ids could never be translated. A `text-entry` value the
player typed is data, not a string id, and is never translated.

**What the frontend world cannot do.** It cannot start a race, change a setting,
open or modify another page, read or write any file but its own blob, read the disc
or another mod's data, open a window, play a sound of its own (the host widgets play
the title's navigation sounds) or reach a clock or the network: the imports are the
three functions above (section 3). The `teams` list in `open-context` is the one piece
of title data a screen is handed, as a copy of names it cannot change; a screen that
wants a disc table's numbers (a tune editor showing the stock handling next to a
slider) is open question 15.

**The data blob.** `reply.save` is the screen's only output. It is the fixed shape
of the thing the maintainer asked for ("a mod's editor saves a tune, a race picks it
up") and it is the same blob the section 3 non-capability row and open question 9
already proposed, so the number is reused: **at most 4 KiB per mod and title**, one
blob (a mod wanting several packs them), checked against the manifest's
`data_bytes`. The host writes it through the records path, not a file of the mod's:

- **Where it lives**: a sibling table in the records `Store`
  (`crates/game/src/records.rs:539`), keyed by mod id and title, beside `campaign` and
  `loyalty` (`:541-551`), which a file written before it existed parses to empty. One
  profile file, no slot, nothing a player manages, and written by
  `records::save_in_background(store)` (`records.rs:1090`), which queues the whole
  store behind one lock so no two writes overlap. On the web that function writes
  inline (`records.rs:1098-1102`: the file is `localStorage`, out of a Worker's
  reach), so the blob save there is inline and small by construction.
- **Encoding**: `records.toml` is text, so the table stores the bytes as base64. The
  4 KiB cap is on the **raw bytes**, checked before encoding, so a mod's budget does
  not depend on the encoding.
- **Who may read it**: this mod's screen at `open` (`open-context.data`), and this
  mod's sim module at `race-start` (below). No other mod; no HUD module. The MODS
  page names the size and offers to clear it; turning a mod off hides its blob and
  uninstalling it leaves an orphan the page lists (section 3's old worry,
  answered). Whether uninstall deletes it is open question 9.
- **Not a replay or a ghost**: records, ghosts and replays do not carry the blob
  except as the next paragraph says.

**A blob that changes a race** enters it at one place only. At `race-start`
(section 2.1) the host passes the sim module of **the same mod** a copy of that
mod's saved blob in `setup.tune` (empty if none).
The guest copies whatever it needs into its **state blob**, and from then on it is
the module's host-owned state: snapshotted, restored and hashed with everything
else (determinism-and-state.md,
[mod state](determinism-and-state.md#mod-state-is-a-fixed-size-blob-in-the-race-state),
[the state hash](determinism-and-state.md#the-state-hash)). The bytes are never
re-read from disk during the race, so a save made mid-race by a second process
cannot reach it. Nothing else crosses: a screen can never write a `Handling`, a
table or a `World` field.

**A blob that changes no race** (a layout the mod remembers, a cosmetic choice, a
bookmark) is never given to a sim module, so it is never in a hash, a replay header
or the lobby fingerprint. This is the line: *a byte is simulation input if and only
if a sim module reads it at `race-start`*, and the manifest says which mods have
one (`simulation = true`).

**Replays and the lobby (phase 3).** A replay re-simulates, so a hash of the tune
is not enough: playback on a machine with a different or no tune must still
reproduce the run. The proposal is that the replay header's `mods` entry
(determinism-and-state.md,
[replays](determinism-and-state.md#replays)) carries the **tune bytes themselves**,
at most 4 KiB per mod, beside the module hash and blob layout version; playback
feeds those bytes to `race-start` in place of the local save. For the lobby: a tune
the whole session shares (a host-chosen setting) is folded into
`content_fingerprint` with the module hash
([network play](determinism-and-state.md#network-play)); a **per-seat** tune (each
player's own craft) cannot be, since folding it would refuse every join where two
players differ. Per-seat tunes are the server receiving each seat's bytes with the
seat at race start, which touches the wire and is open question 13. Until phase 3,
a session whose sim module reads a tune is MODDED anyway: no replay, no ghost, no
multiplayer.

**Where it runs.** The screen model is `oag-ui-screens`'s, in the existing shape
(`Input` and `Pointer` in, `Outcome` and `Draw` out), over a small injected trait
for the guest, so `oag-ui-screens` links no wasmtime; `oag-game` (the composition
root) wires the `oag-modhost` implementation to it, behind the `extensions` feature.
`oag-modhost` is the same crate as the sim and HUD worlds' and is not built until a
module loads (section 4.5). The sim world's gameplay crates never learn a screen
exists.

### 2.4 Where the three worlds run

- **The host is its own crate**, proposed `oag-modhost`: the engine and its
  configuration, the I/O buffer ABI, fuel, the fault path, the pooled per-tick
  instance, the load-time checks and the module rewrite. It knows no game, depends
  on wasmtime and `oag-core` (for `Rng`), and joins the determinism scan
  ([determinism-and-state.md](determinism-and-state.md#dependency-rules)).
- **The sim call is made from `oag-raceplay`'s simulation half**, the half
  `Race::tick` lives in (`crates/raceplay/src/tick.rs:48`) and `oag-server` links
  with `view` off. The gameplay crates (`oag-gameplay`, `oag-physics`, `oag-race`,
  `oag-weapons`, `oag-ai`) gain **no dependency** on the host: commands reach them
  as inputs they already take, the way Zone's auto-speed does
  (`Environment::auto_speed`, `forces.rs:260`). `just check-deps` and `just
  test-wasm` stay meaningful.
- **The HUD host is render side**: the `hud` world runs in `oag-game`'s HUD overlay
  with the same crate, without the determinism needs.
- **The frontend host is menu side**: a mod screen runs in the front end's loop in
  `oag-game`, over `oag-ui-screens`'s model and an injected guest trait (section
  2.3), with the same crate and no determinism needs. It is never called from
  `oag-raceplay` and never during a race.
- **Behind a cargo feature** (`extensions`), because Cranelift adds about 11.3 MB
  to a binary; `oag-server` links the host (wasmtime draws nothing, rule 6 holds)
  and runs sim modules in phase 3; the web build never links wasmtime (section 5).
- **Nothing is built until a module loads** (section 4.5): with no module the host
  does not exist as far as a tick, a menu or a start-up is concerned.
- **A race with any sim module is MODDED** (phase 2). A screen or a HUD module is
  not: only `simulation = true` marks a session, and a saved blob matters to a race
  only through the sim module that reads it. With no sim module loaded the
  host writes nothing to the hash, so every golden hash and determinism reference is
  unchanged by construction ([determinism-and-state.md](determinism-and-state.md#the-state-hash)).

## 3. What is not exposed, and why

Each line is a decision, not an omission. A guest asking for one of these fails at
load (an import outside `oag`) or faults at the command check.

| Not exposed | Why |
| --- | --- |
| **No WASI, no files, no network, no environment, no clock, no threads.** The linker holds one import namespace, `oag`, with the functions of `sim-host` or `hud-host` and nothing else | Determinism: a clock or a file is an input the replay does not carry (`docs/architecture/determinism.md`). Safety: a mod is read in place from an untrusted archive; it gets no reach past its own files. The browser cannot grant them anyway ([determinism-and-state.md](determinism-and-state.md#no-threads-no-clock-no-wasi)) |
| **No state in the guest's own memory across a tick.** One pooled instance per module per tick; the blob is the only thing that crosses a tick boundary | Network rollback restores the race state, not a guest's linear memory; the spike measured a guest with a counter in its own memory **diverging after a rollback** with a persistent instance ([determinism-and-state.md](determinism-and-state.md#mod-state-is-a-fixed-size-blob-in-the-race-state)). Enforced by construction rather than by asking authors to behave |
| **No raw `World`, `Ship` or `Body` pointer, no shared memory with the host, no reading what the observation did not copy** (another module's blob, the game's `Rng` state, a projectile's internals) | `World` is plain data whose layout changes with nearly every simulation commit ([ADR-0003](../adr/0003-no-ecs.md), [ADR-0055](../adr/0055-replays-are-inputs-and-a-ghost-is-poses.md)). A guest reading it by offset breaks on the next field; a guest writing it bypasses every check. Copies in, commands out |
| **No force, torque or position write; a velocity write only through the capped `impulse`**; no replacing the integrator | The force law is the measured original; a guest scales its inputs (`scale-thrust`), it does not step the body |
| **No same-tick reaction to an event raised after the call** | The one-tick latency is the price of one call per tick (section 2); a law that needs same-tick reaction is a measured original's and belongs in the engine |
| **No replacing a recovered law or a disc table**: not an original weapon's law, not a weapon pad's grant, not a handling value, not a pad volume or the spline | Never invent what the assets author (CLAUDE.md). A craft specialty is commands against the disc's own numbers, never a replacement `Handling`; a track's volumes are authored in the track file. A one-shot table rewrite is also the first thing a cheat mod would do |
| **No renderer, GPU, window, camera, draw or sound from the sim world** | The simulation must not know a renderer exists (CLAUDE.md, dependency rule 1). `message` and `publish` are outputs queued beside the cues ([ADR-0018](../adr/0018-audio-mixer-architecture.md)) and drained by the view; a mod-provided sound is open question 6 |
| **No input devices; only the `InputSnapshot` the tick already received, for person-flown slots** | The snapshot is the one thing a replay and a network peer carry (ADR-0055, [ADR-0070](../adr/0070-the-network-transport-is-webtransport.md) decision 4). Reading a pad directly would be an un-replayed input |
| **No reading or writing another module's blob, commands or published keys** | One instance and one blob per module; composition between mods is open question 8 and must not happen by accident |
| **No persistence from the sim or HUD world, and none of a guest's own files.** The one exception is the frontend world's saved blob, at most 4 KiB per mod and title, a host-written table beside the records (section 2.3) | Modern saves: no slots, no files a player manages; a mod must not leave state on disk the MODS page does not know about. The blob is the host's, listed and clearable on the MODS page. The sim guest sees it only as `setup.tune` at `race-start` and never writes it |
| **No change to an original title's faithful modes unless the player opts into the mod** | The MODS page's ENABLED switch is the opt-in (ADR-0078). With a sim module on, the session is MODDED (phase 2): records go to the modded table, ghosts and replays are off, the HUD says so. The original campaigns never load an extension (settled) |
| **No projectile of the mod's own in v1** | `World::projectiles` is a fixed-size pool of plain `Copy` data (`crates/gameplay/src/world.rs:378`); the Quake, Leach Beam and Repulser each have their own single-slot pool. A guest-defined projectile would change the snapshot's size and every golden hash. A custom weapon is a timed self-effect first (section 7.5); a projectile pool for mods, with a visual from the mod's files, is open question 4 |
| **No menu rows beyond ADR-0066's closed vocabulary, no settings, no new disc pages, no behaviour in a disc's own menus.** The one allowance is a mod's own screen in the frontend world: one extra action, `open-mod-screen`, and a page the mod owns (section 2.3, [ADR-0080](../adr/0080-a-wasm-mod-may-own-a-screen-and-save-a-small-blob.md), which supersedes ADR-0066 decision 4 for wasm screens only) | A mod still cannot invent front-end behaviour on the disc's pages; its screen is a page of its own that cannot start a race, change a setting or reach a file. The sim, craft, track, rules and weapon kinds still attach to a race, not to the menus |
| **No AI advantage.** A craft module runs for every slot flying its hull, the AI's included; `controller` is readable so a rules mod can handicap, but nothing in `oag-extras` ever favours the AI | The AI obeys the player's physics (maintainer's rule). A third-party mod that does otherwise is the player's choice and is MODDED |
| **No unbounded work**: fuel per call, a declared memory maximum, a call-depth cap, a command cap, an event cap, string caps | A tick has a budget on a Steam Deck and a phone; a guest that loops forever must be a named, deterministic fault, not a hang ([determinism-and-state.md](determinism-and-state.md#fuel-not-time)) |
| **No floating-point escape hatches**: no relaxed SIMD, no SIMD in v1, no threads, no shared memory, NaN canonicalised, a NaN in any reply a fault | The three-engine rule ([determinism-and-state.md](determinism-and-state.md#floating-point)) |
| **No access to the disc's archives, other mods' files or the asset layers** | A mod's module reads its own files, registered at load (`hud-host.texture`); the HUD sheet by name is the one exception, read-only. The disc is the player's; a mod never streams it |
| **No spawning or removing craft, no grid changes** | `MAX_SHIPS` is a real bound and the grid is placed once (`Race::start`); `race-start` sees the grid and cannot move it. Authored spawn points for an arena are an engine change in the track file first |

## 4. Sandboxing, limits, failure, versioning, trust

### 4.1 Engine configuration and the load-time rewrite

wasmtime 50 (the version the spike pinned; `just test-wasm` runs the simulation
suites under wasmtime already, `docs/tools/web.md`), configured so everything a
guest can observe is pinned. The browser has neither fuel nor NaN canonicalisation,
so both also run as **a rewrite of the module at load**, applied on every engine,
with wasmtime's own switches left on as a second line
([determinism-and-state.md](determinism-and-state.md#fuel-not-time)).

| Setting | Value | Why |
| --- | --- | --- |
| Fuel | on; a budget per call, set before `tick` (`Store::set_fuel`); plus the rewrite's own counter at function entries and loop headers, trapping at zero, identical on every engine; wasmtime's fuel a higher backstop | the only portable work bound; a wall-clock epoch traps at different instructions on different machines. The budget bounds loops and calls, not straight-line length (measured). The number is set with the first real hook; the spike's toy used about 168 units a tick |
| NaN canonicalisation | `cranelift_nan_canonicalization(true)`, plus the rewrite inserting the same canonicalisation after every float op | measured: on, every arithmetic NaN read `7fc00000` on both engines; off, `ffc00000` with a payload |
| Relaxed SIMD, threads, shared memory, memory64, SIMD | off / refused at load | the deterministic subset; `simd128` only after the conformance test passes with it |
| Memory | the manifest's `memory_pages`, the same `StoreLimits` on every engine, ceiling 256 pages (16 MiB, proposed) | `memory.grow` fails at the same size everywhere |
| Call depth | the rewrite's counter, trapping at a fixed depth well below any engine's native stack | a native stack overflow traps at different depths on Cranelift and Pulley; open risk until the pass exists |
| Instance | one pooled instance per module per tick (copy-on-write reset) | rollback safety; 11.4 us (Cranelift) / 27.3 us (Pulley) a tick for 20 calls against a 60 us race tick; one call per tick here |
| State blob | `state_bytes` from the manifest, capped by the host (a few hundred bytes per slot plus a global part covers every example here; the cap is set with the first sim hook) | a fixed-size field in the race state, no allocation |
| Module size | at most 4 MiB on disk (proposed) | a mod is small; a bigger module is a mistake |
| Compilation cache, parallel compilation | off | the host writes nothing a mod asked for; no thread pool in the simulation half |
| Features `wasi`, `async`, `profiling`, `coredump`, `addr2line`, `debug-builtins`, `component-model`, `threads` | off | not needed; `wasi` is the whole non-capability list; the ABI is core wasm |

### 4.2 Faults

- **One answer for every failure** ([hooks.md](hooks.md#the-shape-one-call-per-tick-events-in-commands-out)):
  a trap (out of fuel, `unreachable`, out-of-bounds memory, stack overflow) or a
  refused command **turns the module off for the rest of the race**. That call's
  commands are discarded, the blob keeps its value from before the call, every
  later call is skipped. The race goes on: a player two laps in keeps the race; the
  author gets the tick.
- **The fault edge is simulation state**: the fault flag and the tick are hashed;
  the fuel total is not (its per-operator costs are wasmtime's own and the browser
  has none). Same inputs, same fault tick on every machine: the spike's runaway loop
  trapped at the same iteration on Cranelift and Pulley.
- **Shown, not only logged**: the HUD's MODDED tag and the results screen name the
  mod and the tick.
- **A HUD module that faults** is hidden for the rest of the race; the title's own
  HUD is untouched.
- **A module that will not validate at load** (an import outside `oag`, a missing
  export, a refused feature, another `api` major) leaves its mod out of the race,
  named in the load report; the race runs vanilla.

### 4.3 Versioning

- The ABI is **major 1, edited in place until the first release** (pre-release
  rule: no migration code paths). A module built against a record layout the host no
  longer writes fails at load by name; that is the whole compatibility story before
  1.0. The manifest's `api` names the major. Within the major, every layout edit
  bumps `oag_ext_abi::REVISION`, which a module returns from `oag_abi` beside the
  major and the host checks by name (What is built, "And the SDK").
- After the first release: a new field at the end of a record, a new event variant
  or a new command is a compatible addition inside major 1 (a guest reads the prefix
  it knows and ignores an event it does not); a change to an existing field's
  meaning or position is major 2, a second layout the host writes beside the first
  for a release.
- The replay header's `mods` entry carries the mod's blob layout version, so a
  replay refuses a mod build whose blob layout changed.

### 4.4 Trust

- **Online play is server-authoritative** (networking decision 2): the server runs
  the same modules, accepts inputs only, and its hash per tick is the truth (ADR-0070
  decision 4). A client with a different module mispredicts and is corrected; it
  cannot cheat the result.
- **A LAN host is trusted** (decision 3); the joiner sees the content fingerprint
  and joins or not.
- **The lobby already refuses differing mods**: `Edition::content`
  (`crates/net/src/wire/online.rs:34`) and `content_refusal`
  (`crates/net/src/online/rooms.rs:288`, "your mods differ from the server's"), over
  `oag_net::discovery::content_fingerprint` (`crates/net/src/discovery.rs:168`),
  FNV-1a of `id@version` of the mods loaded (`crates/game/src/mods.rs:446`). **The
  gap**: two builds of one mod version would be admitted together and desync. Phase
  3 folds each sim module's SHA-256 into that fingerprint
  ([determinism-and-state.md](determinism-and-state.md#network-play)). Mod code
  arriving through a lobby is never auto-run (decision 6): the joiner must have the
  mod installed.
- **Replays**: `oag_replay::Header` (`crates/replay/src/header.rs`) has no mod field
  today and `build` is informational, never compared. Proposed: a dedicated `mods`
  list (id, version, module SHA-256, blob layout version), not the opaque `options`
  map, editing format v1 in place. Playback refuses a replay whose modules are not
  present with those hashes, by name; the stored state hashes still decide validity
  (ADR-0055). In phase 2 a modded session records no replay at all
  ([determinism-and-state.md](determinism-and-state.md#replays)).
- **Records**: a modded session's results go to the separate modded table (phase 2),
  keyed with the fingerprint; the networking page already says a modified-sim run
  goes to a separate board tagged with its fingerprint.
- **Rollback cost is the guest's cost times the window**: at most `MAX_WINDOW = 30`
  ticks, typically 7 to 11; estimated +19 % a tick on Cranelift and +45 % on Pulley
  with 20 guest calls a tick (desktop; a phone is open). One call per module per
  tick here keeps it at the low end.

### 4.5 With no mods loaded, nothing runs

*Proposed, not built.* The maintainer's question: does wasmtime run when no mod is
registered? **No.**

- **Without the `extensions` feature**, wasmtime is not linked at all, and the web
  build never links it (section 5): nothing to run.
- **With the feature on and no module loaded**, no wasmtime `Engine`, `Store` or
  instance exists. The host is built **lazily**, at the first load of a module by a
  race (a sim or HUD module of a mod that is on for it) or by a screen (the first
  `open-mod-screen`). Start-up, the main menu, the MODS page, reading a manifest and
  listing a mod's rows compile and instantiate nothing: a mod is discovered by its
  `oag-mod.toml`, and its `.wasm` is opened only for the load that needs it.
- **The per-tick hook iterates an empty list**: the `[ModSlot; MAX_SIM_MODS]` array
  has a count of zero, so there is no guest call, no observation built, no buffer
  copy, and nothing written for the hash
  ([determinism-and-state.md](determinism-and-state.md#the-state-hash)). A vanilla
  race tick costs what it does today, plus one load of a count. The same holds for
  a HUD frame with no widget and a menu with no mod screen.
- **The proof, owed with the host (rollout step 1)**: a test that a vanilla race
  built with `extensions` on produces the same state hashes as the feature-off build
  (the committed determinism references stay untouched), and that it creates no
  `Engine`, observed through a creation counter the host exposes to tests. A second
  test opens the main menu and the MODS page with a module-carrying mod installed and
  asserts the same counter is still zero.

## 5. Targets: the same bits everywhere

| Target | Engine | How the module gets in | Status |
| --- | --- | --- | --- |
| Linux, Windows, macOS | wasmtime 50, Cranelift | compiled at race load on the load thread | spike: Cranelift and Pulley bit-identical over a 36,000-tick race, fuel trap points included; the native Rust oracle of the same law gives the same hash ([determinism-and-state.md](determinism-and-state.md#the-spike)) |
| Android | wasmtime, Cranelift (aarch64) | same | same host code; the mobile glue is `oag-mobile` ([ADR-0065](../adr/0065-one-mobile-crate-for-every-phone-os.md)). **Owed**: the spike on an AArch64 machine, where the default NaN differs; the x86 run says nothing about it |
| iOS | wasmtime, Cranelift compiling to **Pulley** bytecode on the device (no native code), +11.4 MB; the runtime-and-Pulley-only build (+0.8 MB) needs precompiled `.cwasm` per wasmtime build, brittle | same | **not checked on iOS**: `cargo check --target aarch64-apple-ios` of wasmtime 50 stops in a build script needing Apple's `xcrun`, the macOS-only toolchain `just ios-app` needs; whether `signals_based_traps` must be off there is owed to a macOS machine ([determinism-and-state.md](determinism-and-state.md#web-and-ios)) |
| Web | the browser's own `WebAssembly`, through `js-sys`; **the web build never links wasmtime** | compiled asynchronously at race load, never per tick (browsers limit synchronous compiles on the main thread); the race load already runs on a Web Worker (`docs/tools/web.md`, "Threads"); per-tick calls copy the observation into the module's memory as on native | needs the load-time rewrite (fuel, NaN canonicalisation, call depth) before it can match; not run yet |

**Same bits everywhere means the guest's results, not the host's speed.** The
engines agree because the module is the same bytes after the rewrite, the imports
are the same two functions, the limits are the same numbers and the fault edge is
hashed. The simulation itself is already proven deterministic on `wasm32` (`just
test-wasm`, debug and release, 2026-10-09), so the open proof is the nested guest
in the browser and on AArch64, which the conformance test covers
([determinism-and-state.md](determinism-and-state.md#the-conformance-test)).

## 6. Licences

The workspace is `MIT OR Apache-2.0` (`Cargo.toml` `[workspace.package]`,
`LICENSE-MIT`, `LICENSE-APACHE`). There is no `deny.toml`; the policy in practice is
permissive dependencies only, and the lock file already carries `Apache-2.0 WITH
LLVM-exception` (`rustix`, `target-lexicon` under wgpu and quinn), `ISC` (`ring`) and
one weak copyleft, `MPL-2.0` (`symphonia`, file-level, in `oag-music`). Every proposed
dependency fits inside that:

| Crate | Licence | Role | Note |
| --- | --- | --- | --- |
| `wasmtime` 50 (with `cranelift`, `pulley`, `runtime`, `pooling-allocator`; without `wasi`, `cache`, `parallel-compilation`, `async`, `profiling`, `coredump`, `addr2line`, `debug-builtins`, `component-model`, `threads`) | Apache-2.0 WITH LLVM-exception (read off the crate's own `Cargo.toml` in the spike) | the engine | the LLVM exception only loosens Apache's attribution clause for binaries; compatible with both halves of ours |
| `cranelift-codegen` 0.137 and family, `pulley-interpreter` 50 | Apache-2.0 WITH LLVM-exception | wasmtime's backends | transitive |
| `wasmparser`, `wasm-encoder` | Apache-2.0 WITH LLVM-exception | the load-time rewrite (fuel counter, NaN canonicalisation, call depth) and the feature check before compiling | the same bytes go to wasmtime and to the browser |
| `wasmtime-wasi` | Apache-2.0 WITH LLVM-exception | **not used** | listed to say so: no WASI |
| `wit-bindgen`, `wasm-tools` | Apache-2.0 WITH LLVM-exception | **not a runtime dependency**: the WIT on this page is documentation; guests are plain `wasm32-unknown-unknown` Rust over the I/O buffer, as the spike's is | an example mod may generate its bindings with them at its own build time |
| `js-sys`, `web-sys`, `wasm-bindgen` | MIT OR Apache-2.0 | the web host | already in the tree for the web build (`crates/game/Cargo.toml`) |
| `sha2` | MIT OR Apache-2.0 | module hashes in the fingerprint and the replay header | or `ring`'s digest, already linked by `oag-net` |

Two wasmtime optional dependencies to keep **off**: `ittapi` (BSD-3-Clause OR GPL-2.0,
the VTune profiler hook, behind `profiling`) and anything behind `cache` (it writes
under the user's cache directory). A `deny.toml` allowlist (MIT, Apache-2.0 with and
without the LLVM exception, BSD-2/3, ISC, Zlib, Unicode, MPL-2.0 for the existing
case) is proposed as part of phase 1 so this table stays true without re-reading the
lock file (open question 7).

## 7. The maintainer's examples, end to end

Each walk names the extension kind, the manifest row, the events read, the
commands returned, what the host does with them, and what a player sees.
[hooks.md](hooks.md#the-maintainers-examples-mapped) says which of these the engine
can do today and which need an engine change first.

### 7.1 Feisar gets faster on each speed pad

The 2048 prototype idea: each team's fourth craft has a specialty
(`oag_2048::race::SHIP_TYPES`, `"prototype"` the fourth; `docs/formats/2048-campaign.md`
"A prototype craft is not a fifth, independent class"). **The specialties themselves
are not recovered** from 2048; on 2048 and Omega they are the original's laws and
belong to the engine once read, not to a mod, and a one-tick-late mod law would not
match them ([hooks.md](hooks.md#the-2048-prototype-craft)). A mod is how such a
specialty reaches a title that never had it, which is this walk. **Nearly on
today's engine**: `pending_thrust_scale` exists; the per-slot timer and a factor
above 1.0 are the engine change.

1. **Declaration**: `[[extension.craft]] team = "feisar", module = "ext/feisar.wasm"`,
   `state_bytes = 32` (a `u32` per slot), `simulation = true`.
2. **Load**: the race is on Pulse with a slot flying Feisar (or, in Race Remix, any
   title the mod is for); the module compiles on the load thread; `race-start` gets
   the grid and returns a zeroed blob. The session is MODDED.
3. **Tick N**: the hull enters a speed pad; the host raises `pad-crossed{slot,
   speedup, index}` at the entry edge (`Race::test_speedup_pads`,
   `crates/raceplay/src/pads.rs:73`, the edge at `:113`, for a person's craft and an
   AI's alike) into the event buffer.
4. **Tick N+1, the call**: the guest finds the event, increments the slot's counter
   in the blob, and returns `scale-thrust{slot, factor: min(1.0 + 0.02 * n, 1.5),
   ticks: 3600}` with the blob.
5. **Host**: checks the factor against the bound, starts the slot's timer, and each
   tick while it runs multiplies the factor into `pending_thrust_scale`
   (`world.rs:204`), which the step consumes (`player_step.rs:231`) into
   `Environment::thrust_scale` (`forces.rs:292`). The AI slot flying the same hull
   gets the same specialty: the AI obeys the player's physics.
6. **Player sees**: the pad boost as always, then a hull that keeps more speed on the
   straights from the next tick; the speed bar, which reads `speed_kmh` from the
   body's velocity (`crates/raceplay/src/telemetry.rs:137`), shows it with no HUD
   change. `publish{"feisar.pads", n}` lets a HUD module (7.6) show the counter.
7. **Replay and records**: phase 2, none recorded; MODDED on the results. Phase 3: the
   module's hash in the header and the lobby.

### 7.2 A spring jump on a custom track

**Needs the engine change** in [hooks.md](hooks.md#event-sources): the volume node
kind and the velocity write point.

1. **Track**: the author tags a Blender object as a volume; the add-on writes
   `oag/volumes/spring-a` into the glb (the pad's box shape), and the manifest names
   `extension = "spring-loop.wasm"`.
2. **Load**: `setup.volumes = ["spring-a"]`. A build without the `extensions`
   feature reports the node unread and races the track flat.
3. **Tick N**: the host tests every active hull against the volumes in the pad sweep
   (`Race::pad_sweep`, `pads.rs:45`), after the weapon pads, and raises
   `volume-entered{slot, 0, "spring-a"}` on the entry edge.
4. **Tick N+1, the call**: the guest returns `impulse{slot, delta-velocity: up *
   18.0 + forward * 4.0}` (its own numbers).
5. **Host**: refuses a non-finite or over-cap delta (a fault), else adds it to the
   body between the craft's controls and its step, where `oag_physics::rail::apply`
   already writes one (`player_step.rs:256`), not through `pending_impulse` (that
   port arms the collision stun). The craft leaves the floor one tick after the
   volume's edge, a 17 ms offset the author places the volume to absorb; `grounded`
   falls, `time_airborne` runs, the airborne pitch law and the landing behave as
   after a disc jump (the engine already has `Handling::speedpad_jump` and
   `turbo_jump` as upward kicks, `crates/physics/src/params.rs:346-349`). The
   off-track rescue still applies: a jump into the void respawns as any other.
6. **Player sees**: the jump, with the camera and shake the engine already does on
   landing. No sound in v1 (open question 6).

### 7.3 A gravity switch to the underside

Two facts first. **Gravity in this engine acts on world `-Y` only**
(`oag_physics::passive::gravity`, `crates/physics/src/passive.rs:309`;
`crates/physics/src/forces.rs:221-222`: "Gravity still acts on world `.y` only;
neither is a gravity direction"), and **the original's way of racing an inverted
section is the magstrip**, a kinematic attitude hold that fades the hover out and
slaves the hull to the surface normal (`crates/physics/src/maglock.rs`, confidence 90
off `Ship_UpdateMagLock`; `docs/physics/cornering-ground-truth.md` measures a Talon's
Junction inverted section). So there are two routes, and the walk takes both:

1. **The faithful route, no code**: the underside is authored as `mag_floor`
   collision (`oag_kind = "mag_floor"`, `docs/formats/track-file.md`), exactly as a
   disc circuit authors a magstrip. A craft that drives onto it is held through the
   inversion; the hover already casts along the craft's own `-up`
   (`crates/physics/src/hover.rs:610`). This works on a track file today and is what
   a continuous top-to-underside loop should be built from.
2. **The switch moment** is what the extension adds today: a volume at the lip
   (`volume-entered`), and the guest returns `hold-mag-lock{slot, ticks: 120}` so the
   hold engages before the hull has a `mag_floor` sample under it (the blend ramps at
   `0.2` a frame, `maglock.rs`), and, the author choosing, an `impulse` toward the
   surface. The hold releases when `ticks` run out and no magnetic floor is under
   the craft.
3. **The switch the maintainer described** (gravity itself turns, and the craft
   falls to the underside) is **an engine change**, mapped in
   [hooks.md](hooks.md#the-maintainers-examples-mapped): a gravity direction in
   `Environment` defaulting to `-Y` (bit-identical for every race that does not set
   it), per-craft gravity state on `Ship`, and a `set-gravity{slot, direction}`
   command from the same event with a unit direction. The craft side then follows
   (hover and maglock already work in the craft's own frame); the camera and the
   AI's racing line do not, and need their own check before the command is offered
   (open question 3).
4. **Player sees**: routes 1 and 2 today, the hull rolling onto the underside with
   the maglock's own attitude blend; route 3 after the engine change.

### 7.4 A to B instead of a circuit

The thread's order stands: rules as data first, open splines later. The extension's
part is the piece data cannot say. **Partly on today's engine**: a sprint over part
of a closed circuit needs only `set-finished`; a truly open track does not fit the
loop every lap structure assumes ([hooks.md](hooks.md#the-maintainers-examples-mapped)).

1. **Mode**: `modes/a-to-b.toml` with `finish = "mod"` (the rules-as-data step; it
   passes `laps_target: None` to `RaceState` and `Standing`, which already means
   "never ends on its own", `crates/race/src/state.rs:154`) and a grid at a sample
   other than the lap line. A plain `finish = "distance"` needs no module at all.
2. **What the module adds**: the finish itself, checkpoints with time bonuses, or
   "last place every 30 s is out". Each tick the guest reads every slot's `progress`
   and `race-ticks`; it returns `set-finished{slot}` when a slot's progress first
   reaches the finish sample (reading `progress` before the course wraps it,
   `wrapped_forward`, `state.rs:35`), `set-standing-key{slot, key}` for a ranking of
   its own, and `publish{"a_to_b.remaining_m", f32}` for the HUD.
3. **Host**: `set-finished` sets `Standing::finish_tick` for every craft and
   `RaceState::finished` for a person's slot at phases 8 and 9 of the same tick,
   once; `set-standing-key` feeds the key `oag_race::places` sorts by (an engine
   change: `places` takes standings and the course only today). The results screen,
   the records key per mode and the replay header's mode are the mode file's.
4. **An open point-to-point track** stays "later" (the spline, the spawner, the
   rescue, the AI plan and the flyby all assume a loop); the ABI does not change when
   it lands, only the track format does.

### 7.5 A custom weapon

**Needs the engine change** in [hooks.md](hooks.md#command-sinks):
`oag_tables::weapons::Weapon` (`crates/tables/src/weapons.rs:91`) is a closed enum of
the titles' weapons, so a mod weapon needs a custom arm, a pickup icon from the mod's
file, a row in `docs/gameplay/weapon-lineage.md` (a new pickup is a lineage row, the
member rule), and the network weapons path of
[ADR-0071](../adr/0071-weapons-are-predicted-simulation-with-a-discrete-state-correction-trigger.md)
treating it like any other. The pad roll (`oag_weapons::pickup::Driver`,
`crates/weapons/src/pickup.rs:116`) gains the mod's ids with the mod's weight in the
modes the mod is on. **Timed self-effects first** (a Turbo-like boost, a shield),
the easy case; a projectile of the mod's own is open question 4.

1. **Declaration**: `[[extension.weapon]] id = "emp", icon = "ext/emp.png",
   name_string_id = "WEAPON_EMP", module = "ext/emp.wasm"`.
2. **Tick N**: a person spends the pickup (`Race::spend_pickup`,
   `crates/raceplay/src/weapons.rs:148`, at `tick.rs:168`, before the call), so
   `weapon-fired{slot, custom, "emp"}` is in **this** tick's observation; an AI
   craft's fire (`spend_opponent_pickup`) happens after the call and arrives a tick
   later.
3. **The call**: the guest reads `obs.crafts`, picks every other craft within 30
   units, and returns `scale-thrust{target, factor: 0.5, ticks: 120}` and
   `shield-delta{target, amount: -5.0}` for each, `message{slot, "EMP_HIT"}` for the
   person-flown ones, and `clear-pickup{slot}`.
4. **Host**: checks and applies before the step; the damage path raises the shield
   flash and the absorb window as a hit does, so the victim's HUD reacts; the
   victims' modules see `weapon-hit` only if the host raises it for a scripted hit
   (open question 5).
5. **What a scripted weapon cannot do**: fly a projectile, draw a beam, play the
   disc's weapon effects. Those are `oag_fx` visuals bound to the fixed kinds. An
   **AI craft holding a mod weapon** is open in [hooks.md](hooks.md#event-sources):
   our driver's weapon decision knows the titles' weapons only.

### 7.6 A HUD extension

**Mostly on today's engine**: needs the HUD module host and the published-keys
channel; the HUD layout files are planned separately.

1. **Declaration**: `[[extension.hud]] id = "lap-delta", module = "ext/lap_delta.wasm",
   layout = "hud/lap-delta.toml"`; the layout patch declares `[[widget]] id =
   "lap-delta", rect = { pulse = [300, 20, 160, 60], hd = [1480, 60, 400, 150] }`
   in each title layout's own grid, inside the safe area.
2. **Each frame**: `frame(readout, box, alpha, dt)` gets the slot's `Readout` copy
   (`lap-splits`, `best-lap-ticks`, `lap-ticks`) and this mod's published keys; it
   returns `fill` and `text` draws inside `box` (a bar per completed lap scaled by
   its split against the best). No simulation state is read or written; the widget
   runs in the HUD pass on the render side and is absent from every hash.
3. **Host**: appends the draws to the `Frame`'s lists after the title's own
   (`HudOverlay::draw`, `crates/game/src/hud_overlay.rs:335`), scissored to `box`,
   and the overlay rasterises them as it does every `Draw::Fill` and `Draw::Text`.
4. **Player sees**: the widget where the patch put it, on the titles the mod is for,
   gone when the mod is off. The vanilla layout is the default; a HUD mod is opt-in
   (the thread's 1c.4).

## 8. Open questions

1. **The web proof**: the load-time rewrite (fuel counter, NaN canonicalisation,
   call depth) is designed, not built; the conformance test's headless-browser run
   (`wasm-bindgen-test`) is what shows the browser matches.
2. **The fuel budget** per call, set with the first real hook; and whether the HUD
   world gets fuel or a wall-clock deadline (it is not deterministic, so a deadline
   is allowed there). The frontend world follows the answer; fuel is recommended for
   both (section 2.3).
3. **A per-craft gravity direction** in `Environment` (7.3): a measured default of
   `-Y` proven bit-identical for every race that does not set it, then the camera and
   the AI line checked before a `set-gravity` command is offered.
4. **A projectile pool for mods**: a second fixed-size pool outside
   `World::projectiles` so the vanilla snapshot's size is unchanged, with a visual
   from the mod's files, or a disc weapon's visuals the mod names without a stand-in
   for what the disc authors.
5. **Event granularity**: `pad-crossed` on entry only (the edge that exists today)
   is settled; `wall-contact` per contact edge; a scripted hit
   (`shield-delta` from a mod weapon) raises no `hit` for the victim's modules: `hit` is
   a weapon's, settled with the armour fixture.
6. **Sound**: a cue command naming a disc cue by id, or a cue from the mod's own WAV;
   both need the audio architecture (ADR-0018) to accept a mod-provided bank.
7. **A `deny.toml`** allowlist for the workspace, so section 6 is checked by the gate.
8. **Composition**: whether a rules module may read a craft module's published keys,
   and in what order.
9. **Persistence between races**: whether the blob is saved per mod and title (the
   proposal in section 3), and what a mod may do with it (an unlock, a best score)
   without a save slot.
10. **The pilot-controller thread's three questions** (may a pilot fly `"original"`
    outside the campaign; mod controllers in multiplayer; WebAssembly as the mod
    ABI): this page assumes yes to the third and leaves the first two to the
    maintainer.
11. **Hot reload** of a module during a race for mod authors (a `--reload-extensions`
    debug flag): useful, and incompatible with the hash; debug builds only, if at all.
12. **iOS and AArch64**: the spike on an AArch64 machine and the wasmtime build on
    iOS (`xcrun`), both owed to hardware this machine does not have.
13. **A per-seat tune on the wire** (section 2.3): the server receives each seat's
    tune bytes with the seat at race start, so players may differ; what that adds to
    ADR-0070's messages and to the content fingerprint (a session-shared tune folds
    in, a per-seat one cannot). Until answered, a tune is single-player or
    session-shared.
14. **An animated canvas**: whether the frontend world gets a per-frame `frame` export
    (like the HUD world) for a preview that moves, and the cost of calling a guest
    every frame while a menu is open.
15. **What disc data a screen may read**: the `teams` list is the one copy it gets. A
    tune editor showing stock handling beside a slider needs a read-only copy of a
    table; which tables, and whether a disc table may reach a mod at all (the HUD
    sheet by name is the existing exception).
16. **Blob lifetime**: whether uninstalling a mod deletes its blob, whether the
    MODS page's CLEAR is the only way, and whether a title's blob follows a mod to
    another title (today: keyed by mod id and title, no).
17. **A screen during a race**: a pause-menu page or a post-race page of the mod's.
    Excluded here (the frontend world never runs with a race live); the end-of-race
    results screen is the likelier first ask.
18. **A third-party sim guest reading a tune it did not author**: the manifest ties
    a blob to its own mod's module only; whether a craft-tune editor and a separate
    craft mod may be different mods is the composition question 8.

## 9. Phased rollout

In the thread's order, with the extension kinds slotted in; each step is its own
merge and each engine change carries its own proof that a vanilla race's hashes do
not move ([hooks.md](hooks.md#what-has-to-be-built-in-order)):

1. **The host itself** (`oag-modhost`: the engine configuration of section 4.1, the
   I/O buffer ABI, fuel, the fault path, the pooled per-tick instance, the load-time
   checks), no call site wired, behind the `extensions` feature, built lazily at the
   first module load (section 4.5), with the no-`Engine` test and the
   vanilla-hashes-unchanged test. The ADR accepted; the spike's result on this page.
2. **Phase 1, no simulation code**: the `hud` world, `[[extension.hud]]` and the
   `hud/*.toml` widget box, one example widget (the lap-delta graph) in
   `mod-examples`. Proves the host on low-stakes code.
2b. **The frontend world** (ADR-0080), a second harmless-half step beside the HUD
   world and needing no simulation code: the `open-mod-screen` action and
   `[[extension.screen]]`, the host widgets and canvas over `oag-ui-screens`'s model
   (pad and pointer from the first change), the 4 KiB blob in the records `Store`,
   and one example screen that saves a blob nothing reads. It lands after step 2 (the
   host and the HUD world prove the engine) and before phase 3. A screen whose blob a
   sim module consumes (a craft tune) waits for step 4; the `setup.tune` field is
   added with it.
3. **Phase 2, the simulation call**: call site C, the event buffer, the state blob in
   the race state and the hash, and the `assist` command, proven by moving
   Pulse/Pure's Pilot Assist backport into a module with bit-identical replay hashes
   (`pilot_assist_ground_truth.rs`); `simulation = true` and the MODDED session.
4. **The craft kind**: `scale-thrust` with its timer, `pad-crossed`, then the Feisar
   example (7.1), the first player-facing use.
5. **Rules as data** in `oag-race`, then `set-finished` and `set-standing-key`
   (7.4); the drag race as the first mod mode, no module needed.
6. **Engine changes, each its own merge**: track volumes and `volume-entered`, the
   velocity write for `impulse` (7.2); a gravity direction (7.3); a custom `Weapon`
   arm and `give-pickup` (7.5); authored spawn points; the `Controller` trait for a
   pilot controller (the pilot-controller thread).
7. **Phase 3**: module hashes in the replay header's `mods` list and the lobby
   fingerprint, playback and join refusing a mismatch by name, the conformance test
   across Cranelift, Pulley and the browser, `oag-server` running sim modules;
   MODDED stops meaning "no replay, no multiplayer" and starts meaning "with these
   modules". The tune bytes join the replay header's `mods` entry, and a
   session-shared tune joins the lobby fingerprint (section 2.3; a per-seat tune is
   open question 13).
