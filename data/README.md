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
  dlc/          PSN DLC packages, as downloaded - see below
  ghidra/       Ghidra project directory (shared analysis, not versioned)
  tools/        Built third-party tooling, e.g. the Allegrex Ghidra extension
  traces/       Runtime traces captured from PPSSPP / PCSX2
  saves/        Emulator save states used by the behavioural verification suite
```

`tools/` is populated by `just build-allegrex`. It holds a git checkout and a
build output, neither of which belongs in this repository.

`cache/` is where a **repository checkout** caches converted assets. A packaged
build has no checkout around it and uses `~/.cache/oag/` instead, rather than
writing beside wherever it was run from - see
[packaging](../docs/tools/packaging.md#where-the-disc-image-comes-from).
Deleting either is always safe.

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
| `pulse-psp-eu.chd` | Wipeout Pulse | PSP | Europe/Australia |
| `pure-psp-usa.chd` | Wipeout Pure | PSP | USA |
| `hdfury-ps3-eu.iso` | WipEout HD / Fury | PS3 | Europe |

`.iso` works anywhere `.chd` does; the tools sniff the container.

`pulse-psp-eu.chd` is not one of `oag-game`'s auto-detected `IMAGE_NAMES` (it
is a reverse-engineering reference disc, not the played-from target) - point
tools at it explicitly by path. It is a genuinely different `BOOT.BIN` from
`pulse-psp-usa.chd`'s, not a re-labelled copy - see
[source-images.md](../docs/reverse-engineering/source-images.md).

**Pure and HD/Fury are read-only format targets today, not playable ones.**
`oag-view`/`oag-unpack`/`oag-wad` read them opportunistically per
[ADR-0009](../docs/architecture/adr/0009-multi-game-fanout.md). Pointing
`oag-game` at either fails fast with a named error (`Error::WrongTitle` for
Pure's PSP disc, since its archives share Pulse's own filenames; PS3
identification doesn't exist yet, so HD/Fury falls back to the archive's own
"nothing matched" error) rather than silently loading the wrong game -
playable second-title support is tracked at roadmap M8.

Confirm what you have with:

```sh
cargo run -p oag-tools --bin oag-unpack -- info data/images/pulse-psp-usa.chd
```

Record the SHA-256 of every image you use in
[`docs/reverse-engineering/source-images.md`](../docs/reverse-engineering/source-images.md).
Hashes are fine to commit; content is not.

## DLC (`data/dlc/`)

Four PSN DLC packages for Wipeout Pulse EU, as downloaded (`.zip`, each
containing a `UCES00465/` folder): the Auricom, Harimau, Icaras and Mirage
ship packs. Each is `PACKn.edat`, three `PACKn_UIn.edat` files, and one
`PARAM.pbp`.

**Despite the extension, `PACKn.edat` is not encrypted.** It reads directly
as an ordinary `oag-wad`-format archive (32-33 entries, LZSS-compressed,
consistent offset chain) - `cargo run -p oag-tools --bin oag-wad -- list
<extracted PACKn.edat>` lists it like any other WAD. Checked against Ghidra
before assuming otherwise: the game's only NpDrm import,
`sceNpDrmEdataSetupKey`, is called with no key argument and
`sceNpDrmSetLicenseeKey` (which would supply one) is never imported at all -
consistent with there being no per-title secret to find, though what (if
anything) supplies a key at the OS level was not chased further once the file
itself proved to be plaintext. `PARAM.pbp` is the standard, unencrypted PBP
(title/icon metadata for the XMB). No decryption work was needed or done -
the `PACKn_UIn.edat` files (front-end UI assets, presumably the same shape)
have not been checked yet but are expected to match.
