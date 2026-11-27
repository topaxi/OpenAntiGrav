"""Where a script writes, so that nothing lands in the OS temp directory.

On Linux that is often a `tmpfs` with a per-user quota, and a full one fails
every shell and gate on the machine.

- `temp_root()` for files the script removes again: pass it as `dir=` to
  `tempfile`.
- `output(name)` for a default output path a run keeps, under `data/scratch/`.

Shell scripts source `oag-tmp.sh`; Rust tests use `oag_testdata::scratch`.
"""

import os
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent


def temp_root() -> Path:
    """`<target>/tmp/scripts`, made if it is not there."""
    root = Path(os.environ.get("CARGO_TARGET_DIR") or REPO / "target") / "tmp" / "scripts"
    root.mkdir(parents=True, exist_ok=True)
    return root


def output(name: str) -> str:
    """`<repo>/data/scratch/<name>`, its parent made if it is not there."""
    path = REPO / "data" / "scratch" / name
    path.parent.mkdir(parents=True, exist_ok=True)
    return str(path)
