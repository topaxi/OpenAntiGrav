# `data/` - user-supplied game data

Everything in this directory except this README is **gitignored and must never be
committed**. OpenAntiGrav ships code and documentation only. Game content comes
from your own legally obtained copies.

See [`docs/overview/legal.md`](../docs/overview/legal.md) for the policy.

## Layout

```
data/
  images/       Disc images you supply (.chd / .iso). Normalised names, see below.
  extracted/
    psp/        Files extracted from a PSP UMD image
    ps2/        Files extracted from a PS2 DVD image
  cache/        Assets converted from your originals, rebuilt on demand
  ghidra/       Ghidra project directory (shared analysis, not versioned)
  tools/        Built third-party tooling, e.g. the Allegrex Ghidra extension
  traces/       Runtime traces captured from PPSSPP / PCSX2
  saves/        Emulator save states used by the behavioural verification suite
```

`tools/` is populated by `just build-allegrex`. It holds a git checkout and a
build output, neither of which belongs in this repository.

`cache/` holds assets converted from your own originals, where converting once is
better than reproducing the original decoder: see
[ADR-0004](../docs/architecture/adr/0004-asset-pipeline.md). It is derived
content, so the same rule applies as to everything else here, and **deleting it
is always safe**. It is rebuilt from your disc image on demand.

## Image naming

Tooling looks for these names. Copy your own images here and rename:

| Name | Title | Platform | Region |
| --- | --- | --- | --- |
| `pulse-psp-usa.chd` | Wipeout Pulse | PSP | USA |
| `pulse-ps2-eu.chd` | Wipeout Pulse | PS2 | Europe |
| `pure-psp-usa.chd` | Wipeout Pure | PSP | USA |

`.iso` works anywhere `.chd` does; the tools sniff the container.

Confirm what you have with:

```sh
cargo run -p oag-tools --bin oag-unpack -- info data/images/pulse-psp-usa.chd
```

Record the SHA-256 of every image you use in
[`docs/reverse-engineering/source-images.md`](../docs/reverse-engineering/source-images.md).
Hashes are fine to commit; content is not.
