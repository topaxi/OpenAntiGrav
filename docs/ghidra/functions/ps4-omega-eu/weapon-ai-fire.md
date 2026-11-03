# Omega's opponent weapon decision and its `WeaponAIStats` loader

**Binary:** `ps4-omega-eu` base `eboot.bin` (Ghidra `/omega/eboot-ps4-omega-eu.bin`, x86-64, 8-byte
pointers, image base `0x01000000`), re-found on the 1.07 patch `eboot.bin`
(`/omega/eboot-ps4-omega-eu-v107.bin`, [patch-v107-addresses.md](patch-v107-addresses.md)). Read
2026-11-03, lane `omega-eliminator-fire-reader`: Ghidra decompile plus disassembly, **static
reading, no live capture** (Omega has no memory readout).

**Verdict.** The brief's question: what does an Omega Eliminator opponent do when its loaded
`WeaponAIstats.xml` has no `EliminatorAIStats` row? **Answer: it fires, on HD's skeleton with
scales of `1.0`.** It is neither "same as HD: no Eliminator fire" nor Pulse's `use *= 5`. Omega's
loader starts every scale at `1.0` and both flips at `0`, the opposite of HD's loader (scales `0`,
flips `1.0`), so a missing row leaves `use` and `absorb` unscaled and the flip draw can never pass.
In a Race Box Eliminator (mode `8`, loads the unsuffixed file, no game-mode object) the odds are
`dt * 30 * rate_elim[idx] * row.use * AllWeapons.use`, with no `x5`, no `x10.5`.

| Address (base) | Patch 1.07 | Name | Confidence |
| --- | --- | --- | --- |
| `0x01321000` | `0x0134a720` | `WeaponAi_Update` (the decision is inlined) | 80 |
| `0x01322f10` | `0x0134c5a0` | `WeaponAiStats_Load` | 85 |
| `0x012c8d60` | not located | `WeaponAi_Construct` | 80 |
| `0x0183d950` | not located | `g_weapon_ai_rate_eliminator` | 85 |
| `0x0183d970` | not located | `g_weapon_ai_rate` | 85 |

## The loader, `0x01322f10(stats, is2048)`

Evidence: decompile, with the field stores read against the attribute each follows.

- It opens `Data\XML\WeaponAIstats.xml`, or `Data\XML\WeaponAIstats2048.xml` when `param_2 != 0`
  (`0x01322f59` / `0x01322f3b`). Both strings are in Omega's executable, as is
  `EliminatorAIStats` (`0x018156c0`); the 1.07 loader (`0x0134c5a0`) is the same code.
- **Defaults** (stores at function entry, 8 bytes at a time): `+0x10 .. +0xb7` = `1.0` (the thirteen
  12-byte weapon rows from `+0x10`, then `AllWeapons` at `+0xac / +0xb0 / +0xb4`); **`+0xb8 .. +0xbf` = `0`
  (`normalFlip`, `infrontFlip`)**; **`+0xc0 .. +0xef` = `1.0`** (all twelve scales). Identical on 1.07.
- Fields, each store taken from the branch after its `strcasecmp`:

| Offset | Field |
| --- | --- |
| `+0x10 + 12 * id` | row: `useAgainstPlayer`, `useAgainstAI` (`+4`), `absorb` (`+8`) |
| `+0xac / +0xb0 / +0xb4` | `AllWeapons` use-vs-player, use-vs-AI, absorb |
| `+0xb8` / `+0xbc` | `normalFlip` / `infrontFlip` |
| `+0xc0 / +0xc4 / +0xc8` | easy / medium / hard flip scale |
| `+0xcc / +0xd0 / +0xd4` | easy / medium / hard **use** scale |
| `+0xd8 / +0xdc / +0xe0` | easy / medium / hard **absorb** scale |
| `+0xe4 / +0xe8 / +0xec` | easy / medium / hard score scale |

This is HD's layout shifted by `+8` (HD `+0xc4` use scale is Omega `+0xcc`; HD's loader page,
`../ps3-hdfury-eu/weapon-ai-stats.md`). The decision reads `+0xcc + level * 4`, `+0xd8 + level * 4`
and `+0xc0 + level * 4`: the loader's stores and the reader's loads agree (two sites, confidence 85).

### Which object, which file

`RaceManager_Construct` (`0x0128e570` region) registers two stats objects by CRC-32 of the name:
`"WeaponAIStats"` (length `0xd`) with `RM + 0x19188` (`0x0128e63e`, `0x0128e959`), and
`"WeaponAIStats2048"` (`0x11`) with `RM + 0x19810` (`0x0128e98f`, `0x0128ecf9`). It then calls the loader
with `[RSP+0x78]` (that first object, `0x0128ecc3`) and `is2048 = 0` at `0x0129013e`, and with
`[RSP+0x70]` (`R9 = RM + 0x19810`, `0x0128f4a6`) and `is2048 = 1` at `0x0129015f`. **Names and files are
paired, not crossed**; no `R9` write intervenes between `0x0128e98f` and `0x0128f4a6`.

`WeaponAi_Construct` (`0x012c8d60`, object `0xa0` bytes, vtable `0x019157e0`) looks the stats up by name:
**`DAT_01f999e4 < 0x17` takes `"WeaponAIStats"`, otherwise `"WeaponAIStats2048"`**, and stores the pointer
at `+0x38` (`0x012c9513`), the offset the decision reads. `0x01f999e4` is the mode id
([patch-v107-addresses.md](patch-v107-addresses.md): live `3` = Arcade, a Race Box single race). Mode `8`
is `Elimination` and `0x14` `Multiplayer Elimination` in `g_GameModeNames` (`0x01926700`, read from the
table), both `< 0x17`: **an Omega Eliminator reads the unsuffixed file**, the one `oag_omega::TITLE` names
and the one with no `EliminatorAIStats` row (`docs/gameplay/race-modes.md`, census). Modes above `0x16` read
the 2048 file, which has the row.

## The decision, `WeaponAi_Update` `0x01321000` (`(float dt, WeaponAi *self)`)

Called once a world step per weapon AI: `AIManager_Update` (`0x012c6570`) loops `mgr + 0xe0` (count) over
`mgr + 0xe8` (pointers) and does `call [vtable + 0x10]` (`0x012c6d70 .. 0x012c6d91`); the constructor appends
to that list (`0x012c9600 .. 0x012c9617`) and the vtable's third slot is `0x01321000` (slots 0 and 1 are the
destructors). Two independent reads agree, confidence 80. Unlike HD, Omega **inlines** the decision into
`Update` (HD splits `0x000fb928` / `0x000f9a18`), so there is no separate decision function to name.

Fields (Omega offsets, HD in brackets): stats `+0x38` (`+0x1c`), held id `+0x44`, held clock `+0x50`,
absorb / fire skill index `+0x68` / `+0x6c` (`+0x48` / `+0x4c`), `+0x70` / `+0x74` nearest gaps (`+0x50` /
`+0x54`), `+0x84` ahead / behind flags, `+0x88` wait timer, `+0x94` gap to the player-side craft (`+0x74`),
controller `+0x28` with fire `+0x21`, absorb `+0x23`, `+0x22` and the flip request.

```c
k      = dt * 30.0;                                 // param_1 * 30.0
E      = (mode == 8 || mode == 0x14)                // 0x01f999e4
         || (gm = DAT_01e31910, gm && (gm[+0x21c] == 4 || gm[+0x21c] == 1));
table  = E ? 0x0183d950 : 0x0183d970;               // rate_elim : rate; the same five floats as HD and Pulse
if (aimed && wait != 0 && !self.0x86 && rand01() < k * table[4]) { fire = 1; wait = 0; return; }
in_rng = (ahead && 0 < gap && gap < 150) || (behind && -200 < gap && gap < 0);
use    = in_rng ? stats[+0xac] * row.use_player : stats[+0xb0] * row.use_ai;   // AllWeapons x row
ab     = stats[+0xb4] * row.absorb;
if (gm) { use *= gm[+0x290]; ab *= gm[+0x294]; }   // game-mode object, any mode (unread)
if (mode == 2) use *= 2;
if (E) {
    if (!gm) { use *= stats[+0xcc + level*4]; ab *= stats[+0xd8 + level*4]; }  // level = DAT_01f999e0
    if (fire_idx == 0) fire_idx = 1;
    if (mode > 0x16 && self[+0x94] > 100 && self[+0x70] < self[+0x94]) fire_idx = 0;   // 2048-mode branch
}
fire_p = k * table[fire_idx];  if (fire_p != 0) fire_p = 1 / ((1/use) * (1/fire_p));   // "long way"
abs_p  = k * table[abs_idx];   if (abs_p  != 0) abs_p  = 1 / ((1/ab ) * (1/abs_p ));
r_f = rand01(); r_a = rand01();
do_abs = r_a < abs_p;  if (E && craft[+0x727c]) do_abs = false;
if (r_f >= fire_p || held_clock <= 0.8) { if (do_abs && held_clock > 0.2) absorb = 1; return; }
if (E) { abs_idx = 0; if (fire_idx == 0) fire_idx = 1; /* flip block, below, with the mode > 0x16 variant */ }
fire = 1;   // or the wait-3.0 case when aimed and nothing in the path
```

`rand01()` is `rand() * 9.313226e-10` (`2^-30`). Omega takes libc `rand()` here as a 30-bit source, the
convention of the other Omega pages (`ships-effects.md`, "Corrections to the earlier reading"); if the PS4
libc returns 31 bits every chance doubles. Unmeasured, **confidence 60** that the effective scale is the
one the code states.

### The Eliminator flip block: dead on a missing row

For modes `< 0x17`, after a passed fire roll, when the held weapon is not Bomb or Mine (id `& ~1 == 8`)
and the gap is in `(10, 150)` (Quake `(10, 400)`) or nothing is behind, with the flip byte `+0x98` set the
draw is `rand01() < flip * flipScale[level]` (`flip = infrontFlip` `+0xbc` when `+0x70 == 0`, else
`normalFlip` `+0xb8`; `flipScale` from `+0xc0 + level * 4`, or `gm[+0x550]` with a game-mode object). **With no
row both flips are `0`, the draw never passes, and the code falls through to the normal fire**. (HD's authored
`15.5 / 25.0` always pass, so HD's Eliminator opponent turns instead of firing; Omega with a row would too.)
The block before the rolls (`+0x98 == 0`, held timer `+0x9c <= 0`, craft angle outside `[-pi, pi]`: request
bytes `+0x21 / +0x22` set, return) reads no stats and is HD's unnamed turn request (confidence 40 there too).

## Omega versus HD, term by term

| Term | HD | Omega | Same? |
| --- | --- | --- | --- |
| `dt * 30`, `AllWeapons`, gates, window, rate tables, long-way product | read | same constants | same |
| Loader defaults | rows `0`, flips `1.0`, scales `0` | rows `1.0`, flips `0`, scales `1.0` | **differs** (opposite) |
| Missing `EliminatorAIStats` row | `use` and `absorb` scaled by `0`: **no Eliminator fire, no absorb** | scales `1.0`, flips `0`: **fires, unscaled; never flips** | **differs** |
| Index of the scales | `GameState + 0xdc` (difficulty) | `0x01f999e0` (level) | same slot, shifted (`mode - 4`) |
| Which file | one | unsuffixed for modes `< 0x17`, `...2048.xml` above | **new** |
| Eliminator class `E` | mode `8` / `0x14` | those, or the game-mode object's `+0x21c` in `{1, 4}` | **new** |
| Game-mode object `DAT_01e31910` | none | `+0x290` use, `+0x294` absorb on every mode, replaces the file's Eliminator scales; `+0x550` flip scale | **new**, unread (campaign / event) |
| Absorb in an Eliminator | `craft + 0x6ad8 == 0` | `craft + 0x727c == 0` | same gate (different offset) |

## What this means for the port

Where the table has no `EliminatorAIStats` row (Omega, unsuffixed), the port's Pulse terms (`use *= 5`,
absorb index forced `0`, chosen) are wrong by **5x on `use`**. Per decision an Omega Eliminator opponent
rolls `0.5 * 0.9 * rate_elim[idx]` against a held weapon's `row.use` (`0.45`, as outside an Eliminator),
and absorbs `0.5 * rate_elim[idx] * 1.0 * row.absorb` while the craft's `+0x727c` byte is clear. The game-mode
object's multipliers apply in campaign events only and are unread, so the Race Box case is complete
and the campaign case is open.

### Ready to wire (wired 2026-11-03, `omega-eliminator-fire-wire`)

Wired as `OpponentFire::OMEGA` (`eliminator_without_row: Unscaled`) in `oag_raceplay::field::fire_law::fire_terms`; 2048 carries it inherited from Omega (its file has the row, so only a missing row differs). `fire_law_inherit_ground_truth`, seed 1, 120 s under the original driver: Omega Eliminator opponent projectiles 108 before, 86 after (Pulse's `x5` gone); Omega single race 101, HD 157, 2048 128, Pulse 185, Pure 13, all unchanged.

1. `OpponentFire` gains Omega's own value (measured, static, 80): `step_scale 0.5`, `all_weapons true`, and
   an Eliminator rule "scales default `1.0` when the row is absent" instead of `authored_eliminator`'s
   Pulse fallback: in `oag_raceplay::field::fire_law::fire_terms`, when the title is Omega and
   `stats.eliminator()` is `None`, build `EliminatorTerms::Authored` with use, absorb and flip scales `1.0`
   and both flips `0` (no flip block is wired anyway), so the absorb index is the setup's (no forced `0`).
2. Wire with `oag_omega::TITLE`'s unsuffixed file; it must not take `WeaponAIstats2048.xml` for an
   Eliminator (mode `8`/`0x14`), which would give HD's `x10.5`.
3. Test: an Omega Eliminator decision with no row equals the same decision with a row whose scales are
   `1.0`; dropping the law falls back to Pulse's `x5` and the test fails. Gate with
   `fire_law_inherit_ground_truth` before and after (Omega seed 1 was 101 projectiles in 120 s).
4. Open: the game-mode object's `+0x290 / +0x294 / +0x550` (what writes them; campaign), the PS4 `rand()` width,
   and the dead-code 2048 branch (`mode > 0x16`).

## 2048 cross-check

`checked, differs from HD, same shape as Omega's loader; decision not read`. 2048's loader
(`/2048/eboot-vita-2048-eu-v104.elf` `0x811d75c6`, string xrefs from `0x811d7658` / `0x811d7f34`) stores `1.0`
into its rows and the scale fields and `0` into `+0xb0 / +0xb4`, Omega's defaults (rows from `+8`, one word
less of header). So HD's "missing row means scale `0`" is HD's alone. 2048's decision code is not read here.
2048's `WeaponAIStats2048.xml` carries the row, so the port's HD values apply there as before.

## What would raise it

A RPCS3-style live read is not available on Omega. The offline raise is the game-mode object: find the
writers of `DAT_01e31910 + 0x290 / 0x294 / 0x550` and the `+0x21c` values `1` and `4`, which would make the
campaign case readable.
