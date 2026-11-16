#!/usr/bin/env python3
"""Converts a SOFA HRIR set into the `hrtf` crate's sphere file.

The file is: magic `HRIR`, sample rate, taps, vertex count, index count (all
u32 little endian), the triangle indices (u32), then per vertex a position
(3 x f32) and the left and right impulse responses (f32 each).

The positions are in the listener frame this engine uses: x to the listener's
right, y up, z ahead. A farthest-point subset of the measured directions keeps
the file small; the triangles are the convex hull of the kept unit vectors.

    uv run --with h5py --with numpy --with scipy python3 -I \\
        scripts/sofa-to-hrtf-sphere.py D1_48K_24bit_256tap_FIR_SOFA.sofa out.hrir 200
"""
import struct
import sys

import h5py
import numpy as np
from scipy.spatial import ConvexHull


def main(sofa_path, out_path, keep):
    with h5py.File(sofa_path, "r") as f:
        irs = np.array(f["Data.IR"])  # [M, 2, N]
        rate = int(np.array(f["Data.SamplingRate"]).ravel()[0])
        pos = np.array(f["SourcePosition"])  # azimuth, elevation (deg), radius
    az = np.radians(pos[:, 0])
    el = np.radians(pos[:, 1])
    # SOFA: x ahead, y left, z up.
    sofa_xyz = np.stack(
        [np.cos(el) * np.cos(az), np.cos(el) * np.sin(az), np.sin(el)], axis=1
    )
    # This engine: x right, y up, z ahead.
    dirs = np.stack([-sofa_xyz[:, 1], sofa_xyz[:, 2], sofa_xyz[:, 0]], axis=1)

    chosen = [int(np.argmax(dirs[:, 2]))]
    nearest = np.full(len(dirs), -2.0)
    for _ in range(keep - 1):
        nearest = np.maximum(nearest, dirs @ dirs[chosen[-1]])
        chosen.append(int(np.argmin(nearest)))
    dirs = dirs[chosen]
    irs = irs[chosen]

    hull = ConvexHull(dirs)
    faces = []
    for simplex in hull.simplices:
        a, b, c = dirs[simplex]
        if np.dot(np.cross(b - a, c - a), a + b + c) < 0:
            simplex = simplex[[0, 2, 1]]
        faces.extend(int(i) for i in simplex)

    taps = irs.shape[2]
    with open(out_path, "wb") as out:
        out.write(b"HRIR")
        out.write(struct.pack("<IIII", rate, taps, len(dirs), len(faces)))
        out.write(struct.pack(f"<{len(faces)}I", *faces))
        for d, ir in zip(dirs, irs):
            out.write(struct.pack("<3f", *map(float, d)))
            out.write(ir[0].astype("<f4").tobytes())
            out.write(ir[1].astype("<f4").tobytes())
    print(f"{len(dirs)} points, {len(faces) // 3} faces, {taps} taps at {rate} Hz")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], int(sys.argv[3]))
