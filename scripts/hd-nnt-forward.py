#!/usr/bin/env python3
"""Re-run each HD `.nnt` network forward and compare with its stored activations.

  hd-nnt-forward.py <dir>

`<dir>` is the same extracted `netconfigs` directory hd-nnt-decode.py reads.
From each file's stored layer-0 values and stored weights, computes every
hidden and output neuron with the law HD's `AiNet_Forward` (`0x0010a748`)
uses, in `f32`:

  s = 2 * sum(w * x),  y = s / (sqrt(s*s + 1) + 1)

and prints the largest difference from the stored value per file, then the
worst over all files and the set of neuron 0's stored values. The result
quoted in docs/formats/hd-ai-netconfigs.md is all 480 activations in 24 files
within 8e-6. Needs numpy (for `f32` rounding at every step).
"""

import glob
import importlib.util
import os
import struct
import sys

import numpy as np

_spec = importlib.util.spec_from_file_location(
    "hd_nnt_decode", os.path.join(os.path.dirname(os.path.abspath(__file__)), "hd-nnt-decode.py")
)
_decode = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_decode)

f32 = np.float32


def activation(s):
    return f32(s / (f32(np.sqrt(f32(s * s + f32(1)))) + f32(1)))


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    paths = sorted(glob.glob(os.path.join(sys.argv[1], "nnet_*.nnt")))
    if not paths:
        sys.exit(f"no nnet_*.nnt under {sys.argv[1]}")
    worst = 0.0
    neuron0 = set()
    for path in paths:
        with open(path, "rb") as f:
            p = _decode.parse(f.read())
        values = [f32(v) for v, _ in p["neurons"]]
        weights = [f32(c[0]) for c in p["conns"]]
        counts = p["counts"]
        starts = [0]
        for count in counts:
            starts.append(starts[-1] + count)
        wi = 0
        errs = []
        for layer in range(1, len(counts)):
            prev = values[starts[layer - 1] : starts[layer]]
            for j in range(counts[layer]):
                s = f32(0)
                for x in prev:
                    s = f32(s + f32(x * weights[wi]))
                    wi += 1
                y = activation(f32(s + s))
                errs.append(abs(float(y) - float(values[starts[layer] + j])))
        neuron0.add(struct.pack("<f", values[0]).hex())
        worst = max(worst, max(errs))
        exact = sum(1 for e in errs if e == 0)
        print(f"{os.path.basename(path)} max|err| {max(errs):.3g} exact {exact}/{len(errs)}")
    print("files", len(paths), "worst", worst, "neuron0 values", neuron0)


if __name__ == "__main__":
    main()
