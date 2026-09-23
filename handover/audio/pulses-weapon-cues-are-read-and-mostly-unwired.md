# Pulse's weapon cues are read and mostly unwired - the wiring plan

2026-09-23. On Pulse (PSP) only the Plasma, Shield, Mine and Autopilot make
weapon sounds; firing a Rocket, Missile, Quake, Cannon, LeachBeam or
Shuriken plays the pickup chime and nothing else. A research pass produced
the plan below and checked every cue name against the bank
(`oag-wad sounds data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad --cue <name>`).
An implementation lane started from it the same evening. **Whoever lands that
work updates this thread, or deletes it and its index line if nothing is left
open.**

## Bank spellings (case-sensitive; `Banks::pick` needs an exact match)

| Cue | Waveforms | Kind | Plan |
| --- | ---: | --- | --- |
| `~ROCKETTVL` | 1 | loop | wire |
| `ROCKEXPLWALL` | 1 | one-shot | wire, after the timeout question below |
| `ROCKEXPLSHIP` | 3 | one-shot | wire |
| `MISSILE` | 3 | one-shot | wire |
| `~MISSILETVL` | 1 | loop | wire (prose on `missile.md` spells it `_MISSILETVL`; trust the bank) |
| `MISSILEEXPWALL` | 1 | one-shot | wire |
| `MISSILEEXPSHIP` | 2 | one-shot | leave: no call site recovered |
| `CANNON` | 1 | one-shot | wire |
| `CANNONEXPLSHIP` | **0** | empty | wire; plays nothing |
| `CANNONEXPLWALL` | 9 | one-shot | wire |
| `QUAKEHIT` | 1 | one-shot | wire |
| `QUAKELAUNCH` | 3 | one-shot | leave: named only in passing, no address |
| `~QUAKETRAVEL` | 2 | loop | leave: the page says the literal was never resolved |
| `LEACH` | 1 | one-shot | wire, locked fire only |
| `~LEACHATTACH` | 2 | loop | wire (prose spells it `_LEACHATTACH`; trust the bank) |
| `LEACHENERGY` | **0** | empty | open, see below |
| `LEACHFAIL` | 1 | one-shot | leave: undocumented, only a plausible unlocked-fire candidate |
| `SHURIKENHIT` | 2 | one-shot | wire |
| `~SHURIKENTRAVEL` | 1 | loop | wire |
| `SHURIKENEXPL` | **0** | empty | leave |

`CANNONEXPLSHIP` and `LEACHENERGY` are empty cues in `Data.wad`'s weapon bank.
Wired correctly they are still silent, so do not chase that as a bug.
Only that one bank was queried.

## Per weapon: edge and evidence

- **Rocket** (`docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`).
  `~ROCKETTVL` is held per projectile slot while a Rocket occupies it, on the
  bolt's own emitter with **radius 600.0** (`Rocket_Init` writes
  `+0x38 = 0x44160000`), not `CRAFT_RADIUS` (200). Hit cues go through the
  existing `impacts` loop in `crates/game/src/race/tick.rs`, with
  `impact.struck` picking ship or wall. **Open**: the 5.0 s pool-reap
  timeout sets neither the wall (`0x10`) nor the ship (`0x20`) flag, so
  `struck: None` is not automatically a wall hit. Find what tells the two
  apart in `Projectiles::advance`/`flight.rs` before wiring `ROCKEXPLWALL`.
- **Missile** (`missile.md`). `MISSILE` is pushed in `Race::fire_missile`
  only when `projectiles.spawn_guided(...)` returns true, and the opponent
  fire path in `crates/game/src/race/field/opponent_weapons.rs` needs the same
  push with the real slot. `~MISSILETVL` is held per slot; its radius is
  unstated, so the choice is labelled chosen. `MISSILEEXPWALL` fires on
  **every wall bounce**, on the same edge as the built `WO_MISSILE_BOUNCE`
  visual: compare `bounces_before` with the array after it in `tick.rs`.
  Leave the expiry ending unwired: `missile.md`'s by-catch reads its literal
  as `SHURIKENEXPL` (confidence 60, likely a bug in the original).
- **Cannon** (`cannon-quake-leachbeam.md`, about lines 460-474 and 603-627).
  `CANNON` is played at the end of `Cannon_Init` (`0x088648ec`, 80). Push it
  in `Race::advance_one_cannon` only when `projectiles.spawn(...)` returns
  true (that result is discarded today). The hit split is `0x10` wall /
  `0x20` craft at `CannonPool_Update`'s despawn (`0x088582b0`, 88), and
  `crates/gameplay/src/projectile/cannon.rs`'s doc already mirrors it. The
  Rocket's timeout question applies here too.
- **Quake** (same page, about lines 930-1001). `QUAKEHIT` (85) fires on the
  rising edge of the per-craft "wave reached me" latch: snapshot
  `oag_gameplay::projectile::quake::Wave::hit` before `advance_quake()` and
  compare after. It plays on the **struck** craft (the same block's shield
  gate is the victim's own bit, which settles the page's own naming
  ambiguity).
- **LeachBeam** (same page, about lines 2058-2080). `LEACH` is one-shot on the
  shooter's emitter, from `LeachBeam_InitLocked` (`0x08873d3c`, 82). The
  unlocked-fire cue is not confirmed. `~LEACHATTACH` is held on the beam's own
  emitter (radius 600) for as long as the beam is connected; its position is
  unrecovered, so label the choice. **`LEACHENERGY` is open**: it fires once
  per ribbon scroll-cursor wrap (`+0xa8`, about once a second) in
  `LeachBeam_Advance`, not on every drain tick. This engine's beam does not
  model that cursor (`crates/gameplay/src/projectile/leach_beam.rs` says so),
  so `Report::drained` is the wrong edge. Do not invent a ~1 Hz timer.
- **Shuriken** (`shuriken.md`). `SHURIKENHIT` (88, `Shuriken_Bounce`) fires
  on every wall bounce, on the same `bounces_before` edge as the Missile.
  `~SHURIKENTRAVEL` is held per slot. Its emitter is a real open choice:
  `Shuriken_Init` is handed the craft's emitter, with no separate
  `SoundEmitter_Init`. The launch cue stays unwired: the page says its name
  is unverified, and the bank's `SHURIKEN` cue is a name-match guess.

## Implementation notes

- `cue.rs` is about 540 lines. Twelve or so new variants plus `sfx.rs` will
  break the 1,000-line ratchet. Generalise `SfxVoices::plasma_travel` into one
  per-slot travel-voice tracker keyed by weapon (say `sfx/travel.rs`); the
  Plasma keeps its `charge <= 0.0` gate on top.
- Add every new held cue to `Cue::held()`.
- `sfx_ground_truth.rs` needs per-title filtering: Pure ships no Cannon or
  LeachBeam table.
- `oag_audio::Emitter` may need a radius-taking constructor for the 600-unit
  emitters.
- Sound stays a per-tick output, never `World` state (ADR-0018). Everything
  above reads existing `impacts`/`bounces_before` state from `tick.rs`, and
  `Impact::struck`'s own doc comment anticipates this use.
- `docs/overview/status.md`'s Quake Audio cell over-claims: `QUAKELAUNCH` is
  not on the page it cites, and `QUAKETRAVEL` is explicitly unresolved there.
  Correct it to `QUAKEHIT` alone in the same change that wires it.

## Open

- Rocket/Cannon: a timeout reap versus a wall hit, as the edge for the wall
  cue.
- `LEACHENERGY`'s edge needs the ribbon's scroll cursor, which the simulation
  does not carry.
- `QUAKELAUNCH`, `~QUAKETRAVEL`, `MISSILEEXPSHIP`, `LEACHFAIL` and the
  Shuriken's launch cue have no recovered trigger.
- `missile.md` (about lines 174-176) names `Ship_FireHeldWeapon`
  (`0x08844ae8`) as the opener of `ROCKET`, `QUAKELAUNCH` and
  `_AUTOPILOT`/`autopilot_eng`. `autopilot.md` and `Cue::Disengaging` say the
  Autopilot's opener is unlocated. One of the two is wrong; if `missile.md` is
  right, the Autopilot's audio and `QUAKELAUNCH` both have their edge.

## Next Steps

1. Implement the "wire" rows above, with a headless test per cue asserting
   it fires on its edge and renders non-silent samples through the WAV writer.
2. Settle the Rocket/Cannon timeout question in `flight.rs`, then wire the
   wall cues.
3. Read `Ship_FireHeldWeapon` (`0x08844ae8`) to resolve the Autopilot
   conflict and possibly `QUAKELAUNCH`.
