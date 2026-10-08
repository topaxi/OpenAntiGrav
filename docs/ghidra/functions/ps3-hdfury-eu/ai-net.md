# AI neural network: present in HD's EBOOT, dormant in retail

**Binary:** `ps3-hdfury-eu` `EBOOT.elf` (PowerPC, 32-bit pointers, TOC `r2 = 0x8ad4d8`).
Read 2026-10-08 for the `hd-ai-nnt` lane: Ghidra (`/hdfury/EBOOT-ps3-hdfury-eu.elf`), capstone
for every AltiVec stretch Ghidra truncates, a whole-text `bl`/`b` scan, and one live RPCS3 race.
The data side is [hd-ai-netconfigs.md](../../../formats/hd-ai-netconfigs.md).

## The finding

**HD's opponents are not driven by a neural network in the retail game (confidence 90).**
The EBOOT carries a complete small feed-forward network with an in-engine trainer and a
per-craft "network driver" object, but:

1. **Nothing names the shipped files.** No string in `EBOOT.elf`, the PSN build's EBOOT, or the
   decrypted `DFEngine.prx` contains `nnt`, `netconfig`, `controlprm` or `nnet_` (case-insensitive
   byte search of each file; `DFEngine` is the Double Fusion in-game advertising library).
2. **The only file loader reads a different, unshipped file and is never called.**
   `AiNetDriver_LoadAcn` (`0x0010b760`) opens `<dir>\ainet_<name>.acn`; no `.acn` exists in any of
   HD's seven archives (`scripts/psarc.py list`, 11,664 entries). It has no caller: no `bl` or `b`
   anywhere in the text segment targets it, and its OPD descriptor (`0x00875518`) appears nowhere
   in the file as a 32-bit word, so it is not reached through a function pointer either.
   `AiNetDriver_SetName` (`0x0010b8a8`, the `sprintf` that builds that path) is uncalled the same way.
3. **The enable byte is never set.** `AiNetDriver_GetControls` (`0x0010be38`) runs the network only
   when byte `+0` of the driver is non-zero (`lbz r0,0(r3)` at `0x10be48`). `AiNetDriver_Init`
   (`0x0010b978`) clears it; no store elsewhere was found.
4. **Measured live (RPCS3, 2026-10-08, one boot, Racebox race, 8 craft, 16 s in, lap 1 of 3):**
   a scan of the heap around the craft array (`0x98d7c0`) for the driver's seven init defaults
   found **eight drivers in the scanned window (the race had eight craft)**, and on every one: enable byte `0`, network
   neuron and connection pointers `0` (never allocated), and the parameters still at
   `AiNetDriver_Init`'s defaults `10, 20, 30, 0.9, 0.5, -0.2, 0.5`, not any `controlprm_*.txt`
   value. Script `data/scratch/hd-ai-nnt/tools/probe.py`, result
   `data/scratch/hd-ai-nnt/live/controllers.json`, frame `live/race.png`.

Why 90 and not higher: one boot, one mode (Racebox single race). A Campaign or Zone race was not
sampled. The static case (no caller, no file, no name) does not depend on mode.

## Functions

The network object ("net", `0x100`-plus bytes) is embedded twice in the driver, at `+0x12c` and
`+0x298`. Neurons and connections are 8-byte `{value, delta}` / `{weight, delta}` pairs.

| Address | Name | Confidence | Evidence |
| --- | --- | --- | --- |
| `0x0010a368` | `AiNet_SetInputs` | 85 | `fmsubs f0,f0,f13,f12` at `0x10a3b0`: `in = x * 0.8 - 0.4` (TOC `-0x3bf0` = `0.8`, `-0x3bec` = `0.4`), stored at `net + 0xe8 + 4i` for `net+0x18` inputs |
| `0x0010a748` | `AiNet_Forward` | 85 | copies `net+0xe8..` into layer 0's neurons, then per layer `s = 2 * sum(w * x)`, `y = s / (sqrt(s*s + 1) + 1)`; copies the last layer to `net + 0x128` |
| `0x0010ac90` | `AiNet_GetOutputs` | 85 | `out = (net[0x128 + 4i] + 0.4) * 1.25` (TOC `-0x3bc0`, `0x8a9918` = `1.25`), the inverse of the input map |
| `0x0010a938` | `AiNet_Allocate` | 80 | from a `{nlayers, counts[]}` header: sums neurons into `+0xd8` and `prev * cur` connections into `+0xdc`, allocates `8 *` each, fills the 12-byte layer records at `+0x10` |
| `0x0010ab10` | `AiNet_LoadWeights` | 80 | reads `u32 nlayers`, `nlayers x u32 count`, then `+0xdc` `f32` weights into the connection array, zeroing every delta |
| `0x0010a480` | `AiNet_Free` | 80 | frees `+0xd4` and `+0xe0`; called four times from the AI object's teardown (`0xfeda8..0xfee14`) |
| `0x0010a530` | `AiNet_PrintStats` | 90 | `"Network has %d neurons and %d connections"`, `"Connection weights : max= %f, min= %f, mean= %f"`, `"%f %% of weights have very small values"`; uncalled |
| `0x0010ad18` | `AiNet_PrintResponses` | 75 | for each input `i`, a one-hot input vector, a forward pass with `abs(w * x)`, and `"   Output %d : %f"` per output; uncalled |
| `0x0010b150` | `AiNet_TrainStep` | 75 | forward pass, output error `(target * 0.8 - 0.4) - y` and its square summed into `net+0`, then backpropagation with derivative `2 / (q*q + q) + 0.1` (`q = sqrt((2v)^2 + 1)`, `v` the neuron's stored output), learning-rate and momentum arguments in `f1`/`f2`, a weight-decay term scaled by `0.000125`, weights clamped to `[-20, 20]`; uncalled |
| `0x0010b978` | `AiNetDriver_Init` | 80 | clears both nets (`0x10a350`), clears the enable byte, sets `+0x110..+0x128` to `10, 20, 30, 0.9, 0.5, -0.2, 0.5`; called from the two AI-object constructors (`0xfeed8`, `0xff3e8`) with `this + 0x3f0`. Live: those exact seven values on all eight drivers |
| `0x0010b8a8` | `AiNetDriver_SetName` | 80 | `sprintf(this + 0x404, "%s\\ainet_%s", a, b)` (string `0x7830d8`); uncalled |
| `0x0010b760` | `AiNetDriver_LoadAcn` | 80 | appends `".acn"` (string `0x7830d0`) to `this + 0x404`, loads the file (`0x322fc8`), copies seven `f32` to `+0x110..+0x128`, hands the rest to `AiNet_LoadWeights(this + 0x12c, data + 0x1c)`; uncalled |
| `0x0010be38` | `AiNetDriver_GetControls` | 80 | see below; called once, from the AI update at `0x10424c` |
| `0x0010ba08` | `AiNetDriver_BuildInputs` | 65 | sixteen floats from the driver's state; layout below, read off capstone (Ghidra halts at the first `lvx`) |

`0x0010b8e8` is a byte-identical copy of `AiNetDriver_Init` with no caller; left unnamed.

## `AiNetDriver_GetControls` (`0x0010be38`)

Read with capstone (`data/scratch/hd-weapon-blasts/ppcdis.py 10be38 208`); Ghidra's version reads
the stack outputs as globals.

- **Enable byte clear (the retail path, measured live):** return `this+0xb4` in `f1`, and write
  `this+0xb8` and `this+0xbc` to the two out-pointers. Those three are written by `0x0010b6e0`
  (`stfs f1..f3, 0xb4..0xbc`, called at `0x103d2c` with the AI's own `f27..f29`). Live they read
  `0.0` on six of the eight drivers 16 s into a race, so that hand-through is not the path the
  rivals were steering by at that moment either; what the caller at `0x104254..0x104264` does with
  them was not read.
- **Enable byte set (never in retail):** `AiNetDriver_BuildInputs`, `AiNet_SetInputs`,
  `AiNet_Forward`, `AiNet_GetOutputs`, then:
  - return `clamp(out0 * 165 - 15, -100, 100)` (`fmsubs` at `0x10bff4`, `fsel` pair at `0x10c030`);
  - `a = out1 * 200`, `b = out2 * 200`: the larger of the two is kept, zeroed below `20`, capped
    at `100`, and the other is written as `0` (`0x10bf20..0x10bfe8`). One output of a mutually
    exclusive pair is the shape of left/right airbrake, which is a reading, not a measurement;
  - `out3` is computed and never read.

  `out0..out2` therefore encode the same three quantities `0x0010b6e0` stores from the classic
  controller (`+0xb4`, `+0xb8`, `+0xbc`), with the same scales `AiNetDriver_BuildInputs` uses to
  feed those three back in (`in[12..14]`). With `AiNet_TrainStep`'s target-error code, that is the
  shape of a network trained offline to imitate the classic controller. A reading, consistent in
  four places; never observed running.

## `AiNetDriver_BuildInputs` (`0x0010ba08`), confidence 65

Writes sixteen floats; `AiNet_SetInputs` consumes the first `net+0x18` of them, so a 12-input
network sees `in[0..11]`. `r5 = this + 0x30`.

| Slot | Content |
| --- | --- |
| `in[0]` | `1.0` (`lis r0,0x3f80; stw r0,0(r4)`): the bias, which is why the network has no bias term |
| `in[1]` | the length of the vector at `this+0x80` (`vrsqrtefp` plus one Newton step) times `1/300` (TOC `-0x3b7c` = `0.00333`) |
| `in[2..5]` | dot products of the vectors at `this+0x80` and `this+0xa0` with the driver's frame vectors, mapped `(d + 100) * 0.005`, `(d + 4) * 0.125`, `(d + 2.5) * 0.2`, `(d + 2.5) * 0.2` |
| `in[6..8]` | distance from `this+0x40` to each of three points at `+0xd0`, `+0xe0`, `+0xf0`, times `1/300` |
| `in[9..11]` | a dot product per point, mapped `d * 0.5 + 0.5` |
| `in[12..14]` | `(this+0xb4 + 15) / 165`, `this+0xb8 / 200`, `this+0xbc / 200` (`r5+0x84..0x8c`, TOC `0.00606`, `0.005`): the classic controller's three outputs, mapped by **the exact inverse** of how `out0..out2` are decoded above. A 12-input network never sees them |
| `in[15]` | `0.5` |

The three points are written by `0x0010b6f8` (`stvx` at `+0xd0/+0xe0/+0xf0`, count `3` at `+0x100`),
and the three default distances `10, 20, 30` at `+0x110..+0x118` match `controlprm`'s `d1, d2, d3`:
a reading of "three look-ahead points along the line" that is consistent but not measured, since the
path never runs.

## Not read

- `0x0010c040` (called at `0x103d14`): copies four vectors into the driver; Ghidra halts at the
  first `vperm`. Not needed for the finding.
- What actually steers HD's rivals. The strings `Data\XML\AIControlStats.xml`,
  `AIRaceStats_<class>.xml` and `LookAhead` beside them (`0x780c31`) point at the classic
  controller; that is the next lane, not this one.

## Corroboration

- **HD PSN (`/hdpsn/EBOOT-ps3-hdpsn-eu.elf`):** the same strings (`%s\ainet_%s`, `.acn`,
  `Network has %d neurons...`) and the same `AIControlStats.xml`, and no `nnt`/`netconfig`/`controlprm`
  string. Its `data02.psarc` ships the same 48 netconfig files, byte-identical to the disc's.
- **Omega (`eboot-ps4-omega-eu.bin`) and 2048 (`eboot-vita-2048-eu-base`/`-v104`):** none of
  `ainet`, `.acn`, `Network has %d neurons`, `nnt`, `netconfig`, `controlprm`. Both carry
  `AIControlStats.xml` and `AIControlStats2048.xml` instead. Neither ships a netconfig file
  (census of all five Omega base and four patch archives, and 2048's base, v1.04 and both DLC
  archives).
