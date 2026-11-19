#!/usr/bin/env python3
"""Moves the pinned ffmpeg build (crates/ffmpeg/src/pins.rs) to BtbN's newest month-end build.

Dependabot cannot see a URL in a Rust table, so this does the same job: it finds the newest
`autobuild-YYYY-MM-DD-HH-MM` release published on the last day of a month (those are kept; the
daily ones are pruned), takes the `lgpl` (not `gpl`, not `shared`) archives of the release branch
already pinned (n8.1.x stays on 8.1), reads each archive's SHA-256 and size from GitHub's own
asset digest, and rewrites the table. Run by .github/workflows/ffmpeg-pin.yml monthly, which
opens a PR; a person then runs `just test-data` (the movie and music checks) before merging.

  scripts/bump-ffmpeg-pin.py --check   exit 1 and say so when a newer build exists
  scripts/bump-ffmpeg-pin.py           rewrite pins.rs (and print what changed)
"""
import calendar
import json
import re
import sys
import urllib.request
from datetime import datetime
from pathlib import Path

PINS = Path(__file__).resolve().parent.parent / "crates/ffmpeg/src/pins.rs"
API = "https://api.github.com/repos/BtbN/FFmpeg-Builds/releases?per_page=100"
PLATFORMS = [("linux64", "tar.xz", "TarXz"), ("linuxarm64", "tar.xz", "TarXz"),
             ("win64", "zip", "Zip"), ("winarm64", "zip", "Zip")]


def fetch(url):
    with urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": "oag"})) as r:
        return json.load(r)


def main():
    text = PINS.read_text()
    current_tag = re.search(r"autobuild-[\d-]+", text).group(0)
    branch = re.search(r'const VERSION: &str = "n(\d+\.\d+)\.', text).group(1)
    month_end = []
    for rel in fetch(API):
        m = re.fullmatch(r"autobuild-(\d{4})-(\d{2})-(\d{2})-\d{2}-\d{2}", rel["tag_name"])
        if m:
            y, mo, d = map(int, m.groups())
            if d == calendar.monthrange(y, mo)[1]:
                month_end.append(rel)
    month_end.sort(key=lambda r: r["tag_name"])
    newest = month_end[-1]
    if newest["tag_name"] <= current_tag:
        print(f"up to date: {current_tag}")
        return 0
    if "--check" in sys.argv:
        print(f"a newer month-end build exists: {newest['tag_name']} (pinned {current_tag})")
        return 1
    tag = newest["tag_name"]
    entries, version = [], None
    for plat, ext, kind in PLATFORMS:
        pat = re.compile(rf"ffmpeg-n{re.escape(branch)}\.(\d+)-(\d+)-g([0-9a-f]+)-{plat}-lgpl-{re.escape(branch)}\.{re.escape(ext)}")
        asset = next(a for a in newest["assets"] if pat.fullmatch(a["name"]))
        version = asset["name"].split("-" + plat)[0].removeprefix("ffmpeg-")
        sha = asset["digest"].removeprefix("sha256:")
        entries.append(f'''        Self {{
            version: VERSION,
            url: concat!(
                "https://github.com/BtbN/FFmpeg-Builds/releases/download/{tag}/",
                "{asset["name"]}"
            ),
            sha256: "{sha}",
            size: {asset["size"]:_},
            archive: Archive::{kind},
            license: "LGPL-3.0-or-later",
        }},''')
    table = "    // BEGIN TABLE\n    pub const ALL: [Self; 4] = [\n" + "\n".join(entries) + "\n    ];\n    // END TABLE"
    text = re.sub(r"    // BEGIN TABLE.*?    // END TABLE", lambda _: table, text, flags=re.S)
    text = re.sub(r'const VERSION: &str = "[^"]+";', f'const VERSION: &str = "{version}";', text)
    PINS.write_text(text)
    print(f"pinned {tag} ({version}); update the table in docs/overview/installing.md to match")
    return 0


sys.exit(main())
