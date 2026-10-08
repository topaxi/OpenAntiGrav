#!/usr/bin/env python3
"""Decode Wipeout HD's `nnet_<team>_<class>.nnt` network dumps and census them.

  hd-nnt-decode.py <dir>

`<dir>` holds the 24 `nnet_*.nnt` files from `/data/netconfigs/` in HD's
`DATA02.PSARC`, extracted with:

  scripts/psarc.py extract <image>:PS3_GAME/USRDIR/DATA02.PSARC <dir> netconfigs

Prints one line per file: layer counts, neuron and connection totals, the
parsed end offset against the file size, the weight range, whether the three
middle connection words are all zero, the set of the sixth word, and layer
0's weight pointer. The layout (little-endian, a PC training tool's dump) is
documented in docs/formats/hd-ai-netconfigs.md; `parse` is imported by
hd-nnt-forward.py.
"""

import glob
import os
import struct
import sys


def parse(data):
    """Split one `.nnt` into its header, neurons and connections."""
    (nlayers,) = struct.unpack_from("<I", data, 0)
    off = 4
    layers = [struct.unpack_from("<III", data, off + 12 * i) for i in range(nlayers)]
    off += 12 * nlayers
    counts = [count for _, _, count in layers]
    neurons_n = sum(counts)
    conns_n = sum(counts[i - 1] * counts[i] for i in range(1, nlayers))
    neurons = [struct.unpack_from("<ff", data, off + 8 * i) for i in range(neurons_n)]
    off += 8 * neurons_n
    conns = [struct.unpack_from("<ffIIIfI", data, off + 28 * i) for i in range(conns_n)]
    off += 28 * conns_n
    return {
        "layers": layers,
        "counts": counts,
        "neurons": neurons,
        "conns": conns,
        "end": off,
        "size": len(data),
    }


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    paths = sorted(glob.glob(os.path.join(sys.argv[1], "nnet_*.nnt")))
    if not paths:
        sys.exit(f"no nnet_*.nnt under {sys.argv[1]}")
    for path in paths:
        with open(path, "rb") as f:
            p = parse(f.read())
        weights = [c[0] for c in p["conns"]]
        middle_zero = {(c[2], c[3], c[4]) for c in p["conns"]} == {(0, 0, 0)}
        sixth = {c[5] for c in p["conns"]}
        print(
            os.path.basename(path),
            p["counts"],
            f"neurons {len(p['neurons'])} conns {len(p['conns'])}",
            f"end {p['end']} size {p['size']}",
            f"w[min,max]={min(weights):.3f},{max(weights):.3f}",
            f"middle_zero {middle_zero}",
            f"sixth {sixth}",
            f"layer0_wptr {p['layers'][0][1]}",
        )


if __name__ == "__main__":
    main()
