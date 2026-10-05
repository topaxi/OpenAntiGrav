"""A client for PPSSPP's websocket debugger, for M3's verification harness.

PPSSPP exposes a JSON-over-websocket debugger, enabled with `--debugger=PORT` on
`PPSSPPHeadless` or `RemoteDebuggerOnStartup` in the config of the SDL build.
This module wraps the parts the harness needs and encodes the four traps that
cost a session to find; each is documented in
`docs/reverse-engineering/ppsspp-debugger.md`.

The one that shapes every caller: **memory reads cost about 520 ms while the CPU
is running and about 0.3 ms while it is stepping**, and a resume/break cycle
costs 0.5 ms. Per-tick capture is therefore breakpoint-driven - break, read the
whole struct in one call, resume - and never polled against a running CPU.

Needs `websocket-client`; run scripts through `uv run --with websocket-client`.
"""

import base64
import json
import struct
import time

from websocket import create_connection

PSP_CLOCK_HZ = 222_000_000

# Set by `Game_Bootstrap`; see docs/ghidra/functions/psp-pulse-usa/main-loop.md.
# The address holds a *pointer* to the state machine, not the machine itself.
G_STATE_MACHINE = 0x08B31784
STATE_NAME_OFFSET = 0x18C


class DebuggerError(RuntimeError):
    """The debugger answered a command with an error."""


class Debugger:
    """One connection to a running PPSSPP instance."""

    def __init__(self, port=47800, connect_timeout=20.0):
        self.ws = create_connection(
            f"ws://127.0.0.1:{port}/debugger", timeout=connect_timeout
        )
        self.ws.settimeout(0.05)
        self.pending = []
        self._ticket = 0

    def close(self):
        self.ws.close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()

    def send(self, event, **kw):
        self._ticket += 1
        ticket = str(self._ticket)
        self.ws.send(json.dumps({"event": event, "ticket": ticket, **kw}))
        return ticket

    def _recv(self):
        try:
            return json.loads(self.ws.recv())
        except Exception:
            return None

    def call(self, event, timeout=15.0, **kw):
        """Send a command and return its reply.

        Most commands answer the ticket they were sent with, but a few
        (`cpu.resume`) only broadcast an acknowledgement to every connected
        client, with no ticket at all, so both shapes count as the reply.
        """
        ticket = self.send(event, **kw)
        end = time.time() + timeout
        while time.time() < end:
            msg = self._recv()
            if msg is None:
                continue
            if msg.get("ticket") == ticket:
                if msg.get("event") == "error":
                    raise DebuggerError(f"{event}: {msg.get('message')}")
                return msg
            if "ticket" not in msg and msg.get("event") == event:
                return msg
            self.pending.append(msg)
        raise TimeoutError(f"no reply to {event}")

    def is_stepping(self):
        return self.call("cpu.status")["stepping"]

    def brk(self):
        """Stop the CPU, if it is not already stopped."""
        if not self.is_stepping():
            self.call("cpu.stepping")

    def resume(self):
        if self.is_stepping():
            self.call("cpu.resume")

    def add_breakpoint(self, address, condition=None):
        """Arm an execution breakpoint, optionally conditional (`"a0 >= 0x90af000 && a0 < 0x90b1000"`).

        The CPU **must be stepping** for this to take: a breakpoint added while
        the CPU runs is accepted, appears in `cpu.breakpoint.list`, and never
        fires. This is true of the JIT and of `--ir` alike.
        """
        if not self.is_stepping():
            raise DebuggerError(
                "breakpoints only arm while the CPU is stepping; call brk() first"
            )
        extra = {} if condition is None else {"condition": condition}
        self.call("cpu.breakpoint.add", address=address, enabled=True, **extra)

    def remove_breakpoint(self, address):
        self.call("cpu.breakpoint.remove", address=address)

    def wait_for_break(self, address, timeout=30.0):
        """Wait until the CPU stops *at* `address`.

        Changing the breakpoint list makes PPSSPP rebroadcast `cpu.stepping`
        carrying the pc it was already stopped at, so an event alone does not
        mean a breakpoint was hit. Only a matching pc does.
        """
        return self.wait_for_break_any((address,), timeout=timeout)

    def wait_for_break_any(self, addresses, timeout=30.0):
        """Wait until the CPU stops at any address in `addresses`.

        The same pc-matching rule as `wait_for_break`, over a set: the stepping
        rebroadcast trap applies identically, so only a pc in the set counts.
        """
        addresses = set(addresses)
        end = time.time() + timeout
        queued, self.pending = self.pending, []
        for msg in queued:
            if msg.get("event") == "cpu.stepping" and msg.get("pc") in addresses:
                return msg
        while time.time() < end:
            msg = self._recv()
            if msg is None:
                continue
            if msg.get("event") == "cpu.stepping" and msg.get("pc") in addresses:
                return msg
            self.pending.append(msg)
        raise TimeoutError(
            "never stopped at any of %s" % ", ".join("0x%08x" % a for a in addresses)
        )

    def each_hit(self, address, count, timeout=30.0, condition=None):
        """Yield at every one of the next `count` hits of `address`.

        The CPU is stopped inside the loop body, which is where reads are cheap,
        and resumed on the way to the next hit.
        """
        for index, msg in self.each_hit_any((address,), count, timeout=timeout, condition=condition):
            yield index, msg

    def each_hit_any(self, addresses, count, timeout=30.0, condition=None):
        """Yield `(index, msg)` at each of the next `count` hits of any address.

        The sibling of `each_hit` for more than one breakpoint; the pc that was
        actually hit is `msg["pc"]`. The CPU is stopped inside the loop body and
        every breakpoint is removed on the way out, however the loop ends.

        **On v1.20.4 only the most recently added execution breakpoint ever
        fires**, whatever `cpu.breakpoint.list` says: armed ship-then-camera, a
        live race stops only at the camera address, and armed camera-then-ship
        only at the ship address, both reproduced against a running Pulse while
        each breakpoint alone fires every tick. So with more than one address
        this loop is only useful for probes that expect one of them to be hit
        at all, not for interleaving two streams - a capture that needs a
        second structure per tick should learn its address from one hit and
        read it from the surviving breakpoint instead, which is what
        `psp-trace.py --camera` does.
        """
        self.brk()
        for address in addresses:
            self.add_breakpoint(address, condition)
        try:
            for index in range(count):
                # Everything queued so far describes a stop that is already
                # over. Without this, a `cpu.stepping` rebroadcast whose pc
                # happens to equal the breakpoint (the CPU was left stopped
                # there, or the stop being resumed from is the breakpoint
                # itself) satisfies the wait below while the CPU is running,
                # and the next resume fails with "CPU not stepping". A
                # `cpu.status` round trip pulls what is already on the socket
                # into the queue first, so the clear really is a clear.
                self.call("cpu.status")
                self.pending = []
                self.call("cpu.resume")
                yield index, self.wait_for_break_any(addresses, timeout=timeout)
        finally:
            self.brk()
            for address in addresses:
                self.remove_breakpoint(address)

    def read(self, address, size):
        reply = self.call("memory.read", address=address, size=size)
        return base64.b64decode(reply["base64"])

    def write(self, address, data):
        """Write raw bytes into emulated memory.

        **An empty payload kills the emulator outright** (HANDOVER.md records
        the corpse), so it is refused here rather than sent. Writes are only
        meaningful while the CPU is stepping, like everything else in a
        breakpoint-driven session; the proven precedent is the roll
        step-response measurement in `crates/physics/src/hover.rs`, which
        rewrote the rigid body's basis rows through this API.
        """
        if not data:
            raise DebuggerError(
                "refusing memory.write with an empty payload: it crashes PPSSPP"
            )
        self.call(
            "memory.write",
            address=address,
            base64=base64.b64encode(bytes(data)).decode("ascii"),
        )

    def write_u32(self, address, value):
        self.call("memory.write_u32", address=address, value=value)

    def write_f32s(self, address, values):
        """Write consecutive little-endian f32s starting at `address`."""
        values = list(values)
        self.write(address, struct.pack("<%df" % len(values), *values))

    def read_u32(self, address):
        return self.call("memory.read_u32", address=address)["value"]

    def read_u8(self, address):
        return self.read(address, 1)[0]

    def read_f32(self, address):
        return struct.unpack("<f", self.read(address, 4))[0]

    def read_f32s(self, address, count):
        return struct.unpack("<%df" % count, self.read(address, count * 4))

    def read_cstring(self, address, limit=64):
        raw = self.read(address, limit)
        end = raw.find(b"\0")
        return raw[: end if end >= 0 else limit].decode("latin-1", "replace")

    def state_name(self):
        """The front end's current state, e.g. `"LogoFMV"` or `"InGame"`.

        `G_STATE_MACHINE` holds a pointer to the machine; the name is an inline
        character buffer inside it rather than a pointer to `.rodata`.
        """
        machine = self.read_u32(G_STATE_MACHINE)
        if not 0x08000000 <= machine < 0x0A000000:
            return None
        return self.read_cstring(machine + STATE_NAME_OFFSET, 48)

    def press(self, button, duration=8):
        """Hold a button for `duration` frames. Names are PPSSPP's own:
        cross, circle, triangle, square, start, select, up, down, left, right."""
        self.call("input.buttons.press", button=button, duration=duration)

    def hold(self, **buttons):
        """Set the held state of any number of buttons, e.g. `hold(cross=True)`."""
        self.call("input.buttons.send", buttons=buttons)

    def analog(self, x, y, stick="left"):
        self.call("input.analog.send", stick=stick, x=x, y=y)
