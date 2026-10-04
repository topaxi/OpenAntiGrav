# The two random number generators

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`, language `Allegrex:LE:32:default`.

**The names below are applied**, from [names.tsv](names.tsv) via
`just apply-names`. Per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md), applying a
rename needs a page carrying its evidence; this page is that evidence. Library
functions keep their library names, the same convention `powf` and `strtod`
already follow on [particle-system.md](particle-system.md) and
[xml-reader.md](xml-reader.md). Nothing here is under 70, so nothing carries a
`_q`.

Pulse ships **two** generators with **disjoint consumers**, and they scale
their output differently. That is the finding: not one algorithm but a
gameplay generator and a particle generator, and a reimplementation that
reaches for the wrong one gets the wrong range, not merely the wrong sequence.

| Generator | Draws for | Algorithm | Seeded by | Output scale used by callers |
| --- | --- | --- | --- | --- |
| libc `rand` | AI decisions, pickup rolls, opponent shuffle, HUD, loading screen | 32-bit LCG, `s = s * 1103515245 + 12345`, returns `s & 0x7fffffff` | `srand(sceKernelGetSystemTimeWide())`, once per race set-up | `* 4.656613e-10` = **2^-31** (callers do it themselves) |
| Particle RNG (RANROT-B) | Particle systems, clouds, mine drift, Shuriken roll, Zone height jitter | 17-word rotate-and-add lagged generator | `PsysRng_Seed(0)` when the particle manager is built; reseeded from `rand()` by every cloud group | **2^-32**, applied inside `PsysRng_Next` |

Crossing the two is the obvious future bug: `rand()` returns a 31-bit integer
and every caller scales it by 2^-31 to land in `[0, 1)`; `PsysRng_Next`
already returns a float in `[0, 1]`. Neither generator is the placeholder
`oag_core::Rng` uses (`xoshiro128**`), and neither should replace it - see
"Consequences for the reimplementation" below.

## Summary

| Address | Name | Kind | Conf |
| --- | --- | --- | ---: |
| `0x089731c4` | `rand` | function | 95 |
| `0x089731b4` | `srand` | function | 95 |
| `0x088f8b60` | `PsysRng_Next` | function | 92 |
| `0x088f8c98` | `PsysRng_Seed` | function | 92 |
| `0x088f8d30` | `PsysRng_Reseed` | function | 90 |
| `0x088f8b48` | `Bits_RotateLeft` | function | 95 |
| `0x0897e26c` | `ldexpf` | function | 85 |
| `0x08b620a8` | `g_psys_rng_buffer` (17 x u32) | data | 92 |
| `0x08b620ec` | `g_psys_rng_p1` | data | 92 |
| `0x08b620f0` | `g_psys_rng_p2` | data | 92 |
| `0x08b620f4` | `g_psys_rng_scale` | data | 92 |

`Psys_RandIntRange` (`0x088f8c3c`), `Psys_RandFloatRange` (`0x088f8d94`) and
`Psys_RandSpread` (`0x088f8dd0`) were already named on
[particle-system.md](particle-system.md); they are the three range helpers
over `PsysRng_Next` and keep their rows there.

## `rand` (`0x089731c4`) and `srand` (`0x089731b4`) - the gameplay generator

Confidence **95**. Byte-for-byte the classic ANSI C generator as newlib
shipped it in the PSP SDK's libc:

```c
uint rand(void) {
    uint s = *(uint *)(_impure_ptr + 0x58);   // PTR_DAT_08ac29d4 is _impure_ptr
    s = s * 0x41c64e6d + 0x3039;              // 1103515245, 12345
    *(uint *)(_impure_ptr + 0x58) = s;
    return s & 0x7fffffff;
}
void srand(uint seed) { *(uint *)(_impure_ptr + 0x58) = seed; }
```

Evidence:

- The multiplier `0x41c64e6d` (`lui a1,0x41c6 / ori a1,a1,0x4e6d`) and
  increment `0x3039` are the constants every textbook lists for this LCG,
  and the `& 0x7fffffff` is `RAND_MAX`. There is no shift: the low bits are
  returned, which is why `rand() % n` in this binary has the short low-bit
  cycles this generator is notorious for (bit 0 alternates every call).
- The state lives at `+0x58` of the block `PTR_DAT_08ac29d4` points at. The
  same pointer is read by `strtod` (`0x08974930`), `strtol` (`0x089792bc`)
  and the `errno` accessor `FUN_08a2a5d8`, which is newlib's `_impure_ptr`
  and its per-thread reentrancy struct. `_rand_next` at `+0x58` is where
  newlib's `struct _reent` of that era keeps it.
- `srand` has exactly **one** caller in the whole binary: `FUN_08821bd4`, the
  local-race grid set-up (it runs only when the mode index `DAT_08b31048`
  is below `0xe`, i.e. not a network game - the same mode global
  [zone-mode.md](zone-mode.md) and [pads.md](pads.md) switch on). It seeds
  with `sceKernelGetSystemTimeWide()` truncated to 32 bits, then draws
  `rand()` to shuffle the seven opponents it just picked from the roster:
  for each slot `i`, swap it with slot `rand() % count` (the naive
  every-slot-with-any-slot shuffle, not Fisher-Yates - `count` stays 7).
- 33 callers, and they are the gameplay and front-end code: the pickup
  weighting (`WeaponPickup_Grant`, [pickups.md](../../../gameplay/pickups.md)),
  the weapon AI's fire/absorb rolls (`WeaponAi_Update`,
  `WeaponAi_DecideFireOrAbsorb`, [weapon-ai.md](weapon-ai.md)), the AI stats
  draw ([ai-stats.md](ai-stats.md)), `Hud_Update`, the loading screen's wave
  impulse and tip pick (`Loading_DrawWave`, `Loading_Show`,
  [loading-screen.md](loading-screen.md)), and `CloudGroup_Init` (below).
  Every one of the float consumers already documented scales the result by
  `4.656613e-10`, i.e. 2^-31, to reach `[0, 1)`.

**The consequence is that no gameplay sequence is reproducible.** The seed is
the wall clock at race set-up, so the same inputs on the same track produce a
different pickup and AI sequence on every run of the original. Only the
*distribution* of those draws can ever be checked, which is what
[pickups.md](../../../gameplay/pickups.md) already concluded from the other
direction; this page supplies the reason.

## `PsysRng_Next` (`0x088f8b60`) - the particle generator

Confidence **92**. A 17-word additive lagged generator with a rotation on
each tap - Agner Fog's **RANROT type B** shape, with every structural constant
matching his published implementation (buffer length 17, second pointer
starting at 10, seed fill `s = rotl(s, 5) + 97`, 300 discarded draws after
seeding, output scaled by `ldexp(1, -32)`). Read off the disassembly:

```c
uint32 g_psys_rng_buffer[17];   // 0x08b620a8
int    g_psys_rng_p1;           // 0x08b620ec, starts at 0
int    g_psys_rng_p2;           // 0x08b620f0, starts at 10
float  g_psys_rng_scale;        // 0x08b620f4, ldexpf(1.0f, -32) = 2^-32

float PsysRng_Next(void) {
    uint32 x = Bits_RotateLeft(g_psys_rng_buffer[g_psys_rng_p2], 5)
             + Bits_RotateLeft(g_psys_rng_buffer[g_psys_rng_p1], 3);
    g_psys_rng_buffer[g_psys_rng_p1] = x;
    if (--g_psys_rng_p1 < 0) g_psys_rng_p1 = 16;
    if (--g_psys_rng_p2 < 0) g_psys_rng_p2 = 16;
    float f = (float)(int)x;
    if ((int)x < 0) f += 4294967296.0f;      // unsigned conversion by hand
    return f * g_psys_rng_scale;
}
```

`Bits_RotateLeft` (`0x088f8b48`) is `x << (n & 31) | x >> ((32 - n) & 31)`, a
plain rotate-left; the rotation amounts are the immediates `li a1,0x5` at
`0x088f8b98` and `li a1,0x3` at `0x088f8bb8`, and which tap gets which is
fixed by the loads: `p2`'s word (`lw a0,0x20f0(s1)`) is rotated by 5 and
`p1`'s (`lw a0,0x20ec(s0)`) by 3, and the sum is stored back through `p1`
(`sw a0,0x0(a2)` at `0x088f8bcc`, `a2` computed from the `0x20ec` reload).
The shipped rotation pair `(5, 3)` was not checked against the defaults in
Fog's published source; whether it is his or a Studio Liverpool choice does
not change the reading.

**The range is `[0, 1]`, closed at the top.** The unsigned-to-float
conversion goes through `cvt.s.w` and then adds `2^32` when the word was
negative, so a word within `2^8` of `2^32` rounds to exactly `2^32` in
single precision and the product is exactly `1.0`. That is why
`Psys_RandIntRange` carries the clamp `if (i >= interval) i = interval - 1`
(and `Psys_RandFloatRange(a, b)` can return exactly `b`). A port that uses
an exact `u32 as f32 * 2^-32` reproduces this; one that computes
`(x >> 8) as f32 * 2^-24` does not.

### `PsysRng_Seed` (`0x088f8c98`) and `PsysRng_Reseed` (`0x088f8d30`)

Both fill the buffer identically:

```c
for (i = 0; i < 17; i++) { g_psys_rng_buffer[i] = seed; seed = Bits_RotateLeft(seed, 5) + 0x61; }
g_psys_rng_p1 = 0;
g_psys_rng_p2 = 10;
```

`PsysRng_Seed` then discards 300 draws (`slti` against `0x12c`) and sets
`g_psys_rng_scale = ldexpf(1.0f, -32)`; `PsysRng_Reseed` does neither. So
the scale is written exactly once per process and the reseed path relies on
it already being there. `ldexpf` (`0x0897e26c`) is identified by shape:
newlib's wrapper that calls the raw `scalbnf` (`FUN_08980274`) guarded by
`finitef` (`FUN_08980194`) on both sides and writes `errno = 0x22` (`ERANGE`)
through `FUN_08a2a5d8` on overflow - confidence 85 because the identification
is by structure, not by an import NID.

Who seeds:

- `PsysRng_Seed(0)` - **the literal constant zero** - from the particle
  manager's constructor `FUN_088f2f74` (the `ParticleManager.cpp` allocation
  at `0x50800` bytes, 64 slots of `0x1420`). So the particle stream starts
  from the same state on every boot; the sequence only diverges from run to
  run through *how many* draws each emitter has consumed, which depends on
  frame timing and on the clouds below.
- `PsysRng_Reseed(rand())` from `CloudGroup_Init` (`0x08933048`), when the
  node's authored `Seed` is `0`, immediately followed by
  `Psys_RandIntRange(1, 9999)` to pick the seed the group keeps. Then
  `FUN_08933c1c` (the cloud group's build - counts sprites across its cubes,
  allocates the `0x20`-stride record array, populates it, and calls
  `CloudGroup_BuildDisplayList`) calls `PsysRng_Reseed(group->Seed)` at
  `0x08933cc4`, **right before** the per-sprite phase, rate and variant
  draws [clouds.md](clouds.md) documents.

That second point matters in both directions. **Given its `Seed`, a cloud
group's jitter is reproducible** - the reseed happens immediately before the
draws that use it, so an authored non-zero `Seed` yields the same sprites on
every boot, and the `(1, 9999)` re-roll is the only clock-derived input. And
**building a cloud group clobbers the shared particle stream**: every group
constructed mid-run resets the buffer every emitter draws from, so the
particle sequence after a track load is a function of the cloud groups on
that track, not of the boot-time `Seed(0)` alone.

### Consumers

`PsysRng_Next` has four callers: the three range helpers on
[particle-system.md](particle-system.md) and `Weapon_FireShuriken`
(`0x08870240`), whose direct `rand01()` roll [shuriken.md](shuriken.md)
already records. Through the helpers: every emitter parameter draw in the
particle system, the cloud sprite jitter, the mine cluster's random unit
drift ([mine.md](mine.md)), and Zone's `rand(-10, 20)` height offset
([zone-mode.md](zone-mode.md)).

## Cross-platform

| Platform | `rand` | `PsysRng_Seed` / `Reseed` | Notes |
| --- | --- | --- | --- |
| PSP (Pulse, USA) | `0x089731c4` | `0x088f8c98` / `0x088f8d30` | This page |
| PS2 (Pulse, EU) | `0x0025c3b0` | `0x001cbff0` / `0x001cc090` | Same LCG constants, same `_reent+0x58` state, same 17-word fill with `rotl 5 + 0x61`, `p1 = 0`, `p2 = 10`, 300 warm-up draws (`iVar1 = 299` counting down), scale `ldexp(1, -32)` computed in double. Buffer at `0x002f8a60`, pointers at `0x002f8aa4`/`0x002f8aa8`, scale at `0x002f8aac`. |
| PSP (Pure, EU) | `0x088dfe1c` | `0x0882f438` / `0x0882f4dc` | Identical to Pulse in both generators. Buffer at image-relative `0x00054fc8` (`0x08858fc8` absolute), pointers `0x0005500c`/`0x00055010`, scale `0x00055014`. Pure computes the scale in double (`FUN_088d9ae0`, then a double-to-float `FUN_088ebd68`) where Pulse calls `ldexpf` directly. |

Both corroborations were found the same way: `search_instructions` for
`lui` with operand `0x41c6` returns exactly one hit per binary (the LCG
multiplier's upper half), and `addiu` with operand `0x61` inside a 17-count
loop finds the two seed fills.

## Consequences for the reimplementation

- **Do not swap `oag_core::Rng`'s algorithm for either of these.** The
  gameplay generator is clock-seeded in the original, so a faithful port
  cannot make a sequence match anyway; the determinism rules
  ([determinism.md](../../../architecture/determinism.md)) forbid the seed
  source; and a 31-bit LCG returning its low bits is a worse generator than
  the placeholder. What the placeholder's doc comment promised - "replace the
  algorithm once recovered" - turns out to be the wrong promise, and the
  comment now says so.
- **If a particle effect ever needs to match the original frame-for-frame**,
  the thing to port is `PsysRng_Next` as its own type on the render side
  (`oag-render` is exempt from the determinism scan), seeded from the cloud
  group's `Seed` the way `FUN_08933c1c` does. That is a rendering-fidelity
  decision for M6, and it needs an ADR because it introduces a second
  generator; it is not part of the M2 checkbox this page closes.
- **Distribution tests remain the only tests.** `pickups.md`'s weighted
  draw and `weapon-ai.md`'s fire probabilities are checked against the
  authored weights, and this page is the evidence that nothing stronger is
  available.

## Open questions

- Whether `(5, 3)` is one of Fog's published RANROT-B parameter sets or a
  local choice. Cosmetic: the reading does not depend on it.
- `FUN_08821bd4`, the only `srand` caller, is read here only far enough to
  see the seed and the shuffle. It is the opponent-roster picker for a local
  race and deserves its own reading on [grid.md](grid.md); it is not named
  here because the roster-selection half (`FUN_08821dd4`, `FUN_088893e8`)
  was not read.

## History

- 2026-09-16: first reading. Both generators identified statically from the
  USA `BOOT.BIN`, corroborated on the PS2 EU executable and Pure EU
  `BOOT.BIN`. No runtime leg, which caps the particle generator at 92: the
  algorithm is read instruction by instruction, but no buffer dump from a
  running game has been compared against a port of it.
- 2026-10-04 (`pulse-clouds`): the runtime leg for the particle generator. A
  numpy port of `PsysRng_Reseed`/`PsysRng_Next` and the two range helpers,
  fed each `cloudGroup`'s seed out of a running original's RAM, reproduced
  every record those groups drew (369 per boot, four boots; see
  [clouds.md](clouds.md)). The port this page's Consequences section
  anticipated now exists as `oag_render::ranrot`, under
  [ADR-0056](../../../architecture/adr/0056-a-render-side-port-of-the-particle-generator.md).
  The rows above keep their confidences; the runtime match would support
  raising them.
