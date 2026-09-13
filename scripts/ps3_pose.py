"""Finding a camera in a PS3 memory dump, offline.

Wipeout HD's shaders take their camera as two named engine parameters, both
recovered by `~crc32` preimage over `EBOOT.elf`'s own string table at
`0x008b7f08` (see `docs/reverse-engineering/rpcs3-capture.md`):

    viewProj               0x2e7d5f33   float4 x4   the world -> clip matrix
    eyePositionWorldSpace  0x3466fc0e   float3      the camera's position

`viewProj` is the quantity worth capturing, because it is the one that decides
the frame: position, orientation, field of view, aspect and near/far are all
inside it, so a comparison against it cannot be spoiled by disagreeing about a
convention or deriving a field of view wrongly.

**Nothing in the executable points at the parameter table** - no `lis`/`addi`
pair builds its address and the word never appears as a pointer - so the
runtime address of the values is not reachable statically, and this module
exists to find them in a dump instead.

# The test that makes a candidate trustworthy

Sixteen arbitrary floats are not a projection. For a world -> clip matrix in
the row-vector convention this hardware uses (`clip = v * M`), the clip `w` of
a world point is

    w = x*m03 + y*m13 + z*m23 + m33

and for a perspective projection that is exactly the signed distance from the
camera plane, so **`(m03, m13, m23)` is a unit vector** - six digits of
agreement that random data does not produce. The camera position follows from
the same matrix: the frustum's side planes meet at the eye, and three of them
solved simultaneously give it (Gribb-Hartmann plane extraction, then a 3x3
solve).

Both conventions are tried, because which one a title uses is not something to
assume: the transpose is checked with the same test and reported as it is
found.
"""

import math
import struct

#: `~crc32("viewProj")`, from `EBOOT.elf`'s parameter name table.
VIEW_PROJ_HASH = 0x2E7D5F33
#: `~crc32("eyePositionWorldSpace")`, likewise.
EYE_POSITION_HASH = 0x3466FC0E

#: How far from 1.0 the `w` row's length may be and still be a projection.
#: Loose enough for a matrix built in `f32` and multiplied twice, tight enough
#: that noise never passes: on a 900 KB dump the false-positive rate at 1e-3 is
#: nil, and this is 30x looser than that.
UNIT_TOLERANCE = 3e-2

#: The largest world coordinate any circuit on the disc reaches, in the units
#: `oag_formats::rcsmodel` decodes. The widest measured is Anulpha Pass at
#: about 2,200 from the origin; this is an order of magnitude of headroom.
WORLD_LIMIT = 50_000.0

#: How far the view basis may be from orthonormal. Loose for a basis that has
#: been through two `f32` matrix products, tight enough that nothing but a
#: rotation passes.
BASIS_TOLERANCE = 1e-2

#: The vertical field of view a real camera can have, in degrees. A degenerate
#: matrix - all zeros, or an identity - passes every *algebraic* test here and
#: then decomposes to a field of view of nothing, which is what a first run of
#: this against a live dump found and reported as a camera.
FOV_RANGE = (10.0, 170.0)

#: The aspect a console frame can have. 16:9 and 4:3 with room either side.
ASPECT_RANGE = (0.5, 4.0)

#: A camera exactly at the origin is what a zeroed matrix decomposes to, not
#: where a race camera ever is.
ORIGIN_EPSILON = 1e-3


def _finite(values):
    return all(math.isfinite(v) and abs(v) < 1e6 for v in values)


def _solve3(rows):
    """Solves a 3x3 system by Cramer's rule, or `None` if it is singular.

    Cramer rather than elimination because the system is 3x3 and fixed: no
    pivoting to get wrong, and a determinant that is already the singularity
    test.
    """
    (a, b, c, p), (d, e, f, q), (g, h, i, r) = rows
    det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
    if abs(det) < 1e-12:
        return None
    x = (p * (e * i - f * h) - b * (q * i - f * r) + c * (q * h - e * r)) / det
    y = (a * (q * i - f * r) - p * (d * i - f * g) + c * (d * r - q * g)) / det
    z = (a * (e * r - q * h) - b * (d * r - q * g) + p * (d * h - e * g)) / det
    return (x, y, z)


def eye_of(m):
    """The camera position a world -> clip matrix places the frustum apex at.

    `m` is 16 floats in row-major order under the row-vector convention. The
    four side planes are the Gribb-Hartmann combinations of the matrix's
    columns; any three of them meet at the eye, and left/right/top is the
    triple with no degenerate pair for an ordinary frustum.

    Returns `None` when the three planes do not meet in a point, which is what
    an orthographic matrix and most non-matrices do.
    """
    col = [[m[r * 4 + c] for r in range(4)] for c in range(4)]
    left = [col[3][k] + col[0][k] for k in range(4)]
    right = [col[3][k] - col[0][k] for k in range(4)]
    top = [col[3][k] - col[1][k] for k in range(4)]
    rows = [[p[0], p[1], p[2], -p[3]] for p in (left, right, top)]
    return _solve3(rows)


def score(m):
    """How much like a perspective world -> clip matrix these 16 floats are.

    Returns `(unit_error, eye)` or `None`.

    Four things have to hold together, and the first alone is not enough. The
    `w` row must be a unit vector (the module docstring says why); the frustum
    apex must be a place a camera could be; the view basis the matrix
    decomposes to must be **orthonormal**, three mutually perpendicular unit
    vectors, which is six constraints that arbitrary floats do not satisfy; and
    the field of view and aspect must be ones a console frame could have.

    **The last two exist because of what a first live run found.** A region of
    zeroed memory passes the unit test and the apex test - it decomposes to a
    camera at the origin with a field of view of nothing - and was duly
    reported as the camera. An algebraic test that a degenerate matrix passes
    is not a test.
    """
    if not _finite(m):
        return None
    direction = (m[3], m[7], m[11])
    length = math.sqrt(sum(v * v for v in direction))
    error = abs(length - 1.0)
    if error > UNIT_TOLERANCE:
        return None
    eye = eye_of(m)
    if eye is None or not _finite(eye) or max(abs(v) for v in eye) > WORLD_LIMIT:
        return None
    if max(abs(v) for v in eye) < ORIGIN_EPSILON:
        return None
    pose = decompose(m)
    if pose is None:
        return None
    if not FOV_RANGE[0] <= pose["fov_y_deg"] <= FOV_RANGE[1]:
        return None
    if not ASPECT_RANGE[0] <= pose["aspect"] <= ASPECT_RANGE[1]:
        return None
    basis = (pose["right"], pose["up"], pose["forward"])
    for a in range(3):
        for b in range(a, 3):
            dot = sum(basis[a][k] * basis[b][k] for k in range(3))
            if abs(dot - (1.0 if a == b else 0.0)) > BASIS_TOLERANCE:
                return None
    return (error, eye)


def decompose(m):
    """A world -> clip matrix as the numbers a renderer is set up from.

    In the row-vector convention (`clip = v * M`) with a symmetric perspective,
    the matrix's first three columns are the view basis scaled by the
    projection: `c0.xyz` is `right * f/aspect`, `c1.xyz` is `up * f`, and
    `c3.xyz` is the unit forward. So the basis, the field of view and the
    aspect all come out by inspection, and the eye from the frustum apex.

    Returns a dict, or `None` if the matrix does not decompose - a degenerate
    basis rather than a perspective.
    """
    col = [[m[r * 4 + c] for r in range(4)] for c in range(4)]

    def norm(v):
        length = math.sqrt(sum(c * c for c in v))
        return None if length < 1e-9 else ([c / length for c in v], length)

    right = norm(col[0][:3])
    up = norm(col[1][:3])
    forward = norm(col[3][:3])
    eye = eye_of(m)
    if not (right and up and forward and eye):
        return None
    focal = up[1]
    return {
        "eye": list(eye),
        "forward": forward[0],
        "up": up[0],
        "right": right[0],
        "fov_y_deg": math.degrees(2.0 * math.atan(1.0 / focal)),
        "aspect": focal / right[1],
    }


def command_line(pose, track=None):
    """The `oag-game` invocation that renders this project from that camera.

    The harness is only half a comparison without it: `--camera-pose` takes the
    nine numbers below and `--camera-fov` the tenth.
    """
    numbers = ",".join(
        "%.6f" % v for v in list(pose["eye"]) + pose["forward"] + pose["up"])
    parts = ["target/release/oag-game", "--race", "<image>"]
    if track:
        parts += ["--track", track]
    parts += ["--camera-pose", numbers,
              "--camera-fov", "%.4f" % pose["fov_y_deg"],
              "--screenshot", "ours.png"]
    return " ".join(parts)


def transpose(m):
    return [m[c * 4 + r] for r in range(4) for c in range(4)]


def candidates(blob, base=0, stride=4):
    """Every offset in `blob` whose 16 floats read as a perspective matrix.

    Yields `(address, order, unit_error, eye, matrix)`, `order` being
    `"row"` for the matrix as stored and `"col"` for its transpose - which one
    a title writes is a fact to report, not to assume.
    """
    for at in range(0, len(blob) - 64 + 1, stride):
        m = list(struct.unpack_from(">16f", blob, at))
        for order, candidate in (("row", m), ("col", transpose(m))):
            hit = score(candidate)
            if hit:
                yield (base + at, order, hit[0], hit[1], candidate)


#: The RSX command header for a count-17 `NV4097_SET_TRANSFORM_CONSTANT_LOAD`:
#: `((4*4 + 1) << 18) | 0x1efc`, a register-index word followed by a whole
#: `float4x4` in one packet - see `Rsx_UploadVertexConstantBlock` (`0x005c18d8`)
#: in `docs/ghidra/functions/ps3-hdfury-eu/renderer.md` and "A matrix in a
#: FIFO is not sixteen consecutive floats - except when it is" in
#: `docs/reverse-engineering/rpcs3-capture.md`. This is the *only* packet
#: shape decoded here: the count-5 form (`0x00141efc`, one `vec4` at a time,
#: rows 24 bytes apart) is a different emitter and nothing needs it yet - a
#: finder that only understands this one shape says so rather than looking
#: general.
TRANSFORM_CONSTANT_LOAD_HEADER = 0x00441efc

#: Header word (4 bytes) plus register-index word (4 bytes) precede the
#: matrix in a count-17 packet.
_PACKET_PREFIX = 8


def packet_candidates(blob, base=0):
    """Every count-17 `NV4097_SET_TRANSFORM_CONSTANT_LOAD` packet's matrix.

    `candidates` slides over every four-byte offset because it has no idea
    what is and is not a command packet; in a raw RSX pushbuffer dump that is
    tens of thousands of offsets to score for a few hundred real packets.
    This instead looks only at the fixed positions the command stream itself
    marks - `TRANSFORM_CONSTANT_LOAD_HEADER`, then the register index, then
    the matrix - so it scores a few hundred candidates instead of tens of
    thousands on the same input.

    `score` is applied unchanged: a header match on a degenerate payload is
    still degenerate, and this narrows *which offsets* get scored, not *how
    strictly*.

    Yields `(address, order, unit_error, eye, matrix, register)`. `address` is
    where the matrix itself starts (header address + 8), the same convention
    `candidates` uses so a hit from either finder is directly comparable.
    `register` is the RSX constant register the packet loads - `256`/`260` are
    where a live capture found `viewProj`/`worldViewProj` (see
    `rpcs3-capture.md`'s "Picking the camera out" section); nothing here
    assumes that in advance, so any register comes through.
    """
    limit = len(blob) - _PACKET_PREFIX - 64
    at = 0
    while at <= limit:
        header = struct.unpack_from(">I", blob, at)[0]
        if header == TRANSFORM_CONSTANT_LOAD_HEADER:
            register = struct.unpack_from(">I", blob, at + 4)[0]
            m = list(struct.unpack_from(">16f", blob, at + _PACKET_PREFIX))
            for order, candidate in (("row", m), ("col", transpose(m))):
                hit = score(candidate)
                if hit:
                    yield (base + at + _PACKET_PREFIX, order, hit[0], hit[1],
                           candidate, register)
        at += 4


#: The camera has to be the same value in at least `MIN_REGISTER_COUNT`
#: distinct RSX registers within a frame - empirically `256` and `260` in
#: every shot measured so far (`rpcs3-capture.md`'s "Picking the camera out:
#: multiplicity, not a score") - to separate it from a degenerate matrix that
#: a header match turns up in only one of them.
CAMERA_REGISTERS = frozenset({256, 260})


def pick_camera(frames):
    """Picks the one candidate per frame that is the camera, or refuses.

    `frames` is a list of per-shot candidate lists, each one
    `list(packet_candidates(...))` for that shot - every frame in the capture
    session has to be in hand at once, because the discriminator is
    **cross-frame**: `rpcs3-capture.md`'s "Picking the camera out" table. A
    per-object `worldViewProj` (`c[256]`) takes dozens to a hundred distinct
    values a frame and is excluded by being many, not one; a static constant
    (a cube face, a bias matrix) is one value that is bit-identical **across
    frames** and is excluded by recurring there; the camera is the value that
    is one *within* a frame and different *between* frames. Confirmed
    directly against `talons-fifo3`'s three dumps (`rpcs3-capture.md`,
    "A packet-aware finder exists"): two matrices there repeat bit-for-bit
    across all three frames (the degenerate `0.0`-error ties that beat the
    real camera under a plain `min`), and the real camera's own eye and
    `unit_error` differ in all three.

    Within-frame repetition alone is not sufficient - the same section found
    the degenerate matrices recurring dozens of times *within* one frame too
    - so a second filter applies to whatever survives being frame-unique:
    it must be the identical value loaded into at least `CAMERA_REGISTERS`
    registers (`256` **and** `260`) in that frame. That is what breaks a tie
    `talons-fifo3/01` has between the real camera and one more degenerate
    matrix that is itself frame-unique but lands in only one of the two
    registers.

    Returns a list parallel to `frames`. Each element is
    `(camera, reason, candidate_count)`:

    - `camera` is `None` unless exactly one value survives every filter, in
      which case it is a dict: `address`, `order`, `unit_error`, `eye`,
      `view_proj` (the matrix) and `registers` (which of `256`/`260` carried
      it, sorted).
    - `reason` is `None` when a pick was made, else why it was not - carried
      through to the capture record rather than silently emitting nothing,
      the same honest-absence rule this project applies to a missing asset.
    - `candidate_count` is how many frame-unique, dual-register values
      survived: `0` or more than `1` when `camera` is `None`, exactly `1`
      otherwise - present even on a successful pick so the record shows its
      work rather than asserting it.

    This never falls back to the byte-slider (`candidates`): a region with no
    `TRANSFORM_CONSTANT_LOAD` packet is a region with nothing to offer, not a
    reason to widen the search and risk a `"finder"` label beside a wrong
    pose (`rpcs3-capture.md`'s "A packet-aware finder exists" section is the
    reasoning this followed).
    """
    if len(frames) < 2:
        reason = ("the cross-frame discriminator needs at least 2 frames; "
                   "only %d captured" % len(frames))
        return [(None, reason, 0) for _ in frames]

    value_frames = {}
    for index, candidates_this_frame in enumerate(frames):
        frame_values = set()
        for entry in candidates_this_frame:
            frame_values.add(tuple(entry[4]))
        for key in frame_values:
            value_frames.setdefault(key, set()).add(index)

    results = []
    for index, candidates_this_frame in enumerate(frames):
        by_value = {}
        for entry in candidates_this_frame:
            key = tuple(entry[4])
            by_value.setdefault(key, []).append(entry)

        frame_unique = {key: entries for key, entries in by_value.items()
                         if value_frames[key] == {index}}
        dual_register = {
            key: entries for key, entries in frame_unique.items()
            if CAMERA_REGISTERS <= {entry[5] for entry in entries}
        }

        if len(dual_register) == 1:
            (entries,) = dual_register.values()
            address, order, error, eye, matrix, _ = entries[0]
            registers = sorted({entry[5] for entry in entries})
            camera = {
                "address": "%#010x" % address,
                "order": order,
                "unit_error": error,
                "eye": list(eye),
                "view_proj": list(matrix),
                "registers": registers,
            }
            results.append((camera, None, 1))
        elif len(dual_register) == 0:
            results.append((None, "no frame-unique value loaded into both "
                             "register %d and %d this frame"
                             % tuple(sorted(CAMERA_REGISTERS)), 0))
        else:
            results.append((None, "%d frame-unique dual-register candidates, "
                             "not exactly one" % len(dual_register),
                             len(dual_register)))
    return results


def perspective(fov_y, aspect, near, far):
    """A right-handed perspective matrix, row-vector convention.

    Here so the finder can be tested against a matrix whose camera is known
    rather than only against a dump - see `self_test`.
    """
    f = 1.0 / math.tan(fov_y / 2.0)
    return [
        f / aspect, 0.0, 0.0, 0.0,
        0.0, f, 0.0, 0.0,
        0.0, 0.0, far / (far - near), 1.0,
        0.0, 0.0, -near * far / (far - near), 0.0,
    ]


def look_at(eye, target, up=(0.0, 1.0, 0.0)):
    """A world -> view matrix, row-vector convention."""
    def sub(a, b):
        return tuple(a[k] - b[k] for k in range(3))

    def norm(v):
        length = math.sqrt(sum(c * c for c in v))
        return tuple(c / length for c in v)

    def cross(a, b):
        return (a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0])

    def dot(a, b):
        return sum(a[k] * b[k] for k in range(3))

    z = norm(sub(target, eye))
    x = norm(cross(up, z))
    y = cross(z, x)
    return [
        x[0], y[0], z[0], 0.0,
        x[1], y[1], z[1], 0.0,
        x[2], y[2], z[2], 0.0,
        -dot(x, eye), -dot(y, eye), -dot(z, eye), 1.0,
    ]


def multiply(a, b):
    return [sum(a[r * 4 + k] * b[k * 4 + c] for k in range(4))
            for r in range(4) for c in range(4)]


def self_test():
    """Plants a known camera in noise and insists the finder recovers it."""
    import random

    random.seed(20260824)
    eye = (327.2, -42.0, -158.2)
    target = (0.0, -40.0, 0.0)
    m = multiply(look_at(eye, target), perspective(math.radians(60), 16 / 9, 1.0, 8000.0))

    noise = bytearray(random.randbytes(1 << 18))
    at = 0x400
    struct.pack_into(">16f", noise, at, *m)

    found = [c for c in candidates(bytes(noise))]
    planted = [c for c in found if c[0] == at]
    assert planted, "the planted matrix was not found"
    _, order, error, recovered, _ = planted[0]
    assert order == "row", order
    for k in range(3):
        assert abs(recovered[k] - eye[k]) < 0.05, (recovered, eye)
    print(f"self test: recovered eye {recovered} from {len(found)} candidate(s) "
          f"in 256 KB of noise, unit error {error:.2e}")
    others = [c for c in found if c[0] != at]
    print(f"           {len(others)} false positive(s) in the noise")

    pose = decompose(m)
    assert pose, "the planted matrix did not decompose"
    for k in range(3):
        assert abs(pose["eye"][k] - eye[k]) < 0.05, pose["eye"]
    assert abs(pose["fov_y_deg"] - 60.0) < 0.01, pose["fov_y_deg"]
    assert abs(pose["aspect"] - 16 / 9) < 1e-4, pose["aspect"]
    forward = [target[k] - eye[k] for k in range(3)]
    length = math.sqrt(sum(v * v for v in forward))
    for k in range(3):
        assert abs(pose["forward"][k] - forward[k] / length) < 1e-4, pose["forward"]
    print(f"           fov {pose['fov_y_deg']:.3f} deg, aspect {pose['aspect']:.4f}, "
          f"forward {['%.3f' % v for v in pose['forward']]}")
    print("           " + command_line(pose, "/data/environments/talons_junction/track.vex"))
    return 0


def self_test_packet():
    """Plants a `TRANSFORM_CONSTANT_LOAD` packet in noise and insists the
    packet-aware finder recovers it, and only it - `candidates`' byte-slider
    still finds the same matrix at the same 4-byte-aligned offset, since a
    packet's payload is itself 4-byte aligned, but at vastly higher cost.
    """
    import random

    random.seed(20260905)
    eye = (-143.58, -48.44, -175.13)
    target = (0.0, -48.44, 0.0)
    m = multiply(look_at(eye, target), perspective(math.radians(60), 16 / 9, 1.0, 8000.0))

    noise = bytearray(random.randbytes(1 << 18))
    header_at = 0x800
    register = 256
    struct.pack_into(">II", noise, header_at, TRANSFORM_CONSTANT_LOAD_HEADER, register)
    struct.pack_into(">16f", noise, header_at + _PACKET_PREFIX, *m)
    blob = bytes(noise)

    found = list(packet_candidates(blob))
    planted = [c for c in found if c[0] == header_at + _PACKET_PREFIX]
    assert planted, "the planted packet was not found"
    _, order, error, recovered, _, reg = planted[0]
    assert order == "row", order
    assert reg == register, reg
    for k in range(3):
        assert abs(recovered[k] - eye[k]) < 0.05, (recovered, eye)
    print(f"packet self test: recovered eye {recovered} from register {reg}, "
          f"{len(found)} candidate(s) in 256 KB of noise, unit error {error:.2e}")

    # The byte-slider still sees the same matrix - it is 4-byte aligned like
    # everything else - just at far higher cost: it has to score every
    # offset, not only the ones a header marks.
    slid = [c for c in candidates(blob) if c[0] == header_at + _PACKET_PREFIX]
    assert slid, "the byte-slider lost a packet's matrix it should still see"
    print(f"                  byte-slider confirms the same matrix at the same "
          f"offset among {len(list(candidates(blob)))} candidate(s)")

    # A header match on a degenerate payload must not be relaxed into a hit:
    # planting a zeroed matrix behind a real header still has to fail `score`.
    degenerate = bytearray(random.randbytes(1 << 12))
    struct.pack_into(">II", degenerate, 0, TRANSFORM_CONSTANT_LOAD_HEADER, 256)
    struct.pack_into(">16f", degenerate, _PACKET_PREFIX, *([0.0] * 16))
    assert not list(packet_candidates(bytes(degenerate))), \
        "a header match on a zeroed payload must still be rejected by score"
    print("                  a header on a zeroed payload is still rejected")
    return 0


def self_test_pick():
    """Plants two frames - a moving camera and a recurring degenerate matrix
    that is itself frame-unique in one of them - and insists `pick_camera`
    takes the camera in both and is not fooled by the degenerate.

    The degenerate is deliberately shaped after `talons-fifo3/01.json`'s own
    tie (`rpcs3-capture.md`, "A packet-aware finder exists"): bit-identical
    across the two frames it appears in (so the cross-frame filter alone
    would not be enough - see the module test below), loaded into only one
    of the two registers the real camera uses.
    """
    import random

    def packed_frame(seed, eye, target, degenerate_header_at):
        random.seed(seed)
        m = multiply(look_at(eye, target), perspective(math.radians(60), 16 / 9, 1.0, 8000.0))
        noise = bytearray(random.randbytes(1 << 14))
        # The camera packet, loaded into both registers the same way a real
        # capture's `c[256]`/`c[260]` upload does.
        for offset, register in ((0x100, 256), (0x300, 260)):
            struct.pack_into(">II", noise, offset, TRANSFORM_CONSTANT_LOAD_HEADER, register)
            struct.pack_into(">16f", noise, offset + _PACKET_PREFIX, *m)
        # A single-register degenerate, same bytes in every frame that plants
        # one - the shape of the tie `pick_camera` exists to break.
        degenerate = [0.0] * 16
        degenerate[11] = 1.0
        struct.pack_into(">II", noise, degenerate_header_at,
                          TRANSFORM_CONSTANT_LOAD_HEADER, 260)
        struct.pack_into(">16f", noise, degenerate_header_at + _PACKET_PREFIX,
                          *degenerate)
        return bytes(noise), m

    frame0, m0 = packed_frame(20260905, (-143.58, -48.44, -175.13), (0.0, -48.44, 0.0), 0x600)
    frame1, m1 = packed_frame(20260906, (88.88, -46.39, -178.36), (0.0, -46.39, 0.0), 0x600)

    frames = [list(packet_candidates(frame0)), list(packet_candidates(frame1))]
    results = pick_camera(frames)
    assert len(results) == 2, results
    for (camera, reason, count), expected in zip(results, (m0, m1)):
        assert camera is not None, reason
        assert reason is None, reason
        assert count == 1, count
        # Planted as f64, read back through an f32 round trip (the packet is
        # big-endian `f32`), so compare loosely rather than bit-for-bit.
        for got, want in zip(camera["view_proj"], expected):
            assert abs(got - want) < 1e-4, (camera["view_proj"], expected)
        assert camera["registers"] == [256, 260], camera["registers"]
    print("pick self test: recovered the camera in both frames, rejecting "
          "the single-register degenerate in each")

    # Within-frame repetition alone must not be enough: feeding a single
    # frame refuses rather than guessing, even though its own camera packet
    # is there and algebraically valid.
    (camera, reason, count) = pick_camera([list(packet_candidates(frame0))])[0]
    assert camera is None, camera
    assert count == 0, count
    print("               a single frame refuses: %r" % reason)
    return 0


if __name__ == "__main__":
    import sys

    raise SystemExit(self_test() or self_test_packet() or self_test_pick())
