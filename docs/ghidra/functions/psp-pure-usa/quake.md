# Pure's Quake wave: its effect, its frame, its wash - and no LeachBeam

**Binary:** `psp-pure-usa` `BOOT.BIN`, image base `0x08804000`, Ghidra
`/pure/BOOT-psp-pure-usa.BIN`. Read 2026-10-12 (lane `pure-leach-quake`), against
PPSSPP 1.20.4 software renderer running Pure USA (UCUS-98612). Companion to Pulse's
[cannon-quake-leachbeam.md](../psp-pulse-usa/cannon-quake-leachbeam.md) (the same
machine, the template) and HD's [quake.md](../ps3-hdfury-eu/quake.md). The LeachBeam
half of the lane (Pure ships none) is in [weapons.md](weapons.md), "Pure ships no
LeachBeam".

Disassembly was read in Ghidra (`disassemble_function`), decompiles are Ghidra's.
Addresses in `jal` operands are image-base-relative: `jal 0x00137e60` is `0x0893be60`.
Decompile-and-disassembly-level readings, so 85 is the ceiling.

## Functions

| Address | Name | Confidence | What it is |
| --- | --- | ---: | --- |
| `0x08879d54` | `Quake_Update` | 85 | The per-frame advance of the wave's effect: the structural twin of Pulse's `Quake_Update` (`0x0891d268`). Two instances (`slti s1, 2`), each spawning `WO_QUAKE` once (`'QUAK'`, `0x4b415551`) and a `Sound_Play` object with `+0x28 = 600.0`, then re-framing every tick. |
| `0x08879020` | `Quake_SampleSpan` | 75 | `(span index, &A, &B, ...)`, returns nonzero while the wave is on the road; the same call shape as Pulse's `0x0891c028`. Not read inside. |
| `0x0882e648` | `ScreenFlash_Start` | 80 | `(node, kind, &position)`: writes `kind` at `+0x50`, the position at `+0x70`, raises `+0x1b0`, and jumps through the 10-entry table at `0x08a46a78` (`0x242a78` unrelocated). Pulse's (`0x088f00c0`) has twelve kinds and a different node layout. |
| `0x0893bf08` | `ParticleSystem_SetScaleParams` | 80 | Writes the seven-word block into the instance at `+0x28`..`+0x40`, re-derives the scaled parameters (`0x0893c3cc`), raises `+0x160 |= 0x10000` and recurses into the children at `+0x1a8` and `+0x1a4`. Instruction for instruction Pulse's `0x088f44d8`. |

Two readers of the same block are not named: `0x0893be60` (copies the seven words out of
the instance, Pulse's unanalysed `0x088f443c`) and `0x0887862c` (the Gram-Schmidt below).

## What it shares with Pulse

- **The extent co-factor.** `Quake_Update` copies the instance's block out (`0x0893be60`),
  sets slot 1 (`swc1 f12, 0x164(sp)`) to `|A - B| / 50.0` (`div.s f12, f12, f22` with
  `f22 = 0x42480000`), and writes it back (`0x0893bf08`): instance `+0x2c`, the
  **emitter-extent co-factor**, and nothing else. Same law, same constant as Pulse's
  (`particle-system.md`, "the Quake stretches its emitter").
- **The frame's `X`** is `normalize(B - A)` (`vdot.t`, `vrsq.s`, `vscl.q` at
  `0x0887a540`-`0x0887a54c`), and the rows are re-orthonormalised by `0x0887862c`: row 2
  normalised, row 1 made perpendicular to it and normalised, row 0 `= row1 x row2`. That
  is the Gram-Schmidt Pulse inlines at `0x0891db24`-`0x0891dbc0`, and
  `oag_fx::psys::spawn::frame_x` builds it from `(across, up)`.
- **The position** is `(A + B) * 0.5` with `w = 1` (`0x0887a4e0`-`0x0887a524`).
- **The sound.** A `0x50`-byte object with `+0x28 = 600.0` (`lui 0x4416`) goes to
  `Sound_Play`: Pulse's falloff.

## Where it differs from Pulse

| | Pulse (`0x0891d268`) | Pure (`0x08879d54`) |
| --- | --- | --- |
| Frame `Y` | `-(+0x20)` of the track record `AiTrack_LocatePosition` fills at the midpoint: **the track's up** | **world up**: `mtv zero, S000; mtv s6 (1.0), S001; mtv zero, S002; mtv s6, S003` (`0x0887a554`-`0x0887a568`) builds `(0, 1, 0, 1)`; no track lookup is called in the function |
| Anchor | the midpoint | the midpoint **plus `10 * (row2 x across)`** (`0x0887a5dc`-`0x0887a630`, non-VR only): `row2 = across x Y`, so the lift is `10 * (Y - across (across . Y))`, `10` units "up" and perpendicular to the wave's edge-to-edge line |
| Screen wash | `ScreenFlash_Start` kind `4` at the span's first point | kind `3` at the span's first point (`0x0887a718`, `li a1, 3`), every frame the wave lives |
| Effect names | `WO_QUAKE` | `WO_QUAKE` (`0x08a499f8`) and, behind the byte at `0x08a8f604` (`lbu -0x49fc`), `WO_QUAKE_VR` (`0x08a499ec`, fourcc `'QUVR'`, `0x52565551`), which also drops the lift |

The kind-3 case (`0x0882eac4`-`0x0882ec1c`) writes: duration `+0x54 = 0.4`, additive byte
`+0x58 = 1`, near `+0x5c = 0.0`, far `+0x60 = 300.0`, colour keys `+0x80 = (1.0, 0.2, 0.0,
0.5)` at time `0` and `+0x90 = (1.0, 0.2, 0.0, 0.0)` at time `1.0` (`+0xc0`, `+0xc4`).
**Those are exactly the numbers of Pulse's kind 4**, `oag_fx::flash::QUAKE` (confidence 85
for the case read). Pure's flash node is laid out differently from Pulse's (`+0x54`
where Pulse has `+0x44`), and its consumer (the update and draw) is unread; the
consumer law is Pulse's, **inherited**.

## Pure's own `WO_QUAKE.POB` is a different effect

`Data\Psys\WO_QUAKE.POB` on Pure is 21,984 bytes against Pulse's 34,480. Its emitters
(`psys_effect_dump --file`, `oag_fx`):

| Emitter | Flags | Draw | Colour | Spawn |
| --- | --- | --- | --- | --- |
| `WO_QUAKE` (root) | `0x02100299` | additive streak (a bar) | orange `(1, 0.6, 0.28)` to `(0.81, 0.34, 0.26)` | line, extent `50`, depth `0` |
| `bits` | `0x80100201` | additive streak | orange-yellow `(1, 0.84, 0.33)` to `(1, 0.65, 0.26)` | line, extent `50` |
| `bobs` | `0x80100205` | **alpha-over billboard** | **grey** `(0.4)` to `(0.2)` | line, extent `50` |

Pulse's are `fireballs` and `debris`. All three of Pure's are shape-1 line emitters of extent
`50` and depth `0`, so the extent law applies to Pure's file exactly as to Pulse's.
`WO_QUAKE_VR.POB` is a different file of the same size (first difference at byte 89, not
read further). `WO_QUAKE_NEW_ROCKS.POB` (26,112 bytes, same three emitter names, other
flags) is **loaded but not spawned**: the strings `Data\Psys\WO_QUAKE_NEW_ROCKS.POB`
sit only in two path tables (`0x08a47bec`, `0x08a50148`), and no bare name
`WO_QUAKE_NEW_ROCKS` exists for a `Psys_Spawn_q` to take (confidence 75).

## What ours drew, and what it draws now

Before this lane a Pure Quake drew **a single bright blob at the road's midpoint with a few
black specks**: `Effects::engine`'s inherited `WO_QUAKE` trigger played Pure's own file
(it is on the disc), but the extent law was Pulse-on-the-PSP only, so every line emitter
stayed at its anchor and the `/ 50` was fed in as severity (the stand-in
`oag_fx::psys::Effect::without_extents` keeps for sources not read). It drew no screen
wash. Now, through `oag_title::Effects`:

- **`extents`** lists `WO_QUAKE` (the HD mechanism, `crates/hd/src/effects.rs`): the three
  emitters spread edge to edge across the road at their authored size.
- **`quake_look`** (new, `oag_title::QuakeLook`): `up: World`, `lift: 10.0`, `flash: true`,
  `Origin::Measured`. `Race::advance_quake_visual` takes the frame's `Y` and the anchor lift
  from it; every other title keeps the track's up and no lift (`QuakeLook::track`).
- **The wash** is `oag_fx::flash::QUAKE` through a `ScreenFlash::only(QUAKE)`: a Pure race
  draws no other flash (its Rocket, Bomb and Mine kinds are unread), so the flash node
  accepts the Quake's kind alone. Pulse PSP keeps its all-kinds node.
- **The guard band stays Pulse's.** `Race::extend_stage_vertices` took "this race holds a
  `ScreenFlash`" as "this is Pulse's PSP source" and switched the GE guard band on for it;
  Pure now holds a Quake-only flash, so the test is `ScreenFlash::answers_every_kind`.
- **Leaving as it was:** Pulse (PSP and PS2) and HD. Re-rendered at ticks 410, 440, 480 and
  560 of an identical `--give quake` run, every frame is **pixel-identical** to the
  build before this change (ImageMagick `compare -metric AE`, `0` on all twelve;
  Pulse PSP and PS2 and HD show the wave in flight at 410 and 440). Pure's frames differ,
  as intended (AE 165,667 of 1,175,040 at tick 410).

Tests that fail if a piece is dropped: `oag_pure::effects::tests::pures_quake_spreads_by_the_extent_law_world_up_and_washes`
(the table), `quake_orientation_ground_truth::the_pure_quake_frame_is_world_up_and_the_anchor_is_lifted`
(on the disc: the frame is built, is exactly world up where the track's up leans, the anchor
sits `5..=10` units above the road, the wave washes the screen),
`oag_fx::flash::tests::a_flash_that_answers_one_kind_ignores_the_rest`; the Pulse and HD
cases of the same file now also assert no lift and the wash on or off.

## The capture

PPSSPP 1.20.4, software renderer, Pure USA, Time Trial, Venom, Alpha, Vineta K, Feisar:
walked from a cold boot with `input.buttons.press` through the debugger (Language: cross; three
cross presses through the logos; Profile: down, cross for NEW; Set Name: cross through every
slot; Save Profile: down, cross for NO; Warning: cross; Main Menu: cross; Single Player:
down, down, cross; Class: down, cross; League: cross; Track: cross; Team: cross; confirm),
then `start`, RESTART, and `scripts/psp-pure-weapon-fire.py 3 <out> --skip N --frames 80
--every 2` (weapon id `3` is the Quake). The script writes the id into `*(ship+0xd0)+0x1d8`
and a `1` at the fire byte on hit `N` of `Ship_UpdateWeapons`:

- `--skip 150` fires **0.3 s after GO** (the craft at 30 km/h, still at the start gantry);
- `--skip 205` fires **2.2 s after GO** (233 km/h, the state the other Pure weapon captures used).

What the original draws: from the frame after the fire, the **whole screen washes
orange-pink** for about 0.7 s (strongest first, fading as the wave recedes); a **band of fire
spreads across the road at the wave's position**, edge to edge, with a bright core; **dark
grey-brown chunks** (the `bobs` emitter, alpha-over) fly up out of it and tumble; the wave
races away down the road faster than the craft and is a small spark far ahead by 1.1 s.
Our frames after the change show the same four things. Frame pairs are kept in `data/reference/pure-quake/` (gitignored, game-derived; its
`README.txt` says how each was made): the 0.3 s pair is the cleaner one, both craft at the start line.

**The pairs are time-matched, not state-matched.** Pure's physics is not implemented
(`pure-status.md`): at the same race time ours is further along the track and faster, so the
2.2 s pair shows the wave from a different place. Differences left: the original's core is
less blown out than our bloom; our debris reads orange where the original's reads dark (the
additive wash lands on it in both, ours is brighter); our wave's speed is Pulse's fixed 270
units a second, which has not been read on Pure.

## Open

1. **The road ripple.** Vineta K's `track.vex` carries one node of class `0x379` (a
   histogram of its node classes: exactly one), which is `Quake` in the exporter's class
   table (index 13, predicted `0x379`; `pure-status.md`), and the executable carries
   `Quake_Importer.cpp`. Pulse's wave lifts the road along that span table
   (`oag_render::ripple`); Pure's ripple is **unread** and ours draws none. The frames
   captured here show no obvious lift, but nobody compared them frame for frame.
2. **Pure's `ScreenFlash_Update` and its draw** (the consumer of the kind-3 numbers) are
   unread; the wash is Pulse's law, inherited. The other nine kinds are unread too, so no
   Pure detonation washes the screen.
3. **The wave's travel speed** on Pure (ours: Pulse's fixed 270, chosen).
4. **The second instance.** The loop runs twice and `_DAT_0028b668 = 2` on a fork (a span
   with two forward neighbours); Vineta K was not checked for a fork, and ours is a single
   instance.
5. **The VR build:** `WO_QUAKE_VR` and the dropped lift (`0x08a8f604`) are not wired; the
   flag's meaning is not read here.
6. **`WO_QUAKE_NEW_ROCKS`**: no spawn found; if one exists it is behind a name this page did not find.

## Cross-title

- **Pulse:** the template; Pure now takes its law with the three differences above, and
  Pulse is unchanged (pixel-identical, see above).
- **HD:** checked, differs in kind (its own `WO_QUAKE` from `/data/psys`, its own table);
  HD keeps the track-up frame it took from Pulse, which this lane did not re-read.
- **2048 and Omega:** not touched. Neither is Pure's lineage; their `WO_QUAKE` frames are
  unread, so each keeps `QuakeLook::track`.
