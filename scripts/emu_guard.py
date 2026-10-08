"""A stall watchdog and heartbeat for emulator capture scripts.

Why this exists: on 2026-10-08, 24 member waits ran into the 10 minute tool
ceiling (about 4 h) and about 9.6 h went to polling background capture logs.
Most of those runs had stalled - a boot that never produced a screen, a menu walk
parked on one screen, a GDB read that never answered - and nothing said so, so
the member polled the log for a success marker that was never going to arrive.

`Guard` is the one place that knows what a run is doing:

- `guard.stage("menu:Cell Selection")` names the stage. Entering a stage resets
  its clock; staying in one longer than its limit is a stall.
- `guard.beat()` is a heartbeat inside a long stage (a capture loop calls it per
  sample), and also resets the clock.
- Every change is written to `<log_dir>/status.json` (atomically), so a waiter
  can read `stage`, `age` and `state` instead of grepping stdout.
- On a stall the watchdog thread prints `STALLED at <stage> after <n>s`, writes
  `state: stalled`, kills the emulator (by pid, never by name) and exits the
  process with status 3. A thread, not a socket timeout, because a retry loop
  that catches `TimeoutError` never ends on its own.

Limits default per stage family (`LIMITS`); `OAG_EMU_STALL=<seconds>` overrides
the default for every stage, `OAG_EMU_TOTAL=<seconds>` adds a whole-run ceiling
(off by default: some captures are legitimately long, and they have heartbeats).
"""

import json
import os
import signal
import sys
import threading
import time

#: Seconds a stage may last without a beat, by the prefix before the first ":".
#: Measured on 2026-10-08 (emulator-tooling report): a cold RPCS3 boot reaches
#: `Top` in 37-40 s and `Main Menu` in 56-75 s; a menu walk spends 5 s on a
#: dropped press and never more than 25 s on one screen; a track load is 60-70 s.
LIMITS = {
    "boot": 100.0,
    "menu": 90.0,
    "loading": 150.0,
    "countdown": 90.0,
    "capture": 240.0,
    "state": 240.0,
}
DEFAULT_LIMIT = 240.0

EXIT_STALLED = 3

#: The running guard, if any. A script that uses a shared helper (the GDB
#: client, the pad) beats it without being handed the object.
CURRENT = None


def beat(**extra):
    """Heartbeat the current guard, if there is one. Safe to call from anywhere."""
    if CURRENT is not None:
        CURRENT.beat(**extra)


def stage(name, limit=None, **extra):
    """Enter a stage on the current guard, if there is one."""
    if CURRENT is not None:
        CURRENT.stage(name, limit, **extra)


class Guard:
    def __init__(self, status_path, label="", kill=None, stall=None, total=None,
                 poll=2.0, log=None):
        """`kill` is called with no arguments on a stall, before the exit; it must
        stop the emulator by pid. `status_path=None` keeps the guard in memory."""
        self.path = str(status_path) if status_path else None
        self.label = label
        self.kill = kill
        self.stall = float(os.environ.get("OAG_EMU_STALL", 0)) or stall
        self.total = float(os.environ.get("OAG_EMU_TOTAL", 0)) or total
        self.poll = poll
        self.log = log or (lambda text: print(text, file=sys.stderr, flush=True))
        self.started = time.time()
        self.name = "start"
        self.limit = self._limit_for("start")
        self.since = self.started
        self.beats = 0
        self.state = "running"
        self.extra = {}
        self._lock = threading.Lock()
        self._stop = threading.Event()
        self._thread = None
        self.write()

    def _limit_for(self, name, limit=None):
        if limit is not None:
            return float(limit)
        if self.stall:
            return self.stall
        return LIMITS.get(name.split(":", 1)[0], DEFAULT_LIMIT)

    def start(self):
        if self._thread is None:
            self._thread = threading.Thread(target=self._watch, daemon=True)
            self._thread.start()
        return self

    def stage(self, name, limit=None, **extra):
        """Enter a stage. Re-entering the current one is a beat, not a reset of
        its name; a different name restarts the clock."""
        with self._lock:
            if name != self.name:
                self.name = name
                self.limit = self._limit_for(name, limit)
                self.since = time.time()
            elif limit is not None:
                self.limit = float(limit)
                self.since = time.time()
            self.extra.update(extra)
            self.beats += 1
        self.write()

    #: Stage families where only a screen change counts as progress. A press, or
    #: a re-poll of the same screen, must not reset the clock there: a boot that
    #: never leaves `?` and a walk parked on one page both keep pressing.
    SCREEN_PROGRESS = ("boot", "menu")

    def beat(self, **extra):
        with self._lock:
            self.beats += 1
            self.extra.update(extra)
            if self.name.split(":", 1)[0] not in self.SCREEN_PROGRESS:
                self.since = time.time()
        now = time.time()
        if extra or now - getattr(self, "_written", 0.0) > 2.0:
            self._written = now
            self.write()

    def finish(self, ok=True):
        self.state = "done" if ok else "failed"
        self._stop.set()
        self.write()

    def snapshot(self):
        now = time.time()
        with self._lock:
            return dict(
                label=self.label, state=self.state, stage=self.name,
                stage_age=round(now - self.since, 1), stage_limit=self.limit,
                elapsed=round(now - self.started, 1), beats=self.beats,
                pid=os.getpid(), updated=now, **self.extra)

    def write(self):
        if not self.path:
            return
        tmp = self.path + ".tmp"
        try:
            os.makedirs(os.path.dirname(self.path) or ".", exist_ok=True)
            with open(tmp, "w") as handle:
                json.dump(self.snapshot(), handle)
            os.replace(tmp, self.path)
        except OSError:
            pass

    def _watch(self):
        while not self._stop.wait(self.poll):
            snap = self.snapshot()
            reason = None
            if snap["stage_age"] > snap["stage_limit"]:
                reason = ("STALLED at %s after %.0fs (limit %.0fs)"
                          % (snap["stage"], snap["stage_age"], snap["stage_limit"]))
            elif self.total and snap["elapsed"] > self.total:
                reason = ("OVER TIME at %s after %.0fs (total limit %.0fs)"
                          % (snap["stage"], snap["elapsed"], self.total))
            else:
                self.write()
                continue
            self.state = "stalled"
            self.extra["reason"] = reason
            self.write()
            self.log("%s: %s" % (self.label or "emu", reason))
            if self.kill:
                try:
                    self.kill()
                except Exception as exc:
                    self.log("kill failed: %s" % exc)
            os._exit(EXIT_STALLED)


def terminate_group(pid, grace=3.0):
    """TERM then KILL one process group (the emulator is started with
    `start_new_session=True`, so its pid is its group id). By pid only."""
    if not pid:
        return
    for sig in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(pid, sig)
        except (ProcessLookupError, PermissionError):
            return
        deadline = time.time() + grace
        while time.time() < deadline:
            try:
                os.kill(pid, 0)
            except ProcessLookupError:
                return
            time.sleep(0.2)
