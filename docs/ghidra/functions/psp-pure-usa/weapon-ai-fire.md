# Pure's opponent weapon decision: `WeaponAi_DecideFireOrAbsorb`

**Binary:** `psp-pure-usa` `BOOT.BIN` (image base `0x08804000`; file offset = vaddr + `0x80`). Read
2026-11-03 for the `pure-fire-law` lane: Ghidra decompile and disassembly of `0x08931964`, its
caller `0x089316ac` and the loader `0x089322b0`, the rate table decoded from the file as floats.
**Static reading; no live capture** (see "What would raise it").

**Verdict: same as Pulse, fully read. Nothing to wire.** `OpponentFire::PULSE` is the right term
set for Pure, and it was already what Pure ran (inherited); it is now measured (static). Pure has
no Eliminator branch, so `Weapons::elimination` is not what this decision reads.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x089316ac` | `WeaponAi_Update` | 80 |
| `0x08931964` | `WeaponAi_DecideFireOrAbsorb` | 85 |
| `0x08ab3da0` | `g_weapon_ai_rate` | 85 |
| `0x089322b0` | `WeaponAiStats_Load` | 80 |
| `0x08932868` | `WeaponAiStats_ParseWeapon` | 80 |

## How it was found

Pulse's twin by structure. Pulse's rate table (`0, 0.0005, 0.002, 0.008, 0.05`) occurs exactly once in
Pure's image, at file offset `0x2afe20` (vaddr `0x002afda0`, loaded `0x08ab3da0`). Its only code
reference is one `lui 0x2b / addiu -0x260` pair at `0x08931974 / 0x08931988` (`scripts/psp-relocate.py
xrefs 0x08ab3da0`), inside `0x08931964`. Ghidra shows it as `*(float *)(idx * 4 + 0x2afda0)` because
this import applies no relocation.

Pure's table is followed by `0, pi, 0, 0, 0`: **there is no second (Eliminator) rate table**.

## The call chain and cadence

`WeaponAi_Update(dt, self)` at `0x089316ac` is reached through a stored function pointer (the word at
`0x08ac26b8` holds its unrelocated address; no `jal` calls it, as on Pulse). Its walk, with Pulse's
offsets in brackets:

- `+0xe0 -= dt`, floored at `0` (the 3 s wait timer) [Pulse `+0x54`].
- Skipped unless `*(self->entity + 0xac0) != 0` [Pulse `entity + 0xae4`].
- `+0x2a = (weapon record + 0x1d8 != 0)` (is holding), `+0xbc += dt` **with no reset anywhere in the
  function** (the only stores to `+0xbc` are `0x0893172c`, plus the constructor) [Pulse `+0x30`]. Past
  `0.25` (`c.le.s` against the constant, decompile `0.25 <= +0xbc`) the gate stays open: **the decision runs on every call after the
  first quarter second**, exactly Pulse's reading.
- `+0xb8` (the held-seconds scalar, `> 0.8` fire, `> 0.2` absorb) is zeroed while nothing is held and gains `dt`
  per step otherwise [Pulse `+0x2c`].
- `0x0892fdf4` returns the held weapon id (`-1` for none); a 10-way jump table (loaded `0x08a7a880`, unrelocated `0x276880`) sets the direction flags and calls the weapon's setup, then the common tail
  `jal 0x0012d964` = **`0x08931964`**.

Direction flags (`+0xdc` ahead, `+0xdd` behind, `+0xe4` aimed projectile), from the table's ten arms:

| Id (loader order below) | ahead | behind | aimed |
| --- | --- | --- | --- |
| 0 Rocket, 1 Missile, 3 Disruptor, 7 Plasma | yes | - | yes |
| 2 Quake | yes | - | - |
| 4 Turbo, 5 Shield | yes | yes | - |
| 8 Bomb, 9 Mine | - | yes | - |
| 6 Autopilot | none; sets the absorb index `+0xc0 = 3` | | |

This is Pulse's table with Pure's roster (no Cannon, LeachBeam, Repulser or Shuriken; the Disruptor
takes the Cannon's forward-projectile slot). Autopilot's default arm (`+0xc0 = 3`, fire index left `0`)
equals Pulse's `param_2[0xd] = 3` arm, so an opponent never fires an Autopilot (`rate[0] = 0`) on either.
Setup functions: the forward-weapon setup (`0x089304d0`, `0x08930588`) is `fire index = 1`, `+1` if
`+0xd8 == 1`, `+2` on `(+0xd8 < 2 and 0 < +0xcc < 100) or +0xd8 >= 2`, `+1` if `0 < +0xc8 < 300`;
Turbo (`0x08930694`) is `3` unless `0 < +0xe8 < 300` (then `2`; Pulse's `+0x5c` is dead, `+0xe8` was not
checked here). These match the shape Pulse's `weapon-ai.md` records; the setups were spot-read, **not
diffed term by term** (confidence 60).

## The decision

```c
// self: +0x10 controller (fire +0x15, absorb +0x16), +0x18 stats object, +0xac held id,
//       +0xb8 held seconds, +0xc0 absorb index, +0xc4 fire index, +0xde "no craft in path",
//       +0xdc/+0xdd ahead/behind, +0xe0 wait timer, +0xe4 aimed, +0xec gap to the player
rate = 0x08ab3da0;                                    // one table, no Eliminator table
if (self.0xe4 && self.0xe0 != 0.0 && !self.0xde && rand01() < rate[4]) {   // rate[4] = 0.05
    fire = 1; absorb = 0; self.0xe0 = 0.0; return;
}
clamp(self.0xc0, 0, 4); clamp(self.0xc4, 0, 4);
r_fire = rand01(); r_abs = rand01();                  // both drawn up front, fire first
in_range = (self.0xdc && 0 < gap && gap < 150) || (self.0xdd && -200 < gap && gap < 0);   // gap = +0xec
row  = stats + id * 0x1c0 + 8;                        // stride 448
use  = in_range ? *(row + 0x1a4) : *(row + 0x1a8);    // useAgainstPlayer : useAgainstAI (row already +8)
ab   = *(row + 0x1ac);                                // absorb
if (byte@0x08a8c4b3 == 0 && *(0x08b173c8 + 0x1c) == 2) use *= 2.0;
fire_p   = rate[self.0xc4];  if (fire_p   != 0) fire_p   = 1 / ((1/fire_p)   * (1/use));   // "long way"
absorb_p = rate[self.0xc0];  if (absorb_p != 0) absorb_p = 1 / ((1/absorb_p) * (1/ab));
fire = absorb = 0;
if (r_fire < fire_p && self.0xb8 > 0.8) {
    if (self.0xe4 && self.0xde) { self.0xe0 = 3.0; fire = 0; return; }   // aimed, nothing in path: wait 3 s
    fire = 1; return;
}
if (r_abs < absorb_p && self.0xb8 > 0.2) absorb = 1;
```

`rand01()` is `FUN_088e0568() * 4.656613e-10` (`2^-31`), Pulse's width. Decoded constants (from the
`lui` immediates and the table): `0.8` (`0x3f4c`), `0.2` (`0x3e4c`), `150.0` (`0x4316`), `-200.0`
(`0xc348`), `3.0` (`0x4040`), `2.0` (`0x4000`), `1.0` (`0x3f80`), `0.25` gate; rate table
`0, 0.0005, 0.002, 0.008, 0.05` (f32 words `0x3a03126f`, `0x3b03126f`, `0x3c03126f`, `0x3d4ccccd`, decoded from
the file: `0.0005000000237`, `0.00200000009`, `0.0080000004`, `0.0500000007`).

## Two independent agreements (why 85)

1. **Loader and reader agree on the row.** The loader `0x089322b0` opens `WeaponAIstats.xml` and walks
   `Rockets, Mines, Bomb, Missiles, Quake, Turbo, Shield, Plasma, Disruptor, Autopilot` into
   `WeaponAiStats_ParseWeapon` (`0x08932868`) with ids `0, 9, 8, 1, 2, 4, 5, 7, 3, 6`. That parser
   stores `useAgainstPlayer` at `stats + id * 0x1c0 + 0x1ac`, `useAgainstAI` at `+0x1b0` and `absorb`
   at `+0x1b4`. The decision reads exactly `stats + id * 0x1c0 + 8 + {0x1a4, 0x1a8, 0x1ac}`.
   Pure's row is 448 bytes because it also carries per-weapon `PadDecision` and positional tables
   Pulse's 12-byte row lacks.
2. **Pulse's shape.** Same two-roll-up-front, same `0.8` / `0.2` held gates, same `150` / `-200`
   window, same `3.0` wait, same early-out on `rate[4]`, same "long way" product, same
   mode-`2` doubling gated by the same byte-and-mode pattern (Pulse `DAT_08ab07e3` / `DAT_08b31048`;
   Pure `0x08a8c4b3` / `0x08b173e4`).

## Pulse vs Pure, term by term

| Term | Pulse | Pure (`0x08931964`) | Same? |
| --- | --- | --- | --- |
| Gates `0.8` s / `0.2` s, window `0..150` / `-200..0`, `3.0` s wait | yes | yes | same |
| `0.25` s gate never reset; decision every call | yes | yes | same |
| Rate table | `0, .0005, .002, .008, .05` | same five floats | same |
| Eliminator table, `use *= 5`, absorb index forced `0`, fire floor `1` | yes | **absent**: no mode-8 test, one table | n/a (Pure has no Eliminator, `pure-status.md`) |
| Time scale `dt * 30` | none | none | same |
| `AllWeapons` row | none | none | same |
| Difficulty scaling in the decision | none | none (no difficulty read) | same |
| Mode `2` doubling | yes | yes | same (mode `2` unidentified on both) |
| "Long way" product | yes | yes | same |
| Early-out on `rate[4]` | yes | yes | same |
| Per-weapon flags | `WeaponAi_Score*` | 10-way table, same arms | same by shape (setups spot-read) |

**Where difficulty enters:** nowhere in the decision. The skill indices (`+0xc0`, `+0xc4`) are
recomputed from race state every call (the setups above), and `row.use` / `row.absorb` come straight
from the file, as on Pulse. Nothing reads a per-difficulty scale.

**`Weapons::elimination` (Pure's Eliminator tuning, carried and unread):** not read here.
This function has no Eliminator branch and no second rate table, and `pure-status.md` records that Pure
ships no elimination mode. The field stays unread on Pure.

## Cross-check (2048 and Omega)

Not a Pure-to-Vita relationship: Pure is PSP-lineage and the HD/2048/Omega decisions are read on
their own pages ([HD](../ps3-hdfury-eu/weapon-ai-fire.md), [Omega](../ps4-omega-eu/weapon-ai-fire.md)); 2048's is
unread. **Checked, differs** from HD and Omega (they carry `dt * 30`, `AllWeapons` and Eliminator scales; Pure has none).
Nothing recovered here changes their wiring.

## Ready to wire

None: `crates/pure/src/lib.rs` already passes `OpponentFire::PULSE` (no step scale, no `AllWeapons`, no
authored Eliminator), which is this function. Change its provenance from inherited to measured (static,
80) in `docs/gameplay/weapon-lineage.md`.

## What would raise it

A PPSSPP execute breakpoint on `0x089316ac` / `0x08931964` under Pure's disc (counts per second per
opponent and the mode word at `0x08b173e4` in a Single Race) would settle the world-step cadence and
what mode `2` is on Pure. The `0x089303..` setups diffed term by term against Pulse's would lift the
per-weapon flags from 60 to 85. Not run: another member holds the PPSSPP bridge on this lane.
