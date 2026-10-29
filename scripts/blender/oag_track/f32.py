"""Single-precision arithmetic on Python floats.

The simulation reads `f32`, and the fixture generator in `oag-trackfile` computes
in `f32`. Rounding after every operation reproduces its results bit for bit
(`sqrt` included: a double-rounded correctly rounded square root of an `f32` is
the `f32` square root).
"""

import math
import struct

_PACK = struct.Struct("<f")


def r(x):
    """Round a Python float to the nearest `f32`, returned as a Python float."""
    try:
        return _PACK.unpack(_PACK.pack(x))[0]
    except OverflowError:
        return math.copysign(math.inf, x)


def bits(x):
    return struct.unpack("<I", _PACK.pack(x))[0]


def from_bits(b):
    return struct.unpack("<f", struct.pack("<I", b))[0]


def sub(a, b):
    return (r(a[0] - b[0]), r(a[1] - b[1]), r(a[2] - b[2]))


def add_scaled(a, d, s):
    return (r(a[0] + r(d[0] * s)), r(a[1] + r(d[1] * s)), r(a[2] + r(d[2] * s)))


def length(v):
    return r(math.sqrt(r(r(r(v[0] * v[0]) + r(v[1] * v[1])) + r(v[2] * v[2]))))


def unit(v):
    n = length(v)
    return (r(v[0] / n), r(v[1] / n), r(v[2] / n))


def cross(a, b):
    return (
        r(r(a[1] * b[2]) - r(a[2] * b[1])),
        r(r(a[2] * b[0]) - r(a[0] * b[2])),
        r(r(a[0] * b[1]) - r(a[1] * b[0])),
    )


def dist(a, b):
    return length(sub(a, b))


def text(x):
    """The shortest decimal that reads back as the `f32` nearest `x`, as a TOML float."""
    x = r(x)
    for p in range(1, 10):
        s = f"{x:.{p}g}"
        if r(float(s)) == x:
            break
    if "e" not in s and "." not in s and "n" not in s:
        s += ".0"
    return s
