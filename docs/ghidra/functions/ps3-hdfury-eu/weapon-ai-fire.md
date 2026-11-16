# HD's opponent weapon decision: `WeaponAi_DecideFireOrAbsorb`

**Binary:** `ps3-hdfury-eu` `EBOOT.elf` (PowerPC, TOC `r2 = 0x8ad4d8`). Read 2026-11-03 for the
`hd-fire-law` lane: Ghidra decompile of `0x000f9a18`, checked against `llvm-objdump` for the call
site and the TOC constants, which were decoded as floats from the ELF. **Static reading; no live
capture** (see "What would raise it").

**Verdict: differs from Pulse, fully read, ready to wire** (below). The decision has Pulse's
skeleton (same gates, same two rate tables, same `0.8` / `0.2` hold gates, same `3.0` wait) and
four changes: a per-step time scale, the `AllWeapons` multiplier, HD's own Eliminator terms in place
of Pulse's `use *= 5`, and an Eliminator-only "flip" block.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x000fb928` | `WeaponAi_Update` | 80 |
| `0x000f9a18` | `WeaponAi_DecideFireOrAbsorb` | 80 |
| `0x000f98b8` | `WeaponAi_ScanTraffic` | 80 |
| `0x000fa210` | `WeaponAi_Construct` | 80 |
| `0x000fa140` | `WeaponAi_BiasAbsorbByDistance_q` | 55 |
| `0x008c175c` | `g_weapon_ai_rate` | 85 |
| `0x008c1770` | `g_weapon_ai_rate_eliminator` | 85 |

HD splits Pulse's pair the same way Pulse does: `WeaponAi_Update` (`0x000fb928`, `(double dt,
WeaponAi *self)`, the `0.25` s gate and the per-weapon switch) calls `WeaponAi_DecideFireOrAbsorb`
(`0x000f9a18`, same `(dt, self)` signature, **now with a `dt` argument Pulse's lacks**) from one
`bl` at `0x000fc074`. Ghidra's xref index misses that `bl` (it lists only the descriptor word at
`0x874fe8`), which is why the earlier lane saw "no caller". 80 and not 90 because nothing was
observed running.

## Why this is the reader of `RaceManager + 6240`

Two independent agreements, which is what lifts it from a decompile to a reading:

1. **Field layout.** The loader (`weapon-ai-stats.md`, 85) puts a 12-byte row per weapon from `+8`
   (`useAgainstPlayer`, `useAgainstAI`, `absorb`), `AllWeapons` at `+164 .. +172`, the two flips at
   `+176/+180` and the easy/medium/hard scales at `+184`, `+196`, `+208`, `+220`. This function reads
   exactly `stats + id * 0xc + {8, 0xc, 0x10}`, `stats + {0xa4, 0xa8, 0xac}` (`AllWeapons`),
   `stats + 0xb0 / 0xb4` (flips), `stats + 0xb8 + diff * 4` (flip scale), `stats + 0xc4 + diff * 4`
   (use scale) and `stats + 0xd0 + diff * 4` (absorb scale). `diff` is `GameState + 0xdc`. Nothing in
   it reads `+0xdc .. +0xe4` (the score scales); they are read elsewhere or nowhere.
2. **Pulse's shape.** Rate table indexed by skill, `rate * use` the long way (`1 / ((1 / use) /
   rate)`), the same `0.8 s` / `0.2 s` gates, the `150` / `-200` window, the `3.0` s wait and the
   early-out all match `psp-pulse-usa/weapon-ai.md`.

**Where `stats` comes from.** `WeaponAi_Construct` (`0x000fa210`) stores it at `this + 0x1c`
(`param_1[7]`) from a resource-registry lookup by the name `WeaponAIStats` (TOC string
`0x782998`), the same registry that hands it `Race manager` (`0x782978`) and `AI track data`
(`0x782988`). That is why no `addi 6240` outside the RaceManager constructor and destructors takes
the object's address, and why the earlier lane found no `lfs` of `RaceManager + 6416 .. 6468`:
the reads go through this pointer. That the registered `WeaponAIStats` is the `+6240` object the
loader fills is the natural reading (same name, same layout above) and is not shown by an address
chain (confidence 80). A second constructor `0x000fa400` is the same function for the other
`WEAPON SYSTEM %d` path; not diffed.

## How it is called (cadence)

`AIManager_Update` (`0x000bb960`) ends with a loop over `mgr + 0x78` (count) and `mgr + 0x7c`
(pointers): `lwz 9, 0(obj); lwz 11, 8(9); lwz 0, 0(11); ... bctrl` at `0x000bba70..0x000bba84`, with
`dt` in `f1` (`fmr 1, 31` at `0x000bba5c`) and the object in `r3`. The object's vtable pointer is
`0x008637c0` (stored by the constructor `0x000fa210` from TOC `0x8a9404`); its slot at `+8` is the
descriptor at `0x00875070`, whose code is **`0x000fb928`**, `WeaponAi_Update`. So `Update` runs
once per world step for every weapon AI (confidence 85: constructor, vtable word and call site all
read).

`WeaponAi_Update` (`0x000fb928`), outside mode `0xd` (identity open, `mode-manager.md` lists it as
an unassigned bucket; it has its own simpler law, not read for the odds):

- `+0x68 -= dt`, floored at `0` (the wait timer: **Pulse's decrement at the top of its Update is
  kept**), then, if the held id is valid, `+0x34 += dt` with **no reset anywhere** (the only other
  stores of `+0x34` are the two constructors' zero). Once it passes `0.25` (TOC `-0x4034`, decoded
  `0x3e800000`) the gate stays open: **the decision runs every step**, exactly
  Pulse's reading (`weapon-ai.md`, "the decision cadence").
- `+0x30` (the held clock the `0.8` / `0.2` gates read) is zeroed while nothing is held and gains
  `dt` per step while holding; Pulse's `+0x2c`.
- A 13-way jump table (`0x000fbd90 .. 0x000fc06c`) sets the direction flags `+0x64 / +0x65 /
  +0x6c`, the nearest gaps `+0x50 / +0x54` and the skill indices `+0x48 / +0x4c` per weapon id
  (with `WeaponAi_BiasAbsorbByDistance_q`, `0x000fa140`, adding `1..4` to `+0x48` by a distance
  tier, Pulse's `WeaponAi_BiasAbsorbByShield`-shaped step), then falls into the common tail
  `bl 0x000f9a18`. These per-weapon setups were **not diffed case by case** against Pulse's
  `WeaponAi_Score*` functions: the stores are the same fields, so this page treats them as
  **assumed Pulse's, setup unread** (confidence 50).
- A craft whose controller byte `+0x1a` is set (HD only) runs a timer `+0x38 += dt`, cleared past
  `0.7` (`-0x4044`) or when nothing is held, and skips the decision that step.

**The cadence is every step, and the chance is scaled by the step**: `k = dt * 30.0`
(`0x008a9434` holds `0x41f00000`; `param_1 * DAT_008a9434` at the top of `0x000f9a18`). Pulse's
`WeaponAi_DecideFireOrAbsorb` takes **no `dt` at all** (`int param_1` only; re-decompiled on
`/pulse/BOOT-psp-pulse-usa.BIN`: no frame-scale term in the rolls, `rate * use` against a bare
`rand * 4.656613e-10`). At HD's `dt = 1/60` (the world step the other HD pages read, e.g.
`engine-trail.md`), `k = 0.5`: **every per-step chance on HD is half of the number Pulse's law
would give for the same table row**. Confidence 80 for the `dt * 30` (read directly), 70 that HD's
world step is exactly `1/60` at the point this runs (not measured here).

## The law

`this` fields, HD offset (Pulse offset `-0x04` for `+0x00 .. +0x30`, `-0x10` for `+0x48 / +0x4c`,
`-0x14` for `+0x66 .. +0x74`, i.e. the same fields shifted):

| HD | Pulse | What |
| --- | --- | --- |
| `+0x04` | `+0x04` | the craft record |
| `+0x14` | `+0x10` | the controller record the request bytes go to: **fire** `+0x19`, **absorb** `+0x1b`, and `+0x1a`, use held ([eliminator-flip.md](eliminator-flip.md)) |
| `+0x1c` | `+0x18` | the `WeaponAIStats` object |
| `+0x24` | `+0x20` | the held weapon id |
| `+0x30` | `+0x2c` | held time scalar (`> 0.8` fire, `> 0.2` absorb; `0.8` is `0x8a9428`, `0.2` is `0x8a9430`) |
| `+0x48` / `+0x4c` | `+0x34` / `+0x38` | absorb / fire skill indices, clamped `0..4` |
| `+0x50` / `+0x54` | `+0x3c` / `+0x40` | nearest gap ahead / behind (written by `WeaponAi_ScanTraffic`) |
| `+0x64` / `+0x65` / `+0x66` | `+0x50` / `+0x51` / `+0x52` | works ahead / works behind / "no craft in the shot's path" |
| `+0x68` | `+0x54` | wait timer, set to `3.0` (`0x40400000`) |
| `+0x6c` | `+0x58` | aimed projectile weapon |
| `+0x74` | `+0x60` | signed gap to one particular craft: `ScanTraffic` stores it only when the scanned racer's craft equals the word at `+8` of the object at `0x009879f0` (`0x000f994c .. 0x000f9958`; the skip-self test is `0x000f9928 .. 0x000f9930`) |

That `+8` of the object at `0x009879f0` is the player's craft is an inference (its writer was not
found): it matches Pulse's open "`+0x60` is the gap to the player" reading (75 on Pulse) and the
attribute names, so both are at **60** here, not a corroboration.

```c
k     = dt * 30.0;
mode  = GameState.mode;                              // +0xe0
table = (mode == 8 || mode == 0x14) ? rate_elim : rate;    // 0x008c1770 : 0x008c175c
if (*(u8*)0x009384e1 != 0) mode = 0;                 // the same byte Pulse tests; 0 in a single race
E     = (mode == 8);                                 // an Eliminator, HD's id for it

// early out (aimed weapon, a wait running, nothing in the path)
if (self.0x6c && self.0x68 != 0.0 && !self.0x66 && rand01() < k * table[4]) {
    fire = 1; absorb = 0; self.0x68 = 0.0; return;
}

in_range = (self.0x64 && 0 < gap && gap < 150) || (self.0x65 && -200 < gap && gap < 0);   // gap = self.0x74
row  = stats + 12 * id;
use  = in_range ? row[8] * stats[0xa4] : row[0xc] * stats[0xa8];   // x AllWeapons use
ab   = row[0x10] * stats[0xac];                                    // x AllWeapons absorb
if (mode == 2)  use *= 2.0;
else if (E) {                                                       // Pulse: use *= 5.0
    ab  *= stats[0xd0 + diff*4];       // absorb scale, diff = GameState+0xdc
    use *= stats[0xc4 + diff*4];       // use scale
    // (and the flip search below, only when this branch is taken)
}
if (E && self.0x4c == 0) self.0x4c = 1;              // fire skill floor 1 in an Eliminator
fire_p   = k * table[self.0x4c];  if (fire_p   != 0) fire_p   = use * fire_p;   // "long way": 1/((1/use)/fire_p)
absorb_p = k * table[self.0x48];  if (absorb_p != 0) absorb_p = ab  * absorb_p;
r_fire = rand01(); r_abs = rand01();                  // both drawn up front, fire first
do_absorb = r_abs < absorb_p;
if (E) do_absorb &= (craft[0x6ad8] == 0);
fire = 0; absorb = 0;
if (r_fire >= fire_p || self.0x30 <= 0.8) {
    if (do_absorb && self.0x30 > 0.2) absorb = 1;
    return;
}
if (E) { self.0x48 = 0; /* flip block, below */ }
if (self.0x6c && self.0x66) { self.0x68 = 3.0; fire = 0; return; }   // aimed, nothing in path: wait 3 s
fire = 1;
```

`rand01()` is `rand() * 2^-30` (`0x30800000` is `2^-30`, `0x008a9438`); `rand()` returns `x &
0x3fffffff` (`0x0043c280`, a 30-bit draw), so the draw is in `[0, 1)` exactly as Pulse's 31-bit
draw times `2^-31`. **Not** a divergence. Decoded constants: `150.0` (`0x8a943c`), `-200.0`
(`0x8a9440`), `400.0` (`0x8a9448`, the Quake's window in the flip block), `10.0` (`0x8a93e4`), `1.0`
(`0x8a9444`, the long-way numerator), `0.0` (`0x8a93e0`).

### The rate tables are Pulse's

`0x008c175c` (five floats): `0, 0.0005, 0.002, 0.008, 0.05`. `0x008c1770`:
`0.001, 0.002, 0.01, 0.02, 0.1`. Equal to Pulse's `g_weapon_ai_rate` and
`g_weapon_ai_rate_eliminator` entry for entry (decoded from the ELF, confidence 85). The Eliminator
table is chosen for modes `8` and `0x14` (Pulse: `8` and `0x12`); `0x14` is HD's id for the mode
Pulse calls `0x12`, not read further here.

### The Eliminator-only flip block

Interpreted 2026-11-07: [eliminator-flip.md](eliminator-flip.md) has the chain and corrects `+0x1a` below, which is use held, not a flip request.

Two pieces, both only when `E`; `rec` is the record at entry `+4` of this craft's own row in the racer table (`table + 0x14 +
idx * 0x9c0`, `idx` found by walking the table for `self + 4`), which `ai-controls.md`
(`AiManager_AddDriver`) documents as **the driver**; `Ai_Construct` zeroes its `+0x24` / `+0x28`
(`param_1[9]`, `param_1[10]`), fields in the prefix HD's driver adds over Pulse's, a lead for
whoever names the flip, `+0x78` and `+0x79` are bytes on
the weapon AI, `+0x7c` a float:

- **Before the rolls** (after the `use` scale): `+0x7c -= dt`, floored at `0`; if `+0x78 == 0`,
  `rec + 0x24` is outside `[-pi, pi]` (`0x8a9420` / `0x8a9424`) and `+0x7c <= 0`, the controller's
  `+0x1a` and `+0x19` are set, `+0x79 = 1`, and the function **returns** (no fire roll).
- **In the fire-passed branch**: `+0x48 = 0`; if the held id is not Bomb (8) or Mine (9) and
  `(10 < +0x54 < 150 || +0x50 == 0)` (Quake, id 2: `(10 < +0x54 < 400 || +0x50 == 0)`), then with
  `+0x78 != 0` it draws `rand01() < (+0x50 == 0 ? infrontFlip : normalFlip) * flipScale[diff]`; on
  success `+0x7c = 1.2` (`0x8a942c`) for LeachBeam (10) or Missile (1), else `0`;
  `rec + 0x28 = 1.5` for ids 10, 1, 7 (Plasma), 5 (Cannon), else `0.75`; `+0x78 = 0`; return, no
  fire. On a failed draw it takes the normal fire path. With `+0x78 == 0` it returns without firing
  while `+0x7c > 0`.

The authored flips are `15.5 / 25.0` with scales `8.5 / 11 / 15`, so `flip * scale >= 131 > 1`: the
draw always passes. What `+0x78` / `+0x79`, `rec + 0x24` and `+0x1a` are (a turn-around request
toward the craft behind, by the numbers and the controller byte) is a hypothesis, confidence 40;
**nothing is named from it**, and it is why the Eliminator row stays unwired (ready-to-wire item 4).

## Pulse vs HD, term by term

| Term | Pulse (`weapon-ai.md`) | HD (`0x000f9a18`) | Same? |
| --- | --- | --- | --- |
| Gates `0.8` s / `0.2` s, window `0..150` ahead / `-200..0` behind, `3.0` s wait (decremented by `dt` in `Update`) | yes | yes, same constants and decrement | same |
| `0.25` s gate, never reset, decision every step | yes | yes (`0x000fb928`) | same |
| Per-weapon direction flags and skill indices | `WeaponAi_Score*` | same fields set by a jump table | assumed same, **setup unread** |
| Rate tables | `0.0005 .. 0.05`, Eliminator `0.001 .. 0.1` | same five floats each | same |
| Skill index clamp `0..4`, fire floor `1` in mode 8 | yes | yes | same |
| "Long way" product `1/((1/a)/b)` | yes | yes | same |
| Early-out on the top rate | `rand01 < rate[4]` | `rand01 < k * rate[4]` | differs by `k` |
| **Time scale** | none | `k = dt * 30` on fire, absorb and the early-out | **differs**: `0.5` at `1/60` |
| **`AllWeapons` row** | not authored | `use *= AllWeapons.use`, `absorb *= AllWeapons.absorb` on every weapon, every mode | **differs** (HD authors `0.9 / 0.9 / 1.0`) |
| Eliminator use | `use *= 5.0` | `use *= useScale[diff]` (authored `10.5 / 12.5 / 15.5`) | **differs** |
| Eliminator absorb | skill index forced `0`, `absorb` unscaled: `0.001` per call | index as set by the setup, `absorb *= absorbScale[diff]` (authored `3.6 / 2.6 / 0.6`), and only while `craft + 0x6ad8 == 0`; the index is zeroed after a fire pass | **differs** |
| Eliminator flip block | none | the block above | **new** |
| Mode 2 `use *= 2` | yes | yes | same |
| Mode gate byte (`0x009384e1` / Pulse `DAT_08ab07e3`) | gates the Eliminator terms and the `x2` | same | same |

### Effect on a race, by number (authored values; confidence follows the read, 80)

Outside an Eliminator, a forward weapon in range of the player: Pulse's law gives
`rate[i] * row.use`; HD gives `0.5 * rate[i] * row.use * 0.9` per step, i.e. **0.45 of it**, and
`0.5 * 0.9 = 0.45` of it again against another craft (`useAgainstAI`). This project runs HD on
Pulse's law with HD's `row.use` only (`oag_ai::weapon_ai`, `WeaponAiStats::all_weapons` and
`::eliminator` have no caller), so ours fires about **2.2 times** as often as the original's law
says. The 2026-10-19 spend counts (HD 175 opponent projectiles in 240 s) are therefore an upper
bound by that factor before the target-in-path gate, which both share.

In an Eliminator, `use` is `0.5 * 0.9 * useScale = 4.7 / 5.6 / 7.0` (easy / medium / hard) against
Pulse's flat `5`: easy is close, hard fires 40 % more. The absorb chance per step is
`0.5 * rate_elim[idx] * 1.0 * absorbScale` (`0.0018` at idx 0, easy) against Pulse's `0.001`.

## Cross-check (2048 and Omega)

**Omega: checked, differs; read 2026-11-03** ([ps4-omega-eu/weapon-ai-fire.md](../ps4-omega-eu/weapon-ai-fire.md),
`WeaponAi_Update` `0x01321000`, loader `0x01322f10`, both re-found on 1.07). Omega has the same skeleton
(`dt * 30`, `AllWeapons`, gates, rate tables) but its loader's defaults are the opposite of HD's (rows and scales
`1.0`, flips `0`), so a missing `EliminatorAIStats` row does **not** silence the Eliminator as the line this
section used to carry said: Omega's Eliminator (mode `8` / `0x14`, unsuffixed file) fires unscaled and never
flips. The decision is inlined in `Update`, the stats object is chosen by mode (`< 0x17` unsuffixed, else the
2048 file), and a game-mode object adds multipliers in events (unread).

**2048: checked, loader differs from HD's** (its defaults match Omega's: `0x811d75c6`); its decision code is
not read. 2048's `WeaponAIStats2048.xml` is byte-identical to HD's `DATA00` copy, so the same rows exist.

## Ready to wire (items 1 to 3 wired 2026-11-03, `hd-fire-law-wire`)

1. **Wired.** `oag_ai::weapon_ai`: add a per-step time term `k = dt * 30` (`0.5` at 60 Hz) to fire, absorb and
   the early-out, **on HD only** (Pulse and Pure keep none; Omega and 2048 inherit HD's, labelled
   inherited).
2. **Wired.** Multiply `use` by `AllWeapons.useAgainst*` and `absorb` by `AllWeapons.absorb` on HD (and
   inheriting titles).
3. **Wired**, with `DATA00`'s copy (the `DATA00`/`DATA02` question stays open) and the `+0x6ad8` byte treated as always `0` (unidentified). Eliminator on HD: replace `use *= 5` with `use *= useScale[difficulty]`, absorb with
   `rate_elim[idx] * absorb * AllWeapons.absorb * absorbScale[difficulty]` (index as the setup
   gives it, no forced `0`), and only while the craft's `+0x6ad8` byte is `0` (not identified).
   **Conditional on which `weaponaistats.xml` HD loads**: `DATA00`'s copy carries the
   `EliminatorAIStats` row (above); `DATA02`'s has `AllWeapons` but no such row, and the loader
   then leaves every scale `0` (and both flips `1.0`), so in an Eliminator `use` is `0` and an
   HD opponent would **never fire or absorb a forward weapon**. `race-modes.md` records that
   which copy the runtime reads is not established (`Archives::read_name` takes `DATA00`'s). Wire
   with `DATA00`'s, say so, and settle the question with the live breakpoint below (read the
   `+0xc4` scale at the first call).
4. **Not wired.** Do not wire the flip block until `+0x1a` and `+0x78 / +0x79` are named.
5. **Measured after wiring.** Moves two things at once, so gate with the fire-law ground truth
   (`fire_law_inherit_ground_truth.rs`) before and after: the per-decision chance is 0.45 of the old one
   (not 0.5: the `0.9` is separate), but HD's opponent projectile count does **not** fall by that factor:
   seed 1 went 110 to 157 and seeds 1 to 12 summed 1149 to 1037 (0.90), because spend is pickup-limited
   (see "Wired" below). The earlier expectation of about 0.45 was wrong.

## Wired (2026-11-03, `hd-fire-law-wire`)

`Weapons.opponent_fire` (`oag_title::weapons::OpponentFire`, `Sourced`) carries the per-title terms; `oag_raceplay::field::fire_terms` resolves them against the title's `WeaponAIstats.xml` and the race's difficulty tier into `oag_ai::weapon_ai::Law`. Pulse measured, Pure inherited from Pulse, HD measured (static, 80), 2048 and Omega inherited from HD.

Measured, `fire_law_inherit_ground_truth`, seed 1, 120 s of opponent projectiles: HD **110 to 157**, Pulse 185 and Pure 13 unchanged, 2048 122 to 128, Omega 101 (was 102). The page's "about 0.45" does not show in this count: spend is pickup-limited (about 7 fire decisions in 120 s), halving absorb as well keeps pickups held longer, and the count is dominated by which weapons get fired. Over seeds 1 to 12 the total is 1149 before, 1037 after (0.90). The per-decision chance does fall to 0.45 (`oag_ai` unit tests).

Omega: its unsuffixed table has no `EliminatorAIStats` row (its `WeaponAIStats2048.xml` does). The port keeps Pulse's Eliminator terms there (chosen, not measured); Omega's own code was read afterwards and is **not** HD's: a missing row leaves scales `1.0`, so the Eliminator fires with no `x5` (see the Omega page's ready-to-wire item).

2048 and Omega: wired by inheritance from HD. Omega's decision code is now read and differs where the row is missing (cross-check above); 2048's is unread.

## What would raise it

A live count under RPCS3: a breakpoint on `0x000fb928` (the `Update`) and `0x000f9a18` (the `rpcs3-hd-escape-gate.py` recipe, a
`serve --interpreter` instance) logging calls per second per opponent (expect the world step, 60 a
second) and `f1` (`dt`). That settles the cadence and `dt = 1/60` (70 to 90) and the virtual-slot
question. Reading `0x000fa210`'s registry hit under the watchpoint would settle the
`RaceManager + 6240` provenance, and the first call's `stats + 0xc4` settles `DATA00` versus
`DATA02`.
