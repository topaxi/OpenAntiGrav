default: check

# Emulator binaries used by the launch-* recipes below. Override per-invocation with
# `just --set ppsspp_bin /path/to/PPSSPP launch-pulse-psp`, or export PPSSPP_BIN etc.
ppsspp_bin := env_var_or_default("PPSSPP_BIN", "PPSSPPSDL")
pcsx2_bin := env_var_or_default("PCSX2_BIN", "pcsx2")
rpcs3_bin := env_var_or_default("RPCS3_BIN", "rpcs3")
mangohud_bin := env_var_or_default("MANGOHUD_BIN", "mangohud")

# The PSP disc a scripted run reads its track and handling out of, and the ISO
# PPSSPP itself wants (it will not open a CHD). Override with OAG_IMAGE / OAG_ISO.
psp_image := env_var_or_default("OAG_IMAGE", "data/images/pulse-psp-usa.chd")
psp_iso := env_var_or_default("OAG_ISO", "data/cache/pulse-psp-usa.iso")

# The scenario `just scripted-sim` and `just scripted-emu` run when given none:
# the recorded whole lap of Talon's Junction. See docs/tools/oag-trace.md.
default_scenario := "verification/scenarios/talons-junction-time-trial-lap.inputs"

# The `play`-family recipes' own opt-in default: exercise the accelerated
# GStreamer decode path (ADR-0017) rather than always falling to the AV1
# cache, on the one platform it exists for. This is a `just`-only default -
# `oag-game`'s own Cargo feature stays off by default, so `cargo build`,
# `cargo test` and CI never touch GStreamer. Empty (not an error) on any
# other `os()`, since the feature does not exist there.
native_video_flags := if os() == "linux" { "--features native-video" } else { "" }

# fmt + lint + test + docs + architecture rules, the gate every commit must pass
check: fmt-check lint test check-docs check-deps

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

# Run the game: intro video, the Language Selection menu, then a race.
# Boots the EU PSP disc (pulse-psp-eu.chd) by default - a British game, EU
# build. `psp-usa`/`pulse-psp-usa`/`usa` swaps in the USA disc (the
# reverse-engineering target of record - see source-images.md); `ps2`/
# `pulse-ps2` swaps in the PS2 disc (EU). A bare flag (`--ticks 5`) still
# gets the EU default; anything else (a path or `image:entry` spec) is
# passed straight through unchanged, so `just play data/images/foo.chd
# --ticks 5` still works.
play *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    args=({{ARGS}})
    first="${args[0]:-}"
    case "$first" in
        ""|--*)
            args=("data/images/pulse-psp-eu.chd" "${args[@]}")
            ;;
        pulse-psp-eu|psp-eu|eu)
            args=("data/images/pulse-psp-eu.chd" "${args[@]:1}")
            ;;
        pulse-psp-usa|psp-usa|usa)
            args=("data/images/pulse-psp-usa.chd" "${args[@]:1}")
            ;;
        pulse-ps2|ps2)
            args=("data/images/pulse-ps2-eu.chd" "${args[@]:1}")
            ;;
    esac
    ${OAG_PLAY_WRAPPER:-} cargo run -q --release -p oag-game {{native_video_flags}} -- "${args[@]}"

# Same as `play`, but wrapped with MangoHud for an FPS/frametime overlay. Needs
# `mangohud` installed; override the binary with `MANGOHUD_BIN`.
play-mangohud *ARGS:
    OAG_PLAY_WRAPPER={{mangohud_bin}} just play {{ARGS}}

# Capture the boot sequence, the menu and the race it launches, without a display
play-screenshots out="/tmp":
    #!/usr/bin/env bash
    set -euo pipefail
    cargo run -q --release -p oag-game {{native_video_flags}} -- --screenshot "{{out}}/oag-intro.png" --ticks 400
    cargo run -q --release -p oag-game {{native_video_flags}} -- --screenshot "{{out}}/oag-language.png" \
        --until "Language Selection" --hold start
    # Launch Game hands off to a race, so this one is a ship on a track that was
    # reached through the menus. `--press` pulses cross on alternating ticks, which
    # is what picks the language and then leaves the throttle on half of them.
    cargo run -q --release -p oag-game {{native_video_flags}} -- --screenshot "{{out}}/oag-launch.png" \
        --until "Launch Game" --press start,cross --ticks 60

# One race frame per upscaler, plus a supersampled reference, for judging a
# resampler by looking rather than by counting. An aggregate statistic ranks a
# sharpener below a blur every time - see docs/rendering/README.md, "Measuring a
# renderer change" - so this recipe produces images and deliberately no numbers.
#
# `--presented` is what puts the render scale, the upscaler and the grade in the
# way; without it a capture never reaches the blit and every image would be
# identical. The scale is an argument because a magnifier can only be judged
# where it is magnifying.
compare-upscalers image scale="50" out="/tmp":
    #!/usr/bin/env bash
    set -euo pipefail
    for upscaler in off fsr1; do
        cargo run -q --release -p oag-game -- "{{image}}" --race \
            --screenshot "{{out}}/oag-upscale-{{scale}}-$upscaler.png" \
            --ticks 300 --hold cross --presented \
            --render-scale "{{scale}}" --upscaler "$upscaler"
    done
    # The ceiling to judge both against: the same frame with four times the
    # samples, which is what neither of them can do better than.
    cargo run -q --release -p oag-game -- "{{image}}" --race \
        --screenshot "{{out}}/oag-upscale-reference.png" \
        --ticks 300 --hold cross --presented --render-scale 200

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

# Run a committed scenario through OUR OWN physics and print a run report: where
# the ship is every N ticks, how fast, whether it is still on the track. Needs a
# disc image (for the track and the handling stats) and nothing else - no
# emulator, no capture, no GPU. This is the half that always works.
#
#   just scripted-sim                                    # the whole-lap scenario
#   just scripted-sim verification/scenarios/steer-left.inputs
#   just scripted-sim verification/scenarios/steer-left.inputs --every 20 --out /tmp/ours.csv
#
# Add `--script-lead 2` if the run is going to be held next to a capture: every
# capture under data/traces/ was taken through `psp-trace.py --script-lead 2`,
# which never sends the script's first two ticks. See docs/tools/oag-trace.md.
[doc("Run a committed scenario through our own physics and print a run report")]
scripted-sim scenario=default_scenario *ARGS:
    cargo run -q -p oag-trace -- drive {{scenario}} --source {{psp_image}} {{ARGS}}

# Drive the same scenario into the ORIGINAL, running in PPSSPP, and capture a
# per-tick trace of what it did. Checks its preconditions first and explains them
# rather than failing deep inside a capture, then puts the craft back on the start
# line and sends the scenario with the measured three-frame input correction.
#
# Fully autonomous from a booted emulator: it walks the front end into a Time
# Trial itself, whatever screen the game is sitting on. All it needs from you is a
# PPSSPP with its websocket debugger enabled, and it prints how to start one if
# there is none. See docs/reverse-engineering/ppsspp-debugger.md.
#
#   just scripted-emu                                    # the whole-lap scenario
#   just scripted-emu verification/scenarios/steer-left.inputs data/traces/left.csv
[doc("Drive the same scenario into the original in PPSSPP and capture a trace")]
scripted-emu scenario=default_scenario out="data/traces/scripted-emu.csv" *ARGS:
    uv run --with websocket-client python scripts/psp-drive.py preflight \
        --image {{psp_image}} --iso {{psp_iso}} --emulator {{ppsspp_bin}}
    uv run --with websocket-client python scripts/psp-drive.py menu
    uv run --with websocket-client python scripts/psp-drive.py restart
    uv run --with websocket-client python scripts/psp-trace.py \
        --script {{scenario}} --script-lead 2 --out {{out}} {{ARGS}}
    @echo "captured {{out}}; compare it with:"
    @echo "    just trace-compare run {{out}} --source {{psp_image}} --script {{scenario}} --script-lead 2"

# Capture a per-tick trace out of the original running in PPSSPP. Needs a PPSSPP
# with its websocket debugger enabled and the game already in a race; see
# docs/reverse-engineering/ppsspp-debugger.md.
trace *ARGS:
    uv run --with websocket-client python scripts/psp-trace.py {{ARGS}}

# Put the original back on the start line, or send an input script at a
# free-running emulator in real time. `just drive restart`, `just drive drive
# --script F`; see docs/reverse-engineering/ppsspp-debugger.md.
[doc("Restart a race, or send a script at a free-running emulator in real time")]
drive *ARGS:
    uv run --with websocket-client python scripts/psp-drive.py {{ARGS}}

# Fly the original round a lap, steering off the track's own spline, and write
# down what it pressed. Needs `oag-trace track` output; see
# docs/tools/oag-trace.md#a-whole-lap-and-where-it-came-from.
[doc("Fly the original round a lap and record the input it took to do it")]
autopilot *ARGS:
    uv run --with websocket-client python scripts/psp-autopilot.py {{ARGS}}

# Compare our simulation against a captured trace: first divergent tick, and by
# how much. See docs/tools/oag-trace.md.
trace-compare *ARGS:
    cargo run -q -p oag-trace -- {{ARGS}}

# Where a track's speed/weapon pads are, with approach points for `just drive
# place`. See docs/tools/frame-compare.md.
#
#   just pads --before 50 > /tmp/pads.csv
#   just drive place --pads-csv /tmp/pads.csv --pad 3 --before 50 --speed 120
[doc("Dump a track's pad trigger volumes, with approach points for teleports")]
pads *ARGS:
    cargo run -q -p oag-trace -- pads --source {{psp_image}} {{ARGS}}

# One frame of ours, at PSP size, from a row of a capture: the ship pose always,
# the recorded camera too when the capture was taken with `--camera`. The other
# half of a comparison whose emulator half `psp-trace.py --shot-every` took.
# See docs/tools/frame-compare.md.
[doc("Render one frame from a captured pose at PSP size")]
frame-shot trace tick out="/tmp/oag-frame.png" *ARGS:
    cargo run -q --release -p oag-game -- --race --screenshot {{out}} \
        --ticks 0 --size 480x272 --pose-from {{trace}} --pose-tick {{tick}} {{ARGS}}

# `compare` exits nonzero whenever the images differ, which here is always -
# hence the `|| true`; the diff image is the answer, not the exit code.
[doc("Side-by-side and difference image of an emulator shot and one of ours")]
frame-compare theirs ours out="/tmp":
    #!/usr/bin/env bash
    set -euo pipefail
    command -v magick >/dev/null || { echo "frame-compare needs ImageMagick"; exit 1; }
    magick "{{theirs}}" -resize '480x272!' "{{out}}/oag-theirs.png"
    magick montage "{{out}}/oag-theirs.png" "{{ours}}" -tile 2x1 -geometry +2+2 "{{out}}/oag-side.png"
    magick compare "{{out}}/oag-theirs.png" "{{ours}}" -compose src "{{out}}/oag-diff.png" || true
    echo "side-by-side: {{out}}/oag-side.png"
    echo "difference:   {{out}}/oag-diff.png"

# Build the Allegrex processor module against the installed Ghidra.
# Stock Ghidra mis-decodes PSP vector code; see docs/psp/allegrex-vfpu.md.
build-allegrex *ARGS:
    ./scripts/build-ghidra-allegrex.sh {{ARGS}}

# Build the Emotion Engine (PS2) processor module against the installed
# Ghidra. Stock Ghidra mis-decodes R5900 code as MIPS Release 6; see
# docs/reverse-engineering/toolchain.md. Needs build-allegrex to have run at
# least once - this borrows its Gradle wrapper.
build-emotionengine *ARGS:
    ./scripts/build-ghidra-emotionengine.sh {{ARGS}}

# Resolve the PSP import stubs from the binary's own NID tables
resolve-imports boot="data/extracted/psp/PSP_GAME/SYSDIR/BOOT.BIN":
    python3 scripts/resolve-psp-imports.py {{boot}} --modules -o data/ghidra/psp-imports.tsv

# Apply every documented symbol name to the open Ghidra program
apply-names *ARGS: resolve-imports
    python3 scripts/apply-ghidra-names.py {{ARGS}}

# Report names in the open Ghidra program that names.tsv does not sanction
audit-names *ARGS:
    python3 scripts/audit-ghidra-names.py {{ARGS}}

# Assert no game content (or a reproduction this project itself writes) is tracked
audit-leakage:
    python3 scripts/check-leakage.py

# Assert the two architecture dependency rules from CLAUDE.md still hold
check-deps:
    python3 scripts/check-dependency-rules.py
