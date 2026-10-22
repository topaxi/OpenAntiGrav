#!/usr/bin/env python3
"""Turn Wipeout HD's drawn-hull shake off (or back on) in a lane's private RPCS3 config.

    source data/scratch/<lane>/env.sh
    python3 scripts/rpcs3-hd-no-hull-shake.py            # on: the shake is patched out
    python3 scripts/rpcs3-hd-no-hull-shake.py --remove   # off: the stock game again

The original adds a random shake to every hull's display matrix each frame (`HullShake_Apply`,
`docs/ghidra/functions/ps3-hdfury-eu/hull-shake.md`), so a still pair cannot match ours frame for
frame. The patch turns the one call to it, `bl 0x000d39d0` at `0x000ebc84`, into a `nop`
(`0x60000000`). `HullShake_Update` still runs, so `rand` is drawn exactly as before and nothing
else moves. Ours draws its side with `oag-game --no-hull-shake`.

It is an RPCS3 `patch.yml` entry keyed to HD's PPU executable hash, which RPCS3 applies when it
loads the executable, before the code is interpreted or compiled, so it holds for both
`Interpreter (static)` and the LLVM recompiler. (A GDB write is not an option: the stub serves one
session per launch and the pair script needs it.)

Only a lane's config is touched: `XDG_CONFIG_HOME/rpcs3` must not be `~/.config/rpcs3`, and the
lane's `patches` (a symlink to the maintainer's, `scripts/emu-env.sh`) is replaced by a private copy
first. RPCS3 must be restarted for a change to apply; confirm with `Applied patch` in
`XDG_CACHE_HOME/rpcs3/RPCS3.log`.
"""

import argparse
import os
import shutil
import sys
from pathlib import Path

PPU_HASH = "PPU-b1c6a86617b246b2df74dbe90f8400cd3809d4dc"  # BCES00664, read from RPCS3.log
DESCRIPTION = "OpenAntiGrav: no drawn hull shake"
TITLE = "WipEout HD Fury"
SERIAL = "BCES00664"
CALL_SITE = 0x000EBC84  # `bl HullShake_Apply` in `Ship_UpdateEntity`
NOP = 0x60000000
MARKER = "# written by scripts/rpcs3-hd-no-hull-shake.py"

PATCH = f"""{MARKER}
Version: 1.2

{PPU_HASH}:
  "{DESCRIPTION}":
    Games:
      "{TITLE}":
        {SERIAL}: [ All ]
    Author: "OpenAntiGrav"
    Notes: "bl HullShake_Apply (0x000d39d0) at {CALL_SITE:#010x} becomes a nop: hulls draw at the body's pose"
    Patch Version: 1.0
    Patch:
      - [ be32, {CALL_SITE:#010x}, {NOP:#010x} ]
"""

CONFIG = f"""{MARKER}
{PPU_HASH}:
  "{DESCRIPTION}":
    "{TITLE}":
      {SERIAL}:
        All:
          Enabled: true
"""


def config_dir():
    xdg = os.environ.get("XDG_CONFIG_HOME")
    if not xdg:
        sys.exit("XDG_CONFIG_HOME is not set: source the lane's env.sh first")
    path = Path(xdg) / "rpcs3"
    if path.resolve() == (Path.home() / ".config" / "rpcs3").resolve():
        sys.exit("refusing to patch the maintainer's own RPCS3 config; use a lane (scripts/emu-env.sh)")
    if not (path / "config.yml").exists():
        sys.exit(f"{path} is not an RPCS3 config dir")
    return path


def private_patches(root):
    """The lane's `patches` as a directory of its own, copied from the shared one if a symlink."""
    patches = root / "patches"
    if patches.is_symlink():
        shared = patches.resolve()
        patches.unlink()
        shutil.copytree(shared, patches) if shared.is_dir() else patches.mkdir()
    patches.mkdir(exist_ok=True)
    return patches


def ours_or_absent(path):
    return not path.exists() or path.read_text().startswith(MARKER)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--remove", action="store_true", help="take the patch out again")
    args = parser.parse_args()
    root = config_dir()
    patch_file = private_patches(root) / "patch.yml"
    config_file = root / "patch_config.yml"
    for path in (patch_file, config_file):
        if not ours_or_absent(path):
            sys.exit(f"{path} exists and is not this script's; merge by hand")
    if args.remove:
        for path in (patch_file, config_file):
            path.unlink(missing_ok=True)
        print(f"hull shake restored in {root}")
        return
    patch_file.write_text(PATCH)
    config_file.write_text(CONFIG)
    print(f"hull shake patched out in {root} ({CALL_SITE:#010x} -> {NOP:#010x}); restart RPCS3")


if __name__ == "__main__":
    main()
