# HD's AI netconfigs: `nnet_*.nnt` and `controlprm_*.txt`

**Status: understood, and unused by the retail game.** Read 2026-10-08 (`hd-ai-nnt` lane).

`/data/netconfigs/` in `DATA02.PSARC` holds 24 `nnet_<team>_<class>.nnt` and 24
`controlprm_<team>_<class>.txt`: twelve teams (`ag_systems`, `assegai`, `auricom`, `egx`,
`feisar`, `goteki`, `harimau`, `icaras`, `mantis`, `piranha`, `qirex`, `triakis`) by two speed
classes (`flash`, `venom`). The PSN release's `data02.psarc` ships the same 48 files, byte-identical.

**Nothing in the game reads them** (confidence 90). The executable carries a matching neural
network and a per-craft driver for it, but its only loader reads a different file
(`ainet_<name>.acn`, which does not ship), nothing calls that loader, and on a live RPCS3 race all
eight craft's drivers sat disabled with no network allocated. The evidence is on the function page,
[ai-net.md](../ghidra/functions/ps3-hdfury-eu/ai-net.md). These files are offline training output
left on the disc, so **HD's opponents follow its classic controller** (`AIControlStats.xml`,
`AIRaceStats_<class>.xml`), not a network. Porting HD's AI is reading that controller, not this.

## `.nnt`: a little-endian dump of a training tool's network

Every file is 3,468 bytes. **Little-endian**, on a big-endian console: the file was written by a PC
tool, and the PPC code cannot read it (the loader that exists expects a different, compact layout,
below).

| Offset | Size | Content |
| --- | --- | --- |
| `0x000` | 4 | `u32` layer count, `6` in every file |
| `0x004` | 6 x 12 | per layer `{u32 neurons_ptr, u32 weights_ptr, u32 count}`. The two pointers are the training tool's own heap addresses (`0x04ab7160`, `0x0590fae0`...), meaningless here; layer 0's weight pointer is `0` |
| `0x04c` | 32 x 8 | neurons, `{f32 value, f32 delta}` in layer order |
| `0x14c` | 112 x 28 | connections, `{f32 weight, f32 delta, u32 0, u32 0, u32 0, f32 -1000.0, u32 junk}` |

Topology `12-4-4-4-4-4` in all 24 files: 32 neurons, `12*4 + 4*4*4` = 112 connections, and
`4 + 72 + 256 + 3,136` = 3,468 bytes exactly. Connections are grouped by target neuron, each
group running over the previous layer's neurons in order. The last word of a connection record
holds bytes like `0x22222222`, `0x77777777`, `0x00ffffff`: uninitialised padding, not data. The
three zeros and the `-1000.0` are constant across all 2,688 records and are not named.

**The network** (the same law the EBOOT's `AiNet_Forward` computes, `0x0010a748`):

- input `x` enters as `x * 0.8 - 0.4`, output `y` leaves as `(y + 0.4) * 1.25`;
- no bias term: input 0 is the constant `1.0`, so neuron 0's stored value is `0.4` in every file;
- per neuron `s = 2 * sum(w * x)`, `y = s / (sqrt(s*s + 1) + 1)`, a sigmoid-shaped curve onto
  `(-1, 1)` built from a square root only, no `exp` or `tanh`.

**Confidence 92, from an exact-structure invariant plus a numeric one.** The byte count is exact
in 24 of 24 files; and running that law forward from each file's stored layer-0 values with its
stored weights reproduces **all 480 stored hidden and output values in 24 files to within
`8e-6`** (`data/scratch/hd-ai-nnt/tools/fwd.py`). Random weights, a different connection order or a
different activation do not come within orders of magnitude of that. The residual is not zero
because the dump was taken after a training step: the trainer updates the weights after the
forward pass that produced the stored values. Weights lie in `-1.62..1.79`, inside the trainer's
`[-20, 20]` clamp.

The runtime's own compact format, read by `AiNet_LoadWeights` (`0x0010ab10`) after seven driver
parameters in an `.acn`, is big-endian `u32 layers`, `layers x u32 count`, then one `f32` per
connection in the same order. Converting an `.nnt` into it is a byte-swap and a strip; nothing
on the disc did so.

## `controlprm_*.txt`: one line of a training log

Each file is one CRLF-terminated line, matched by
`d1=%f, d2=%f, d3=%f, bl1=%f, bl2=%f, bl3=%f -> err %f` in all 24:

```text
d1= 8.8, d2=36.3, d3=35.3, bl1=0.04, bl2=0.35, bl3=-1.17 -> err 15.08
```

`d1..d3` range `2.9..57.4`, `bl1..bl3` `-1.27..1.12`, `err` `11.54..18.25`. The driver's
`AiNetDriver_Init` defaults its first three parameters to `10, 20, 30` and
`AiNetDriver_BuildInputs` measures the distance to three points it is handed, so `d1..d3` read as
three look-ahead distances; that is a reading (60), and what `bl1..bl3` and `err` are is not
established. No code parses this text: there is no `d1=` or `controlprm` string in the EBOOT.

## Other titles

- **HD PSN:** same files, byte-identical; same dormant code.
- **Omega:** **checked, differs.** No `netconfigs`, `.nnt` or `controlprm` entry in any of its
  five base or four patch archives, and none of the network's strings in `eboot.bin`.
- **2048:** **checked, differs.** Same census over the base, v1.04 and both DLC archives: none.
  Both later titles load `AIControlStats.xml` and `AIControlStats2048.xml` and nothing like this.
- **Pulse:** has no such files; its AI is [the XML controller](../gameplay/ai.md).

## No reader

No crate reads these files, deliberately: nothing in any title consumes them, so a reader would be
code without a caller. `data/scratch/hd-ai-nnt/tools/nnt.py` is the scratch decoder that proved
the layout on all 24.
