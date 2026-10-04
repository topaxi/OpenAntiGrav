#!/usr/bin/env python3
"""Read which team flies each racer index in a loaded Pulse PSP race.

Run against a PPSSPP debugger with a race already loaded (for example after
`psp-drive.py menu --single-race`):

    uv run --with websocket-client scripts/psp-team-roster.py --port 45091 \\
        --label run1 --out data/scratch/<lane>/samples.jsonl

Reads `g_race_session` (`0x08b34320`) `+0x3c..+0x58`, the player's team
definition (`g_player_team_definition`, `0x08b3104c`) `+0x74`, and every
craft's own `+0x370 -> +0x74` through `g_race_manager` (`0x08b317b4`) `+0x78`.
Memory reads only, no breakpoints: breakpoints inside the roster draw did not
fire in the session that recovered it. See
docs/ghidra/functions/psp-pulse-usa/grid.md, "Which team flies which slot,
recovered".
"""

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

G_RACE_SESSION = 0x08B34320
G_PLAYER_TEAM_DEFINITION = 0x08B3104C
G_RACE_MANAGER = 0x08B317B4
G_GAME_MODE = 0x08B31048
SESSION_TEAMS = 0x3C
MANAGER_CRAFTS = 0x78
CRAFT_TEAM = 0x370
CRAFT_ROLE = 0x368
TEAM_NAME = 0x74


def name_at(d, pointer):
    return d.read_cstring(pointer) if pointer else None


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--port", type=int, default=47810)
    parser.add_argument("--label", default="")
    parser.add_argument("--out", type=Path, help="append one JSON line here")
    args = parser.parse_args()

    d = Debugger(args.port)
    session = d.read_u32(G_RACE_SESSION)
    slots = [name_at(d, d.read_u32(session + SESSION_TEAMS + 4 * i)) for i in range(8)]
    player = d.read_u32(G_PLAYER_TEAM_DEFINITION)
    manager = d.read_u32(G_RACE_MANAGER)
    crafts = []
    for i in range(8):
        craft = d.read_u32(manager + MANAGER_CRAFTS + 4 * i)
        team = d.read_u32(craft + CRAFT_TEAM) if craft else 0
        crafts.append({
            "craft": hex(craft),
            "role": d.read_u32(craft + CRAFT_ROLE) if craft else None,
            "team": name_at(d, d.read_u32(team + TEAM_NAME)) if team else None,
        })
    record = {
        "label": args.label,
        "mode": d.read_u32(G_GAME_MODE),
        "session_vtable": hex(d.read_u32(session + 0x38)),
        "player": name_at(d, d.read_u32(player + TEAM_NAME)) if player else None,
        "slots": slots,
        "crafts_by_racer_index": crafts,
    }
    print(json.dumps(record))
    if args.out:
        with args.out.open("a") as f:
            f.write(json.dumps(record) + "\n")
    d.close()


if __name__ == "__main__":
    main()
