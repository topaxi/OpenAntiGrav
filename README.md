# OpenAntiGrav

A clean-room reimplementation of **Wipeout Pulse** in Rust, built as a
foundation for the wider Studio Liverpool anti-gravity racing lineage.

This is **not** a decompilation. The original executables are treated as a
specification: behaviour is studied, documented and then reimplemented with a
modern architecture. The goal is that gameplay is indistinguishable from the
original while the code underneath is something a contributor can read.

> **Status: early.** The disc tooling works and the documentation tree is
> established. There is no playable game yet. See
> [`docs/overview/roadmap.md`](docs/overview/roadmap.md) for where this is going
> and [`docs/psp/pulse-disc-layout.md`](docs/psp/pulse-disc-layout.md) for what
> has actually been found so far.

## Scope

In scope, in the order they will be tackled:

| Title | Platform | Role |
| --- | --- | --- |
| **Wipeout Pulse** | PSP, PS2 | Primary target |
| Wipeout Pure | PSP | Direct format ancestor, cross-reference |
| Wipeout HD / Fury | PS3 | Later |
| Wipeout 2048 | Vita | Long-term goal |
| Omega Collection | PS4 | If feasible |

Anything before Pure is out of scope.

## Legal

**No game content is distributed with this project, and none ever will be.** No
assets, no executables, no extracted data. You supply your own legally obtained
copies. See [`docs/overview/legal.md`](docs/overview/legal.md).

## Prerequisites

### Required

- **Rust** stable (1.85+, edition 2024). Install via [rustup](https://rustup.rs).
- **[cargo-nextest](https://nexte.st)** for the test runner.
- **[just](https://github.com/casey/just)** for the task runner.

```sh
cargo install cargo-nextest --locked
cargo install just --locked          # or: pacman -S just
```

### For working with disc images

- **`chdman`** (from `mame-tools`) is the reference implementation used to
  validate our own CHD reader. Not needed for day-to-day use; the Rust reader
  handles both `createdvd` and single-track `createcd` images natively.

```sh
sudo pacman -S mame-tools
```

### For reverse engineering

- **[Ghidra](https://ghidra-sre.org)** 11 or newer, with a JDK 21+.
- **[GhidraMCP](https://github.com/LaurieWired/GhidraMCP)**, which lets an agent
  drive Ghidra directly.
- **[PPSSPP](https://ppsspp.org)** for PSP runtime tracing, memory inspection
  and save states. This is what behavioural verification is measured against.
- **[PCSX2](https://pcsx2.net)** for the PS2 side.
- **`binwalk`** (optional) for faster triage of unknown containers.

```sh
sudo pacman -S ghidra ppsspp pcsx2
paru -S binwalk
```

Setting up the Ghidra bridge is described in
[`docs/reverse-engineering/toolchain.md`](docs/reverse-engineering/toolchain.md).

## Getting started

```sh
git clone <this repo> && cd OpenAntiGrav

# Put your own disc images here, named as described in data/README.md
cp "/path/to/WipEout Pulse (USA).chd" data/images/pulse-psp-usa.chd

# What is this disc?
just unpack info data/images/pulse-psp-usa.chd

# What is on it?
just unpack list data/images/pulse-psp-usa.chd

# Which files are worth reverse engineering?
just unpack sniff data/images/pulse-psp-usa.chd

# Pull the executable out for Ghidra
just unpack extract data/images/pulse-psp-usa.chd -o data/extracted/psp '*BOOT.BIN'
```

`just` on its own runs the full check: format, lint and test.

## Layout

```
crates/
  core/      deterministic math, fixed-timestep clock, seeded PRNG, state hashing
  disc/      CHD and raw ISO readers, ISO 9660 filesystem walker
  formats/   asset container identification and parsing
  tools/     command line tools (oag-unpack)
docs/        the primary deliverable, see docs/README.md
data/        your disc images and extracted data (gitignored)
```

More crates arrive as their milestone opens; the full intended shape is in
[`docs/architecture/workspace-layout.md`](docs/architecture/workspace-layout.md).

## Documentation

Documentation is a first-class deliverable, not an afterthought. Every discovery
gets written down with its evidence and a confidence score, so a future
contributor can understand the engine without reopening Ghidra.

Start at [`docs/README.md`](docs/README.md).

## Licence

Dual licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your
option.
