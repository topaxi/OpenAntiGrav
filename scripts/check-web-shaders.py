#!/usr/bin/env python3
"""Hands every WGSL module the web build emits to a real browser's compiler.

    python3 scripts/check-web-shaders.py [--browser chromium|firefox|all]
        [--chromium PATH] [--firefox-binary PATH] [--software|--no-software]
        [--keep DIR] [--no-build]

`naga` (oag-shader-check, at build time) accepts shaders that Chrome's Tint
rejects: Tint enforces WGSL's uniformity rules where naga lets some through,
and Firefox runs its own pinned naga. A shader the browser rejects makes every
pipeline built from it invalid, and the page draws nothing. This script is the
only thing in `just` that asks a browser: it calls `device.createShaderModule`
on each module and fails on any `getCompilationInfo()` message of type `error`,
printing the module, the line and the function it is in. See docs/tools/web.md,
"Shaders".

The modules are the build scripts' artifacts (`$OUT_DIR/*.wgsl` of oag-mesh,
oag-fx, oag-post, oag-render, oag-present and oag-game, located through
`cargo check --message-format=json`, so no stale build directory is read) plus
the five built-in screen filters, which `oag-post` joins at run time
(`crates/post/examples/screen_filters_wgsl.rs`). `oag-view`'s shader is not in
the web build and is left out. The text a build script writes is what wgpu hands
the browser unchanged, on every target. A variant that enables
`wgpu_binding_array` is native only (no WebGPU device has binding arrays) and is
skipped by name.

What this does not catch: a module is created alone, so a pipeline layout that
disagrees with its bindings, an `override` given a bad value, or a limit the
device lacks is still found only by a race.

The browser runs on a CPU adapter (`--use-webgpu-adapter=swiftshader`, a
software Vulkan in Chromium itself), so no GPU is needed: the check is of the
WGSL compiler, which is the same on every GPU. Firefox is run with the two
WebGPU prefs and needs a Vulkan device; point it at lavapipe with
`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json` on a machine without a
GPU.

A missing browser is a loud skip, never a silent pass: the script names what it
looked for and exits 0, unless `OAG_REQUIRE_WEB_SHADERS=1` (CI) makes it exit 1.
Playwright comes from `uv run --with playwright`, which the script re-runs
itself under when the module is missing.
"""

from __future__ import annotations

import argparse
import http.server
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import threading
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# The `enable` directive of a shader variant a native device feature selects, which the
# web build never creates.
NATIVE_ONLY = "enable wgpu_binding_array;"

PACKAGES = ["oag-mesh", "oag-fx", "oag-post", "oag-render", "oag-present", "oag-game"]

CHROMIUM_CANDIDATES = [
    "chromium",
    "chromium-browser",
    "google-chrome-stable",
    "google-chrome",
    "google-chrome-beta",
    "chrome",
]

CHROMIUM_FLAGS = ["--enable-unsafe-webgpu"]
SOFTWARE_FLAGS = ["--use-webgpu-adapter=swiftshader"]

FIREFOX_PREFS = {
    "dom.webgpu.enabled": True,
    "gfx.webgpu.ignore-blocklist": True,
}

COMPILE_JS = """async (modules) => {
  const adapter = await navigator.gpu.requestAdapter();
  if (!adapter) return { adapter: null, results: [] };
  const info = adapter.info || {};
  const device = await adapter.requestDevice();
  const results = [];
  for (const [name, code] of modules) {
    const module = device.createShaderModule({ label: name, code });
    const compilation = await module.getCompilationInfo();
    results.push([name, compilation.messages.map(
      (m) => [m.type, m.lineNum, m.linePos, m.message])]);
  }
  return {
    adapter: [info.vendor || "", info.architecture || "", info.device || "",
              info.description || "", !!adapter.isFallbackAdapter],
    results,
  };
}"""


def skip(why: str) -> int:
    print(f"check-web-shaders: SKIPPED, nothing was checked: {why}")
    if os.environ.get("OAG_REQUIRE_WEB_SHADERS") == "1":
        print("check-web-shaders: OAG_REQUIRE_WEB_SHADERS=1, so a skip is a failure")
        return 1
    return 0


def collect_modules(build: bool) -> dict[str, str]:
    """`{crate/artifact: WGSL text}` for every module the web build emits."""
    command = ["cargo", "check", "--message-format=json"]
    for package in PACKAGES:
        command += ["-p", package]
    print("check-web-shaders: locating the build scripts' output:", " ".join(command))
    run = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, text=True)
    if run.returncode != 0 and build:
        sys.exit(f"check-web-shaders: `cargo check` failed ({run.returncode})")
    modules: dict[str, str] = {}
    wanted = set(PACKAGES)
    for line in run.stdout.splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message.get("reason") != "build-script-executed":
            continue
        # `package_id` is `path+file:///...#oag-mesh@0.1.0` or `... oag-mesh 0.1.0 (...)`.
        found = re.search(r"(oag-[a-z0-9]+)", message["package_id"].split("#")[-1])
        if not found or found.group(1) not in wanted:
            continue
        for artifact in sorted(Path(message["out_dir"]).glob("*.wgsl")):
            text = artifact.read_text()
            # A native-only variant (oag-mesh's `mesh_bindless`, binding arrays): built for
            # a device feature WebGPU does not have, so never handed to the browser.
            if NATIVE_ONLY in text:
                print(f"check-web-shaders: {found.group(1)}/{artifact.name} is native only, skipped")
                continue
            modules[f"{found.group(1)}/{artifact.name}"] = text
    with tempfile.TemporaryDirectory(prefix="oag-screen-filters-") as scratch:
        subprocess.run(
            [
                "cargo", "run", "-q", "-p", "oag-post", "--example", "screen_filters_wgsl",
                "--", str(ROOT / "assets/shaders/screen"), scratch,
            ],
            cwd=ROOT,
            check=True,
        )
        for artifact in sorted(Path(scratch).glob("*.wgsl")):
            modules[f"oag-post/screen-filter-{artifact.name}"] = artifact.read_text()
    return modules


def expected_crates(modules: dict[str, str]) -> None:
    """A package whose artifacts did not show up must fail, not shrink the check."""
    seen = {name.split("/")[0] for name in modules}
    missing = [package for package in PACKAGES if package not in seen]
    if missing:
        sys.exit(f"check-web-shaders: no WGSL found for {', '.join(missing)}")


class Quiet(http.server.BaseHTTPRequestHandler):
    def do_GET(self) -> None:
        body = b"<!doctype html><title>check-web-shaders</title>"
        self.send_response(200)
        self.send_header("Content-Type", "text/html")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_: object) -> None:
        pass


def find_chromium(given: str | None) -> str | None:
    for candidate in [given, os.environ.get("OAG_CHROMIUM")] + CHROMIUM_CANDIDATES:
        if candidate and (path := shutil.which(candidate)):
            return path
    return None


def explain(name: str, source: str, messages: list) -> list[str]:
    """Each error with its source line and function; its `info` notes follow."""
    lines = source.splitlines()
    out = []
    for kind, line, column, text in messages:
        shown = lines[line - 1].strip() if 0 < line <= len(lines) else ""
        owner = ""
        for index in range(min(line, len(lines)) - 1, -1, -1):
            found = re.match(r"(?:@\w+(?:\([^)]*\))?\s*)*fn\s+(\w+)", lines[index])
            if found:
                owner = f" in fn {found.group(1)}"
                break
        out.append(f"  {name}:{line}:{column}{owner}: {kind}: {text}")
        if shown:
            out.append(f"      | {shown}")
    return out


def run_browser(label: str, launch, modules: dict[str, str], url: str) -> tuple[int, bool]:
    """Returns `(errors, ran)`. `ran` is false when the browser gave no adapter."""
    try:
        browser = launch()
    except Exception as why:  # noqa: BLE001 - Playwright raises its own Error
        print(f"check-web-shaders: {label}: SKIPPED, it did not launch: {str(why).splitlines()[0]}")
        return 0, False
    try:
        page = browser.new_page()
        page.goto(url)
        reply = page.evaluate(COMPILE_JS, sorted(modules.items()))
    finally:
        browser.close()
    if reply["adapter"] is None:
        print(f"check-web-shaders: {label}: no WebGPU adapter, nothing was checked")
        return 0, False
    vendor, architecture, device, description, fallback = reply["adapter"]
    print(
        f"check-web-shaders: {label}: adapter {vendor} {architecture} {device} "
        f"{description} (fallback: {fallback})".replace("  ", " ")
    )
    errors = 0
    for name, messages in reply["results"]:
        failing = [m for m in messages if m[0] == "error"]
        notes = [m for m in messages if m[0] != "error"]
        if failing:
            errors += len(failing)
            print(f"check-web-shaders: {label}: FAIL {name}")
            print("\n".join(explain(name, modules[name], failing + notes)))
        else:
            for message in messages:
                print(f"check-web-shaders: {label}: {name}:{message[1]}: {message[0]}: {message[3]}")
    clean = len(reply["results"]) - sum(1 for _, m in reply["results"] if any(x[0] == "error" for x in m))
    print(f"check-web-shaders: {label}: {clean} of {len(reply['results'])} modules compile, {errors} errors")
    return errors, True


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--browser", choices=("chromium", "firefox", "all"), default="chromium")
    parser.add_argument("--chromium", help="a Chromium or Chrome binary (default: found on PATH)")
    parser.add_argument("--firefox-binary", help="a Firefox binary for Playwright (default: its own build)")
    parser.add_argument("--no-software", action="store_true", help="the browser's own adapter, not swiftshader")
    parser.add_argument("--keep", help="also write every module under this directory")
    parser.add_argument("--no-build", action="store_true", help="tolerate a failing `cargo check`")
    args = parser.parse_args()

    want_chromium = args.browser in ("chromium", "all")
    want_firefox = args.browser in ("firefox", "all")
    chromium = find_chromium(args.chromium) if want_chromium else None
    if want_chromium and chromium is None and not want_firefox:
        return skip(
            "no Chromium or Chrome found (looked for "
            f"{', '.join(CHROMIUM_CANDIDATES)}, $OAG_CHROMIUM and --chromium)"
        )

    try:
        import playwright  # noqa: F401
    except ImportError:
        uv = shutil.which("uv")
        if uv is None or os.environ.get("OAG_WEB_SHADERS_UV") == "1":
            return skip("the `playwright` Python module is missing and `uv` is not installed")
        os.environ["OAG_WEB_SHADERS_UV"] = "1"
        os.execv(uv, [uv, "run", "--with", "playwright", "python3", *sys.argv])

    from playwright.sync_api import sync_playwright

    modules = collect_modules(not args.no_build)
    expected_crates(modules)
    print(f"check-web-shaders: {len(modules)} modules from {len(PACKAGES)} crates")
    if args.keep:
        for name, source in modules.items():
            target = Path(args.keep) / name.replace("/", "--")
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(source)

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Quiet)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    url = f"http://127.0.0.1:{server.server_address[1]}/"

    errors = 0
    ran = 0
    with sync_playwright() as p:
        if want_chromium and chromium:
            flags = CHROMIUM_FLAGS + ([] if args.no_software else SOFTWARE_FLAGS)
            version = subprocess.run([chromium, "--version"], stdout=subprocess.PIPE, text=True).stdout.strip()
            print(f"check-web-shaders: chromium: {version} ({chromium}) {' '.join(flags)}")
            found, did = run_browser(
                "chromium",
                lambda: p.chromium.launch(executable_path=chromium, headless=True, args=flags),
                modules,
                url,
            )
            errors += found
            ran += did
        elif want_chromium:
            print("check-web-shaders: chromium: SKIPPED: no Chromium or Chrome found")
        if want_firefox:
            kwargs = {"executable_path": args.firefox_binary} if args.firefox_binary else {}
            found, did = run_browser(
                "firefox",
                lambda: p.firefox.launch(headless=True, firefox_user_prefs=FIREFOX_PREFS, **kwargs),
                modules,
                url,
            )
            errors += found
            ran += did
    server.shutdown()
    if ran == 0:
        return skip("no browser gave a WebGPU adapter")
    if errors:
        print(f"check-web-shaders: FAILED, {errors} errors")
        return 1
    print("check-web-shaders: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
