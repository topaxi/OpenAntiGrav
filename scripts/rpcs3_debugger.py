"""A client for RPCS3's GDB remote-serial-protocol stub, for PS3 titles.

The PSP side of this project drives PPSSPP through a JSON-over-websocket
debugger (`scripts/ppsspp_debugger.py`). RPCS3 has no equivalent API; what it
does have is a GDB stub, enabled by the `GDB Server: 127.0.0.1:2345` key that
is already in a stock `config.yml` and needs neither `Debug Console Mode` nor a
special build. This module speaks that protocol directly rather than shelling
out to `gdb`, because the interesting operations are memory reads at known
addresses and there is no PPC64 `gdb` to rely on being installed.

**The stub is a much smaller surface than PPSSPP's debugger**, and the four
traps below each cost a full 40-second reboot to find. They are the reason this
module exists as something other than a socket:

1. **Connecting pauses emulation.** The stub pauses the moment a client
   attaches, so a connect is never passive - `resume()` before timing anything.
2. **The server thread dies for good when the client disconnects**, with
   `Tried to read char, but no data was available` in `RPCS3.log`. It does not
   start listening again, but the *socket stays bound*, so a later relaunch
   silently has no debugger and every packet times out. One debug session per
   emulator launch, and kill the old process before starting a new one.
3. **Disconnecting while paused freezes the emulator** (`Emulation has been
   frozen!`). Disconnecting while resumed leaves the game running, debugger-less
   but alive - so `resume()` before dropping the socket if the run should
   continue.
4. **Bare `c` and `s` are not implemented** - they return an empty packet, which
   reads exactly like an immediate breakpoint hit and will happily fake a whole
   session's worth of results. `vCont;c` and `vCont;s:<tid>` are the real ones.

**Nothing but `\x03` is answered while the target is running.** An `m` sent
after `vCont;c` gets no reply at all until something stops the CPU - not a slow
one, none - so a client that polls a running target hangs rather than paying for
it. PPSSPP's equivalent is a cost (520 ms against 0.3 ms); this is a wall.

Round trips against a *stopped* target cost about 41 ms each with `TCP_NODELAY`
set and about 83 ms without, so `TCP_NODELAY` is not optional here. That budget
- roughly 24 operations a second - is what shapes every caller: read a whole
struct in one `m` packet (the stub advertises `PacketSize=1200`, so ~590 bytes a
read), and do not expect to inject or sample anything per-frame at 60 Hz.

Guest addresses are the ELF's own virtual addresses, with no rebasing: `m10000`
reads the `\x7fELF` header of the decrypted EBOOT, so every address in
`docs/ghidra/functions/ps3-hdfury-eu/names.tsv` can be used as-is.
"""

import os
import socket
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import emu_guard  # noqa: E402

DEFAULT_PORT = 2345

# Offsets into the `g` register dump, in bytes. Derived from the dump's own
# length (556) and confirmed by LR landing in the EBOOT's code range: 32 GPRs
# and 32 FPRs of 8 bytes each, then PC, MSR, CR, LR, CTR and a trailing pair.
# MSR, XER and FPSCR come back as literal `x` characters - the stub's way of
# saying "not supported" - so nothing here reads them.
REG_GPR = 0
REG_FPR = 256
REG_PC = 512
REG_CR = 528
REG_LR = 532
REG_CTR = 540
REG_DUMP_BYTES = 556

# WipEout HD / Fury (BCES-00664): the decrypted EBOOT's code and data ranges,
# used only to tell "a PC inside the game" from one inside a loaded PRX.
HD_CODE_START = 0x00010000
HD_CODE_END = 0x00858E48


class GdbError(RuntimeError):
    """The stub answered a packet with an error, or did not answer at all."""


class Debugger:
    """One GDB session against a running RPCS3.

    There is only ever one per emulator launch - see trap 2 in the module
    docstring - so this deliberately owns the connection for its whole life and
    has no reconnect path.
    """

    def __init__(self, port=DEFAULT_PORT, host="127.0.0.1", timeout=25.0):
        emu_guard.stage("capture")
        self.sock = socket.create_connection((host, port), timeout)
        self.sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        self.sock.settimeout(timeout)
        self.buf = b""
        self.running = False
        self.packet_size = 0x400
        reply = self.cmd("qSupported")
        for field in reply.split(";"):
            if field.startswith("PacketSize="):
                self.packet_size = int(field.split("=", 1)[1], 16)

    def close(self):
        try:
            self.sock.close()
        except OSError:
            pass

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()

    def _fill(self):
        chunk = self.sock.recv(4096)
        if not chunk:
            raise GdbError("stub closed the connection")
        self.buf += chunk

    def send(self, body):
        checksum = sum(body.encode()) & 0xFF
        self.sock.sendall(b"$" + body.encode() + b"#" + b"%02x" % checksum)

    def read_packet(self):
        """The next `$...#xx` packet, acknowledged. Bare `+`/`-` are skipped."""
        while b"$" not in self.buf:
            self._fill()
        self.buf = self.buf[self.buf.index(b"$") :]
        while b"#" not in self.buf[1:] or len(self.buf) < self.buf.index(b"#") + 3:
            self._fill()
        end = self.buf.index(b"#")
        body = self.buf[1:end].decode(errors="replace")
        self.buf = self.buf[end + 3 :]
        self.sock.sendall(b"+")
        return body

    def cmd(self, body):
        self.send(body)
        packet = self.read_packet()
        emu_guard.beat()
        return packet

    def pause(self, timeout=25.0):
        """Interrupt a running target. A no-op if it is already stopped.

        The stop reply to `\x03` can be seconds late - the stub only notices at
        the next scheduling point - so this reads it on its own generous
        timeout rather than the connection's.
        """
        if not self.running:
            return
        self.sock.sendall(b"\x03")
        previous = self.sock.gettimeout()
        self.sock.settimeout(timeout)
        try:
            self.read_packet()
        finally:
            self.sock.settimeout(previous)
        self.running = False

    def resume(self):
        """`vCont;c`. Does not wait - call `wait_for_stop` for that."""
        self.send("vCont;c")
        self.running = True

    def step(self, tid, timeout=5.0):
        """`vCont;s:<tid>` - one instruction on one thread, every other thread
        left stopped. Returns the stop reply, or `None` if none came in time
        (the target is then still running: `pause()` before reading).

        The one way to move a thread off a breakpoint address without letting
        the rest of the frame run: `run_for(0.03)` lets every later call in
        the same frame go by, so a breakpoint on a per-object function samples
        the *first* object of each frame and almost nothing else - measured
        2026-09-15, 155 of 160 hits on one of eight crafts.
        """
        self.send("vCont;s:%s" % tid)
        self.running = True
        return self.wait_for_stop(timeout)

    def wait_for_stop(self, timeout=30.0):
        """The stop reply after a resume, or `None` if nothing stopped in time.

        A timeout leaves the target running and the read buffer clean, so the
        caller can go on to `pause()`.
        """
        previous = self.sock.gettimeout()
        self.sock.settimeout(timeout)
        try:
            reply = self.read_packet()
            self.running = False
            return reply
        except (TimeoutError, socket.timeout):
            self.buf = b""
            return None
        finally:
            self.sock.settimeout(previous)

    def wait_at(self, address, tries=12, slice_seconds=0.5):
        """`(tid, registers)` for a thread parked at `address`, or `(None, None)`.

        **Use this and not `wait_for_stop()` for breakpoint work.** RPCS3 parks
        a thread at a `Z0` breakpoint *without ever sending a stop reply for
        it*, so a `wait_for_stop()` on a breakpoint that has already hit blocks
        until its timeout and then reports nothing - which reads exactly like
        "the address never executes" and is how this harness twice concluded a
        function does not run during a race when a thread was sitting in it the
        whole time. The only reliable question is "is any thread's PC here",
        and asking it means stopping the target first.
        """
        for _ in range(tries):
            self.resume()
            time.sleep(slice_seconds)
            self.pause()
            for tid in self.threads():
                regs = self.registers(tid)
                if regs is None:
                    continue
                pc = int.from_bytes(regs[REG_PC:REG_PC + 8], "big")
                if pc == address:
                    return tid, regs
        return None, None

    def run_for(self, seconds):
        """Let the game run for a wall-clock stretch, then stop it again."""
        self.resume()
        time.sleep(seconds)
        self.pause()

    def drain(self, settle=0.4):
        """Throw away packets the stub volunteered, and say how many there were.

        **Call this after every breakpoint stop.** A breakpoint set on a
        function several threads run - `FwMutex_Lock`, say - is hit by more than
        one of them, and each hit queues its own stop reply. The first read
        issued afterwards then consumes a *stop reply* as if it were its answer,
        every later packet is off by one, and the session dies on a timeout that
        looks exactly like a hung emulator.
        """
        previous = self.sock.gettimeout()
        self.sock.settimeout(settle)
        dropped = 0
        try:
            while True:
                self.read_packet()
                dropped += 1
        except (TimeoutError, socket.timeout):
            self.buf = b""
        finally:
            self.sock.settimeout(previous)
        return dropped

    def read(self, address, size):
        """`size` bytes of guest memory. Split to respect `PacketSize`.

        Raises rather than hanging if the target is running - the stub would
        never answer, and a blocked `recv` looks like a dead emulator.
        """
        self._require_stopped("read")
        out = bytearray()
        step = max(16, (self.packet_size - 16) // 2)
        while size:
            take = min(size, step)
            reply = self.cmd("m%x,%x" % (address, take))
            if reply.startswith("E") or not reply:
                raise GdbError("read %#x+%#x: %r" % (address, take, reply))
            out += bytes.fromhex(reply)
            address += take
            size -= take
        return bytes(out)

    def write(self, address, data):
        """Write guest memory. Read-only pages answer `E03` rather than lying."""
        self._require_stopped("write")
        reply = self.cmd("M%x,%x:%s" % (address, len(data), data.hex()))
        if reply != "OK":
            raise GdbError("write %#x (%d bytes): %r" % (address, len(data), reply))

    def _require_stopped(self, what):
        if self.running:
            raise GdbError(
                "cannot %s while the target is running - the stub answers "
                "nothing but an interrupt; pause() first" % what
            )

    def threads(self):
        reply = self.cmd("qfThreadInfo")
        return reply.lstrip("m").split(",") if reply.startswith("m") else []

    def select(self, tid):
        return self.cmd("Hg" + tid) == "OK"

    def registers(self, tid=None):
        """The raw `g` dump as bytes, or `None` if the stub declines the thread.

        Unsupported registers come back as `x` characters; those bytes are
        zero-filled here so the offsets above stay usable.
        """
        if tid is not None and not self.select(tid):
            return None
        dump = self.cmd("g")
        if len(dump) < REG_DUMP_BYTES * 2:
            return None
        clean = "".join("0" if c == "x" else c for c in dump[: REG_DUMP_BYTES * 2])
        return bytes.fromhex(clean)

    def pc(self, tid=None):
        regs = self.registers(tid)
        return None if regs is None else int.from_bytes(regs[REG_PC : REG_PC + 8], "big")

    def gpr(self, index, tid=None):
        regs = self.registers(tid)
        if regs is None:
            return None
        at = REG_GPR + index * 8
        return int.from_bytes(regs[at : at + 8], "big")

    def add_breakpoint(self, address, kind=4):
        """A software breakpoint.

        **`OK` here is not a promise that it will ever fire.** RPCS3 only
        honours PPU breakpoints under an interpreter decoder; under the default
        `PPU Decoder: Recompiler (LLVM)` the stub still answers `OK` and the
        breakpoint is silently dead. HLE import stubs (the addresses
        `ppu_loader` logs as `sys_io import: [cellPadGetData] -> 0x...`) never
        fire under either decoder - they are not PPU code paths.
        """
        reply = self.cmd("Z0,%x,%x" % (address, kind))
        if reply != "OK":
            raise GdbError("breakpoint at %#x: %r" % (address, reply))

    def remove_breakpoint(self, address, kind=4):
        self.cmd("z0,%x,%x" % (address, kind))
