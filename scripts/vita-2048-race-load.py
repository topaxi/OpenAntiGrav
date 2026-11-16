#!/usr/bin/env python3
"""Walk Vita3K's WipEout 2048 into a campaign race load and record it: frames plus the loading screen's own fields.

    python3 scripts/emu-run.py --silence 140 --total 500 -- \\
        python3 scripts/vita-2048-race-load.py --profile <scratch>/v3k --out <scratch>/frames/run --node 295 60

The race load is `LoadingScreen` kind 4 (`docs/ghidra/functions/vita-2048-eu-v104/race-load.md`). The walk is
`vita-2048-pair.py`'s as far as the event card; from the Play tap on this script samples the guest's memory every few
milliseconds (the screen object behind `g_LoadingScreen`, the current game mode, the event card's page list and the
meter's frame counter) and grabs a frame about four times a second until the screen object is gone. Everything is
written to `<out>/samples.json` beside the frames. Vita3K is this script's child (the memory is read through
`/proc/<pid>/mem`) and is killed by pid at the end.

`--profile` is a private Vita3K profile: `<profile>/vhome/.config/Vita3K/config.yml` and `<profile>/vc/config.yml`
with `pref-path` pointing at a private tree (`docs/reverse-engineering/vita3k-capture.md`, "A private profile").
`--node` is the event's node on this save's campaign map (`--stop map` grabs the map and stops, to read it off).
"""
import argparse, importlib.util, json, os, signal, struct, sys, time
from pathlib import Path

HERE = Path(__file__).resolve().parent


def private_env(profile):
    """Point this process, and so Vita3K and the drive script, at the private profile."""
    os.environ["HOME"] = str(profile / "vhome")
    os.environ["SDL_AUDIODRIVER"] = "dummy"
    os.environ["OAG_VITA3K_CACHE"] = str(profile / "vc")
    os.environ["OAG_VITA3K_CONFIG"] = str(profile / "vc" / "config.yml")
    for v in ("XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_STATE_HOME", "XDG_CACHE_HOME"):
        os.environ.pop(v, None)


def load_pair():
    sys.path.insert(0, str(HERE))
    spec = importlib.util.spec_from_file_location("vita_pair", HERE / "vita-2048-pair.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod

class Mem:
    def __init__(self, pid, base):
        self.f = open("/proc/%d/mem" % pid, "rb", 0)
        self.base = base

    def raw(self, a, n):
        self.f.seek(self.base + a)
        return self.f.read(n)

    def u32(self, a):
        return struct.unpack("<I", self.raw(a, 4))[0]

    def f32(self, a):
        return struct.unpack("<f", self.raw(a, 4))[0]

    def cstr(self, a, n=48):
        return self.raw(a, n).split(b"\0")[0].decode("latin1")


def sample(m):
    out = {"t": time.monotonic()}
    try:
        ls = m.u32(0x815248C8)
        out["screen"] = ls
        if ls:
            out.update(kind=m.u32(ls + 4), alpha=m.u32(ls + 0x80), shown=m.f32(ls + 0x88), ceiling=m.f32(ls + 0x8C),
                       rate=m.f32(ls + 0x90), rate_reset=m.f32(ls + 0x94))
        mode = m.u32(0x81965B58)
        out["mode"] = mode
        if mode:
            out.update(mode_vtable=m.u32(mode), mode_name=m.cstr(mode + 0xC4), options=m.u32(mode + 0x1A0), f188=m.u32(mode + 0x188),
                       laps=m.u32(mode + 0x18C), cls=m.u32(mode + 0x190), track=m.cstr(mode + 0xEA))
        out["mode_word"] = m.u32(0x8153FD24)
        out["fc40_e4"] = m.u32(0x8153FC40 + 0xE4)
        out["control_scheme"] = m.u32(0x818A5098)
        out["page"] = m.u32(0x816C782C); out["pages"] = m.u32(0x816C7830)
        out["page_list"] = [m.u32(0x816C7804 + 4 * i) for i in range(6)]
        out["photo"] = m.raw(0x816C7834, 1)[0]; out["photo2"] = m.raw(0x816C7838, 1)[0]; out["slide"] = m.f32(0x816C783C)
        out["meter_counter"] = m.u32(0x8151CD38)
    except Exception as e:  # a read can fail while the guest remaps
        out["error"] = repr(e)
    return out


def walk(args, vp, s, pid, frames):
    d = s.drive
    s.window(args.boot_timeout)
    time.sleep(8.0)
    s.key("Return")
    end = time.monotonic() + args.boot_timeout
    while True:
        time.sleep(4.0)
        if vp.on_game_mode(s.shot("walk-title.png")):
            break
        if time.monotonic() > end:
            sys.exit("STALLED on the way to the Game Mode grid; pid %d" % pid)
        s.key("x")
    print("stage: game mode grid", flush=True)
    m = Mem(pid, vp.HOST_WINDOW[0])
    log = [dict(sample(m), stage="grid")]
    time.sleep(2.0)
    tile = vp.GAME_MODES[args.game_mode]
    s.tap(tile); time.sleep(3.0)
    for _ in range(3):
        if not vp.tile_highlighted(s.shot("walk-tile.png"), tile):
            break
        s.tap(tile); time.sleep(3.0)
    time.sleep(3.0)
    s.shot("map.png"); log.append(dict(sample(m), stage="map"))
    print("stage: map", flush=True)
    if args.stop != "map":
        s.tap(tuple(args.node)); time.sleep(3.0)
        if args.select_team:
            s.tap(vp.HD_SELECT_TEAM); time.sleep(3.0)
            s.shot("select-team.png")
            s.tap(vp.CONFIRM); time.sleep(3.0)
            s.shot("map2.png")
            s.tap(vp.RECENTRED_NODE); time.sleep(3.0)
        s.shot("card.png"); log.append(dict(sample(m), stage="card"))
        print("stage: card", log[-1].get("mode_name"), flush=True)
    if args.stop == "modecard":
        s.tap(vp.CONFIRM); time.sleep(3.0)
        s.shot("modecard.png"); log.append(dict(sample(m), stage="modecard"))
        print("stage: mode card", flush=True)
    if args.stop == "load":
        t0 = time.monotonic()
        log.append(dict(sample(m), stage="load", note="before the Play tap"))
        s.tap(vp.CONFIRM)
        log.append(dict(sample(m), stage="load", note="Play tap released"))
        tb = time.monotonic()
        while time.monotonic() - tb < 2.5:
            log.append(dict(sample(m), stage="load")); time.sleep(0.004)
        i = 0
        dark = 0
        seen_screen = False
        while time.monotonic() - t0 < args.load_timeout:
            for _ in range(6):
                smp = dict(sample(m), stage="load")
                log.append(smp)
                seen_screen = seen_screen or bool(smp.get("screen"))
                time.sleep(0.016)
            name = "load-%03d.png" % i
            f = s.shot(name)
            log.append(dict(sample(m), stage="load", frame=name))
            if i % 10 == 0: print("load frame", i, "%.2f" % (time.monotonic() - t0), log[-1].get("kind"), log[-1].get("shown"), log[-1].get("ceiling"),
                  log[-1].get("rate"), flush=True)
            i += 1
            if seen_screen and not log[-1].get("screen"):
                dark += 1
                if dark >= 4:
                    break
        print("stage: load over", flush=True)
    (frames / "samples.json").write_text(json.dumps({"pid": pid, "samples": log}, indent=1))
    print("wrote", frames / "samples.json", flush=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--display", type=int, default=96)
    ap.add_argument("--game-mode", default="single")
    ap.add_argument("--node", type=int, nargs=2, default=None)
    ap.add_argument("--select-team", action="store_true")
    ap.add_argument("--stop", choices=["map", "card", "modecard", "load"], default="load")
    ap.add_argument("--profile", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--boot-timeout", type=float, default=100.0)
    ap.add_argument("--load-timeout", type=float, default=150.0)
    args = ap.parse_args()
    if args.stop != "map" and args.node is None:
        sys.exit("--node X Y is needed past the map (read it off a `--stop map` frame)")
    private_env(args.profile.resolve())
    vp = load_pair()
    frames = args.out.resolve()
    s = vp.Session(args.display, frames)
    d = s.drive
    d.bring_up_display(args.display, "wayland-oag%d" % args.display)
    if d.vita3k_alive() is not None:
        sys.exit("a Vita3K of this cache is already up: stop it first")
    pid = d.launch_vita3k(vp.TITLE, args.display)
    print("vita3k pid", pid, flush=True)
    try:
        walk(args, vp, s, pid, frames)
    finally:
        # Vita3K ignores SIGTERM when wedged, and a stalled walk must not leave it running
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        print("killed vita3k", pid, flush=True)


if __name__ == "__main__":
    main()
