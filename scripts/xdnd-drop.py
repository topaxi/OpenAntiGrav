#!/usr/bin/env python3
"""Drops files on an X11 window by the XDND protocol, so a native window's
drag-and-drop path (winit's `HoveredFile` / `DroppedFile`) can be driven without
a pointer. Run it inside the same Xvfb as the window:

    DISPLAY=:93 uv run --with python-xlib python3 scripts/xdnd-drop.py \\
        --window "OpenAntiGrav" /path/to/a.dkey [/path/to/image.iso ...]

Finds the window by name with `xdotool`, sends XdndEnter / XdndPosition /
XdndDrop, answers the target's selection request with a `text/uri-list` of the
files, and waits for XdndFinished. Exits 0 when the target accepted the drop.
"""

import argparse
import subprocess
import sys
import time
from pathlib import Path
from urllib.parse import quote

from Xlib import X, display, protocol


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--window", required=True, help="the target window's name")
    parser.add_argument("--hold", type=float, default=0.0, help="seconds to hover before the drop")
    parser.add_argument("files", nargs="+", type=Path)
    args = parser.parse_args()

    found = subprocess.run(
        ["xdotool", "search", "--name", args.window], capture_output=True, text=True
    ).stdout.split()
    if not found:
        print(f"no window named {args.window!r}", file=sys.stderr)
        return 2
    target_id = int(found[-1])

    d = display.Display()
    root = d.screen().root
    target = d.create_resource_object("window", target_id)
    src = root.create_window(0, 0, 1, 1, 0, d.screen().root_depth)
    atom = d.intern_atom
    uri_list = atom("text/uri-list")
    selection = atom("XdndSelection")
    action_copy = atom("XdndActionCopy")
    body = "".join(f"file://{quote(str(f.resolve()))}\r\n" for f in args.files).encode()
    src.set_selection_owner(selection, X.CurrentTime)

    aware = target.get_full_property(atom("XdndAware"), X.AnyPropertyType)
    if aware is None:
        print("the target window is not XDND aware", file=sys.stderr)
        return 3
    version = min(5, aware.value[0])

    def send(window, message_type, data):
        event = protocol.event.ClientMessage(
            window=window, client_type=message_type, data=(32, data)
        )
        window.send_event(event, event_mask=0)
        d.flush()

    finished = False

    def pump(seconds: float) -> None:
        """Answers the target's selection request, which carries the file list
        to it: the target asks for it on XdndEnter and a drop that arrives
        before the answer is ignored."""
        nonlocal finished
        end = time.monotonic() + seconds
        while time.monotonic() < end and not finished:
            while d.pending_events():
                ev = d.next_event()
                if ev.type == X.SelectionRequest:
                    ev.requestor.change_property(ev.property, uri_list, 8, body)
                    notify = protocol.event.SelectionNotify(
                        time=ev.time,
                        requestor=ev.requestor,
                        selection=ev.selection,
                        target=ev.target,
                        property=ev.property,
                    )
                    ev.requestor.send_event(notify, event_mask=0)
                    d.flush()
                elif ev.type == X.ClientMessage and ev.client_type == atom("XdndFinished"):
                    finished = True
            time.sleep(0.05)

    send(target, atom("XdndEnter"), [src.id, version << 24, uri_list, 0, 0])
    send(target, atom("XdndPosition"), [src.id, 0, (100 << 16) | 100, X.CurrentTime, action_copy])
    pump(max(args.hold, 3.0))
    send(target, atom("XdndPosition"), [src.id, 0, (110 << 16) | 110, X.CurrentTime, action_copy])
    send(target, atom("XdndDrop"), [src.id, 0, X.CurrentTime, 0, 0])
    pump(10.0)
    print("drop finished" if finished else "no XdndFinished from the target")
    return 0 if finished else 1


if __name__ == "__main__":
    sys.exit(main())
