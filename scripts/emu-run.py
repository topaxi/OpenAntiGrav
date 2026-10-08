#!/usr/bin/env python3
"""Run an emulator capture in the foreground with a stall watchdog, or wait on one.

    python3 scripts/emu-run.py [--silence 150] [--total 570] [--status F] -- CMD ARGS...
    python3 scripts/emu-run.py --bg --status F -- CMD ARGS...     # detached, prints the pid
    python3 scripts/emu-run.py wait F [--max 540]                 # block until it ends

Why: a capture that stalls (a boot that never shows a screen, a menu walk parked
on one page, a GDB read nobody answers) used to look like "still running" until
the member's own 10 minute tool ceiling ended the wait. This wrapper decides
instead:

- the child runs in its own process group; if it prints nothing for `--silence`
  seconds (default 150) it is stalled - the wrapper says so, with the last line
  and the child's own stage if it writes a status file, kills the group *by pid*,
  and exits 3;
- `--total` seconds (default 570, under the tool ceiling; 0 = none for `--bg`)
  is a hard ceiling, exit 124;
- if the child's `--child-status` file (the `status.json` an `emu_guard.Guard`
  writes into the log dir) names an `emulator_pid`, that emulator is stopped too
  on a kill, since RPCS3 and PPSSPP run in sessions of their own;
- exit status is the child's, so `&&` chains still work.

`wait` is the correct way to wait for a `--bg` run: it prints each stage change,
returns 0 when the run finished, 1 on failure, 3 on a stall, and 4 when `--max`
(default 540, again under the ceiling) ran out with the run still healthy, so
the caller loops on it instead of grepping a log for a success marker.
Never end a turn waiting for a notification; call `wait` again.
"""

import argparse
import json
import os
import signal
import subprocess
import sys
import threading
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import emu_guard  # noqa: E402


def read_json(path):
    try:
        return json.loads(Path(path).read_text())
    except (OSError, ValueError):
        return {}


def write_status(path, **fields):
    if not path:
        return
    tmp = str(path) + ".tmp"
    Path(tmp).write_text(json.dumps(fields))
    os.replace(tmp, path)


def kill_group(pid):
    for sig in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(pid, sig)
        except (ProcessLookupError, PermissionError):
            return
        time.sleep(1.5 if sig == signal.SIGTERM else 0.2)


def run(args, command):
    started = time.time()
    proc = subprocess.Popen(
        command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        start_new_session=True, bufsize=0)
    state = {"last": time.time(), "line": ""}
    log = open(args.log, "ab") if args.log else None

    def pump():
        buf = b""
        while True:
            chunk = proc.stdout.read(4096)
            if not chunk:
                break
            state["last"] = time.time()
            if log:
                log.write(chunk)
                log.flush()
            sys.stdout.buffer.write(chunk)
            sys.stdout.buffer.flush()
            buf += chunk
            lines = buf.split(b"\n")
            buf = lines[-1]
            for line in lines[:-1]:
                if line.strip():
                    state["line"] = line.decode(errors="replace")[:200]

    reader = threading.Thread(target=pump, daemon=True)
    reader.start()
    verdict = None
    while proc.poll() is None:
        time.sleep(1.0)
        now = time.time()
        child = read_json(args.child_status) if args.child_status else {}
        # A guarded child reports its own stage ages; trust those over silence.
        write_status(args.status, state="running", pid=proc.pid,
                     wrapper=os.getpid(), elapsed=round(now - started, 1),
                     silence=round(now - state["last"], 1),
                     last_line=state["line"], child=child, command=command)
        if args.silence and now - state["last"] > args.silence and \
                child.get("state") != "running":
            verdict = ("STALLED", 3, "no output for %.0fs" % (now - state["last"]))
        elif args.silence and now - state["last"] > args.silence and \
                child.get("stage_age", 0) > args.silence:
            verdict = ("STALLED", 3, "stage %s idle %.0fs"
                       % (child.get("stage"), child.get("stage_age")))
        elif args.total and now - started > args.total:
            verdict = ("OVER TIME", 124, "total ceiling %.0fs" % args.total)
        if verdict:
            break
    rc = proc.returncode
    if verdict:
        name, rc, why = verdict
        child = read_json(args.child_status) if args.child_status else {}
        print("\nemu-run: %s - %s. last line: %r; child stage: %s"
              % (name, why, state["line"], child.get("stage", "?")),
              file=sys.stderr, flush=True)
        kill_group(proc.pid)
        emulator = child.get("emulator_pid")
        if emulator:
            emu_guard.terminate_group(int(emulator))
    reader.join(timeout=2)
    final = "stalled" if verdict else ("done" if rc == 0 else "failed")
    write_status(args.status, state=final, rc=rc, elapsed=round(time.time() - started, 1),
                 last_line=state["line"], command=command,
                 child=read_json(args.child_status) if args.child_status else {},
                 reason=verdict[2] if verdict else "")
    print("emu-run: %s rc=%s elapsed=%.0fs" % (final, rc, time.time() - started),
          file=sys.stderr, flush=True)
    return rc


def wait(args):
    deadline = time.time() + args.max
    seen = None
    while True:
        status = read_json(args.file)
        child = status.get("child") or {}
        stage = child.get("stage") or status.get("last_line", "")[:60]
        if stage != seen:
            print("[%4.0fs] %s | %s" % (status.get("elapsed", 0), status.get("state", "?"), stage),
                  flush=True)
            seen = stage
        state = status.get("state")
        if state == "done":
            return 0
        if state == "failed":
            print("failed rc=%s; last line: %s" % (status.get("rc"), status.get("last_line")))
            return 1
        if state == "stalled":
            print("STALLED: %s; last line: %s" % (status.get("reason"), status.get("last_line")))
            return 3
        if state == "running":
            pid = status.get("wrapper")
            try:
                os.kill(pid, 0)
            except (ProcessLookupError, TypeError):
                print("the run's wrapper died without a verdict (last line: %s)"
                      % status.get("last_line"))
                return 1
        if time.time() > deadline:
            print("still running after %.0fs (healthy); wait again" % args.max)
            return 4
        time.sleep(2.0)


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if argv and argv[0] == "wait":
        ap = argparse.ArgumentParser(prog="emu-run.py wait")
        ap.add_argument("cmd")
        ap.add_argument("file")
        ap.add_argument("--max", type=float, default=540.0)
        return wait(ap.parse_args(argv))
    if "--" not in argv:
        sys.exit(__doc__)
    cut = argv.index("--")
    ap = argparse.ArgumentParser()
    ap.add_argument("--silence", type=float, default=150.0)
    ap.add_argument("--total", type=float, default=570.0)
    ap.add_argument("--status", default="")
    ap.add_argument("--child-status", default="",
                    help="the child's emu_guard status.json (its log dir)")
    ap.add_argument("--log", default="", help="also append the child's output here")
    ap.add_argument("--bg", action="store_true", help="detach; needs --status")
    args = ap.parse_args(argv[:cut])
    command = argv[cut + 1:]
    if args.bg:
        if not args.status:
            sys.exit("--bg needs --status")
        args.total = args.total if "--total" in argv[:cut] else 0.0
        flags = [a for a in argv[:cut] if a != "--bg"]
        if "--total" not in flags:
            # The detached child re-parses its flags; without this it would fall
            # back to the 570 s foreground default and kill the long run it exists for.
            flags += ["--total", "0"]
        out = open(args.log or os.devnull, "ab")
        proc = subprocess.Popen(
            [sys.executable, os.path.abspath(__file__)] + flags + ["--"] + command
            + [], stdout=out, stderr=subprocess.STDOUT, start_new_session=True)
        write_status(args.status, state="running", wrapper=proc.pid, pid=proc.pid,
                     elapsed=0, last_line="", child={})
        print("emu-run: detached pid %d; wait with: python3 scripts/emu-run.py wait %s"
              % (proc.pid, args.status))
        return 0
    return run(args, command)


if __name__ == "__main__":
    sys.exit(main())
