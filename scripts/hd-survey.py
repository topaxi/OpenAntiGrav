#!/usr/bin/env python3
"""Measure Wipeout HD / Fury's assets against Pulse's own format readings.

  hd-survey.py <hd-image> [pulse-image]
  hd-survey.py --match <hd-image> <pulse-image> <pure-image>

Every number `docs/formats/hd-status.md` quotes comes out of this script, so a
reader can re-derive the page rather than trust it. The HD image must already be
layer-1 decrypted (scripts/ps3iso.py); the Pulse argument is optional and only
feeds the control column, which is what says a check is measuring HD rather than
measuring the check.

`--match` is the separate question of which HD circuit *is* which PSP circuit.
It scores every HD spline against every candidate by mean distance to the
nearest control point, which does not care how either side resampled the curve.

Nothing here is a parser this project ships. It is a deliberately separate,
throwaway reading of the same layouts `oag-formats` implements, in the opposite
byte order - which is the point: if the two agree, the layout claim is about the
format and not about one implementation.
"""

import os
import struct
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from psarc import open_archives  # noqa: E402

# Pulse's version-6 class table, as docs/formats/vex.md records it. A class this
# does not name is one HD authors and Pulse's table does not carry.
CLASS = {
    0x06E: "Transform", 0x0F4: "World", 0x0F7: "Camera", 0x123: "NurbsSurface",
    0x125: "Mesh", 0x12C: "AmbientLight", 0x131: "DirectionalLight",
    0x132: "PointLight", 0x2EE: "LodGroup", 0x3B9: "Floor Collision",
    0x3BA: "Wall Collision", 0x3BB: "WO Track", 0x3BC: "Start Position",
    0x3BD: "Speedup Pad", 0x3BE: "Weapon Pad", 0x3BF: "Engine Flare",
    0x3C0: "Anim Transform", 0x3C1: "Texture", 0x3C2: "Dynamic Point Light",
    0x3C3: "Dynamic Shadow Occluder", 0x3C4: "ParticleSystem", 0x3C5: "Airbrake",
    0x3C6: "Skycube", 0x3C7: "Quake", 0x3C8: "Trail", 0x3C9: "section",
    0x3CA: "gate", 0x3CB: "shadow", 0x3CC: "speaker", 0x3CD: "Reset Collision",
    0x3CE: "wospot", 0x3CF: "wopoint", 0x3D0: "Ship Collision Fx",
    0x3D3: "fogCube", 0x3D4: "MeshNode_Ghost", 0x3D5: "sea", 0x3D6: "seaweed",
    0x3D7: "seareflect", 0x3D8: "cloudCube", 0x3D9: "cloudGroup",
    0x3DA: "weatherPos", 0x3DB: "Unused 1", 0x3DC: "animationTrigger",
    0x3DD: "gridCamera", 0x3DE: "lensflare", 0x3DF: "textureBlob", 0x3E0: "blob",
    0x3E1: "sound", 0x3E2: "Ship Muzzle", 0x3E4: "exitglow",
    0x3E5: "engine_fire", 0x3E6: "Mag Floor Collision", 0x3E7: "Cage Collision",
    0x3E9: "soundcone", 0x3EB: "cannon_flash",
}
COLLISION = {0x3B9: "floor", 0x3BA: "wall", 0x3CD: "reset", 0x3E6: "magfloor"}
POINT_LEN = 0x70


def nodes(data, order):
    """Walk the node tree. Yields (class_id, name, payload); raises if it drifts."""
    _version, tree_len, _tex_len = struct.unpack(order + "III", data[:12])
    off, end = 16, 16 + tree_len
    while off < end:
        class_id = struct.unpack_from(order + "I", data, off)[0]
        header_len = struct.unpack_from(order + "H", data, off + 4)[0]
        data_len = struct.unpack_from(order + "I", data, off + 8)[0]
        children = struct.unpack_from(order + "H", data, off + 12)[0]
        if header_len == 0 or off + header_len + data_len > end:
            raise ValueError(f"node at {off:#x} runs past the tree")
        name = ""
        if header_len >= 0x20:
            stop = data.index(b"\0", off + 16)
            name = data[off + 16 : stop].decode("ascii", "replace")
        yield class_id, name, children, data[off + header_len : off + header_len + data_len]
        off += header_len + data_len
    if off != end:
        raise ValueError("the walk overshot the tree")


def check_container(data, order):
    version, tree_len, tex_len = struct.unpack(order + "III", data[:12])
    magic = data[12:16] if order == "<" else data[15:11:-1]
    count = kids = 0
    hist = {}
    for class_id, _name, children, _payload in nodes(data, order):
        count += 1
        kids += children
        hist[class_id] = hist.get(class_id, 0) + 1
    return {
        "version": version,
        "magic_ok": magic == b"VEXX",
        "size_ok": 16 + tree_len + tex_len == len(data),
        "tree_ok": kids == count - 1,
        "nodes": count,
        "classes": hist,
    }


def check_spline(payload, order):
    magic, version, paths, junctions = struct.unpack_from(order + "IIII", payload, 0)
    base = 0x20 + (0x20 if version >= 0x101 else 0)
    counts = [
        struct.unpack_from(order + "I", payload, base + i * 0x20)[0] for i in range(paths)
    ]
    first = base + paths * 0x20 + junctions * 0x10
    tail = len(payload) - first
    total = sum(counts)
    off_frame = 0
    gap_min, gap_max = float("inf"), 0.0
    index = 0
    for count in counts:
        previous = None
        for _ in range(count):
            at = first + index * POINT_LEN
            index += 1
            pos = struct.unpack_from(order + "3f", payload, at)
            for field in (0x10, 0x20, 0x30):
                vec = struct.unpack_from(order + "3f", payload, at + field)
                length = sum(c * c for c in vec) ** 0.5
                off_frame += abs(length - 1.0) > 1e-3
            if previous is not None:
                gap = sum((a - b) ** 2 for a, b in zip(pos, previous)) ** 0.5
                gap_min, gap_max = min(gap_min, gap), max(gap_max, gap)
            previous = pos
    return {
        "magic_ok": magic == 0x574F_7464,
        "version": version,
        "paths": paths,
        "points": total,
        "closes": tail == total * POINT_LEN,
        "off_frame": off_frame,
        "frames": total * 3,
        "gap": (gap_min, gap_max),
    }


def check_collision(payload, order):
    _ones, objects = struct.unpack_from(order + "II", payload, 0)
    at = 8
    vertices = triangles = 0
    for _ in range(objects):
        chunks = struct.unpack_from(order + "I", payload, at)[0]
        at += 4
        for _ in range(chunks):
            kind = struct.unpack_from(order + "I", payload, at)[0]
            stride, count = struct.unpack_from(order + "HH", payload, at + 4)
            vertices += count if kind == 1 else 0
            triangles += count if kind == 2 else 0
            at += 8 + stride * count
    return {
        "objects": objects,
        "closes": at + (-at % 16) == len(payload),
        "vertices": vertices,
        "triangles": triangles,
    }


def pvs_masks(sections, order):
    """Dangling-bit counts for the three ways to read the 64-bit mask."""
    ids = {sid for sid, _raw in sections}
    out = {"u64": [0, 0], "word-pair": [0, 0]}
    for _sid, raw in sections:
        one = int.from_bytes(raw, "big" if order == ">" else "little")
        lo = int.from_bytes(raw[:4], "big" if order == ">" else "little")
        hi = int.from_bytes(raw[4:], "big" if order == ">" else "little")
        for name, mask in (("u64", one), ("word-pair", lo | (hi << 32))):
            for bit in range(64):
                if mask >> bit & 1:
                    out[name][0] += 1
                    out[name][1] += bit not in ids
    return out


def survey_tracks(archives, order, label):
    files = ok = 0
    spline_files = spline_total = off_frame = frames = 0
    versions = set()
    collision_nodes = collision_ok = 0
    gap_min, gap_max = float("inf"), 0.0
    pvs = {"u64": [0, 0], "word-pair": [0, 0]}
    clean_pvs_files = pvs_files = 0
    classes = {}
    for arc, path, data in archives:
        files += 1
        try:
            container = check_container(data, order)
        except ValueError as err:
            print(f"  BAD {path}: {err}")
            continue
        for class_id, count in container["classes"].items():
            classes[class_id] = classes.get(class_id, 0) + count
        good = container["magic_ok"] and container["size_ok"] and container["tree_ok"]
        sections = []
        for class_id, _name, _children, payload in nodes(data, order):
            if class_id == 0x3BB:
                spline = check_spline(payload, order)
                versions.add(spline["version"])
                spline_files += 1
                spline_total += spline["points"]
                off_frame += spline["off_frame"]
                frames += spline["frames"]
                gap_min = min(gap_min, spline["gap"][0])
                gap_max = max(gap_max, spline["gap"][1])
                good = good and spline["magic_ok"] and spline["closes"]
            elif class_id in COLLISION:
                soup = check_collision(payload, order)
                collision_nodes += 1
                collision_ok += soup["closes"]
                good = good and soup["closes"]
            elif class_id == 0x3C9:
                sections.append((payload[0], payload[8:16]))
        if sections:
            pvs_files += 1
            counts = pvs_masks(sections, order)
            for key in pvs:
                pvs[key][0] += counts[key][0]
                pvs[key][1] += counts[key][1]
            clean_pvs_files += counts["u64"][1] == 0
        ok += good
    print(f"\n{label}")
    print(f"  {ok} of {files} track files pass every container check")
    print(f"  WO Track: {spline_files} splines, versions {sorted(hex(v) for v in versions)}, "
          f"{spline_total} control points")
    print(f"    frame vectors off unit length: {off_frame} of {frames}")
    print(f"    control-point spacing: {gap_min:.2f}..{gap_max:.2f}")
    print(f"  collision: {collision_ok} of {collision_nodes} chunk walks close on the pad")
    if pvs_files:
        for key, (bits, dangling) in pvs.items():
            share = dangling / bits if bits else 0.0
            print(f"  PVS mask as {key:<9}: {dangling} of {bits} bits name no section "
                  f"({share:.2%})")
        print(f"    files with no dangling bit under the u64 reading: "
              f"{clean_pvs_files} of {pvs_files}")
    print("  classes seen:")
    for class_id, count in sorted(classes.items(), key=lambda kv: -kv[1]):
        print(f"    {count:6}  {class_id:#06x}  {CLASS.get(class_id, '(unrecovered)')}")


def survey_pob(image):
    total = magic_ok = words_ok = name_ok = length_ok = 0
    slack = set()
    for arc in open_archives(image):
        for path, _size, index in arc.files():
            if not path.lower().endswith(".pob"):
                continue
            data = arc.read(index)
            total += 1
            magic_ok += data[:4] == b"PSYS"
            declared = struct.unpack_from(">I", data, 4)[0]
            slots = struct.unpack_from(">H", data, 8)[0]
            words_ok += struct.unpack_from(">H", data, 10)[0] == 1 and (
                struct.unpack_from(">I", data, 12)[0] == 1
            )
            name = data[0x10 + slots * 4 : 0x10 + slots * 4 + 32].split(b"\0")[0]
            # The name has to predict the file's own name, not merely be ASCII -
            # that is what makes the slot table's end a measured boundary.
            stem = path.rsplit("/", 1)[-1][:-4]
            name_ok += name.decode("ascii", "replace").lower() == stem.lower()
            length_ok += declared == len(data)
            slack.add(len(data) - declared)
    print("\n.pob particle containers")
    print(f"  {total} files: magic PSYS on {magic_ok}, the two constant words on "
          f"{words_ok}, a name predicting the file name on {name_ok}")
    print(f"  the +0x04 length equals the file length on {length_ok}; "
          f"shortfalls seen: {sorted(slack)}")


def hd_tracks(image):
    for arc in open_archives(image):
        for path, _size, index in arc.files():
            name = path.rsplit("/", 1)[-1]
            if path.startswith("/data/environments/") and name.startswith("track") and (
                name.endswith(".vex")
            ):
                yield arc, path, arc.read(index)


def read_wad_entry(image, entry):
    """One entry out of a UMD's Data.wad, through this project's own reader."""
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    done = subprocess.run(
        ["cargo", "run", "-q", "-p", "oag-tools", "--bin", "oag-wad", "--", "cat",
         f"{image}:PSP_GAME/USRDIR/Data.wad", entry],
        cwd=root, capture_output=True,
    )
    return done.stdout if done.returncode == 0 and done.stdout else None


def pulse_tracks(image):
    """Pulse's own track files, for the control column."""
    for number in range(1, 21):
        for variant in ("track", "track_reversed"):
            entry = f"Data\\Environments\\{number:02}_Track\\{variant}.vex"
            data = read_wad_entry(image, entry)
            if data is not None:
                yield None, f"{number:02}_Track/{variant}", data


# Pure's environment directories, as far as their entry names have been
# recovered. Pure stores names as a hash like every WAD does, so this list is
# what `just mine-names` and guesswork have turned up rather than a reading of
# the disc - `--match` reports the ones it could not open instead of hiding them.
PURE_ENVIRONMENTS = [
    "01_Vineta_K",
    "03_Modesto_Heights",
    "04_Chenghou_Project",
    "07_Blue_Ridge",
    "08_Sinucit",
    "10_Sebenco_Climb",
    "12_Sol_2",
]
GRID = 32.0
MATCH_LIMIT = 20.0


def spline_points(payload, order):
    _magic, version, paths, junctions = struct.unpack_from(order + "IIII", payload, 0)
    base = 0x20 + (0x20 if version >= 0x101 else 0)
    counts = [
        struct.unpack_from(order + "I", payload, base + i * 0x20)[0] for i in range(paths)
    ]
    first = base + paths * 0x20 + junctions * 0x10
    return [
        struct.unpack_from(order + "3f", payload, first + i * POINT_LEN)
        for i in range(sum(counts))
    ]


def bucket(points):
    cells = {}
    for p in points:
        key = (int(p[0] // GRID), int(p[1] // GRID), int(p[2] // GRID))
        cells.setdefault(key, []).append(p)
    return cells


def nearest(cells, point):
    cx, cy, cz = (int(point[i] // GRID) for i in range(3))
    best = float("inf")
    radius = 1
    while radius < 8:
        for dx in range(-radius, radius + 1):
            for dy in range(-radius, radius + 1):
                for dz in range(-radius, radius + 1):
                    for other in cells.get((cx + dx, cy + dy, cz + dz), ()):
                        d = sum((a - b) ** 2 for a, b in zip(point, other)) ** 0.5
                        best = min(best, d)
        if best < (radius - 1) * GRID:
            return best
        radius += 2
    return best


def spline_of(data, order, class_id):
    for cls, _name, _children, payload in nodes(data, order):
        if cls == class_id:
            return spline_points(payload, order)
    return None


def cmd_match(hd_image, pulse_image, pure_image):
    """Which HD circuit is which PSP circuit, by shape rather than by name."""
    candidates = {}
    for _arc, name, data in pulse_tracks(pulse_image):
        candidates["pulse " + name] = spline_of(data, "<", 0x3BB)
    missing = []
    for name in PURE_ENVIRONMENTS:
        entry = f"Data\\Environments\\{name}\\track.vex"
        data = read_wad_entry(pure_image, entry)
        if data is None:
            missing.append(name)
            continue
        # Pure renumbers the class space; WO Track is 0x36d there. See
        # docs/formats/pure-status.md.
        candidates["pure  " + name] = spline_of(data, "<", 0x36D)
    grids = {k: bucket(v) for k, v in candidates.items() if v}
    print(f"\n{len(grids)} candidate splines"
          + (f"; {len(missing)} Pure name(s) did not resolve: {missing}" if missing else ""))
    print(f"{'HD circuit':<38} {'points':>6}  {'best match':<28} {'mean':>7} {'max':>8}  next")
    for _arc, path, data in hd_tracks(hd_image):
        points = spline_of(data, ">", 0x3BB)
        if not points:
            continue
        sample = points[::4]
        scored = []
        for key, cells in grids.items():
            d = [nearest(cells, p) for p in sample]
            scored.append((sum(d) / len(d), max(d), key))
        scored.sort()
        mean, worst, key = scored[0]
        runner = scored[1][0] if len(scored) > 1 else float("inf")
        # A real match scores under a unit or two; a wrong candidate scores in
        # the hundreds. Nothing lands in between, so the cut is not delicate.
        if mean > MATCH_LIMIT:
            print(f"{path[19:-4]:<38} {len(points):6}  "
                  f"{'- no candidate within ' + str(MATCH_LIMIT) + ' units':<28}")
            continue
        print(f"{path[19:-4]:<38} {len(points):6}  {key:<28} {mean:7.2f} {worst:8.2f}  "
              f"{runner:.1f}")
    return 0


def main(argv):
    if len(argv) >= 5 and argv[1] == "--match":
        return cmd_match(argv[2], argv[3], argv[4])
    if len(argv) < 2 or argv[1].startswith("-"):
        print(__doc__.strip())
        return 2
    survey_tracks(hd_tracks(argv[1]), ">", "Wipeout HD / Fury, big-endian")
    survey_pob(argv[1])
    if len(argv) > 2 and argv[2]:
        survey_tracks(pulse_tracks(argv[2]), "<", "Wipeout Pulse, little-endian (control)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv) or 0)
