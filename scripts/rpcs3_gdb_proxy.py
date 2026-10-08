"""A GDB proxy that keeps RPCS3's stub alive across client sessions.

RPCS3's GDB stub allows exactly one client per emulator launch: when the client
disconnects the server thread dies for good (`rpcs3_debugger.py`, trap 2) and
RPCS3 shows "The PS3 application has likely crashed". A long-lived emulator
that scripts attach to one after another therefore needs something that stays
connected. This is it: `rpcs3-drive.py serve` connects upstream once, and every
script connects to the proxy's port instead of the stub's.

What it keeps true, so a script sees what a fresh launch gave it:

- **A client connects to a paused target** (trap 1: connecting pauses
  emulation). If the target is running the proxy interrupts it and swallows the
  stop reply before the client's first packet.
- **A client leaving does not freeze or kill anything** (traps 2 and 3): the
  upstream connection stays, and a target left paused is resumed.
- `qSupported` is answered from the first reply, so the stub never sees a
  second handshake.

Everything else is forwarded verbatim, acknowledgements included. One client at
a time; a second waits in `accept`.
"""

import select
import socket
import threading
import time


def packet(body):
    return b"$" + body + b"#" + b"%02x" % (sum(body) & 0xFF)


class Proxy:
    def __init__(self, upstream, listen, host="127.0.0.1"):
        self.upstream_addr = upstream
        self.listen_addr = listen
        self.host = host
        self.up = None
        self.running = False
        self.supported = b""
        self.stop = threading.Event()
        self.clients = 0

    def connect_upstream(self, tries=90):
        last = None
        for _ in range(tries):
            try:
                self.up = socket.create_connection((self.host, self.upstream_addr), 10)
                break
            except OSError as exc:
                last = exc
                time.sleep(1.0)
        else:
            raise last
        self.up.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        self.up.sendall(packet(b"qSupported"))
        reply = self._read_packet(self.up, timeout=25)
        self.supported = reply
        self.up.sendall(b"+")
        # Connecting paused the target; let it run until a client asks.
        self.up.sendall(packet(b"vCont;c"))
        self.running = True

    def _read_packet(self, sock, timeout):
        sock.settimeout(timeout)
        buf = b""
        while True:
            buf += sock.recv(4096)
            if b"$" in buf and b"#" in buf[buf.index(b"$"):] and \
                    len(buf) >= buf.index(b"#", buf.index(b"$")) + 3:
                start = buf.index(b"$")
                end = buf.index(b"#", start)
                return buf[start + 1:end]

    def _drain(self, seconds=0.2):
        self.up.settimeout(seconds)
        try:
            while self.up.recv(4096):
                pass
        except (socket.timeout, OSError):
            pass

    def pause_upstream(self):
        if not self.running:
            return
        self.up.sendall(b"\x03")
        self._read_packet(self.up, timeout=25)
        self.up.sendall(b"+")
        self.running = False

    def serve_client(self, client):
        client.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        self.pause_upstream()
        self._drain(0.05)
        try:
            while not self.stop.is_set():
                ready, _, _ = select.select([client, self.up], [], [], 1.0)
                if client in ready:
                    data = client.recv(65536)
                    if not data:
                        break
                    if b"$qSupported" in data:
                        client.sendall(packet(self.supported))
                        continue
                    if b"vCont;" in data:
                        self.running = True
                    self.up.sendall(data)
                if self.up in ready:
                    data = self.up.recv(65536)
                    if not data:
                        break
                    if b"$" in data:
                        self.running = False
                    client.sendall(data)
        except OSError:
            pass
        finally:
            client.close()
            if not self.running:
                try:
                    self.up.sendall(packet(b"vCont;c"))
                    self.running = True
                except OSError:
                    pass

    def run(self):
        server = socket.socket()
        server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        server.bind((self.host, self.listen_addr))
        server.listen(2)
        server.settimeout(1.0)
        while not self.stop.is_set():
            try:
                client, _ = server.accept()
            except socket.timeout:
                continue
            self.clients += 1
            self.serve_client(client)
        server.close()

    def start(self):
        thread = threading.Thread(target=self.run, daemon=True)
        thread.start()
        return thread
