"""Screenshot the emulator's window through the niri compositor.

Split out of `scripts/psp-autopilot.py` when `psp-trace.py` grew `--shot-every`:
the same three traps live here for both callers rather than twice.

- `gpu.buffer.screenshot` does not work on either PPSSPP path tried here (see
  ppsspp-debugger.md), which is why the window is read through the compositor
  at all.
- niri only writes screenshots to the clipboard, so the pipeline is
  `niri msg action screenshot-window` then `wl-paste`.
- **`wl-paste` can return the PREVIOUS clipboard image** (HANDOVER.md records a
  lap "verification" shot that was a menu from an earlier session), so the
  clipboard is sampled before the shot and a paste identical to it is retried,
  then flagged in the filename rather than saved as if it were evidence.

A failed screenshot warns and returns rather than raising: a missing frame must
not end the capture or the race that was producing it.
"""

import json
import subprocess
import sys
import time


def find_window(app_id="PPSSPPSDL"):
    """The niri window id of the first window running `app_id`, or `None`.

    Window ids are assigned per compositor session, so a hardcoded id goes
    stale on every relaunch; the app id is the stable name. `None` also covers
    a machine without niri at all, so callers can turn it into their own
    error message.
    """
    try:
        out = subprocess.run(
            ["niri", "msg", "--json", "windows"],
            check=True,
            capture_output=True,
            timeout=10,
        ).stdout
        for window in json.loads(out):
            if window.get("app_id") == app_id:
                return window.get("id")
    except Exception:  # noqa: BLE001 - "not found" and "no niri" answer the same
        return None
    return None


def _clipboard_png(timeout=10):
    """The clipboard's current PNG, or `None` when it holds none."""
    try:
        done = subprocess.run(
            ["wl-paste", "-t", "image/png"],
            check=True,
            capture_output=True,
            timeout=timeout,
        )
        return done.stdout or None
    except Exception:  # noqa: BLE001 - an empty clipboard is a normal state
        return None


def screenshot(window, directory, name):
    """Grab window `window` into `directory/name.png`.

    Returns the written path, or `None` when no image could be taken. A shot
    that could not be told apart from the clipboard's previous content is
    written with a `.stale.png` suffix: it is probably fine after the retry,
    but a comparison pipeline must not cite an image the trap above could have
    substituted.
    """
    if not directory:
        return None
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / ("%s.png" % name)
    try:
        before = _clipboard_png()
        # One action, then poll the clipboard until its content changes: the
        # compositor writes the clipboard asynchronously, measured lagging a
        # full shot behind at capture cadence, and triggering the action again
        # would only queue a second late write behind the first.
        subprocess.run(
            ["niri", "msg", "action", "screenshot-window", "--id", str(window),
             "--write-to-disk", "false"],
            check=True, capture_output=True, timeout=10,
        )
        data = None
        for _ in range(8):
            data = _clipboard_png()
            if data is not None and data != before:
                break
            time.sleep(0.25)
        if data is None:
            raise RuntimeError("wl-paste returned no image")
        if data == before:
            # Two consecutive frames can also genuinely be identical (a paused
            # scene), so this suffix means "could not be told apart from the
            # previous clipboard", not "certainly wrong".
            path = directory / ("%s.stale.png" % name)
        with path.open("wb") as out:
            out.write(data)
        print("shot %s" % path, file=sys.stderr, flush=True)
        return path
    except Exception as error:  # noqa: BLE001 - a missing screenshot must not end a race
        print("screenshot %s: %s" % (name, error), file=sys.stderr)
        return None
