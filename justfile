default: check

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

# View assets straight from a disc image
view *ARGS:
    cargo run -q -p oag-view -- {{ARGS}}

# Read a WAD archive
wad *ARGS:
    cargo run -q -p oag-tools --bin oag-wad -- {{ARGS}}

# Rebuild the WAD entry-name candidate list from a disc image
mine-names image:
    python3 scripts/mine-names.py {{image}}

# Build the Allegrex processor module against the installed Ghidra.
# Stock Ghidra mis-decodes PSP vector code; see docs/psp/allegrex-vfpu.md.
build-allegrex *ARGS:
    ./scripts/build-ghidra-allegrex.sh {{ARGS}}

# Assert no game content has ever been committed
audit-leakage:
    #!/usr/bin/env bash
    set -euo pipefail
    if git ls-files | grep -Ei '\.(chd|iso|cso|pkg|pbp|wad|elf|prx|self|bin|img)$'; then
        echo "FAIL: game content is tracked by git" >&2
        exit 1
    fi
    echo "OK: no game content tracked"
