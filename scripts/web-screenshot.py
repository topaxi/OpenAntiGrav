#!/usr/bin/env python3
"""Boots the web build in headless Chromium on a disc image and screenshots it.

    uv run --with playwright python3 scripts/web-screenshot.py \\
        --dist target/web/dist --image data/images/pulse-psp-eu.chd \\
        --out data/web-shots --at 5,10,15 [--press Enter@12] [--pick 30] [--eval JS@20] [--port 8000] [--software] [--resize 1280x720@12]

Serves `--dist` on 127.0.0.1, opens it in Chromium with WebGPU enabled
(`--chromium` names the binary, `/usr/bin/chromium` by default; Playwright's
own build has no WebGPU on Linux), hands the image to the page's file input,
and writes `<out>/<seconds>s.png` at each `--at` time after the pick.
`--press KEY@SECONDS` presses a key into the canvas; `--down`/`--up KEY@SECONDS`
hold one and let it go (Playwright key names: `x`, `Enter`, `Space`, `ArrowLeft`).
`--click X,Y@SECONDS` clicks the mouse at a viewport position.
`--fps SECONDS` counts animation frames for that long at the end. The page's console goes to
stdout. The browser is muted. `--url` drives a page some other server already
serves (`npx wrangler pages dev`, which sends the Cloudflare `_headers`) instead
of serving `--dist`. See docs/tools/web.md.
"""

from __future__ import annotations

import argparse
import http.server
import sys
import threading
import time
from pathlib import Path

from playwright.sync_api import sync_playwright

FLAGS = [
    "--enable-unsafe-webgpu",
    "--enable-features=Vulkan,WebGPU",
    "--use-angle=vulkan",
    "--ignore-gpu-blocklist",
    "--mute-audio",
]


# Firefox on Linux ships WebGPU off; these two prefs turn it on (Firefox 155,
# 2026-10-09: a hardware adapter in headless mode). Muted like Chromium.
FIREFOX_PREFS = {
    "dom.webgpu.enabled": True,
    "gfx.webgpu.ignore-blocklist": True,
    "media.volume_scale": "0.0",
}


# `--software`: a CPU adapter that presents (swiftshader through Vulkan).
# `--software bare`: only `--use-webgpu-adapter=swiftshader`, which hands out
# the same `(cpu)` adapter but never presents the canvas (it stays white) and
# is where the grade buffer's `mappedAtCreation` RangeError reproduced.
SOFTWARE_FLAGS = {
    "vulkan": [
        "--enable-unsafe-webgpu",
        "--enable-features=Vulkan",
        "--use-vulkan=swiftshader",
        "--enable-unsafe-swiftshader",
        "--use-webgpu-adapter=swiftshader",
        "--mute-audio",
    ],
    "bare": [
        "--enable-unsafe-webgpu",
        "--use-webgpu-adapter=swiftshader",
        "--mute-audio",
    ],
}


def serve(directory: Path, port: int) -> http.server.ThreadingHTTPServer:
    # The same server `just web --serve` runs, COOP/COEP included.
    sys.path.insert(0, str(Path(__file__).parent))
    import importlib

    server = importlib.import_module("serve-web").server(directory, port)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--dist", type=Path, default=Path("target/web/dist"))
    parser.add_argument("--image", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--at", default="5,10,15")
    parser.add_argument("--press", action="append", default=[])
    parser.add_argument("--down", action="append", default=[])
    parser.add_argument("--up", action="append", default=[])
    parser.add_argument("--click", action="append", default=[])
    parser.add_argument(
        "--resize",
        action="append",
        default=[],
        help="WxH@SECONDS: resize the viewport (a window resize, which rebuilds the output target)",
    )
    parser.add_argument(
        "--reload",
        action="append",
        default=[],
        help="SECONDS: reload the page then (leaving mid-load, say); repeatable",
    )
    parser.add_argument(
        "--pick",
        action="append",
        default=[],
        help="SECONDS: hand the image to the page's file input again then (after a quit reloads it)",
    )
    parser.add_argument(
        "--eval",
        action="append",
        default=[],
        help="JS@SECONDS: evaluate JS in the page then and print the result",
    )
    parser.add_argument("--fps", type=float, default=0.0)
    parser.add_argument(
        "--stalls",
        type=float,
        default=0.0,
        help="MS: at the end, list every gap between animation frames longer than this",
    )
    parser.add_argument("--log", default="", help="the page's ?log= level")
    parser.add_argument("--read", default="", help="the page's ?read= (memory: no slices)")
    parser.add_argument("--port", type=int, default=8000)
    parser.add_argument("--url", default="", help="a page already served elsewhere")
    parser.add_argument("--size", default="960x544")
    parser.add_argument("--dpr", type=float, default=1.0, help="the page's devicePixelRatio")
    parser.add_argument(
        "--software",
        nargs="?",
        const="vulkan",
        choices=sorted(SOFTWARE_FLAGS),
        help="force WebGPU's software (swiftshader) adapter, the CPU path of docs/tools/web.md",
    )
    parser.add_argument(
        "--flag",
        action="append",
        default=[],
        help="an extra Chromium flag, repeatable (added after the default or --software set)",
    )
    parser.add_argument("--chromium", default="/usr/bin/chromium")
    parser.add_argument(
        "--browser",
        choices=("chromium", "firefox"),
        default="chromium",
        help="firefox: Playwright's own build, WebGPU switched on by pref",
    )
    args = parser.parse_args()

    shots = sorted(float(t) for t in args.at.split(","))
    def keyed(specs: list[str], kind: str) -> list[tuple[float, str, str]]:
        return [(float(at), kind, key) for key, at in (s.rsplit("@", 1) for s in specs)]

    keys = (
        keyed(args.press, "press")
        + keyed(args.down, "down")
        + keyed(args.up, "up")
        + keyed(args.click, "click")
        + keyed(args.resize, "resize")
        + [(float(at), "reload", "") for at in args.reload]
        + [(float(at), "pick", "") for at in args.pick]
        + keyed(args.eval, "eval")
    )
    width, height = (int(v) for v in args.size.split("x"))
    args.out.mkdir(parents=True, exist_ok=True)
    server = None if args.url else serve(args.dist, args.port)
    url = args.url or f"http://127.0.0.1:{args.port}/"
    try:
        with sync_playwright() as p:
            if args.browser == "firefox":
                browser = p.firefox.launch(headless=True, firefox_user_prefs=FIREFOX_PREFS)
            else:
                browser = p.chromium.launch(
                    executable_path=args.chromium, headless=True,
                    args=(SOFTWARE_FLAGS[args.software] if args.software else FLAGS) + args.flag,
                )
            page = browser.new_page(
                viewport={"width": width, "height": height}, device_scale_factor=args.dpr
            )
            if args.stalls > 0:
                # Every animation frame's time, from before the page's own
                # script runs: a gap is the page's thread busy, a load stall.
                page.add_init_script(
                    "window.oagFrames = []; const tick = (t) => {"
                    " window.oagFrames.push(t); requestAnimationFrame(tick); };"
                    " requestAnimationFrame(tick);"
                )
            page.on("console", lambda m: print(f"console.{m.type}: {m.text}", flush=True))
            page.on("pageerror", lambda e: print(f"pageerror: {e}", flush=True))
            query = "&".join(q for q in (args.log and f"log={args.log}", args.read and f"read={args.read}") if q)
            page.goto(url + (f"?{query}" if query else ""))
            print("webgpu:", page.evaluate("!!navigator.gpu"), flush=True)
            print("crossOriginIsolated:", page.evaluate("crossOriginIsolated"), flush=True)
            page.screenshot(path=str(args.out / "picker.png"))
            page.set_input_files("#file", str(args.image))
            start = time.monotonic()
            picked = page.evaluate("performance.now()")
            events = [(t, "shot", "") for t in shots] + keys
            for at, kind, key in sorted(events, key=lambda e: e[0]):
                wait = at - (time.monotonic() - start)
                if wait > 0:
                    page.wait_for_timeout(wait * 1000)
                if kind == "shot":
                    path = args.out / f"{at:g}s.png"
                    page.screenshot(path=str(path))
                    print(f"wrote {path}", flush=True)
                elif kind == "reload":
                    page.reload(wait_until="commit", timeout=60000)
                    print(f"reloaded at {at:g}s", flush=True)
                elif kind == "pick":
                    page.wait_for_selector("#file", state="attached")
                    page.set_input_files("#file", str(args.image))
                    print(f"picked again at {at:g}s", flush=True)
                elif kind == "eval":
                    print(f"eval {key!r} at {at:g}s: {page.evaluate(key)}", flush=True)
                elif kind == "resize":
                    w, h = (int(v) for v in key.split("x"))
                    page.set_viewport_size({"width": w, "height": h})
                    print(f"resized {w}x{h} at {at:g}s", flush=True)
                elif kind == "click":
                    x, y = (float(v) for v in key.split(","))
                    page.mouse.move(x, y, steps=5)
                    page.wait_for_timeout(300)
                    page.mouse.click(x, y)
                    print(f"clicked {x:g},{y:g} at {at:g}s", flush=True)
                else:
                    getattr(page.keyboard, kind)(key)
                    print(f"{kind} {key} at {at:g}s", flush=True)
            if args.fps > 0:
                frames = page.evaluate(
                    """(ms) => new Promise((done) => {
                        let n = 0; const end = performance.now() + ms;
                        const tick = (t) => { n++; t < end ? requestAnimationFrame(tick) : done(n); };
                        requestAnimationFrame(tick);
                    })""",
                    args.fps * 1000,
                )
                print(f"fps: {frames / args.fps:.1f} over {args.fps:g}s", flush=True)
            if args.stalls > 0:
                frames = page.evaluate("window.oagFrames")
                gaps = [
                    (b - picked, b - a)
                    for a, b in zip(frames, frames[1:])
                    if b - a > args.stalls and b > picked
                ]
                for at, gap in gaps:
                    print(f"stall: {gap:.0f} ms ending {at / 1000:.2f}s after the pick", flush=True)
                longest = max((gap for _, gap in gaps), default=0.0)
                print(f"stalls: {len(gaps)} over {args.stalls:g} ms, longest {longest:.0f} ms", flush=True)
            browser.close()
    finally:
        if server:
            server.shutdown()


if __name__ == "__main__":
    main()
