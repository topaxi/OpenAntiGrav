"""Shared virtual-display lifecycle for the emulator drive scripts.

`scripts/pcsx2-drive.py` and `scripts/rpcs3-drive.py` each start their own
`Xvfb` and, until this module existed, never tore it down: `display`, `boot`,
`race` and the rest are deliberately separate invocations against one
long-lived display, so nothing could tear it down at the *end* of a call
without breaking the next one. The result was measured directly - three
orphaned `Xvfb` processes, all two or more days old, none with a client
attached.

The fix is ownership tracking, not a naive `atexit`. `bring_up` records, in a
small marker file, the display number and the pid of the `Xvfb` process *this
call* started - and only when it actually started one, never when it merely
found one already listening. `tear_down` reads that marker back, in a later
and possibly unrelated process, and acts only when it names the display being
asked about and the pid it names is still that same `Xvfb`. Anything else -
no marker, a marker for a different display, a pid that has since become a
different process - means "not ours to touch", and `tear_down` leaves it
alone and still returns cleanly. That is what makes it safe to call twice, or
call when nothing is up, or call against a display someone else started by
hand: never a `pkill -x Xvfb`, which would reach every virtual display on the
machine, not just this tooling's own.
"""

import json
import os
import subprocess
import sys
import time


def display_running(number):
    """Whether something is already listening on TCP display `number`."""
    probe = subprocess.run(
        [sys.executable, "-c",
         "import socket,sys;s=socket.socket();s.settimeout(2);"
         "sys.exit(0 if s.connect_ex(('127.0.0.1', %d)) == 0 else 1)"
         % (6000 + number)],
        capture_output=True)
    return probe.returncode == 0


def _is_xvfb(pid):
    """Whether `pid` exists and is actually an `Xvfb` process.

    A marker file outlives the process it names - a reboot, or just a long
    enough gap, can let the pid be reused by something else entirely. This is
    what stops `tear_down` from killing that unrelated process.
    """
    try:
        with open("/proc/%d/comm" % pid) as handle:
            return handle.read().strip() == "Xvfb"
    except (FileNotFoundError, ProcessLookupError, PermissionError):
        return False


def _read_marker(marker):
    try:
        with open(marker) as handle:
            return json.load(handle)
    except (FileNotFoundError, ValueError):
        return None


def _remove_marker(marker):
    try:
        os.unlink(marker)
    except FileNotFoundError:
        pass


def bring_up(number, geometry, marker, timeout=10.0):
    """Start `Xvfb :{number}` if nothing is listening there yet.

    Returns True if this call started it, False if it was already up. Only
    the True case writes `marker` - an attach to someone else's display must
    never look like ownership later.
    """
    if display_running(number):
        return False
    proc = subprocess.Popen(
        ["Xvfb", ":%d" % number, "-screen", "0", geometry,
         "-listen", "tcp", "-nolisten", "unix"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        start_new_session=True)
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        time.sleep(0.5)
        if display_running(number):
            os.makedirs(os.path.dirname(marker), exist_ok=True)
            with open(marker, "w") as handle:
                json.dump({"display": number, "pid": proc.pid}, handle)
            return True
    proc.kill()
    raise RuntimeError("Xvfb :%d did not come up" % number)


def tear_down(number, marker):
    """Stop the display this tooling owns for `:{number}`, and only that.

    Idempotent and quiet in every case that is not "we started this and it is
    still ours": no marker, a marker naming a different display, or a pid
    that is no longer `Xvfb` all return False without touching anything.
    """
    owned = _read_marker(marker)
    if owned is None or owned.get("display") != number:
        return False
    pid = owned.get("pid")
    if not isinstance(pid, int) or not _is_xvfb(pid):
        _remove_marker(marker)
        return False
    subprocess.run(["kill", str(pid)], capture_output=True)
    for _ in range(20):
        time.sleep(0.25)
        if not _is_xvfb(pid):
            break
    else:
        subprocess.run(["kill", "-9", str(pid)], capture_output=True)
    _remove_marker(marker)
    return True
