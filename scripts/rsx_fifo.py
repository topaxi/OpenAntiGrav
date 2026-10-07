"""Walk an RSX command stream dumped from RPCS3's IO-mapped pushbuffer.

The dump is guest memory from 0x40000000 (IO offset == address - 0x40000000).
`walk` follows the stream from a start offset through JUMP and CALL/RETURN and
yields `(io_offset, method, args)` for every method packet it can read; a
target outside the dump is recorded in `Walk.missing` rather than guessed.
Method numbers are libgcm / RPCS3 `NV4097_*` register offsets.
"""
import struct

# NV4097 method offsets used by the wipeout reading (RPCS3 rsx_methods.h)
NAMES = {
    0x0a00: "VIEWPORT_HORIZONTAL", 0x0a04: "VIEWPORT_VERTICAL",
    0x0300: "ALPHA_FUNC", 0x0304: "ALPHA_REF", 0x0310: "BLEND_ENABLE",
    0x0314: "BLEND_FUNC_SFACTOR", 0x0318: "BLEND_FUNC_DFACTOR",
    0x031c: "BLEND_COLOR", 0x0320: "BLEND_EQUATION",
    0x0a60: "DEPTH_FUNC", 0x0a6c: "DEPTH_MASK", 0x0a74: "DEPTH_TEST_ENABLE",
    0x1d94: "CLEAR_SURFACE", 0x1808: "BEGIN_END", 0x1814: "INDEX_ARRAY_ADDRESS",
    0x1800: "DRAW_ARRAYS_ZZ", 0x1824: "DRAW_INDEX_ARRAY",
    0x0b40: "VERTEX_PROGRAM_LOAD_SLOT",
    0x1e9c: "TRANSFORM_PROGRAM_LOAD", 0x1ea0: "TRANSFORM_PROGRAM_START",
    0x1efc: "TRANSFORM_CONSTANT_LOAD",
    0x08e4: "SHADER_PROGRAM", 0x0b60: "SHADER_CONTROL",
    0x1d60: "FRAGMENT_PROGRAM_OFFSET_Q",
    0x08e0: "SHADER_PROGRAM",
    0x0fe0: "FOG_MODE", 0x0fe4: "FOG_PARAMS0", 0x0fe8: "FOG_PARAMS1",
}


def u32s(blob):
    return struct.unpack(">%dI" % (len(blob) // 4), blob[: len(blob) // 4 * 4])


class Mem:
    """Dumped spans of guest memory, addressed by IO offset (addr - 0x40000000)."""

    def __init__(self, spans):
        self.spans = [(base, bytes(blob)) for base, blob in spans]

    @classmethod
    def load(cls, paths):
        out = []
        for p in paths:
            base = int(p.rsplit("-", 1)[1].split(".")[0], 16)
            out.append((base - 0x40000000, open(p, "rb").read()))
        return cls(out)

    def word(self, io):
        for base, blob in self.spans:
            if base <= io < base + len(blob) - 3:
                return struct.unpack_from(">I", blob, io - base)[0]
        return None

    def words(self, io, n):
        out = []
        for i in range(n):
            v = self.word(io + 4 * i)
            if v is None:
                return None
            out.append(v)
        return tuple(out)

    def bytes(self, io, n):
        for base, blob in self.spans:
            if base <= io and io + n <= base + len(blob):
                return blob[io - base: io - base + n]
        return None


class Walk:
    def __init__(self, mem):
        self.mem = mem
        self.missing = set()

    def run(self, start, limit=2_000_000):
        pos, stack, steps = start, [], 0
        seen_jumps = set()
        while steps < limit:
            v = self.mem.word(pos)
            if v is None:
                self.missing.add(pos)
                return
            if v == 0:                              # NOP
                zeros = getattr(self, "_z", 0) + 1
                self._z = zeros
                if zeros > 256:
                    return
                pos += 4
                continue
            self._z = 0
            steps += 1
            if v & 0xe0000003 == 0x20000000:       # JUMP
                tgt = v & 0x1ffffffc
                if pos in seen_jumps:
                    return
                seen_jumps.add(pos)
                pos = tgt
                continue
            if v & 3 == 2:                          # CALL
                stack.append(pos + 4)
                pos = v & ~3
                continue
            if v == 0x00020000:                     # RETURN
                if not stack:
                    return
                pos = stack.pop()
                continue
            count = (v >> 18) & 0x7ff
            meth = v & 0x1ffc
            noinc = bool(v & 0x40000000)
            args = self.mem.words(pos + 4, count)
            if args is None:
                self.missing.add(pos)
                return
            yield pos, meth, noinc, args
            pos += 4 + count * 4
