#!/usr/bin/env python3
"""Boots the web build in headless Chromium on a disc image and screenshots it.

    uv run --with playwright python3 scripts/web-screenshot.py \\
        --dist target/web/dist --image data/images/pulse-psp-eu.chd \\
        --out data/web-shots --at 5,10,15 [--press Enter@12] [--port 8000]

Serves `--dist` on 127.0.0.1, opens it in Chromium with WebGPU enabled
(`--chromium` names the binary, `/usr/bin/chromium` by default; Playwright's
own build has no WebGPU on Linux), hands the image to the page's file input,
and writes `<out>/<seconds>s.png` at each `--at` time after the pick.
`--press KEY@SECONDS` presses a key into the canvas; `--down`/`--up KEY@SECONDS`
hold one and let it go (Playwright key names: `x`, `Enter`, `Space`, `ArrowLeft`).
`--fps SECONDS` counts animation frames for that long at the end. The page's console goes to
stdout. The browser is muted. See docs/tools/web.md.
"""

from __future__ import annotations

import argparse
import functools
import http.server
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


def serve(directory: Path, port: int) -> http.server.ThreadingHTTPServer:
    handler = functools.partial(
        http.server.SimpleHTTPRequestHandler, directory=str(directory)
    )
    handler.log_message = lambda *args: None  # type: ignore[method-assign]
    server = http.server.ThreadingHTTPServer(("127.0.0.1", port), handler)
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
    parser.add_argument("--fps", type=float, default=0.0)
    parser.add_argument("--port", type=int, default=8000)
    parser.add_argument("--size", default="960x544")
    parser.add_argument("--chromium", default="/usr/bin/chromium")
    args = parser.parse_args()

    shots = sorted(float(t) for t in args.at.split(","))
    def keyed(specs: list[str], kind: str) -> list[tuple[float, str, str]]:
        return [(float(at), kind, key) for key, at in (s.rsplit("@", 1) for s in specs)]

    keys = keyed(args.press, "press") + keyed(args.down, "down") + keyed(args.up, "up")
    width, height = (int(v) for v in args.size.split("x"))
    args.out.mkdir(parents=True, exist_ok=True)
    server = serve(args.dist, args.port)
    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(
                executable_path=args.chromium, headless=True, args=FLAGS
            )
            page = browser.new_page(viewport={"width": width, "height": height})
            page.on("console", lambda m: print(f"console.{m.type}: {m.text}", flush=True))
            page.on("pageerror", lambda e: print(f"pageerror: {e}", flush=True))
            page.goto(f"http://127.0.0.1:{args.port}/")
            print("webgpu:", page.evaluate("!!navigator.gpu"), flush=True)
            page.screenshot(path=str(args.out / "picker.png"))
            page.set_input_files("#file", str(args.image))
            start = time.monotonic()
            events = [(t, "shot", "") for t in shots] + keys
            for at, kind, key in sorted(events, key=lambda e: e[0]):
                wait = at - (time.monotonic() - start)
                if wait > 0:
                    page.wait_for_timeout(wait * 1000)
                if kind == "shot":
                    path = args.out / f"{at:g}s.png"
                    page.screenshot(path=str(path))
                    print(f"wrote {path}", flush=True)
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
            browser.close()
    finally:
        server.shutdown()


if __name__ == "__main__":
    main()
