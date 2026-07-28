default: check

# Emulator binaries used by the launch-* recipes below. Override per-invocation with
# `just --set ppsspp_bin /path/to/PPSSPP launch-pulse-psp`, or export PPSSPP_BIN etc.
ppsspp_bin := env_var_or_default("PPSSPP_BIN", "PPSSPPSDL")
pcsx2_bin := env_var_or_default("PCSX2_BIN", "pcsx2")
rpcs3_bin := env_var_or_default("RPCS3_BIN", "rpcs3")

# fmt + lint + test + docs, the gate every commit must pass
check: fmt-check lint test check-docs

# Documentation is a deliverable, so its links are checked like any other build output
check-docs:
    python3 scripts/check-doc-links.py

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo nextest run --workspace

# Behavioural / ground-truth tests that need data/images populated
test-data:
    cargo nextest run --workspace --run-ignored all

build:
    cargo build --workspace

docs:
    cargo doc --workspace --no-deps --document-private-items

unpack *ARGS:
    cargo run -q -p oag-tools --bin oag-unpack -- {{ARGS}}

# Report platform + serial for every image present in data/images/
unpack-info:
    #!/usr/bin/env bash
    set -euo pipefail
    shopt -s nullglob
    for img in data/images/*.chd data/images/*.iso; do
        echo "=== $img ==="
        cargo run -q -p oag-tools --bin oag-unpack -- info "$img"
    done

# Full file listing for every image present in data/images/
unpack-list:
    #!/usr/bin/env bash
    set -euo pipefail
    shopt -s nullglob
    for img in data/images/*.chd data/images/*.iso; do
        echo "=== $img ==="
        cargo run -q -p oag-tools --bin oag-unpack -- list "$img"
    done

# Record SHA-256 of every source image (paste into docs/reverse-engineering/source-images.md)
hash-images:
    #!/usr/bin/env bash
    set -euo pipefail
    shopt -s nullglob
    for img in data/images/*; do
        [ -f "$img" ] || continue
        printf '%s  %s\n' "$(sha256sum "$img" | cut -d' ' -f1)" "$(basename "$img")"
    done

# Reads data/images/pulse-psp-usa.chd unless a source is given. The first run
# transcodes the intro through ffmpeg into data/cache/, which is gitignored and
# always safe to delete. Without ffmpeg the sequence still plays, without a
# picture. See docs/architecture/frontend-boot.md.

# Run the game: intro video, the Language Selection menu, then a race
play *ARGS:
    cargo run -q --release -p oag-game -- {{ARGS}}

# Capture the boot sequence, the menu and the race it launches, without a display
play-screenshots out="/tmp":
    #!/usr/bin/env bash
    set -euo pipefail
    cargo run -q --release -p oag-game -- --screenshot "{{out}}/oag-intro.png" --ticks 400
    cargo run -q --release -p oag-game -- --screenshot "{{out}}/oag-language.png" \
        --until "Language Selection" --hold start
    # Launch Game hands off to a race, so this one is a ship on a track that was
    # reached through the menus. `--press` pulses cross on alternating ticks, which
    # is what picks the language and then leaves the throttle on half of them.
    cargo run -q --release -p oag-game -- --screenshot "{{out}}/oag-launch.png" \
        --until "Launch Game" --press start,cross --ticks 60

# The engine only: no game content is ever packaged, and the script refuses to
# build if any found its way in. The player's own disc image is looked for
# beside the AppImage at runtime. See docs/tools/packaging.md, which also
# records why AppImage rather than Flatpak.

# Package the game as one x86_64 AppImage, built against this machine's glibc
appimage *ARGS:
    ./scripts/build-appimage.sh {{ARGS}}

# The same package built in Debian bookworm, which is what a Steam Deck needs: a
# native build links libm symbols an older glibc does not have and refuses to
# start there. Needs podman or docker; the image is built once from
# packaging/appimage/Containerfile. Slower, so `just appimage` stays the fast
# path for a local run. See docs/tools/packaging.md#glibc.

# Package the AppImage against an older glibc, so it also runs on a Steam Deck
appimage-portable *ARGS:
    ./scripts/build-appimage.sh --container {{ARGS}}

# View assets straight from a disc image
view *ARGS:
    cargo run -q -p oag-view -- {{ARGS}}

# Read a WAD archive
wad *ARGS:
    cargo run -q -p oag-tools --bin oag-wad -- {{ARGS}}

# Rebuild the WAD entry-name candidate list from a disc image
mine-names image:
    python3 scripts/mine-names.py {{image}}

# Extract a raw ISO from a CHD, which is what the emulators want
extract-iso image="data/images/pulse-psp-usa.chd" out="data/cache/pulse-psp-usa.iso":
    chdman extractdvd -i {{image}} -o {{out}} -f

# Run an original disc image in its platform's emulator, for reference and
# behavioural ground-truth comparison. One recipe per (to be) supported title, see
# data/README.md for expected image names. Both PPSSPP and PCSX2 read .chd directly,
# no extract-iso needed. Override the emulator binary with PPSSPP_BIN / PCSX2_BIN /
# RPCS3_BIN, e.g. `PPSSPP_BIN=PPSSPP just launch-pulse-psp`.

# Wipeout Pulse (PSP) in PPSSPP
launch-pulse-psp image="data/images/pulse-psp-usa.chd" *ARGS:
    {{ppsspp_bin}} {{image}} {{ARGS}}

# Wipeout Pulse (PS2) in PCSX2
launch-pulse-ps2 image="data/images/pulse-ps2-eu.chd" *ARGS:
    {{pcsx2_bin}} -fullscreen {{image}} {{ARGS}}

# Wipeout Pure (PSP) in PPSSPP
launch-pure-psp image="data/images/pure-psp-usa.chd" *ARGS:
    {{ppsspp_bin}} {{image}} {{ARGS}}

# WipEout HD / Fury (PS3) in RPCS3. PS3 titles usually need installing rather than
# booting a raw image directly; see RPCS3's own docs if this doesn't boot as-is.
launch-hdfury-ps3 image="data/images/hdfury-ps3-eu.iso" *ARGS:
    {{rpcs3_bin}} {{image}} {{ARGS}}

# Capture a per-tick trace out of the original running in PPSSPP. Needs a PPSSPP
# with its websocket debugger enabled and the game already in a race; see
# docs/reverse-engineering/ppsspp-debugger.md.
trace *ARGS:
    uv run --with websocket-client python scripts/psp-trace.py {{ARGS}}

# Compare our simulation against a captured trace: first divergent tick, and by
# how much. See docs/tools/oag-trace.md.
trace-compare *ARGS:
    cargo run -q -p oag-trace -- {{ARGS}}

# Build the Allegrex processor module against the installed Ghidra.
# Stock Ghidra mis-decodes PSP vector code; see docs/psp/allegrex-vfpu.md.
build-allegrex *ARGS:
    ./scripts/build-ghidra-allegrex.sh {{ARGS}}

# Resolve the PSP import stubs from the binary's own NID tables
resolve-imports boot="data/extracted/psp/PSP_GAME/SYSDIR/BOOT.BIN":
    python3 scripts/resolve-psp-imports.py {{boot}} --modules -o data/ghidra/psp-imports.tsv

# Apply every documented symbol name to the open Ghidra program
apply-names *ARGS: resolve-imports
    python3 scripts/apply-ghidra-names.py {{ARGS}}

# Assert no game content has ever been committed
audit-leakage:
    #!/usr/bin/env bash
    set -euo pipefail
    if git ls-files | grep -Ei '\.(chd|iso|cso|pkg|pbp|wad|elf|prx|self|bin|img)$'; then
        echo "FAIL: game content is tracked by git" >&2
        exit 1
    fi
    echo "OK: no game content tracked"
