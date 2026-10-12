#!/usr/bin/env python3
"""Read a running Vita3K's guest memory: a daemon that owns Vita3K, and a client.

`ptrace_scope` 1 lets only an ancestor open `/proc/<pid>/mem`, so the daemon
launches Vita3K itself (through `scripts/vita3k-drive.py`'s own launcher and
display pipeline) and stays up, serving reads on a unix socket. The mechanism
is `docs/reverse-engineering/vita3k-capture.md`'s: guest address = host
address - `0x400000000`, a pause is `SIGSTOP`/`SIGCONT`.

    OAG_VITA3K_CACHE=<dir> OAG_VITA3K_CONFIG=<dir>/config.yml \\
        python3 scripts/vita3k-mem.py serve --display 92 --socket wayland-oag92
    python3 scripts/vita3k-mem.py read 0x8153fd24 4      # hex bytes
    python3 scripts/vita3k-mem.py u32 0x8153fd24
    python3 scripts/vita3k-mem.py ai-sample --out s.json  # 2048 opponents, paused
    python3 scripts/vita3k-mem.py quit                    # stops Vita3K too

Menus are driven with `scripts/vita3k-drive.py tap/key/hold/shot` against the
same `OAG_VITA3K_CACHE` and `--display`. Vita3K's audio is whatever the config
says: set `audio-volume: 0` in it.

`ai-sample` reads Wipeout 2048's (EU v1.04) racer table and opponent drivers
at the offsets `docs/ghidra/functions/vita-2048-eu-v104/ai-controls.md`
records, inside one stop, so the eight records are from one frame (a stop can
still land mid-frame; the reader of the samples has to allow for it).
"""

import argparse
import importlib.util
import json
import os
import signal
import socket
import struct
import sys
import time
from pathlib import Path

HOST_OFFSET = 0x400000000
HERE = Path(__file__).resolve().parent


def cache_dir():
    path = os.environ.get("OAG_VITA3K_CACHE")
    if not path:
        sys.exit("set OAG_VITA3K_CACHE to a directory of this session's own")
    return Path(path)


def sock_path():
    return cache_dir() / "mem.sock"


def load_drive():
    spec = importlib.util.spec_from_file_location("vita3k_drive", HERE / "vita3k-drive.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def serve(args):
    drive = load_drive()
    drive.bring_up_display(args.display, args.socket)
    pid = drive.launch_vita3k(args.title, args.display)
    print("vita3k pid %d" % pid, flush=True)
    mem = None
    path = sock_path()
    path.unlink(missing_ok=True)
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(str(path))
    server.listen(4)
    while True:
        conn, _ = server.accept()
        with conn:
            line = conn.makefile().readline().split()
            if not line:
                continue
            op = line[0]
            try:
                if mem is None:
                    mem = open("/proc/%d/mem" % pid, "rb", buffering=0)
                if op == "R":
                    addr, length = int(line[1], 0), int(line[2], 0)
                    mem.seek(addr + HOST_OFFSET)
                    conn.sendall(("OK " + mem.read(length).hex() + "\n").encode())
                elif op == "W":
                    addr, data = int(line[1], 0), bytes.fromhex(line[2])
                    with open("/proc/%d/mem" % pid, "r+b", buffering=0) as rw:
                        rw.seek(addr + HOST_OFFSET)
                        rw.write(data)
                    conn.sendall(b"OK\n")
                elif op == "STOP":
                    os.kill(pid, signal.SIGSTOP)
                    conn.sendall(b"OK\n")
                elif op == "CONT":
                    os.kill(pid, signal.SIGCONT)
                    conn.sendall(b"OK\n")
                elif op == "QUIT":
                    os.kill(pid, signal.SIGCONT)
                    os.kill(pid, signal.SIGTERM)
                    # Vita3K has been seen to ignore SIGTERM for 30 s and more.
                    for _ in range(50):
                        if os.waitpid(pid, os.WNOHANG)[0]:
                            break
                        time.sleep(0.1)
                    else:
                        os.kill(pid, signal.SIGKILL)
                        os.waitpid(pid, 0)
                    conn.sendall(b"OK\n")
                    path.unlink(missing_ok=True)
                    return
                else:
                    conn.sendall(b"ERR unknown\n")
            except OSError as error:
                mem = None
                conn.sendall(("ERR %s\n" % error).encode())


def call(line):
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.settimeout(10.0)
    client.connect(str(sock_path()))
    client.sendall((line + "\n").encode())
    reply = client.makefile().readline().strip()
    client.close()
    if not reply.startswith("OK"):
        raise RuntimeError(reply)
    return reply[3:]


def read(addr, length):
    return bytes.fromhex(call("R 0x%x %d" % (addr, length)))


def u32(addr):
    return struct.unpack("<I", read(addr, 4))[0]


def f32(addr):
    return struct.unpack("<f", read(addr, 4))[0]


# Wipeout 2048 EU v1.04, read statically (ai-controls.md).
GAME_MODE = 0x8153FD24  # g_GameState+0xe4, the mode id
CLASS = 0x8153FD18  # +0xd8
LEVEL = 0x8153FD20  # +0xe0
PLAYER_RACER = 0x8189D8C8  # pointer to the reference (player) racer record
PLAYER_CRAFT = 0x8189D8D0
RACERS = 0x8189D8D8  # racer i at RACERS + i * 0x990
RACER_STRIDE = 0x990
RACER_COUNT = 0x818A2558  # DAT_818a2558, the racer count the thrust scan walks
RACE_MANAGER = 0x8189BF3C  # the race manager; race time at +0x10e3c
GAME_MODE_OBJECT = 0x81965B58  # the current event (GameModeBase); m_skillScale at +0x1a8


def racer_fields(base):
    blob = read(base, RACER_STRIDE)

    def fl(off):
        return struct.unpack_from("<f", blob, off)[0]

    def word(off):
        return struct.unpack_from("<I", blob, off)[0]

    return {
        "addr": base,
        "craft": word(0x0),
        "driver": word(0x4),
        "finished": blob[0x7A],
        "lap_distance": fl(0x88),
        "progress": fl(0x890),
        "place": word(0x96C),
        "lane_tick": blob[0x988],
        "lap_fraction": fl(0x8E8),
    }


def driver_fields(addr):
    blob = read(addr, 0x5D0)

    def fl(off):
        return struct.unpack_from("<f", blob, off)[0]

    return {
        "addr": addr,
        "hd_lineage": blob[0x98],
        "steer": fl(0xD8),
        "thrust": fl(0xDC),
        "left": fl(0xE0),
        "right": fl(0xE4),
        "pad_skill": fl(0x9C),
        "avoid_skill": fl(0xA0),
        "ring_rate": fl(0xC4),
        "coin": struct.unpack_from("<I", blob, 0xC8)[0],
        "chase": blob[0xCC],
        "avoid_boost": fl(0x160),
        "spread": fl(0x16C),
        "wander": [fl(0x170 + 4 * k) for k in range(7)],
        "gap": fl(0x194),
        "accumulator": fl(0x198),
        "class_record": struct.unpack_from("<I", blob, 0x234)[0],
        "look_ahead_secs": fl(0x23C),
        "steer_mul": fl(0x240),
        "smoothed_speed": fl(0x26C),
        "halved": struct.unpack_from("<I", blob, 0x5BC)[0],
    }


def ai_sample():
    call("STOP")
    try:
        sample = {
            "time": time.time(),
            "mode": u32(GAME_MODE),
            "class": u32(CLASS),
            "level": u32(LEVEL),
            "player_racer": u32(PLAYER_RACER),
            "player_craft": u32(PLAYER_CRAFT),
            "count": u32(RACER_COUNT),
            "racers": [],
        }
        race_manager = u32(RACE_MANAGER)
        sample["race_manager"] = race_manager
        if race_manager:
            sample["race_time"] = f32(race_manager + 0x10E3C)
        for i in range(8):
            racer = racer_fields(RACERS + i * RACER_STRIDE)
            if racer["driver"]:
                racer["driver_state"] = driver_fields(racer["driver"])
                record = racer["driver_state"]["class_record"]
                if record and "class_record" not in sample:
                    sample["class_record"] = {"addr": record, "hex": read(record, 0x118).hex()}
            sample["racers"].append(racer)
    finally:
        call("CONT")
    return sample


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("serve")
    p.add_argument("--display", type=int, required=True)
    p.add_argument("--socket", required=True)
    p.add_argument("--title", default="PCSF00007")
    p = sub.add_parser("read")
    p.add_argument("addr")
    p.add_argument("length")
    p = sub.add_parser("u32")
    p.add_argument("addr")
    p = sub.add_parser("write-f32", help="write one float, e.g. a 2048 event's m_skillScale")
    p.add_argument("addr")
    p.add_argument("value", type=float)
    p = sub.add_parser("skill", help="2048: the current event's m_skillScale; with a value, write it")
    p.add_argument("value", type=float, nargs="?")
    p = sub.add_parser("ai-sample")
    p.add_argument("--out")
    p.add_argument("--count", type=int, default=1)
    p.add_argument("--gap", type=float, default=1.0)
    sub.add_parser("stop")
    sub.add_parser("cont")
    sub.add_parser("quit")
    args = parser.parse_args()
    if args.command == "serve":
        serve(args)
    elif args.command == "read":
        print(read(int(args.addr, 0), int(args.length, 0)).hex())
    elif args.command == "u32":
        print("0x%08x" % u32(int(args.addr, 0)))
    elif args.command == "write-f32":
        call("W 0x%x %s" % (int(args.addr, 0), struct.pack("<f", args.value).hex()))
    elif args.command == "skill":
        event = u32(GAME_MODE_OBJECT)
        if not event:
            sys.exit("no current game-mode object (not a 2048 event)")
        if args.value is not None:
            call("W 0x%x %s" % (event + 0x1A8, struct.pack("<f", args.value).hex()))
        print("event 0x%08x m_skillScale %g" % (event, f32(event + 0x1A8)))
    elif args.command == "ai-sample":
        samples = []
        for k in range(args.count):
            if k:
                time.sleep(args.gap)
            samples.append(ai_sample())
        text = json.dumps(samples, indent=1)
        if args.out:
            Path(args.out).write_text(text)
        else:
            print(text)
    else:
        print(call(args.command.upper()))


if __name__ == "__main__":
    main()
