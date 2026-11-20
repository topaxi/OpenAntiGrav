#!/usr/bin/env python3
"""Every `Redirect` of the front-end XML in a directory: its screen, `forward`, `backward` and targets.

    oag-wad extract <image>:<wad> -o DIR --expand      # a WAD title: shortened XML is written expanded
    scripts/fe-redirect-census.py DIR [--buttons]      # tab-separated: file, screen, forward, backward, goto

`--buttons` keeps only the redirects that name a pad button other than a direction or `start`
(`docs/ui/back-and-help.md`). Reads plain XML (the PS2 `WADS2.WAD`, a PSARC's `.xml` entries) and the
shortened dialect's `<m>` redirects and `c=` screen names. A line-oriented scan, not an XML parser:
the PS2 disc ships files a parser refuses.
"""

import re
import sys
from pathlib import Path

TAG = re.compile(r"<(/?)(Screen|Redirect|m)\b([^>]*?)(/?)>")
NAME = re.compile(r'\b(?:name|c)="([^"]*)"')
VALUES = re.compile(r"<(?:Values|a)\b([^>]*)>")
TARGET = re.compile(r"<(?:Default|Entry|Goto|q|e)\b([^>]*)>")
DIRECTIONS = {"up", "down", "left", "right", "start", "none"}


def attr(text, key):
    found = re.search(r'\b%s="([^"]*)"' % key, text)
    return found.group(1) if found else None


def census(path):
    text = path.read_bytes().decode("utf-8", "replace")
    if "<Redirect" not in text and "<m>" not in text:
        return
    screens, redirect = [], None
    for line in text.splitlines():
        for close, tag, attrs, selfclosing in TAG.findall(line):
            if tag == "Screen":
                if close:
                    if screens:
                        screens.pop()
                elif not selfclosing:
                    name = NAME.search(attrs)
                    screens.append(name.group(1) if name else "")
            elif close:
                if redirect is not None:
                    yield (path.name, "/".join(s for s in screens if s), redirect[0], redirect[1], "; ".join(redirect[2]))
                redirect = None
            else:
                redirect = [None, None, []]
        if redirect is not None:
            values = VALUES.search(line)
            if values:
                redirect[0] = attr(values.group(1), "forward") or redirect[0]
                redirect[1] = attr(values.group(1), "backward") or redirect[1]
            target = TARGET.search(line)
            if target:
                goto = attr(target.group(1), "goto") or ""
                item, equals = attr(target.group(1), "item"), attr(target.group(1), "equals")
                redirect[2].append(goto + (" if %s==%s" % (item, equals) if item else ""))


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    only_buttons = "--buttons" in sys.argv
    if len(args) != 1 or not Path(args[0]).is_dir():
        sys.exit(__doc__)
    for path in sorted(Path(args[0]).iterdir()):
        if path.is_file():
            for file, screen, forward, backward, target in census(path):
                if only_buttons and {forward, backward} <= DIRECTIONS | {None}:
                    continue
                print("\t".join([file, screen, "forward=%s" % forward, "backward=%s" % backward, target]))


if __name__ == "__main__":
    main()
