#!/usr/bin/env python3
"""Runs every failure class the web page reports in a headless browser and checks the report.

    OAG_WEB_FAULT_TEST=1 just web --dev
    uv run --with playwright python3 scripts/check-web-faults.py \\
        --pulse data/images/pulse-psp-eu.chd --out data/web-faults [--browser chromium|firefox|both] [--only NAME ...]

Each case drives `scripts/web-screenshot.py --expect-fault KIND`: the run must end
with the one report of that kind on the page (`#oag-fault`, `web/fault.js`), exactly
one `oag:` console error and no uncaught page error (a cascade fails it). A
screenshot (`fault.png`) and the copy-details text (`fault-details.txt`) land in
`<out>/<browser>-<case>/` with the run's whole console (`run.log`). The module cases (panic, trap, GPU errors) need a build made
with `OAG_WEB_FAULT_TEST=1` (the hook `oagFaultTest`); a published build has none, and
those cases say so and are skipped. Not part of `just`: it wants browsers and a disc
image. See docs/tools/web.md, "Troubleshooting".
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

HERE = Path(__file__).parent
SHOT = HERE / "web-screenshot.py"

# Page scripts that stand in for a failure this machine cannot produce for real.
NO_MEMORY = (
    "const Real = WebAssembly.Memory; WebAssembly.Memory = function (d) {"
    " if (d && d.shared) throw new RangeError('WebAssembly.Memory(): could not allocate memory');"
    " return new Real(d); }; WebAssembly.Memory.prototype = Real.prototype;"
)
NO_COMPILE = (
    "WebAssembly.instantiateStreaming = async () => {"
    " throw new WebAssembly.CompileError('WebAssembly.instantiateStreaming(): invalid opcode'); };"
)


SCREEN_FILTER = (
    "localStorage.setItem('oag:/config/oag/settings.toml',"
    " '[render_profiles.\"Wipeout Pulse (PSP)\"]\\nscreen_filter = \"psp-3000\"\\n');"
)


NO_DEVICE = (
    "if (window.GPUAdapter) GPUAdapter.prototype.requestDevice ="
    " () => Promise.reject(new Error('requestDevice: the adapter is not able to provide this'));"
)


@dataclass
class Case:
    name: str
    kind: str
    args: list[str] = field(default_factory=list)
    needs_hook: bool = False
    browsers: tuple[str, ...] = ("chromium", "firefox")
    real: bool = True
    reports: int = 1


def hook(name: str, at: float) -> list[str]:
    # A timer inside the page, so a trap thrown by the hook is the page's to handle.
    return ["--eval", f"setTimeout(() => oagFaultTest('{name}'), 0)@{at:g}"]


def cases(pulse: Path, scratch: Path, boot: float) -> list[Case]:
    late = boot + 4
    out = [
        Case("panic", "panic", hook("panic", boot), True),
        Case("trap", "trap", hook("trap", boot), True),
        Case("stack-overflow", "trap", hook("stack", boot), True),
        Case("recursion", "trap", hook("recursion", boot), True),
        Case("memory", "out-of-memory", hook("memory", boot), True),
        Case("worker-panic", "panic", hook("worker-panic", boot), True),
        Case("worker-trap", "trap", hook("worker-trap", boot), True),
        # The page's thread taking a lock a worker keeps: a report, not a frozen page.
        Case("page-wait", "panic", hook("page-wait-hold", boot) + hook("page-wait", boot + 2), True),
        Case("gpu-validation", "gpu-validation", hook("gpu-validation", boot), True),
        Case("gpu-out-of-memory", "gpu-out-of-memory", hook("gpu-out-of-memory", boot), True, real=False),
        Case("gpu-internal", "gpu-internal", hook("gpu-internal", boot), True, real=False),
        Case("gpu-lost-synthetic", "gpu-lost", hook("gpu-lost", boot), True, real=False),
        Case("gpu-lost", "gpu-lost", ["--crash-gpu", f"{boot:g}"], False, ("chromium",)),
        # A notice on screen must not hide the failure that follows it.
        Case("validation-then-trap", "trap", hook("gpu-validation", boot) + hook("trap", boot + 2), True, reports=2),
        Case("device-request", "device-request", ["--init-script", NO_DEVICE], real=False),
        Case("no-adapter", "no-adapter", ["--plain"], False, ("chromium",)),
        Case("no-webgpu", "no-webgpu", ["--plain"], False, ("firefox",)),
        Case("no-isolation", "no-isolation", ["--no-isolation"]),
        Case("module-load-wasm", "module-load", ["--block", "**/*.wasm"]),
        Case("module-load-glue", "module-load", ["--block", "**/pkg/*/oag_web.js"]),
        Case("wasm-memory", "wasm-memory", ["--init-script", NO_MEMORY], real=False),
        Case("wasm-unsupported", "wasm-unsupported", ["--init-script", NO_COMPILE], real=False),
        # Saved settings the boot reads, which must still reach the menus with no report: a
        # screen filter once froze Firefox there (docs/tools/web.md, "The page's thread may not wait").
        Case("saved-screen-filter", "clean", ["--init-script", SCREEN_FILTER]),
    ]
    for name, kind, extra in (
        ("disc-garbage", "disc-unsupported", ["--image", str(scratch / "garbage.iso")]),
        ("disc-empty", "disc-unsupported", ["--image", str(scratch / "empty.iso")]),
        ("disc-truncated", "disc-unreadable", ["--truncate", "5000000"]),
    ):
        out.append(Case(name, kind, extra))
    out.append(Case("disc-lost-mid-run", "disc-unreadable",
                    ["--image", str(scratch / "changing.chd"), "--lose-image", f"{boot:g}",
                     "--eval", f"1@{late:g}"]))
    return out


def run(case: Case, browser: str, args, pulse: Path, boot: float) -> tuple[str, str]:
    """Returns (verdict, note)."""
    if case.needs_hook and not args.hook:
        return "SKIP", "needs a build made with OAG_WEB_FAULT_TEST=1 (--hook)"
    out = args.out / f"{browser}-{case.name}"
    command = [
        sys.executable, str(SHOT), "--dist", str(args.dist), "--image", str(pulse), "--out", str(out),
        "--at", f"{boot + 3:g}", "--browser", browser, "--port", str(args.port),
    ] + (["--expect-clean"] if case.kind == "clean" else
         ["--expect-fault", case.kind, "--expect-reports", str(case.reports)]) + case.args
    if "--image" in case.args:
        # The case's own image replaces the default one.
        first = command.index("--image")
        del command[first : first + 2]
    try:
        done = subprocess.run(command, capture_output=True, text=True, timeout=400)
    except subprocess.TimeoutExpired:
        return "FAIL", "timed out"
    out.mkdir(parents=True, exist_ok=True)
    (out / "run.log").write_text(done.stdout + done.stderr)
    tail = (done.stdout + done.stderr).strip().splitlines()
    note = next((line for line in reversed(tail) if line.startswith(("report:", "fault ", "clean:", "other console"))), "")
    if done.returncode == 0:
        return "PASS", note
    return "FAIL", (tail[-1] if tail else "no output")[:200]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--dist", type=Path, default=Path("target/web/dist"))
    parser.add_argument("--pulse", type=Path, required=True, help="a Pulse PSP .chd (data/ only)")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--browser", choices=("chromium", "firefox", "both"), default="chromium")
    parser.add_argument("--only", nargs="*", default=[], help="case names")
    parser.add_argument("--hook", action="store_true", default=True,
                        help="the build has oagFaultTest (the default; --no-hook skips those cases)")
    parser.add_argument("--no-hook", dest="hook", action="store_false")
    parser.add_argument("--port", type=int, default=8200)
    parser.add_argument("--boot", type=float, default=0, help="seconds from the pick to the menus (default: 14, Firefox 30)")
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    scratch = Path(tempfile.mkdtemp(prefix="oag-faults-", dir=args.out))
    (scratch / "garbage.iso").write_bytes(bytes(range(256)) * 4096)
    # A valid ISO 9660 volume with an empty root: reads fine, holds no game.
    sys.path.insert(0, str(HERE))
    (scratch / "empty.iso").write_bytes(empty_iso())
    # A real image the case deletes while the game runs: a copy, never the original.
    browsers = ("chromium", "firefox") if args.browser == "both" else (args.browser,)
    results = []
    for browser in browsers:
        boot = args.boot or (30.0 if browser == "firefox" else 14.0)
        for case in cases(args.pulse, scratch, boot):
            if args.only and case.name not in args.only:
                continue
            if browser not in case.browsers:
                continue
            if case.name == "disc-lost-mid-run":
                restore_image(args.pulse, scratch / "changing.chd")
            verdict, note = run(case, browser, args, args.pulse, boot)
            label = "real" if case.real else "synthetic"
            print(f"{verdict:4} {browser:8} {case.name:22} {case.kind:18} {label:9} {note}", flush=True)
            results.append(verdict)
    sys.exit(1 if "FAIL" in results else 0)


def restore_image(original: Path, copy: Path) -> None:
    """A path the case may delete while the game runs, to the same bytes as `original`.

    A hard link, not a copy: removing the link takes the path away from the page and leaves
    the original image alone.
    """
    copy.unlink(missing_ok=True)
    try:
        os.link(original, copy)
    except OSError:  # another filesystem
        copy.write_bytes(original.read_bytes())


def empty_iso() -> bytes:
    import struct

    def both(n: int, size: int) -> bytes:
        fmt = "I" if size == 4 else "H"
        return struct.pack("<" + fmt, n) + struct.pack(">" + fmt, n)

    sector = 2048
    root = bytes([34, 0]) + both(19, 4) + both(sector, 4) + bytes(7) + bytes([2, 0, 0]) + both(1, 2) + bytes([1, 0])
    pvd = bytearray(sector)
    pvd[0:7] = b"\x01CD001\x01"
    pvd[8:40] = b"EMPTY".ljust(32)
    pvd[40:72] = b"EMPTY".ljust(32)
    pvd[80:88] = both(24, 4)
    pvd[120:124] = both(1, 2)
    pvd[124:128] = both(1, 2)
    pvd[128:132] = both(sector, 2)
    pvd[156:190] = root
    end = bytearray(sector)
    end[0:7] = b"\xffCD001\x01"
    directory = bytearray(sector)
    directory[0:34] = root
    return bytes(16 * sector) + bytes(pvd) + bytes(end) + bytes(sector) + bytes(directory) + bytes(5 * sector)


if __name__ == "__main__":
    main()
