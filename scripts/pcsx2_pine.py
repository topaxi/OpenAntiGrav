#!/usr/bin/env python3
"""PINE client for PCSX2 - read and write EE memory while the game runs.

PINE ("Protocol for Instrumentation of Emulators", the old "PCSX2 IPC") is a
length-prefixed binary protocol over a unix socket. It is compiled in to the
stock `pcsx2-qt` build, needs no debug build and no rebuild, and it is what
turns a screenshot into a measurement. `docs/reverse-engineering/pcsx2-debugger.md`
is the evidence page; the traps that cost a run are repeated here because a
caller needs them at the call site:

- **`EnablePINE = true` under `[EmuCore]` is off by default.** Without it the
  socket never appears and every connect gets `FileNotFoundError`, which reads
  exactly like "the emulator did not start".
- **The socket is `$XDG_RUNTIME_DIR/pcsx2.sock`, not `/tmp/pcsx2.sock`.** The
  `/tmp` path is only the fallback for a session with no runtime dir, and it is
  also the path most of the PINE documentation on the internet quotes.
  A non-default `PINESlot` appends `.<slot>` to the name.
- **PINE answers before the VM exists.** `MsgVersion` succeeds within a second
  of launch, while `MsgTitle`/`MsgID` return `FAIL` and `MsgStatus` returns
  `Shutdown` for another twenty. Poll `wait_for_game()`; a single early query
  looks like a title that failed to boot.
- **Opcodes are `0x00`-`0x0F`.** Not the `0xF0`-`0xF7` range that several
  third-party PINE clients use for the metadata commands - those are a
  different (later, unadopted) numbering, and every one of them returns `FAIL`
  here.
- **One request may carry many commands and gets exactly one result byte** for
  the whole batch, followed by the payloads concatenated in order. This is the
  difference between 14 000 reads a second and 50 000 000: use `batch()`.
- **Keep a request under ~256 KiB.** 65 536 read32s (a 320 KiB request) answer
  in 1.3 ms; a 640 KiB request never answers at all and the client hangs until
  its timeout. The emulator survives it, the connection does not.

Addresses are the EE physical addresses the Ghidra corpus already uses -
`docs/ghidra/functions/ps2-pulse-eu/names.tsv` rows work verbatim, no rebasing.
Verified against `g_crc32_table` and `g_tolower_table`; see the doc page.

CLI:

    python3 scripts/pcsx2_pine.py version
    python3 scripts/pcsx2_pine.py title
    python3 scripts/pcsx2_pine.py status
    python3 scripts/pcsx2_pine.py read 0x002849e8 8      # 8 words, hex
    python3 scripts/pcsx2_pine.py write32 0x00302ec0 0
    python3 scripts/pcsx2_pine.py savestate 3
    python3 scripts/pcsx2_pine.py loadstate 3
"""

import argparse
import os
import socket
import struct
import sys
import time

DEFAULT_SLOT = 28011

MSG_READ8 = 0x00
MSG_READ16 = 0x01
MSG_READ32 = 0x02
MSG_READ64 = 0x03
MSG_WRITE8 = 0x04
MSG_WRITE16 = 0x05
MSG_WRITE32 = 0x06
MSG_WRITE64 = 0x07
MSG_VERSION = 0x08
MSG_SAVESTATE = 0x09
MSG_LOADSTATE = 0x0A
MSG_TITLE = 0x0B
MSG_ID = 0x0C
MSG_UUID = 0x0D
MSG_GAME_VERSION = 0x0E
MSG_STATUS = 0x0F

RES_OK = 0x00
RES_FAIL = 0xFF

STATUS_RUNNING = 0
STATUS_PAUSED = 1
STATUS_SHUTDOWN = 2
STATUS_NAMES = {0: "running", 1: "paused", 2: "shutdown"}

READ_SIZES = {MSG_READ8: 1, MSG_READ16: 2, MSG_READ32: 4, MSG_READ64: 8}

#: The largest request this module will put on the wire in one go. Measured:
#: 320 KiB answers in 1.3 ms, 640 KiB hangs forever. See the module docstring.
MAX_REQUEST = 256 * 1024


class PineError(RuntimeError):
    pass


def socket_path(slot=DEFAULT_SLOT):
    """Where PCSX2 puts the PINE socket on this machine.

    `$XDG_RUNTIME_DIR/pcsx2.sock`, falling back to `/tmp`. A non-default slot
    appends `.<slot>`.
    """
    root = os.environ.get("XDG_RUNTIME_DIR") or "/tmp"
    name = "pcsx2.sock" if slot == DEFAULT_SLOT else "pcsx2.sock.%d" % slot
    return os.path.join(root, name)


class Pine:
    """A PINE connection. Each request opens and closes its own socket.

    PCSX2 serves one client at a time and closes the connection after every
    reply, which is why this is not a persistent-connection class - the
    per-request connect costs about 70 us and is not the bottleneck. Batching
    is (see `batch`).
    """

    def __init__(self, slot=DEFAULT_SLOT, timeout=10.0):
        self.path = socket_path(slot)
        self.timeout = timeout

    def _request(self, payload):
        if len(payload) + 4 > MAX_REQUEST:
            raise PineError(
                "request of %d bytes exceeds the %d-byte ceiling; split the batch"
                % (len(payload) + 4, MAX_REQUEST))
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.settimeout(self.timeout)
        try:
            s.connect(self.path)
        except (FileNotFoundError, ConnectionRefusedError) as exc:
            raise PineError(
                "no PINE socket at %s (%s). Is PCSX2 running with "
                "`EnablePINE = true` under [EmuCore]?" % (self.path, exc)) from exc
        try:
            s.sendall(struct.pack("<I", 4 + len(payload)) + payload)
            header = self._recv_exact(s, 4)
            total = struct.unpack("<I", header)[0]
            body = self._recv_exact(s, total - 4)
        finally:
            s.close()
        if not body:
            raise PineError("empty PINE reply")
        if body[0] != RES_OK:
            raise PineError("PINE returned FAIL (0x%02x)" % body[0])
        return body[1:]

    @staticmethod
    def _recv_exact(sock, n):
        buf = b""
        while len(buf) < n:
            chunk = sock.recv(n - len(buf))
            if not chunk:
                raise PineError("PINE closed the connection mid-reply")
            buf += chunk
        return buf

    def _string_command(self, opcode):
        body = self._request(bytes([opcode]))
        length = struct.unpack("<I", body[:4])[0]
        return body[4:4 + length].rstrip(b"\x00").decode("utf-8", "replace")

    def version(self):
        """The emulator build string, e.g. `PCSX2 v2.7.494`. Works with no game."""
        return self._string_command(MSG_VERSION)

    def title(self):
        """The running game's title. FAILs when no VM is up."""
        return self._string_command(MSG_TITLE)

    def game_id(self):
        """The disc serial, e.g. `SCES-54748`."""
        return self._string_command(MSG_ID)

    def game_uuid(self):
        """The ELF CRC PCSX2 keys per-game settings on, e.g. `f8ae6ff2`."""
        return self._string_command(MSG_UUID)

    def game_version(self):
        return self._string_command(MSG_GAME_VERSION)

    def status(self):
        """0 running, 1 paused, 2 shutdown."""
        return struct.unpack("<I", self._request(bytes([MSG_STATUS]))[:4])[0]

    def savestate(self, slot):
        """Write save slot `slot`. The file lands in the datapath's `sstates/`."""
        self._request(bytes([MSG_SAVESTATE, slot & 0xFF]))

    def loadstate(self, slot):
        self._request(bytes([MSG_LOADSTATE, slot & 0xFF]))

    def read(self, addr, opcode=MSG_READ32):
        body = self._request(struct.pack("<BI", opcode, addr))
        return int.from_bytes(body[:READ_SIZES[opcode]], "little")

    def read8(self, addr):
        return self.read(addr, MSG_READ8)

    def read16(self, addr):
        return self.read(addr, MSG_READ16)

    def read32(self, addr):
        return self.read(addr, MSG_READ32)

    def read64(self, addr):
        return self.read(addr, MSG_READ64)

    def read_words(self, addr, count):
        """`count` consecutive u32 from `addr`, in as few requests as fit."""
        out = []
        per_request = (MAX_REQUEST - 8) // 5
        done = 0
        while done < count:
            n = min(per_request, count - done)
            payload = b"".join(
                struct.pack("<BI", MSG_READ32, addr + 4 * (done + i))
                for i in range(n))
            body = self._request(payload)
            out.extend(struct.unpack("<%dI" % n, body[:4 * n]))
            done += n
        return out

    def read_bytes(self, addr, length):
        """`length` bytes from `addr`, assembled from u32 reads."""
        words = self.read_words(addr & ~3, (length + (addr & 3) + 3) // 4)
        blob = b"".join(struct.pack("<I", w) for w in words)
        return blob[addr & 3:(addr & 3) + length]

    def write(self, addr, value, opcode=MSG_WRITE32):
        width = {MSG_WRITE8: 1, MSG_WRITE16: 2, MSG_WRITE32: 4, MSG_WRITE64: 8}[opcode]
        payload = struct.pack("<BI", opcode, addr) + value.to_bytes(width, "little")
        self._request(payload)

    def write8(self, addr, value):
        self.write(addr, value, MSG_WRITE8)

    def write32(self, addr, value):
        self.write(addr, value, MSG_WRITE32)

    def batch(self, commands):
        """Run many commands in one request and return the raw payload blob.

        `commands` is a list of already-packed command bytes. The reply carries
        one result byte for the whole batch, then every command's payload
        concatenated in order - so the caller has to know the widths it asked
        for. `read_words` is the common case already done.
        """
        return self._request(b"".join(commands))

    def wait_for_game(self, timeout=120.0, poll=1.0):
        """Block until a VM is up and answering. Returns the game id.

        This is the poll the module docstring's third trap needs: PINE answers
        `MsgVersion` long before there is a game, so a script that queries once
        and gives up sees a boot failure that is not there.
        """
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                if self.status() == STATUS_RUNNING:
                    return self.game_id()
            except PineError:
                pass
            time.sleep(poll)
        raise PineError("no running VM after %.0f s" % timeout)


def _parse_int(text):
    return int(text, 0)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--slot", type=int, default=DEFAULT_SLOT,
                        help="PINE slot, matching [EmuCore] PINESlot")
    parser.add_argument("--timeout", type=float, default=10.0)
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("version")
    sub.add_parser("title")
    sub.add_parser("status")
    sub.add_parser("info")

    p_read = sub.add_parser("read")
    p_read.add_argument("addr", type=_parse_int)
    p_read.add_argument("count", type=int, nargs="?", default=1)

    p_write = sub.add_parser("write32")
    p_write.add_argument("addr", type=_parse_int)
    p_write.add_argument("value", type=_parse_int)

    # Deliberately `index`, not `slot`: the top-level `--slot` is the PINE
    # socket slot, and a positional of the same name silently overwrote it -
    # `loadstate 2` then looked for `pcsx2.sock.2` and reported no emulator.
    p_save = sub.add_parser("savestate")
    p_save.add_argument("index", type=int)
    p_load = sub.add_parser("loadstate")
    p_load.add_argument("index", type=int)

    p_wait = sub.add_parser("wait")
    p_wait.add_argument("--timeout", type=float, default=120.0)

    args = parser.parse_args(argv)
    pine = Pine(slot=args.slot, timeout=args.timeout)

    try:
        if args.command == "version":
            print(pine.version())
        elif args.command == "title":
            print(pine.title())
        elif args.command == "status":
            code = pine.status()
            print("%d (%s)" % (code, STATUS_NAMES.get(code, "?")))
        elif args.command == "info":
            print("emulator    %s" % pine.version())
            code = pine.status()
            print("status      %d (%s)" % (code, STATUS_NAMES.get(code, "?")))
            if code != STATUS_SHUTDOWN:
                print("title       %s" % pine.title())
                print("id          %s" % pine.game_id())
                print("uuid        %s" % pine.game_uuid())
                print("version     %s" % pine.game_version())
        elif args.command == "read":
            words = pine.read_words(args.addr, args.count)
            for i in range(0, len(words), 8):
                print("%08x  %s" % (args.addr + 4 * i,
                                    " ".join("%08x" % w for w in words[i:i + 8])))
        elif args.command == "write32":
            pine.write32(args.addr, args.value)
            print("wrote %08x to %08x" % (args.value, args.addr))
        elif args.command == "savestate":
            pine.savestate(args.index)
            print("saved slot %d" % args.index)
        elif args.command == "loadstate":
            pine.loadstate(args.index)
            print("loaded slot %d" % args.index)
        elif args.command == "wait":
            print(pine.wait_for_game(timeout=args.timeout))
    except PineError as exc:
        print("error: %s" % exc, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
