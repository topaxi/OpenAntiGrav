#!/usr/bin/env python3
"""Serves a built web folder on 127.0.0.1 with the headers Cloudflare sends.

    python3 scripts/serve-web.py [--dir target/web/dist] [--port 8000]

The page's threads need `crossOriginIsolated`, which only COOP/COEP response
headers give, so a plain `python3 -m http.server` no longer runs the game. This
sends the two from `web/_headers` on every response; `npx wrangler pages dev
target/web/dist` is the closer copy of the real host. See docs/tools/web.md.
"""

from __future__ import annotations

import argparse
import functools
import http.server
from pathlib import Path

ISOLATION = {
    "Cross-Origin-Opener-Policy": "same-origin",
    "Cross-Origin-Embedder-Policy": "require-corp",
}


class Handler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self) -> None:
        for name, value in ISOLATION.items():
            self.send_header(name, value)
        super().end_headers()

    def log_message(self, *args) -> None:  # noqa: ANN002 - quiet
        pass


def server(directory: Path, port: int) -> http.server.ThreadingHTTPServer:
    """A server for `directory` on 127.0.0.1:`port`, not yet serving."""
    handler = functools.partial(Handler, directory=str(directory))
    return http.server.ThreadingHTTPServer(("127.0.0.1", port), handler)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--dir", type=Path, default=Path("target/web/dist"))
    parser.add_argument("--port", type=int, default=8000)
    args = parser.parse_args()
    print(f"serving {args.dir} on http://127.0.0.1:{args.port}/ (cross-origin isolated)")
    server(args.dir, args.port).serve_forever()


if __name__ == "__main__":
    main()
