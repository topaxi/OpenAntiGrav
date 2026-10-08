#!/usr/bin/env python3
"""Fire HD's Rocket on RPCS3 and poll the SPU light candidate list without pausing.

    OAG_RPCS3_DISPLAY=97 OAG_RPCS3_GDB=127.0.0.1:23497 ... \\
        uv run --with evdev python3 scripts/rpcs3-hd-lightpoll.py <out-dir> [shots]

Reads `0x00f4b300 + 0x2098` (count) and `+ 0x20a0` (32-byte records: position,
exponent w | colour, range D) through `/proc/PID/mem` as fast as Python goes and
writes every distinct list with its time since the press to `<out-dir>/poll.json`.
A pause-and-snapshot driver misses a light that lives one to three frames; this
is how the Rocket's `(14, 10, 2)`, `D = 100` wall-contact flash was caught.
"""
import importlib.util, json, struct, sys, time
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / 'scripts'))
spec = importlib.util.spec_from_file_location('rpcs3_drive', ROOT / 'scripts/rpcs3-drive.py')
drive = importlib.util.module_from_spec(spec); spec.loader.exec_module(drive)
import rpcs3_place as place
from rpcs3_debugger import Debugger
OUT = Path(sys.argv[1]); OUT.mkdir(parents=True, exist_ok=True)
SHOTS = int(sys.argv[2]) if len(sys.argv) > 2 else 2
IMAGE = str(ROOT / 'data/images/hdfury-ps3-eu-dec.iso')
u32 = lambda g, a: struct.unpack('>I', g.read(a, 4))[0]
BASE = 0x00f4b300
GUEST = 0x300000000

port = int(drive.GDB_SERVER.rsplit(':', 1)[1])
with drive.Session(IMAGE, str(OUT / 'logs')) as session:
    print('rpcs3 pid %d' % session.proc.pid, flush=True)
    (OUT / 'rpcs3.pid').write_text(str(session.proc.pid))
    if not session.wait_for_screen_pressing('Main Menu', 240): sys.exit('no main menu')
    session.settle_menu(20)
    if session.walk_to_race() not in drive.RACE_ARRIVED: sys.exit('no race')
    session.wait_for_load(70)
    session.tap('cross', settle=22.0)
    g = Debugger(port=port)
    g.pause(); ship, body = place.find_player(g)
    slot = u32(g, ship + 0x5edc); g.resume()
    mem = open('/proc/%d/mem' % session.proc.pid, 'rb', 0)
    def rd(a, n):
        mem.seek(GUEST + a); return mem.read(n)
    def snap():
        hdr = struct.unpack('>I', rd(BASE + 0x2098, 4))[0]
        cnt = min(hdr, 64)
        raw = rd(BASE + 0x20a0, 32 * cnt)
        return hdr, tuple(tuple(round(x, 3) for x in struct.unpack('>8f', raw[i*32:(i+1)*32])) for i in range(cnt))
    log = []
    for shot in range(SHOTS):
        g.pause(); g.write(slot + 0x204, struct.pack('>ii', 0, 0)); g.resume(); time.sleep(0.6)
        last = None
        t_press = time.time()
        session.pad.set('triangle', True)
        pressed = True
        end = t_press + 3.0
        n = 0
        while time.time() < end:
            now = time.time()
            if pressed and now - t_press > 0.12:
                session.pad.set('triangle', False); pressed = False
            s = snap(); n += 1
            if s != last:
                log.append({'shot': shot, 't': round(now - t_press, 4), 'hdr': s[0], 'cand': s[1]})
                last = s
        print('shot', shot, 'polls', n, 'distinct', sum(1 for e in log if e['shot'] == shot), flush=True)
        time.sleep(2.0)
    (OUT / 'poll.json').write_text(json.dumps(log))
    print('done', len(log), flush=True)
