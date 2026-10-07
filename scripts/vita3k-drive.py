#!/usr/bin/env python3
"""Boot WipEout 2048 in Vita3K, walk it into a race, and capture a frame.

The Vita counterpart of `scripts/rpcs3-drive.py`, shaped much smaller than
that one because Vita3K gives and withholds different things than RPCS3
does:

- **There is no per-tick state channel at all.** RPCS3's HD build writes
  `Switching Screen "A" to "B"` to `TTY.log` on every front-end transition,
  which is what that script's menu walk waits on. Vita3K's log has nothing
  equivalent (checked: `~/.cache/Vita3K/vita3k.log`, thousands of
  `Unhandled SIGSEGV` lines from the guest fault handler and nothing about
  screens). So the menu walk below is **fixed-timing, not state-driven** -
  the same `sleep 3` cadence `~/.cache/oag/2048-hud/{hold,race}.sh` already
  used by hand. A slower machine may need longer waits than the constants
  here assume; nothing here polls for "did the tap land".
- **Input is XTEST, not a kernel device.** Vita3K reads real keyboard/mouse
  events via SDL on its own X11 window, so `xdotool key`/`mousedown` after
  `windowfocus --sync` is enough - no `evdev`, no pad permissions, unlike
  RPCS3's `scripts/rpcs3_pad.py`.
- **Every menu is touch-first.** The Game Mode grid, the campaign nodes, the
  Play/confirm buttons - almost all navigation is a tap (`mousedown 1; sleep
  0.3; mouseup 1` at a screen coordinate), not a button. A bare click is too
  short to register - measured directly, twice, on the Game Mode grid.

**Xvfb cannot present a Vulkan swapchain from a real GPU** - it has no DRI3,
so Vita3K's Vulkan renderer fails `Failed to select proper Vulkan queues`
before ever reaching a window. The fix this script automates is a headless
Wayland compositor on the real GPU (`weston --backend=headless
--renderer=gl`) with a rooted Xwayland on top of it; Vita3K's own X11 window
on that Xwayland presents at full speed. See
`docs/reverse-engineering/vita3k-capture.md` for the full trap and the
manual recipe this script replaces.

    python3 scripts/vita3k-drive.py display
    python3 scripts/vita3k-drive.py race --screenshot /tmp/2048-race.png
    python3 scripts/vita3k-drive.py stop

**Always run `stop` at the end of a session.** `boot`/`race`/`shot` each
leave Vita3K and the display pipeline running on a normal exit, the same way
`rpcs3-drive.py`'s `boot`/`race` leave RPCS3 running - deliberately, so a
`shot` after a `race` reads the frame the race left on screen rather than a
fresh boot. Nothing here tears anything down on its own; a run that is
killed or loses its terminal leaves Vita3K and the weston/Xwayland pair
running with nothing attached, the same orphan `rpcs3-drive.py`'s own
`xvfb_display` module exists to catch for its own display - `stop` is the
only thing that catches it here. `stop` only ever kills the weston/Xwayland
pair *this tooling* started (tracked in `~/.cache/oag-vita3k-drive/
pipeline.json`, the same ownership-marker idiom `scripts/xvfb_display.py`
uses for `Xvfb`) - never a bare `pkill`, which would reach a compositor or
Xwayland someone else is using.

Subcommands:

    display   start (or check) the headless weston + rooted Xwayland pair.
    stop      stop vita3k (if up) and the display pair this tooling started.
    boot      launch vita3k and wait for its game window; skip the intro
              movie and the PSN prompt so the front end is reachable.
    race      boot, walk into a race on a fresh save (Single Player
              Campaign's first event, Empire Climb - the one path this
              recipe measured as deterministic from a new save), optionally
              hold accelerate for a while, then capture a frame.
    shot      screenshot the current game window to a named file.
    tap       touch the game window at (x, y) - the manual step for
              whatever the save's own state needs that `race` does not know.
    key       press one or more keys on the game window (buttons; see
              `docs/reverse-engineering/vita3k-capture.md`'s own binding
              table for what each one is).
    hold      hold a key down for N seconds (accelerate is `e`/R1).

**`race`'s menu walk targets Empire Climb from a genuinely fresh save and is
not proven reliable unattended even then** - see `FIRST_EVENT_TAPS`'s own
doc comment for what 2026-09-20's own end-to-end run measured: `boot`,
`display`, `tap`, `key`, `hold` and `shot` are each individually correct
(driven by hand into a live race and back), but the Game Mode grid's own
highlight-then-confirm tap did not reproduce reliably back-to-back inside
`race` itself. Every event past the first needs the previous one actually
won (5th or better), which unattended Pilot Assist driving does not
reliably reach either - `docs/reverse-engineering/vita3k-capture.md`
records this from the manual pass. `tap`/`key`/`hold` after `boot` is the
reliable path today for whatever a session's own save and campaign map
actually show - see those subcommands below.

Needs `weston`, `Xwayland`, `xdotool`, `vita3k` and ImageMagick's `import`
on `PATH`.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

#: `OAG_VITA3K_CACHE` moves every file this tool writes (config, pid file,
#: frames, pipeline marker) to a directory of its own, so two members' Vita3K
#: instances never share a pid file; `OAG_VITA3K_CONFIG` names the config
#: (pref-path, gdbstub, audio) instead of the generated copy.
CACHE = Path(os.environ.get("OAG_VITA3K_CACHE") or Path.home() / ".cache" / "oag" / "2048-hud")
FRAMES = CACHE / "frames"
CONFIG = Path(os.environ.get("OAG_VITA3K_CONFIG") or CACHE / "config.yml")
VITA3K_PID_FILE = CACHE / "vita3k.pid"
PIPELINE_MARKER = (CACHE / "pipeline.json") if os.environ.get("OAG_VITA3K_CACHE") else (
    Path.home() / ".cache" / "oag-vita3k-drive" / "pipeline.json")

#: Matches both EU (`PCSF00007`) and USA (`PCSA00015`) window titles - the
#: same pattern `~/.cache/oag/2048-hud/shot.sh` already used by hand.
WINDOW_TITLE_RE = "PCS[AF]000"

#: The five lines `vita3k-capture.md` records the private config differing
#: from the user's own `~/.config/Vita3K/config.yml` in. `log-level: 0` is
#: TRACE and floods; the rest would land an overlay or a dialog in a frame.
CONFIG_OVERRIDES = {
    "log-level": "2",
    "discord-rich-presence": "false",
    "show-compile-shaders": "false",
    "validation-layer": "false",
    "check-for-updates-mode": "0",
}

DEFAULT_TITLE = "PCSF00007"  # EU - this project's default region.


def xdg_runtime_dir():
    return os.environ.get("XDG_RUNTIME_DIR", "/run/user/%d" % os.getuid())


def log(message):
    print(message, file=sys.stderr)


# Process ownership, modelled on `scripts/xvfb_display.py`'s `Xvfb` tracking:
# a marker is written only when *this* call started a process, and a later
# `stop` acts only when the marker still names a live process of the right
# kind - never a bare `pkill`.


def _comm(pid):
    try:
        with open("/proc/%d/comm" % pid) as handle:
            return handle.read().strip()
    except OSError:
        return None


def _is_process(pid, name_substring):
    comm = _comm(pid)
    return comm is not None and name_substring.lower() in comm.lower()


def display_up(display_num):
    """Whether an X server already answers on `:{display_num}`."""
    probe = subprocess.run(
        ["xdpyinfo", "-display", ":%d" % display_num],
        capture_output=True,
    )
    return probe.returncode == 0


def bring_up_display(display_num, socket, timeout=15.0):
    """Start the headless weston + rooted Xwayland pair if nothing is up.

    Returns True if this call started it, False if `:{display_num}` was
    already answering (someone else's, or a previous call's still running).
    Only the True case writes the ownership marker.
    """
    if display_up(display_num):
        log("display :%d already up" % display_num)
        return False

    CACHE.mkdir(parents=True, exist_ok=True)
    weston_env = os.environ.copy()
    weston_env.pop("DISPLAY", None)
    weston_env["XDG_RUNTIME_DIR"] = xdg_runtime_dir()
    weston = subprocess.Popen(
        [
            "weston", "--backend=headless", "--renderer=gl",
            "--width=1280", "--height=800",
            "--socket=%s" % socket, "--idle-time=0",
        ],
        stdout=open(CACHE / "weston.log", "wb"),
        stderr=subprocess.STDOUT,
        env=weston_env,
        start_new_session=True,
    )

    socket_path = Path(xdg_runtime_dir()) / socket
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline and not socket_path.exists():
        time.sleep(0.2)
    if not socket_path.exists():
        weston.kill()
        raise RuntimeError("weston did not create %s in %.0fs" % (socket_path, timeout))

    xwayland_env = os.environ.copy()
    xwayland_env.pop("DISPLAY", None)
    xwayland_env["WAYLAND_DISPLAY"] = socket
    xwayland_env["XDG_RUNTIME_DIR"] = xdg_runtime_dir()
    xwayland = subprocess.Popen(
        ["Xwayland", ":%d" % display_num, "-noreset", "-geometry", "1280x800"],
        stdout=open(CACHE / "xwayland.log", "wb"),
        stderr=subprocess.STDOUT,
        env=xwayland_env,
        start_new_session=True,
    )

    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        time.sleep(0.3)
        if display_up(display_num):
            break
    else:
        weston.kill()
        xwayland.kill()
        raise RuntimeError(":%d did not come up in %.0fs" % (display_num, timeout))

    PIPELINE_MARKER.parent.mkdir(parents=True, exist_ok=True)
    PIPELINE_MARKER.write_text(json.dumps({
        "display": display_num,
        "socket": socket,
        "weston_pid": weston.pid,
        "xwayland_pid": xwayland.pid,
    }))
    log("display :%d up (weston %d, Xwayland %d)" % (display_num, weston.pid, xwayland.pid))
    return True


def tear_down_display(display_num):
    """Stop the weston/Xwayland pair this tooling owns for `:{display_num}`.

    Idempotent and quiet whenever there is nothing of ours to stop: no
    marker, a marker for a different display, or pids that are no longer
    what they claim to be all return False without touching anything.
    """
    if not PIPELINE_MARKER.exists():
        return False
    owned = json.loads(PIPELINE_MARKER.read_text())
    if owned.get("display") != display_num:
        return False
    for key, name in (("xwayland_pid", "Xwayland"), ("weston_pid", "weston")):
        pid = owned.get(key)
        if isinstance(pid, int) and _is_process(pid, name):
            subprocess.run(["kill", str(pid)], capture_output=True)
    for _ in range(20):
        time.sleep(0.25)
        if not display_up(display_num):
            break
    else:
        for key, name in (("xwayland_pid", "Xwayland"), ("weston_pid", "weston")):
            pid = owned.get(key)
            if isinstance(pid, int) and _is_process(pid, name):
                subprocess.run(["kill", "-9", str(pid)], capture_output=True)
    PIPELINE_MARKER.unlink(missing_ok=True)
    log("display :%d down" % display_num)
    return True


def ensure_config():
    """The private Vita3K config `vita3k-capture.md` records: a copy of the
    user's own with five keys patched, read with `-c ... -w` so Vita3K never
    writes back to the user's real config.

    Generated once and cached at `CONFIG`; a config already there (from a
    previous run, or copied by hand) is left alone - the five keys are
    applied at generation time, not re-checked on every call, so a change
    made to the cached copy by hand sticks.
    """
    if CONFIG.exists():
        return CONFIG
    user_config = Path.home() / ".config" / "Vita3K" / "config.yml"
    if not user_config.exists():
        raise RuntimeError(
            "%s not found - run Vita3K once by hand first so it exists to copy"
            % user_config
        )
    lines = user_config.read_text().splitlines()
    seen = set()
    out = []
    for line in lines:
        match = re.match(r"^([A-Za-z0-9_-]+):\s*.*$", line)
        if match and match.group(1) in CONFIG_OVERRIDES:
            key = match.group(1)
            out.append("%s: %s" % (key, CONFIG_OVERRIDES[key]))
            seen.add(key)
        else:
            out.append(line)
    for key, value in CONFIG_OVERRIDES.items():
        if key not in seen:
            out.append("%s: %s" % (key, value))
    CACHE.mkdir(parents=True, exist_ok=True)
    CONFIG.write_text("\n".join(out) + "\n")
    log("wrote %s (%d override(s) applied)" % (CONFIG, len(CONFIG_OVERRIDES)))
    return CONFIG


def launch_vita3k(title_id, display_num):
    config = ensure_config()
    env = os.environ.copy()
    env.pop("WAYLAND_DISPLAY", None)
    env["DISPLAY"] = ":%d" % display_num
    env["SDL_VIDEODRIVER"] = "x11"
    env["SDL_VIDEO_DRIVER"] = "x11"
    CACHE.mkdir(parents=True, exist_ok=True)
    proc = subprocess.Popen(
        ["vita3k", "-c", str(config), "-w", "-r", title_id],
        stdout=open(CACHE / ("vita3k-%s.out" % title_id), "wb"),
        stderr=subprocess.STDOUT,
        env=env,
        cwd=str(CACHE),
        start_new_session=True,
    )
    VITA3K_PID_FILE.write_text(str(proc.pid))
    return proc.pid


def find_window(display_num, timeout=90.0):
    """The emulated screen's own window id, waiting for it to appear.

    Not the Qt shell window (1280x720, app list and log pane) - the child
    window titled `WipEout(R) 2048 (PCSF00007) | ...`, sized exactly
    960x544. `xdotool search --name` matches either by title, so the
    "PCS[AF]000" pattern is what actually picks the game window out from
    the shell.
    """
    deadline = time.monotonic() + timeout
    env = {**os.environ, "DISPLAY": ":%d" % display_num}
    while time.monotonic() < deadline:
        result = subprocess.run(
            ["xdotool", "search", "--name", WINDOW_TITLE_RE],
            env=env, capture_output=True, text=True,
        )
        wid = result.stdout.strip().splitlines()
        if wid:
            return wid[0]
        time.sleep(1.0)
    return None


def press(display_num, wid, *keys, delay=400):
    env = {**os.environ, "DISPLAY": ":%d" % display_num}
    subprocess.run(["xdotool", "windowfocus", "--sync", wid], env=env, capture_output=True)
    subprocess.run(["xdotool", "key", "--delay", str(delay), *keys], env=env, check=True)


def hold(display_num, wid, key, down):
    env = {**os.environ, "DISPLAY": ":%d" % display_num}
    subprocess.run(["xdotool", "windowfocus", "--sync", wid], env=env, capture_output=True)
    subprocess.run(["xdotool", "keydown" if down else "keyup", key], env=env, check=True)


def tap(display_num, x, y, hold_seconds=0.3):
    """A touch at `(x, y)` in the game window's own 960x544 coordinates.

    **A plain click does not register** - measured directly, twice, on the
    Game Mode grid. `mousedown` / `sleep` / `mouseup` at the same spot did.
    Touch coordinates double as root coordinates because the game window
    sits at the display's own origin.
    """
    env = {**os.environ, "DISPLAY": ":%d" % display_num}
    subprocess.run(["xdotool", "mousemove", str(x), str(y), "mousedown", "1"], env=env, check=True)
    time.sleep(hold_seconds)
    subprocess.run(["xdotool", "mouseup", "1"], env=env, check=True)


def screenshot(display_num, wid, out_path):
    env = {**os.environ, "DISPLAY": ":%d" % display_num}
    out_path = Path(out_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["import", "-window", wid, str(out_path)], env=env, check=True)
    return out_path


def _frame_digest(display_num, wid):
    import hashlib
    probe = Path("/tmp/.vita3k-drive-probe-%d.png" % display_num)
    screenshot(display_num, wid, probe)
    return hashlib.md5(probe.read_bytes()).hexdigest()


def tap_until_change(display_num, wid, x, y, settle=2.5, attempts=4):
    """Tap `(x, y)` and wait for the frame to actually change before
    returning, retrying the same tap if nothing did.

    There is no per-tick state channel to wait on instead (see the module
    docstring), so this is the closest substitute: screenshot, tap, wait
    `settle` seconds, screenshot again - a change means *something* moved
    and this returns `True`; no change taps again, up to `attempts` times,
    and returns `False` if none of them changed anything.

    **Not a reliable "did the tap land" signal, measured directly - a
    screen with its own idle animation defeats it.** The Game Mode grid's
    selected tile has a looping glow, so two screenshots a `settle` apart
    almost always differ regardless of whether the tap did anything, and
    this returns `True` on the first attempt whether or not the tile
    actually activated. What it *is* good for: a screen transition with no
    idle animation of its own (a menu card, a loading screen's absence) -
    where a real change is the only thing that can make the digest move.
    Treat a `True` on an animated screen as "at least `settle` seconds
    passed", not as confirmation.
    """
    before = _frame_digest(display_num, wid)
    for _ in range(attempts):
        tap(display_num, x, y)
        time.sleep(settle)
        after = _frame_digest(display_num, wid)
        if after != before:
            return True
        before = after
    return False


def vita3k_alive():
    if not VITA3K_PID_FILE.exists():
        return None
    try:
        pid = int(VITA3K_PID_FILE.read_text().strip())
    except ValueError:
        return None
    return pid if _is_process(pid, "vita3k") else None


def cmd_display(args):
    bring_up_display(args.display, args.socket)


def cmd_stop(args):
    pid = vita3k_alive()
    if pid is not None:
        log("stopping vita3k (pid %d)" % pid)
        subprocess.run(["kill", str(pid)], capture_output=True)
        for _ in range(20):
            time.sleep(0.25)
            if vita3k_alive() is None:
                break
        else:
            subprocess.run(["kill", "-9", str(pid)], capture_output=True)
        VITA3K_PID_FILE.unlink(missing_ok=True)
    else:
        log("vita3k not running (or not ours)")
    tear_down_display(args.display)


def boot(args):
    """Bring up the display if needed, launch vita3k, wait for its window,
    and skip the intro movie and the PSN prompt - steps 1-2 of
    `vita3k-capture.md`'s own walk-in table. Returns the window id.

    **Measured, not the recipe's own two-step count.** The recipe's manual
    pass had a human watching the screen between presses; timed blind, the
    sequence needs a fourth press - the intro movie/logo swallows the first
    two, `PRESS ANY BUTTON TO START` needs the third once it has actually
    finished fading in, and the PSN dialog needs the fourth. Four presses,
    4s apart, reached the GAME MODE grid with the dialog gone in every boot
    this pass tried; a slower machine may still need more.
    """
    bring_up_display(args.display, args.socket)
    if vita3k_alive() is None:
        log("launching vita3k -r %s" % args.title)
        launch_vita3k(args.title, args.display)
    else:
        log("vita3k already running (pid %d)" % vita3k_alive())
    wid = find_window(args.display, timeout=args.boot_timeout)
    if wid is None:
        raise RuntimeError(
            "no %r window after %.0fs - check %s"
            % (WINDOW_TITLE_RE, args.boot_timeout, CACHE / ("vita3k-%s.out" % args.title))
        )
    log("window %s up" % wid)
    time.sleep(8.0)  # the intro movie/logo, before anything on screen reacts
    press(args.display, wid, "Return")
    for _ in range(3):
        time.sleep(4.0)
        press(args.display, wid, "x")
    time.sleep(2.0)
    return wid


def cmd_boot(args):
    boot(args)


#: Single Player Campaign's first event, from a fresh save -
#: `vita3k-capture.md`'s own walk-in table, steps 3-8, run through
#: `tap_until_change` rather than a bare tap-and-sleep - `settle` and
#: `attempts` are that function's own parameters, not a promise every step
#: below is confirmed before the next one fires (see its own caveat about
#: an animated screen).
#:
#: **Save-state dependent AND not reliably unattended, both measured
#: directly, 2026-09-20.** This tooling's own `pref-path` is the user's real
#: Vita3K install, so a save the manual capture pass already progressed
#: (`vita3k-capture.md` itself records finishing the first three events)
#: makes these coordinates land on whatever is actually next on the
#: campaign map, not "Empire Climb". Driving this **by hand** - `tap`,
#: `shot`, look, `tap` again - reached a live race on this save's own next
#: event (Metro Park, Time Trial) and `hold e` moved the craft for real, so
#: every primitive below is individually correct. Driving `race` itself
#: **unattended did not reproduce that same path**: the Game Mode tile
#: needs a highlight tap and a second, later confirm tap, an interval this
#: pass could not pin to a fixed number of seconds or `tap_until_change`
#: retries - one run reached the grid and the welcome screen and then
#: looped back to the grid instead of reaching the map. Left as the
#: fresh-save coordinates the doc recipe measured, not as a validated
#: unattended path; `tap`/`key`/`hold` after `boot` are how to finish the
#: walk by hand for whatever a session's own save actually shows, the same
#: position the original manual recipe was already in.
FIRST_EVENT_TAPS = [
    (295, 162, 2.5, 6, "GAME MODE grid: Single Player Campaign"),
    (882, 480, 2.5, 4, "welcome text: checkmark"),
    (487, 270, 2.5, 4, "campaign map: first node, TOUCH TO START"),
    (882, 480, 2.5, 4, "event card (Empire Climb, No Weapons): Play"),
    (882, 480, 2.5, 4, "mode description card"),
]

#: The ship intro screen's own tap does not change the frame the way the
#: others do - it starts a load, whose own progress bar keeps the frame
#: changing regardless of whether the tap landed - so this step is a plain
#: tap and a long fixed wait rather than `tap_until_change`.
SHIP_INTRO_TAP = (882, 480)
SHIP_INTRO_LOAD_SECONDS = 25.0


def cmd_race(args):
    wid = boot(args)
    for x, y, settle, attempts, label in FIRST_EVENT_TAPS:
        log("tap (%d, %d) x%d - %s" % (x, y, attempts, label))
        if not tap_until_change(args.display, wid, x, y, settle=settle, attempts=attempts):
            log("  no change after %d attempt(s) - continuing anyway" % attempts)
    log("tap %r - ship intro: wait %.0fs for the race to load" % (SHIP_INTRO_TAP, SHIP_INTRO_LOAD_SECONDS))
    tap(args.display, *SHIP_INTRO_TAP)
    time.sleep(SHIP_INTRO_LOAD_SECONDS)

    if args.hold_seconds > 0:
        log("holding accelerate (R1) for %.0fs" % args.hold_seconds)
        hold(args.display, wid, "e", True)
        elapsed = 0.0
        shot_index = 0
        while elapsed < args.hold_seconds:
            time.sleep(min(3.0, args.hold_seconds - elapsed))
            elapsed += 3.0
            if args.shot_prefix:
                shot_index += 1
                out = FRAMES / ("%s-%d.png" % (args.shot_prefix, shot_index))
                screenshot(args.display, wid, out)
                log("wrote %s" % out)
        hold(args.display, wid, "e", False)

    if args.screenshot:
        out = screenshot(args.display, wid, args.screenshot)
        log("wrote %s" % out)


def cmd_shot(args):
    wid = find_window(args.display, timeout=5.0)
    if wid is None:
        raise RuntimeError("no game window on :%d - run `boot` or `race` first" % args.display)
    out = screenshot(args.display, wid, args.out)
    print(out)


def cmd_tap(args):
    if find_window(args.display, timeout=5.0) is None:
        raise RuntimeError("no game window on :%d - run `boot` or `race` first" % args.display)
    tap(args.display, args.x, args.y, hold_seconds=args.hold)


def cmd_key(args):
    wid = find_window(args.display, timeout=5.0)
    if wid is None:
        raise RuntimeError("no game window on :%d - run `boot` or `race` first" % args.display)
    press(args.display, wid, *args.keys)


def cmd_hold(args):
    wid = find_window(args.display, timeout=5.0)
    if wid is None:
        raise RuntimeError("no game window on :%d - run `boot` or `race` first" % args.display)
    hold(args.display, wid, args.key, True)
    time.sleep(args.seconds)
    hold(args.display, wid, args.key, False)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--display", type=int, default=94, help="X display number (default: 94)")
    parser.add_argument("--socket", default="wayland-oag94", help="weston's Wayland socket name")
    sub = parser.add_subparsers(dest="command", required=True)

    p_display = sub.add_parser("display", help="start (or check) the display pipeline")
    p_display.set_defaults(func=cmd_display)

    p_stop = sub.add_parser("stop", help="stop vita3k and the display pipeline this tooling owns")
    p_stop.set_defaults(func=cmd_stop)

    p_boot = sub.add_parser("boot", help="launch vita3k and reach the front end")
    p_boot.add_argument("--title", default=DEFAULT_TITLE)
    p_boot.add_argument("--boot-timeout", type=float, default=90.0)
    p_boot.set_defaults(func=cmd_boot)

    p_race = sub.add_parser("race", help="boot, walk into a race, optionally hold accelerate, capture a frame")
    p_race.add_argument("--title", default=DEFAULT_TITLE)
    p_race.add_argument("--boot-timeout", type=float, default=90.0)
    p_race.add_argument("--hold-seconds", type=float, default=0.0,
                         help="hold accelerate (R1) this long after the grid")
    p_race.add_argument("--shot-prefix", default=None,
                         help="write frames/<prefix>-N.png every 3s while holding")
    p_race.add_argument("--screenshot", default=None,
                         help="also write one final frame to this path")
    p_race.set_defaults(func=cmd_race)

    p_shot = sub.add_parser("shot", help="screenshot the current game window")
    p_shot.add_argument("out", help="output PNG path")
    p_shot.set_defaults(func=cmd_shot)

    p_tap = sub.add_parser("tap", help="touch the game window at (x, y)")
    p_tap.add_argument("x", type=int)
    p_tap.add_argument("y", type=int)
    p_tap.add_argument("--hold", type=float, default=0.3, help="hold duration in seconds")
    p_tap.set_defaults(func=cmd_tap)

    p_key = sub.add_parser("key", help="press one or more keys on the game window")
    p_key.add_argument("keys", nargs="+")
    p_key.set_defaults(func=cmd_key)

    p_hold = sub.add_parser("hold", help="hold a key down for N seconds")
    p_hold.add_argument("key")
    p_hold.add_argument("seconds", type=float)
    p_hold.set_defaults(func=cmd_hold)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
