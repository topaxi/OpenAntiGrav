---
status: ready-to-wire
titles: [all]
updated: 2026-11-04
summary: "No rumble exists. RE done for HD, Omega, PS2; proposal: one 20-voice pattern player, cues in, motors out; seven decisions for the maintainer."
categories: [tooling]
size: L (60)
difficulty: medium (50)
---

# Controller rumble: one runtime for every title

OpenAntiGrav has no rumble. The reading is done:
[gameplay/rumble.md](../../docs/gameplay/rumble.md) (playback law, triggers,
provenance) and [formats/rumble-xml.md](../../docs/formats/rumble-xml.md) (file
format, census). Evidence pages: HD, Omega and PS2 `rumble.md`. Read those first;
none of it is repeated here.

Maintainer's instructions for this thread (2026-11-04): carry the patterns to
Pulse (PSP and PS2) and Pure **as our own**; consider extra patterns for mod
support, perhaps on the slipstream branch; propose a unified handling across all
titles, with implementation detail and alternatives.

## Decided

Maintainer, 2026-11-04, replacing decisions 1 to 3 below: a `RUMBLE` setting with the
values `None`, `Original`, `PS3`, `PS4`. The default is the newest law, `PS4`. A title
whose original has no pattern rumble (Pulse PSP, Pulse PS2, Pure, 2048) does not show
`Original`. `PS3` is HD's small motor (any non-zero is full on), `PS4` is Omega's (the
byte sum). Pulse PS2 takes the same rumble as Pulse PSP, our own authored patterns in
the fallback layer; the PS2 build's own level and timer law stays documented and is not
played, so the `Ship_Damage` unit below no longer blocks anything.

Open under it: whether the row keeps `Original` where it equals a named value (HD and
`PS3`, Omega and `PS4`).

## Open

Decisions, each with a recommendation. The first two change what gets built.

1. **Small motor on Omega.** HD: any non-zero means full on. Omega: sums to 1..5 of
   255, close to nothing. Recommend HD's law everywhere, Omega labelled as a
   departure: the patterns were authored for an on/off motor, and a measured
   near-silence helps nobody. Alternative: keep Omega's `Sum` as the `original`
   setting value.
2. **What Pulse and Pure get.** See "Backporting" below. Recommend own authored
   patterns, default on, with PS2's law as the `original` value on Pulse PS2.
3. **`RUMBLE` setting** (none in HD or Omega, `VibOnOff` on PS2). Recommend
   `OFF` / `ORIGINAL` / `FULL` where the original has any, `OFF` / `FULL` on PSP
   titles and 2048. `FULL` adds the Omega impacts to HD and fires the loaded, never
   fired patterns. Also an intensity scale, for accessibility.
4. **2048.** The original does nothing, but a PC has motors. Inherit Omega's table
   (lineage rule), labelled inherited.
5. **Device layer.** See "Device output". Recommend a thin own backend trait with
   gilrs first, because it needs no new dependency.
6. **Positional patterns in one player.** The original drops the position outside
   split screen, so wall hits and explosions (48, 49, 50, 51, 53) are near silent on
   the large motor ([the law](../../docs/gameplay/rumble.md#playback-law)). Recommend
   the evident intent under `FULL` (pass the position), the measured silence under
   `ORIGINAL`.
7. **Remote and bot craft.** The originals rumble the local player only. Keep that.
   Positional patterns measure from each local pad's own ship; the original uses
   one active ship (chosen for split screen).

RE left open, none blocking (the first two need an emulator, off for this pass):

- PS2 `Ship_Damage`'s damage unit decides whether `damage * 1800` clamps on nearly
  every hit. Measure on PCSX2 before setting PS2's `ORIGINAL`.
- Live watch on Omega's or HD's five voices: raises the laws above 84 and settles
  the "never fired" list (16, 26, 45, 46, 52, 54, 55, 57, 58, 61).
- `m` in the contact law: `ship+0x64d4` (Omega), `ship+0x5f78` (HD), its unit and
  who writes it. Map it to our contact response; a value in the bands' 2 to 55 range
  should come from the wall contact's impact measure.
- Omega `*(race + 0x19b20) == 2` gates impacts and side shifts: decides whether
  they fire in a normal race.
- Landing `v`: its sign and unit.
- HD's positional gain: the decompiler truncates it (`0x00340a88`). Omega's law
  stands, inherited.
- Omega `FUN_012ff700`: which case fires 59 and which 60.
- HD `heartbeat`, `FUN_00055848`: what `+0x1960`, `+0x1964` count.
- 2048: `SceCtrl_A497B150` and `SceCtrl_67E7AB83`'s NIDs resolve to no name locally.
  The likely names are a sampling-mode setter and a buffer read; a NID table settles
  it.

## Proposal: one runtime

```text
simulation                 playing side                         device
  CueEvent (edges)   --->  RumbleMap: event -> pattern    --->  Player per pad
  held set (levels)  --->    per title, moddable                  20 voices, mix 5
                                                                  large u8, small bool
                                                                --->  Motors trait
```

**One player, data per title.** The 20-voice, five-mix player from the law is the
engine. A title supplies a `PatternLibrary` and a `RumbleMap`. Nothing branches on
the title (ADR-0058).

| Title | Library | Map |
| --- | --- | --- |
| HD | disc XML | measured table |
| Omega | disc XML | measured table, with impacts and explosions |
| 2048 | disc XML (ships, unread by the original) | inherited from Omega |
| Pulse PS2 | ours; `ORIGINAL`: synthesised from the PS2 law | measured: damage, destroy, preview |
| Pulse PSP | ours, same as PS2 | ours, same as PS2 |
| Pure | ours | mirrors Pulse |

**PS2's law fits as data.** A request `(duration, strength)` becomes a flat pattern
of `duration * 60` frames with strength on the large track, 1 on the small track at
240 or more, and four times the length. The one-at-a-time rule and the 0.3 s gate
become two player settings: `max_voices` (20, or 1) and `min_gap` (0, or 18 ticks).

**Alternatives for the runtime.**

- *Two players, patterns and level-and-timer.* Simpler each, but two code paths and
  PS2's law is the one with least use. Rejected.
- *Lift the 20 and 5 entirely and sum everything.* Would feel the same in nearly
  all play: five voices rarely overflow. Keep the law; the code is no longer and
  overlap in a pile-up is where it differs.
- *Bake every pattern to one curve per event, at load.* Drops the mixing, so holds
  and overlaps cannot work. Rejected.

### Reader

`oag_tables::rumble` (zero dependencies, fits the crate): `Pattern { large, small,
looping, loop_start, loop_end, class, priority, positional, inner, outer }`, parsed
with the crate's XML reader. Tokenise plainly, so a missing trailing space costs no
digit. Do not read `FPS`, or read it only to refuse a value other than 60 in a mod.

`PatternId` is an enum over 0..=64 with id 11 absent, or a `u8` with named consts.
Paths are title data: `data/xml/rumble/` differs in case across the three titles.
HD's and Omega's archive lookups are by digest of the exact path, so use the
manifest spellings on the format page's census, and load the 64 the original loads.

Ground truth: add to an existing walker (`rg` for the PSARC walk in
`crates/*/tests/`): 69 files per title, `RumbleData` identical across the three, 64
named paths resolve.

### Events out of the simulation (ADR-0018)

Rumble is an output, never `World` state.

**Edges reuse `CueEvent`.** `oag_gameplay::cue` already raises most of the triggers:
weapon fired, plasma and rocket wall hits, absorbs, pads. `drain_cues` is a
`mem::take`, so the composition root drains once and fans the list out to sound and
rumble. That is the reason not to add a
second event stream. New `Cue` variants cover only what has no sound: side shift
if unvoiced, landing band, wrong-way edge. Check each against `Cue` first.

**Holds are levels, not edges.** A per-tick `held: RumbleHeld` bitset on the tick's
output: mag strip, Autopilot, Leech beam, one contact band and side. The player
starts a voice when a bit rises and finishes or stops it when it falls: mag and
Autopilot finish, contact stops, as on the original. A cue pair (start, stop) would
leak a running motor across a replay seek, rollback or restart; a level cannot,
since it is recomputed each tick. Same reasoning as a looping sound's owner, stronger
here because a stuck motor is physical.

The contact and landing selector is pure and small: `(m, side, v, dt, state)` gives
a band. Its timers (0.1, 0.3, 0.4, 1.5, 1.0 s) can run on the playing side, fed by
the level, so no simulation state grows. Run it in `oag-gameplay` only if a test
needs it headless; then it is still an output.

**Alternatives for the events.**

- *A dedicated `RumbleEvent` stream from the simulation.* Clean names, but it repeats
  every trigger and costs each weapon site a second line.
- *Derive everything on the playing side from state deltas*: no simulation change,
  like a HUD reading a `Readout`. Works for mag strip and Autopilot; fails for one-
  frame events. Good for the held set, so use it there if a field already says so.
- *Rumble from audio envelopes.* Cheap and universal, and wrong: it ignores 64
  authored patterns.

### Player

A pure, deterministic struct in `oag-input` (`rumble.rs`), which already owns devices
and is outside every gameplay crate, so rule 1 and rule 6 hold. A new crate is
justified only if the player gets tests heavy enough to want its own binary. It
needs `oag-tables`.

```rust
pub struct Player { voices: [Voice; 20], cursor: u8, gate: u16 }
impl Player {
    pub fn one_shot(&mut self, p: PatternId, at: Option<Vec3>);
    pub fn hold(&mut self, p: PatternId) -> Option<Handle>;
    pub fn finish(&mut self, h: Handle);   // release flag set
    pub fn stop(&mut self, h: Handle);     // silent at once
    pub fn step(&mut self, ctx: StepContext) -> Motors; // class mask, paused, listener
}
pub struct Motors { pub large: u8, pub small: bool }
```

Step on the race's tick, 60 Hz (the shipped tracks' rate), so a catch-up frame steps
several times and the device gets only the last value. In a menu, step at 60 Hz
from the front end's clock. A `Handle` carries a generation, so a stale handle
cannot stop a reused voice. One `Player` per local seat; the seat already knows its
pad (`oag_input::lobby`).

Unit tests that fail if the law is dropped: release flag against `Looping`; track
lengths differing; ring full drops; six voices overwriting the lowest; positional
gain at 10, 15, 20; class mask and pause; a level dropping during a seek leaves
zero voices.

### Device output

```rust
pub trait MotorOutput { fn set(&mut self, seat: Seat, m: Motors); fn silence(&mut self); }
```

`silence` on pause exit to the desktop, focus loss, disconnect, race teardown and
panic hook.

| Option | Coverage | For | Against |
| --- | --- | --- | --- |
| **gilrs `ff`** (0.11.2, in tree) | Linux evdev, Windows XInput and WGI. **Stubs on macOS, wasm, others** (`is_ff_supported` false; read in `gilrs-core` 0.6.8) | no new dependency; Deck runs Linux | its server ticks at **50 ms**, so a 60 Hz envelope lands at 20 Hz; effects are built objects, not a level |
| `gilrs-core` direct, `FfDevice::set_ff_state(strong, weak, duration)` | same | a plain level at any rate, which is exactly `Motors` | `Gilrs` hides its inner core, so a second device open, or a patch |
| **SDL3 gamepad** (`SDL_RumbleGamepad`) | all desktop, Android, iOS; DualSense and Switch by HID | one call, a level, widest coverage, adds trigger rumble | a C dependency for one feature, or replacing gilrs |
| `hidapi` per device | anything | exact | a per-controller maintenance burden |
| Android `VibratorManager`, wasm `Gamepad.vibrationActuator` | phone, browser | fills gilrs's gaps | per platform, later |

**Recommendation.** `MotorOutput` first, gilrs effects behind it: two looping effects
(one `Strong`, one `Weak`), each `set_gain` per update. Accept 20 Hz at first, then
judge on a real pad whether the 13-frame patterns read as authored; if not, move to
`gilrs-core` direct or SDL. The trait keeps that a one-file change. Nothing here can
be judged headless: a test on the Deck or the S24 goes through their own rules.

Scale: `large as u16 * 257` to strong, `small as u16 * u16::MAX` to weak. Sony's
large and small map to an Xbox pad's low and high motors. Do not mix in the triggers'
impulse motors.

### Faithfulness labels

`Provenance` per title on the map: measured for HD, Omega, PS2; **inherited** for
2048; **chosen** for PSP titles. A `FULL` row on HD that fires an Omega impact is
labelled borrowed, the wording split screen uses.

## Backporting to Pulse and Pure

**The disc's patterns cannot be committed**: no game content in the repository, so
HD's 69 cannot simply be copied to Pulse, and a title run with no HD disc has none.
Three routes:

1. **Own authored patterns in the fallback layer** (ADR-0061): our own XML, in the
   same format, with the same 64 ids, written from the event table and not copied.
   Works for every player, fits "own art, fallback layer". Costs authoring, about a
   day for a useful 25. **Recommended.** Same files on PSP and PS2 pressings, one
   definition.
2. **Synthesised from PS2's law only.** No authoring, faithful, and thin: damage,
   destruction. Ships as `ORIGINAL` on Pulse PS2 regardless.
3. **Read HD's disc when present.** Faithful data, but a per-install difference,
   against how a title stands alone. Rejected.

Pure mirrors Pulse (lineage). The map maps our events: Pure's and Pulse's weapons
overlap most of HD's list. Id gaps stay silent, and the mapping names which it
fires. Pulse's Leech, Quake and mag strip map directly. Anything new goes on
`docs/gameplay/weapon-lineage.md` if a weapon row is touched.

Author by shape, not by copying: one-shots of 6 to 30 frames at 60 Hz, a held loop of
20 to 40; the format page's table gives the scale, and the small motor is a short
accent over a hit. Keep the tracks' trailing space or our reader's plain tokeniser
makes it moot.

## Extra patterns for mods

- **Override**: a mod's `Data/XML/Rumble/<name>.xml` overrides by the override layer
  (ADR-0061), so a mod changes a feel with no code.
- **New**: a mod declares a pattern by name in `oag-mod.toml` or by path, and gets a
  `PatternId` above 64. The library is a `Vec`, not a fixed 65.
- **Remap**: `RumbleMap` as TOML in the fallback layer, schema in `schemas/v1/`
  (ADR-0064), a mod replaces a row.
- **Fire one**: a guest output record, `rumble(pattern, seat)`, through `oag-ext-abi`.
  **It changes the ABI, so the same lane updates the `oag-mod-author` skill** and
  `oag-ext`. No revision bump pre-release.
- **Scope**: rumble is cosmetic, so a pattern never enters the state hash or the LAN
  fingerprint. It belongs with the `view` picks (ADR for query companions open), and
  a view output must not need a simulation reply.

**Slipstream** lives on its branch: if the player reaches `main` first, the Slipstream
mod gets three patterns there: drafting held (a low `travel_mag`-shaped loop), a
portal transit one-shot, and a release. Its lane wires them once the ABI record
exists. Do not hold the engine lane for it.

## Next Steps

Each is one lane, in order.

1. **Reader and census** (S, 80): `oag_tables::rumble`, ground truth. No behaviour.
2. **Player and device** (M, 60): `Player`, `MotorOutput`, gilrs backend, settings
   row with string ids and pointer support; a debug way to play a pattern by id.
3. **Wire HD, carry to Omega and 2048** (L, 50): the edge map, then held levels, then
   the contact and landing selector. 2048 by inheritance; check against Omega in the
   same change.
4. **Pulse and Pure** (M, 50): own patterns in the fallback layer, the PS2 law as
   `ORIGINAL`. Measure the damage unit first.
5. **Mods** (S, 60): override and remap, then the ABI record and the skill.
