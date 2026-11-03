# 2048's opponent weapon decision: `WeaponAi_DecideFireOrAbsorb`

**Binary:** `eboot.elf` (`/2048/eboot-vita-2048-eu-v104.elf`, ARM Thumb-2, image base `0x81000000`, patch
v1.04). Read 2026-11-03, lane `2048-fire-law`: Ghidra decompile checked against the disassembly for every
stats offset (the decompiler dropped the game-mode multiplier and merged two registers, see "Decompiler
traps"). **Static reading, no live capture.**

**Verdict: same as Omega, term by term; ready to wire as it is already wired** (2048 carries
`OpponentFire::OMEGA` inherited, `eliminator_without_row: Unscaled`). 2048's own `WeaponAIStats2048.xml`
(for modes `>= 0x17`) carries the `EliminatorAIStats` row, so the "missing row" rule never fires there.
Only offsets and the game-mode object's field numbers differ from Omega's (table below).

| Address | Name | Confidence |
| --- | --- | --- |
| `0x811d59b0` | `WeaponAi_DecideFireOrAbsorb` | 80 |
| `0x811d664a` | `WeaponAi_Update` | 80 |
| `0x811d5738` | `WeaponAi_Construct` | 80 |
| `0x8100193c` | existing row (`ai-controls.md`), the Eliminator-class test | - |
| `0x8151fe68` | `g_weapon_ai_rate` | 85 |
| `0x8151fe7c` | `g_weapon_ai_rate_eliminator` | 85 |

## Evidence

- **Rate tables.** Searching the ELF for the five-float runs finds them once each: `0, 0.0005, 0.002, 0.008, 0.05`
  at `0x8151fe68` and `0.001, 0.002, 0.01, 0.02, 0.1` at `0x8151fe7c`: Pulse's, HD's and Omega's tables
  entry for entry. The decision picks the second when `0x8100193c` says Eliminator class (`0x811d59dc ..
  0x811d59ec`).
- **Eliminator class, `0x8100193c(&0x8153fc40)`.** True when the mode word (`+0xe4`) is `8` or `0x14`, or the
  game-mode object (`0x812b5890`, `GameMode_Current`) has its field `+400 == 1` or `== 4`
  (`0x812b0880`). Omega's equivalent reads `gm[+0x21c]` in `{1, 4}`.
- **Which stats file, `WeaponAi_Construct` `0x811d5738`.** Stores the stats pointer at `this + 0x1c`
  (`param_1[7]`): `"WeaponAIStats2048"` when `0x81000930` (`mode < 0x17`) is false, `"WeaponAIStats"` when
  true. That is Omega's choice with the polarity read straight: a race-box Eliminator (`8`) reads the
  unsuffixed file.
- **Cadence, `WeaponAi_Update` `0x811d664a` (vtable `0x81511fa0`, slot 2 `0x811d664b`).** `+0x6c -= dt` floored
  at `0` (the wait timer); `+0x34 += dt` with no reset (only the constructor zeroes it); once `>= 0.25`
  (`0x811d6900 ..`) the decision runs **every step**; a pending flip-request byte (`controller + 0x1e`)
  skips it. Same as HD and Omega. The caller is `AIManager_Update`'s per-object virtual call (not
  re-traced here: the vtable word and the constructor agree with HD's and Omega's shape).
- **Mode `0xd`** (`DAT_8153fd24`) has its own simpler law at the top of `Update` (HD the same); not read for
  the odds.

## The law

`k = dt * 30.0` (`vmov s16,#30.0; vmul s16,s0,s16`, `0x811d59ba .. 0x811d59cc`). Fields (2048 offset):
stats `+0x1c`, held id `+0x24`, held clock `+0x30`, absorb / fire skill index `+0x48` / `+0x4c`, nearest gaps
`+0x50` / `+0x54`, ahead / behind flags `+0x68` / `+0x69`, "no craft in the shot's path" `+0x6a`, wait timer
`+0x6c`, aimed weapon `+0x70`, gap to the player-side craft `+0x78`, flip-armed byte `+0x7c`, controller `+0x14`
with fire `+0x1d`, flip `+0x1e`, absorb `+0x1f`.

```c
k      = dt * 30.0;
E      = IsEliminatorClass();                       // 0x8100193c
table  = E ? 0x8151fe7c : 0x8151fe68;
gm     = GameMode_Current();                        // 0x812b5890
mode   = [0x8153fc40 + 0xe4];  level = [0x8153fc40 + 0xe0];
if (aimed && wait != 0 && !self.0x6a && rand01() < k * table[4]) { fire = 1; absorb = 0; wait = 0; return; }
in_rng = (ahead && 0 < gap && gap < 150.0) || (behind && -200.0 < gap && gap < 0);
use    = in_rng ? row[+8]  * stats[+0xa4] : row[+0xc] * stats[+0xa8];   // row x AllWeapons use
ab     = row[+0x10] * stats[+0xac];                                      // x AllWeapons absorb
if (gm) { use *= gm[+0x1e0]; ab *= gm[+0x1e4]; }
if (mode == 2) use *= 2.0;
if (E) {
    if (!gm) { use *= stats[+0xc4 + level*4]; ab *= stats[+0xd0 + level*4]; }
    if (fire_idx == 0) fire_idx = 1;
    if (mode >= 0x17 && gap > 100.0 && self.0x50 < gap) fire_idx = 0;
}
fire_p = k * table[fire_idx];  if (fire_p != 0) fire_p = 1 / ((1/fire_p) * (1/use));
abs_p  = k * table[abs_idx];   if (abs_p  != 0) abs_p  = 1 / ((1/abs_p)  * (1/ab));
r_f = rand01(); r_a = rand01();                    // fire first
do_abs = r_a < abs_p;  if (E && craft[+0x6128]) do_abs = false;
fire = absorb = 0;
if (r_f >= fire_p || held_clock <= 0.8) { if (do_abs && held_clock > 0.2) absorb = 1; return; }
if (E) { abs_idx = 0; if (fire_idx_was_0) fire_idx = 1; /* flip block */ }
if (aimed && no_path_clear) { wait = 3.0; fire = 0; } else fire = 1;
```

Decoded constants: `30.0` `0x41f00000`; `150.0` `0x43160000`; `-200.0` `0xc3480000`; `100.0` `0x42c80000`;
`10.0`, `400.0` (Quake window) and `0.8` / `0.2` held gates read from the compares; wait `3.0`
(`0x40400000`); flip timers `1.2` (`0x3f99999a`, LeachBeam and Missile), `1.5` (`0x3fc00000`: LeachBeam,
Missile, Plasma, Cannon), `0.75` (`0x3f400000`). `rand01` is `rand * 4.656613e-10` (`0x30000000` = `2^-31`):
**`SceLibc` `rand()` is read as a 31-bit draw**, so the draw is `[0, 1)`. (Omega's page reads `2^-30` and
doubts the PS4 width; HD masks to 30 bits. Here the constant is `2^-31` and the width is the Vita libc's,
which was not measured: confidence 65 that the draw spans `[0, 1)`.)

The per-weapon setup in `Update` (the 13-way switch that sets the flags and skill indices, then
`0x811d61a8` / `0x811d605e` helpers) is the same shape as HD's and Pulse's `WeaponAi_Score*`; **not diffed
case by case** (confidence 50, as on HD).

### Decompiler traps

1. Ghidra shows `use *= AllWeapons` only inside `if (gm != 0)`. The disassembly (`0x811d5b90 .. 0x811d5bb8`)
   always multiplies `row * AllWeapons`; with a game-mode object it additionally multiplies `gm[+0x1e0]`
   (use) and `gm[+0x1e4]` (absorb). A reader trusting the decompile would drop `AllWeapons` outside
   events.
2. The mode-2 `use *= 2.0` (`0x811d5bbc`) is absent from the decompile.

## 2048 versus Omega versus HD

| Term | HD | Omega | 2048 | Same as Omega? |
| --- | --- | --- | --- | --- |
| `dt * 30`, gates, window, rate tables, long-way product, cadence | read | same | same | yes |
| `AllWeapons` on use and absorb | yes | yes | yes (offsets `+0xa4 / +0xa8 / +0xac`) | yes |
| Eliminator scales | `use *= +0xc4`, `ab *= +0xd0` by difficulty | `+0xcc`, `+0xd8` by `0x01f999e0` | `+0xc4`, `+0xd0` by `[0x8153fc40+0xe0]` | yes (HD's layout, Omega's defaults) |
| Class `E` | modes `8`, `0x14` | those, or `gm[+0x21c] in {1,4}` | those, or `gm[+400] in {1,4}` | yes |
| Game-mode object | none | use `+0x290`, absorb `+0x294`, flip scale `+0x550` | use `+0x1e0`, absorb `+0x1e4`, flip scale `+0x43c` | yes, field numbers differ |
| File choice | one | `< 0x17` base, else `...2048` | same | yes |
| `mode > 0x16` fire-index zeroing | none | yes | yes (`0x811d5b12 .. 0x811d5b40`) | yes |
| Absorb gate in an Eliminator | `craft + 0x6ad8` | `craft + 0x727c` | `craft + 0x6128` | yes |
| Loader defaults | rows `0`, flips `1.0`, scales `0` | rows `1.0`, flips `0`, scales `1.0` | as Omega (loader `0x811d75c6`, read on the earlier page; not re-read here) | yes |
| `rand01` scale | `2^-30` | `2^-30` | `2^-31` | width-dependent, 65 |

## Port and cross-check

- **Omega: ported.** 2048 already runs `OpponentFire::OMEGA` inherited from Omega
  ([`../ps4-omega-eu/weapon-ai-fire.md`](../ps4-omega-eu/weapon-ai-fire.md)); this reading makes the
  inheritance a measurement (static, 80). No code change.
- **HD: checked, differs** only where Omega differs (loader defaults, game-mode object, file choice).
- **Open, same as Omega:** what writes the game-mode object's `+0x1e0 / +0x1e4 / +0x43c` and `+400`
  (campaign events), the Vita `rand()` width, the per-weapon setup diff. A Vita3K poll of
  `[0x8153fc40+0xe0]` and the stats pointer at the first call would settle the file choice live; none was run.
- The LeachBeam `repair="30.0"` per-trip law (the thread's item 2) was not read in this lane.
