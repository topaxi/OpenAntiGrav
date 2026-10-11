#!/usr/bin/env python3
"""Writes a cross-check table of what UnityPy reads from Unity bundles.

`oag-unity`'s ground truth (`crates/unity/tests/oracle_ground_truth.rs`) compares
its own mesh and texture reads against this table when `OAG_UNITY_ORACLE` points
at it. UnityPy is the second source: a different implementation of the same
format, used as an oracle and never as code to port.

    python3 -m venv /tmp/unitypy-venv && /tmp/unitypy-venv/bin/pip install UnityPy
    /tmp/unitypy-venv/bin/python -I scripts/unity-oracle.py data/unity/workshop-473770 /tmp/oracle.tsv

Each item file may be a bare bundle or wrap one at an offset (the signature is
searched for). Rows, tab separated:

    M item file path_id v f vn vt |v|x |v|y |v|z |n|x |n|y |n|z |uv|u |uv|v
    T item file path_id width height format mips complete_image_size
    E item file path_id class            (UnityPy itself failed on this object)

`|x|` columns are sums of absolute values over the exported OBJ's vertices, so
the exporter's axis flips do not matter.
"""

import glob
import sys

import UnityPy
from UnityPy.enums import ClassIDType


def abs_sums(rows, n):
    return [repr(sum(abs(float(r[i])) for r in rows)) for i in range(n)] if rows else ["0"] * n


def main(root, dest):
    out = open(dest, "w")
    for path in sorted(glob.glob(root + "/*/*")):
        data = open(path, "rb").read()
        at = data.find(b"UnityFS\0")
        if at < 0:
            continue
        item = path.split("/")[-2]
        for o in UnityPy.load(data[at:]).objects:
            key = [item, o.assets_file.name, str(o.path_id)]
            try:
                if o.type == ClassIDType.Mesh:
                    m = o.read()
                    lines = m.export().split("\n")
                    v = [l.split()[1:] for l in lines if l.startswith("v ")]
                    vn = [l.split()[1:] for l in lines if l.startswith("vn ")]
                    vt = [l.split()[1:] for l in lines if l.startswith("vt ")]
                    f = sum(1 for l in lines if l.startswith("f "))
                    row = ["M", *key, len(v), f, len(vn), len(vt),
                           *abs_sums(v, 3), *abs_sums(vn, 3), *abs_sums(vt, 2)]
                elif o.type == ClassIDType.Texture2D:
                    t = o.read()
                    row = ["T", *key, t.m_Width, t.m_Height, int(t.m_TextureFormat),
                           t.m_MipCount, t.m_CompleteImageSize]
                else:
                    continue
            except Exception:
                row = ["E", *key, o.type.name]
            out.write("\t".join(map(str, row)) + "\n")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
