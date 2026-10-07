# Every WAD-internal path on these discs is authored with backslashes
# (`Data\Ships\Feisar\ship.vex`). `just` interpolates a bare `{{ARGS}}` into the
# shell command line unquoted, and an unquoted backslash is an escape
# character to `sh` - it deletes the separators silently, with no error; the
# failure surfaces later as "no such entry", which reads like a wrong path
# rather than a mangled one. `positional-arguments` makes each recipe's own
# parameters available as real, already-quoted shell arguments (`$1`, `$2`,
# ... and `$@`), so a recipe body that references `"$@"` instead of `{{ARGS}}`
# gets the argument text back exactly as typed. This setting alone changes
# nothing - `{{ARGS}}` is still textually substituted unquoted wherever a
# recipe still uses it - so every recipe below that only ever took `*ARGS` had
# its body's trailing `{{ARGS}}` changed to `"$@"` to actually pick this up.
#
# A recipe with named parameters *before* `*ARGS` (the `launch-*`, `rpcs3-*`
# and `pcsx2-*` families, plus `scripted-sim`, `scripted-emu`, `frame-shot`
# and `capture-ghidra-state`) was deliberately left alone: `$1` there is the
# first *declared parameter*, not the first extra argument, so `"$@"` would
# re-include whatever that parameter already names via its own `{{...}}`
# interpolation. Fixing those needs a `#!/usr/bin/env sh` body plus `shift N`
# for the right `N`, which is a real edit with no WAD-path bug behind it -
# none of them take a WAD-internal path, only an image, a scenario file or an
# emulator flag - so it stays undone here.
set positional-arguments

default: check

# Emulator binaries used by the launch-* recipes below. Override per-invocation with
# `just --set ppsspp_bin /path/to/PPSSPP launch-pulse-psp`, or export PPSSPP_BIN etc.
ppsspp_bin := env_var_or_default("PPSSPP_BIN", "PPSSPPSDL")
pcsx2_bin := env_var_or_default("PCSX2_BIN", "pcsx2")
rpcs3_bin := env_var_or_default("RPCS3_BIN", "rpcs3")
mangohud_bin := env_var_or_default("MANGOHUD_BIN", "mangohud")

# The PSP disc a scripted run reads its track and handling out of, and the ISO
# PPSSPP itself wants (it will not open a CHD). Override with OAG_IMAGE / OAG_ISO.
#
# Deliberately left on USA, unlike `just play`'s own default - ADR-0048 makes
# psp-pulse-eu the Ghidra *static-analysis* target of record, but every
# scripted trace/watch recipe below (`trace-compare`, `pads`) pairs this image
# with `scripts/psp-*.py`'s hardcoded memory addresses, which were all derived
# by breakpointing the *running* USA build. Flipping this without re-deriving
# every one of those addresses against a live EU session would have a scripted
# run read the wrong offsets silently, not fail loudly - worse than leaving the
# two policies decoupled and documented as such.
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
#
# `[parallel]` so the script checks (about four seconds of Python in all) and
# `rustfmt` run while cargo is compiling rather than after it. `lint` and `test`
# still take cargo's own build-directory lock one after the other, in whichever
# order they get there, so nothing here changes what either asserts. Output of
# the recipes interleaves; a failure names its recipe.
[parallel]
check: fmt-check lint test check-docs check-deps check-unused-deps check-determinism check-size check-title-branching check-title-reach check-names check-captures check-handover check-link-data check-strings check-just-args check-status

# Documentation is a deliverable, so its links are checked like any other build output
check-docs:
    python3 scripts/check-doc-links.py

# Every assets/ui/menu.toml row/title needs a string_id and real English text
# in assets/ui/strings/english.toml, unless it predates the rule (a ratchet,
# see scripts/check-strings.py's own module doc for the baseline it freezes).
check-strings:
    python3 scripts/check-strings.py

# Fixture for check-just-args.py, never called directly: proves a `*ARGS`
# recipe that references `"$@"` hands its arguments back exactly as typed -
# backslashes, spaces and all - rather than mangled the way an unquoted
# `{{ARGS}}` interpolation is. See the `positional-arguments` comment at the
# top of this file. `_`-prefixed, so `just --list` does not offer it.
_just-args-echo *ARGS:
    printf '%s\n' "$@"

# Regression test for the backslash-eating bug the `positional-arguments`
# comment at the top of this file describes: every `*ARGS` recipe that was
# switched to `"$@"` must hand its arguments back unmangled. No disc data
# needed, so this runs in the plain gate.
check-just-args:
    python3 scripts/check-just-args.py

# Proves scripts/link-worktree-data.sh's own guarantees (never clobber a real
# file or directory, still reach an untracked entry beside a tracked one)
# against a throwaway temp directory - no worktree or real data/ needed, so
# this runs anywhere, including the main checkout.
check-link-data:
    bash scripts/link-worktree-data.sh --self-check

# HANDOVER.md must stay under 256 KiB - the Read tool's own ceiling, not a style preference
check-handover:
    python3 scripts/check-handover-size.py

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo nextest run --workspace

# Behavioural / ground-truth tests that need data/images populated.
#
# `--no-fail-fast` because this run's job is to produce a *list*. Without it the
# sweep stops at the first failure - and the two `oag-formats` ones sort early
# enough to stop it at 138 of 3,300, which reads as a far worse tree than it is
# and hides the rest entirely. Six pre-existing reds sat documented as one
# because of exactly that; see HANDOVER.md's "Read this first".
#
# The run is teed to `target/test-data.log` and handed to `check-test-budget`,
# which is the only place the suite's own cost is measured - see that script for
# why a per-test ceiling catches what a total cannot.
#
# Both exit codes are kept, and a red test wins: the budget report is worth
# printing either way - a run that failed still measured every test that
# passed - but "the suite is red" is the more urgent of the two things to say,
# so it is the one this recipe exits on.
test-data:
    #!/usr/bin/env bash
    set -uo pipefail
    mkdir -p target
    cargo nextest run --workspace --run-ignored all --no-fail-fast 2>&1 \
        | tee target/test-data.log
    tests=${PIPESTATUS[0]}
    budget=0
    just check-test-budget || budget=$?
    [ "$tests" -ne 0 ] && exit "$tests"
    exit "$budget"

# The ratchet on how long `just test-data` takes, per test and in total.
#
# Not in the default `just` gate: that runs `test`, which skips every
# `#[ignore]`d test, so there would be nothing to measure.
check-test-budget log="target/test-data.log":
    python3 scripts/check-test-budget.py {{log}}

# Symlinks the main checkout's data/ subdirectories into this worktree.
# `data/` is gitignored, so it does not travel into a `git worktree add` and
# every disc-backed test skips here until this is run. No-op in the main
# checkout, and it never replaces a real file or directory - a directory
# that is only partly tracked (data/traces, data/keys) is descended into
# rather than skipped whole, so its untracked entries still get linked.
link-data:
    bash scripts/link-worktree-data.sh

# Fetches the FidelityFX SDK shader sources the FSR ports are diffed against,
# into ~/.cache/oag-fsr and never into the repository. Nothing is vendored:
# ADR-0012 chose a WGSL port over linking the SDK, and the port's only defence
# against drifting from upstream is that it stays diffable against a pinned
# tag. See docs/rendering/fsr3.md for the pin and why it is v1.1.4.
fsr-reference:
    bash scripts/fetch-fsr-reference.sh

build:
    cargo build --workspace

# A release `oag-game` tuned for a CPU tier, in its own target directory so the
# baseline build is untouched: `just build-cpu` for `x86-64-v3` (AVX2, FMA,
# BMI2 - any Intel since Haswell, any Zen), `just build-cpu znver2` for the
# Steam Deck's own core. The simulation hashes are unchanged under either -
# see docs/tools/packaging.md, "A CPU-tier build", for what was measured.
build-cpu cpu="x86-64-v3" *ARGS:
    CARGO_TARGET_DIR=target/cpu-{{cpu}} RUSTFLAGS="-C target-cpu={{cpu}}" cargo build --release -p oag-game {{ARGS}}

docs:
    cargo doc --workspace --no-deps --document-private-items

# Workspace-internal crate dependency graph (normal deps only - dev-deps like
# oag-testdata's fan-out into nearly every crate are excluded) as SVG. Built
# straight from `cargo metadata` and `dot` (graphviz), no cargo-depgraph
# install needed. A PNG is a one-liner from the SVG if a raster is ever
# wanted: `dot -Tpng target/deps.dot -o deps.png` (this recipe keeps the
# intermediate .dot file next to the .svg for that).
crate-graph out="target/deps.svg":
    #!/usr/bin/env bash
    set -euo pipefail
    out={{out}}
    mkdir -p "$(dirname "$out")"
    dotfile="${out%.svg}.dot"
    {
        echo 'digraph oag {'
        echo '  rankdir=LR; node [shape=box, fontname="Helvetica", fontsize=11];'
        cargo metadata --format-version=1 --no-deps | jq -r '
          .packages as $pkgs
          | ($pkgs | map(.name)) as $names
          | .packages[] | .name as $from | .dependencies[]
          | select(.kind == null)
          | select(.name as $d | $names | index($d))
          | "  \"\($from)\" -> \"\(.name)\";"
        ' | sort -u
        echo '}'
    } > "$dotfile"
    dot -Tsvg "$dotfile" -o "$out"
    echo "wrote $out (and $dotfile)"

unpack *ARGS:
    cargo run -q -p oag-tools --bin oag-unpack -- "$@"

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
# `pulse-ps2` swaps in the PS2 disc (EU). `pure`/`pure-eu`/`pure-psp-eu`
# swaps in the Pure EU PSP disc, `pure-usa`/`pure-psp-usa` the Pure USA one -
# oag-game does not play Pure yet, so these fail fast with a named
# `WrongTitle` error rather than booting a race; they exist so that error
# path stays reachable by keyword, ready for whenever Pure becomes playable
# (roadmap M8).
#
# `2048`/`wipeout2048`/`vita` swaps in Wipeout 2048:
#
#     just play 2048
#
# **This one names a directory, not an image**: 2048 ships as a `.pkg` and what
# the recipe reads is the decrypted package extracted under
# `data/extracted/vita/PCSF00007`. The recipe says so if it is not there.
#
# **`--race` is no longer required** (2026-09-21). The boot walks 2048's own
# declared chain - `Boot Connect`, the Studio Liverpool card for its authored
# 4.0 s, `Boot Intro Movie`, the save check, `TitleScreen` - and lands on the
# `GameModeChoice` touch grid, drawn off the disc's own `<TouchButton>`s in the
# disc's own font; see docs/formats/2048-frontend.md. Two things to know:
# `intro.mp4` has no demuxer yet, so the movie screen is black and waits for a
# button (any of the six the XML authors - press X), and the grids answer the
# pad (left/right, X) and the mouse alike. `--race` still skips straight to a
# race. The circuit and the craft both draw - `.rcsmodel`'s normal, diffuse
# UV, material table and per-submesh binding are all read, and `PVRTII4BPP`
# (almost every 2048 texture's pixel format) decodes - see
# docs/formats/2048-status.md. Still missing: `.envsettings` (lighting/fog/
# bloom fall back to the stand-in rig) and `track.pvs` (every chunk draws).
#
# `hd`/`fury`/`hd-ps3-eu` swaps in Wipeout HD / Fury:
#
#     just play hd
#
# **Only the decrypted image works**, so the keyword names
# `hdfury-ps3-eu-dec.iso` and never the `.iso` beside it; a PS3 disc has to be
# layer-1 decrypted before a single `.psarc` header reads. `just ps3iso decrypt`
# does it, and docs/formats/ps3-disc.md is why. The recipe says so rather than
# letting the archive open fail with something obscure.
#
# **`--race` is no longer needed**, and this paragraph used to say the opposite
# twice over. The front end refused HD by name while `oag_hd::TITLE` set
# `front_end: None`; ADR-0025 replaced that with a boot chain that carries its
# provenance, and the boot now walks HD's own declared order, plays its logo
# reel and opens the menus - in HD's 1920x1080 grid, in its own `helv.fnt`.
# **The order is no longer only declared, either**: three cold boots on RPCS3
# (2026-09-05) walked all eight steps and the report now says `Measured`, not
# `Declared` - see docs/formats/hd-frontend.md. That capture watched the
# screen order, not the audio; which of HD's front-end music cuts the
# original actually plays is still unobserved.
#
# Six of the eight declared steps are dialogs this build cannot drive, so the
# chain it walks is the picker and `Studio Logo`; running out of it opens this
# build's own menu tree, and the report names the screen it ran out on. See
# docs/formats/hd-frontend.md.
#
# **`--race` still skips straight to one**, verified on this disc: Talon's Junction with
# Assegai, which is `oag_hd::race::DEFAULTS` and the same circuit as Pulse's
# `16_Track` in the same coordinates, so an HD run is directly comparable with
# an existing Pulse capture. The circuit and the ships both draw, textured -
# `.rcsmodel`'s geometry and `.gtf` (HD's own PS3 texture container) are both
# read - see docs/formats/hd-status.md.
#
# `omega`/`omega-collection`/`omega-ps4-eu` swaps in Wipeout: Omega
# Collection:
#
#     just play omega
#
# **Ships as a base `.pkg` plus a mandatory day-one patch**, two sibling
# directories the recipe reads together (`data/extracted/ps4/omega-eu` and
# `-patch`); the recipe names the extraction step if the patch is missing.
# Its front end is HD's own `PI001` plugin carried forward - see
# docs/formats/omega-frontend.md - so the boot chain and menu layout are the
# same shape HD's are, walked with `Provenance::Declared` rather than
# `Measured`: no PS4 emulator exists in this project's toolchain to watch a
# real boot; `crates/omega/tests/omega_title_ground_truth.rs` names which of
# the nine archives serves each front-end file, as a tripwire rather than a
# log line.
#
# **`--screenshot` alone stops on `Language Selection`** - German is not
# offered (its plugin's own `Definition.xml` is one of the disc's own
# all-zero copies, `docs/formats/omega-frontend.md`'s census already found
# this for every title), so the picker is shown and the sequence waits for a
# choice the same way it would for a real player. `--menu-page main` draws
# this build's own reimplemented main menu instead - the disc's own `Main
# Menu` screen is never parsed for an HD-idiom front end at all, on HD's own
# terms (`crates/game/src/boot/includes.rs`'s module doc): only a title whose
# `FrontEnd::touch` is `Some` (2048) has its `LoadXML` includes followed.
# `--menu-page grid-select`/`cell-select` draw the campaign screens off
# Omega's own nineteen grids (`oag_omega::campaign::GRID_COUNT`, HD has
# sixteen) - twelve of nineteen parse this lane, the rest failing per-row
# the same tolerant way a bad HD grid already does.
#
# **A race starts**, on a whole extraction only (2026-09-29, `omega-race`):
# real spline, collision, hull and textured circuit geometry, see
# docs/formats/omega-status.md. `data/extracted/ps4` is the whole extraction
# since 2026-09-29; a short-read copy (the old one is `data/extracted/ps4.bak`)
# fails a race on `tech_de_ra\track.vex` having no `WO Track` node. To read a
# different pair of directories, set `OAG_OMEGA_SRC`:
#
#     OAG_OMEGA_SRC=data/extracted/ps4.bak \
#         just play omega --race --no-audio --screenshot out.png
#
# A debug build spends about three minutes decoding the circuit's ~460 textures.
# The node table is read (scenery is placed by its bind matrix); not read:
# the skeleton and clip, lightmaps, the sound banks.
#
# RACE REMIX itself - track from one title, craft from another, picked live
# ([ADR-0034](docs/architecture/adr/0034-a-race-may-open-two-titles-at-once.md)) -
# is not a `play` keyword: it needs no source of its own, only a normal boot
# with more than one title's disc on the search path. `just play` (any
# keyword) reaches it the same way a player would - START at the title
# screen, then down to REMIX. The backend alone, no menu, is
# `--race --craft-source <source>` on any keyword, e.g.
# `just play 2048 --race --craft-source data/images/pure-psp-eu.chd`.
#
# A bare flag (`--ticks 5`) still gets the EU default; anything
# else (a path or `image:entry` spec) is passed straight through unchanged,
# so `just play data/images/foo.chd --ticks 5` still works.
play *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    args=("$@")
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
        pure|pure-psp-eu|pure-eu)
            args=("data/images/pure-psp-eu.chd" "${args[@]:1}")
            ;;
        pure-psp-usa|pure-usa)
            args=("data/images/pure-psp-usa.chd" "${args[@]:1}")
            ;;
        2048|wipeout2048|vita)
            src="data/extracted/vita/PCSF00007"
            # 2048 ships as a `.pkg`, not a disc, so this recipe reads a
            # *directory* where every other keyword reads an image - the
            # decrypted package, extracted. Nothing under `data/` is shipped;
            # name the step that produces it rather than letting the archive
            # search fail with "no archive named PSP2/data.psarc".
            if [ ! -f "$src/base/PSP2/data.psarc" ]; then
                echo "$src/base/PSP2/data.psarc is missing." >&2
                if [ -f "data/images/2048-vita-eu.pkg" ]; then
                    echo "The package is there but not extracted. Decrypt and" >&2
                    echo "unpack it into $src/base first - see" >&2
                    echo "docs/formats/2048-status.md and data/README.md." >&2
                else
                    echo "No 2048 package at all under data/images/; this recipe" >&2
                    echo "reads your own copy and none is shipped. See data/README.md." >&2
                fi
                exit 1
            fi
            args=("$src" "${args[@]:1}")
            ;;
        omega|omega-collection|omega-ps4-eu)
            src="${OAG_OMEGA_SRC:-data/extracted/ps4}"
            # Omega ships as a base `.pkg` plus a mandatory day-one patch, two
            # sibling directories under `$src` - `omega-eu` and
            # `omega-eu-patch`. The patch is not optional: its own
            # `uroot/data09.psarc` is the only archive carrying a `skin.xml`
            # at all (`crates/omega/src/lib.rs`'s own `DATA_CANDIDATES` doc),
            # so a base-only extract refuses to open rather than booting a
            # front end with no menu XML in it.
            if ! find "$src" -maxdepth 3 -path '*/uroot/data09.psarc' 2>/dev/null | grep -q .; then
                echo "$src has no */uroot/data09.psarc (the patch's front end)." >&2
                if [ -f "data/images/omega-ps4-eu.pkg" ] || [ -f "data/images/omega-ps4-eu-patch.pkg" ]; then
                    echo "The package(s) are there but not extracted. Decrypt and" >&2
                    echo "unpack both into $src/omega-eu and $src/omega-eu-patch -" >&2
                    echo "see docs/reverse-engineering/source-images.md and data/README.md." >&2
                else
                    echo "No Omega package at all under data/images/; this recipe" >&2
                    echo "reads your own copy and none is shipped. See data/README.md." >&2
                fi
                exit 1
            fi
            args=("$src" "${args[@]:1}")
            ;;
        hd|fury|hd-ps3-eu)
            img="data/images/hdfury-ps3-eu-dec.iso"
            # A PS3 disc reads as noise until it is layer-1 decrypted, and the
            # encrypted image sits right beside the decrypted one under the same
            # stem - so the likely mistake is having only the wrong one. Name the
            # fix rather than letting the archive open fail on a bad header.
            if [ ! -f "$img" ]; then
                echo "$img is missing." >&2
                if [ -f "data/images/hdfury-ps3-eu.iso" ]; then
                    echo "The encrypted image is there. Decrypt it first, with" >&2
                    echo "the disc's own .dkey as the key:" >&2
                    echo "  just ps3iso decrypt data/images/hdfury-ps3-eu.iso \\" >&2
                    echo "      \"\$(cat data/images/hdfury-ps3-eu.dkey)\" $img" >&2
                    echo "See docs/formats/ps3-disc.md." >&2
                else
                    echo "No HD image at all under data/images/; this recipe reads" >&2
                    echo "your own disc and none is shipped. See data/README.md." >&2
                fi
                exit 1
            fi
            args=("$img" "${args[@]:1}")
            ;;
    esac
    ${OAG_PLAY_WRAPPER:-} cargo run --release -p oag-game {{native_video_flags}} -- "${args[@]}"

# Start with no image named: survey the search path and choose one on screen
#
# The other half of `play`, which names an image for you. This one names none,
# reads every `.chd` and `.iso` under `data/images/` - and beside the AppImage,
# and under `~/.local/share/oag/images` - and puts them on screen with what each
# one turned out to be. Up and down choose, Enter or X boots, Escape quits.
#
# `--launcher` is passed explicitly rather than relying on the bare no-argument
# path, so this recipe shows the screen even for a developer who has `$OAG_IMAGE`
# set or a `[source] image` in their `settings.toml`.
#
# An image that will not open is listed and marked rather than hidden - the
# encrypted `hdfury-ps3-eu.iso` sitting beside its decrypted twin is what that
# is for; see `just ps3iso decrypt` and docs/formats/ps3-disc.md.
launch *ARGS:
    cargo run --release -p oag-game {{native_video_flags}} -- --launcher "$@"

# Same as `play`, but wrapped with MangoHud for an FPS/frametime overlay. Needs
# `mangohud` installed; override the binary with `MANGOHUD_BIN`.
play-mangohud *ARGS:
    OAG_PLAY_WRAPPER={{mangohud_bin}} just play "$@"

# Capture the boot sequence, the menu and the race it launches, without a display
#
# `--no-audio` on every leg: a machine with a real device otherwise paces the
# boot movie against wall-clock time (ADR-0019), and this headless run
# finishes in far less real time than the movie takes to play. The two
# `--until` legs below both hold or press a skip button, so they reach their
# target long before that would matter - but a machine slow enough to miss the
# skip's own timing window would silently start depending on it, so this pins
# every leg to the tick clock rather than to whatever audio happens to be
# attached. See `crates/game/src/capture.rs`'s `MAX_TICKS` doc comment.
play-screenshots out="/tmp":
    #!/usr/bin/env bash
    set -euo pipefail
    cargo run -q --release -p oag-game {{native_video_flags}} -- --screenshot "{{out}}/oag-intro.png" --ticks 400 --no-audio
    cargo run -q --release -p oag-game {{native_video_flags}} -- --screenshot "{{out}}/oag-language.png" \
        --until "Language Selection" --hold start --no-audio
    # Launch Game hands off to a race, so this one is a ship on a track that was
    # reached through the menus. `--press` pulses cross on alternating ticks, which
    # is what picks the language and then leaves the throttle on half of them.
    cargo run -q --release -p oag-game {{native_video_flags}} -- --screenshot "{{out}}/oag-launch.png" \
        --until "Launch Game" --press start,cross --ticks 60 --no-audio

# One race frame per upscaler, plus a supersampled reference, for judging a
# resampler by looking rather than by counting. An aggregate statistic ranks a
# sharpener below a blur every time - see docs/rendering/README.md, "Measuring a
# renderer change" - so this recipe produces images and deliberately no numbers.
#
# `--presented` is what puts the render scale, the upscaler and the grade in the
# way; without it a capture never reaches the blit and every image would be
# identical. The scale is an argument because a magnifier can only be judged
# where it is magnifying.
#
# `fsr3` is temporal, which makes its image mean something different from the
# other two: `off` and `fsr1` resolve one frame, while `fsr3` has been
# accumulating since the race began and the `--ticks` count is therefore part of
# what it is showing. On an adapter with no compute shaders it degrades to
# `fsr1` and the two images are identical - that is the fallback working, not
# the capture failing. See docs/rendering/fsr3.md.
compare-upscalers image scale="50" out="/tmp":
    #!/usr/bin/env bash
    set -euo pipefail
    for upscaler in off fsr1 fsr3; do
        cargo run -q --release -p oag-game -- "{{image}}" --race \
            --screenshot "{{out}}/oag-upscale-{{scale}}-$upscaler.png" \
            --ticks 300 --hold cross --presented \
            --render-scale "{{scale}}" --reconstruction "$upscaler"
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
    ./scripts/build-appimage.sh "$@"

# The Android APK (NativeActivity, arm64): target/apk/OpenAntiGrav-<v>-android-arm64.apk.
# Needs cargo-ndk, the aarch64-linux-android target and an SDK + NDK; see
# docs/tools/android.md.
apk *ARGS:
    ./scripts/build-apk.sh {{ARGS}}

# The same package built in Debian bookworm, which is what a Steam Deck needs: a
# native build links libm symbols an older glibc does not have and refuses to
# start there. Needs podman or docker; the image is built once from
# packaging/appimage/Containerfile. Slower, so `just appimage` stays the fast
# path for a local run. See docs/tools/packaging.md#glibc.

# Package the AppImage against an older glibc, so it also runs on a Steam Deck
appimage-portable *ARGS:
    ./scripts/build-appimage.sh --container "$@"

# The Steam Deck's AppImage: the portable container build, compiled for the
# Deck's own Zen 2 core (`-C target-cpu=znver2`), written as
# data/appimage/OpenAntiGrav-x86_64-steamdeck.AppImage. It refuses to start on a
# CPU without AVX2/FMA/BMI2, which is why `appimage-portable` stays the
# baseline. See docs/tools/packaging.md, "A CPU-tier build".
appimage-deck *ARGS:
    ./scripts/build-appimage.sh --container --target-cpu znver2 --out data/appimage/OpenAntiGrav-x86_64-steamdeck.AppImage "$@"

# Install a .desktop entry and icon for this checkout into ~/.local/share, so a
# Wayland taskbar has something to resolve the window's app_id against - see
# docs/tools/packaging.md and scripts/install-desktop-file.sh
install-desktop-file *ARGS:
    ./scripts/install-desktop-file.sh "$@"

# Build the portable AppImage and copy it onto the Steam Deck (or any host
# reachable over ssh); no game data - that is `just push-data deck`. See
# docs/tools/packaging.md#steam-deck and scripts/deploy-to-deck.sh
deploy-deck *ARGS:
    ./scripts/deploy-to-deck.sh "$@"

# Build the APK and `adb install -r` it; no game data - that is
# `just push-data android`. See docs/tools/android.md and
# scripts/deploy-to-android.sh
deploy-android *ARGS:
    ./scripts/deploy-to-android.sh "$@"

# Start the app on an adb device (asks which when several are connected) and
# follow its log; --no-logs, --stop (cold start), --serial S. See
# scripts/launch-android.sh
launch-android *ARGS:
    ./scripts/launch-android.sh "$@"

# Follow the app's log (`adb logcat -s oag`) on the picked adb device; --clear,
# --dump, `-- ARGS` for another filter. See scripts/logcat-android.sh
logcat-android *ARGS:
    ./scripts/logcat-android.sh "$@"

# Pick game data (disc images, DLC, unpacked 2048/Omega packages, Pure's DLC
# keys) in an fzf multiselect, each row marked as already on the device or not,
# and copy it: `just push-data deck [user@host]` or `just push-data android`.
# Globs instead of the picker: `just push-data android 'images/pulse-psp-*'`.
# See scripts/push-game-data.sh
push-data *ARGS:
    ./scripts/push-game-data.sh "$@"

# Run the tick loop with no renderer, no window and no disc, and print its hash
headless-sim *ARGS:
    cargo run -q -p oag-raceplay --bin oag-headless-sim -- "$@"

# View assets straight from a disc image
view *ARGS:
    cargo run -q -p oag-view -- "$@"

# Read a WAD archive
wad *ARGS:
    cargo run -q -p oag-tools --bin oag-wad -- "$@"

# Rebuild the WAD entry-name candidate list from a disc image
mine-names image:
    python3 scripts/mine-names.py {{image}}

# Inspect or decrypt a PS3 disc image: map|extract|oracle|decrypt. `map` and a
# plain-region `extract` need no key; `decrypt` needs the disc's own .dkey,
# which is never committed. See docs/formats/ps3-disc.md.
ps3iso *ARGS:
    python3 scripts/ps3iso.py "$@"

# Read a PS3 .psarc archive in place inside a decrypted disc image:
# info|list|cat|extract. See docs/formats/psarc.md.
psarc *ARGS:
    python3 scripts/psarc.py "$@"

# Classify one set of PSARC archives against another (new / replaced / identical,
# grouped by asset family): a patch or DLC pack against its base. Names, sizes and
# MD5s only. `just psarc-diff --other <patch.psarc>... --base <base.psarc>... --tsv out.tsv`
# See docs/formats/patches.md.
psarc-diff *ARGS:
    cargo run -q -p oag-tools --bin oag-psarc-diff -- {{ARGS}}

# Re-derive every number on docs/formats/hd-status.md from the discs themselves.
# The second argument is optional and only feeds the Pulse control column.
hd-survey hd="data/images/hdfury-ps3-eu-dec.iso" pulse="":
    python3 scripts/hd-survey.py {{hd}} {{pulse}}

# Extract a raw ISO from a CHD, which is what the emulators want
extract-iso image="data/images/pulse-psp-usa.chd" out="data/cache/pulse-psp-usa.iso":
    chdman extractdvd -i {{image}} -o {{out}} -f

# Extract every CHD in data/images/ to data/cache/, which is also what the
# ground-truth tests want.
#
# `oag_testdata::image` hands a test `data/cache/<stem>.iso` in place of
# `data/images/<stem>.chd` whenever the extract exists and is no older than the
# image - a CHD is compressed, so every read of one decompresses a hunk, and
# `Archives::open` measures 31 ms against the CHD against 0.05 ms against the
# extract. Entirely optional: with no extract the tests read the CHD and are
# only slower.
#
# It costs disk. Budget roughly 2x the CHD per disc, and note the PS2 image is
# 3.7 GB compressed. Delete anything under data/cache/ to reclaim it; nothing
# breaks.
extract-isos:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p data/cache
    for chd in data/images/*.chd; do
        out="data/cache/$(basename "${chd%.chd}").iso"
        if [ -f "$out" ] && [ "$out" -nt "$chd" ]; then
            echo "up to date: $out"
            continue
        fi
        echo "extracting $chd -> $out"
        chdman extractdvd -i "$chd" -o "$out" -f
    done

# Run an original disc image in its platform's emulator, for reference and
# behavioural ground-truth comparison. One recipe per (to be) supported title, see
# data/README.md for expected image names. Both PPSSPP and PCSX2 read .chd directly,
# no extract-iso needed. Override the emulator binary with PPSSPP_BIN / PCSX2_BIN /
# RPCS3_BIN, e.g. `PPSSPP_BIN=PPSSPP just launch-pulse-psp`.

# Wipeout Pulse (PSP) in PPSSPP
launch-pulse-psp image="data/images/pulse-psp-eu.chd" *ARGS:
    {{ppsspp_bin}} {{image}} {{ARGS}}

# Wipeout Pulse (PS2) in PCSX2
launch-pulse-ps2 image="data/images/pulse-ps2-eu.chd" *ARGS:
    {{pcsx2_bin}} -fullscreen {{image}} {{ARGS}}

# Wipeout Pure (PSP) in PPSSPP
launch-pure-psp image="data/images/pure-psp-eu.chd" *ARGS:
    {{ppsspp_bin}} {{image}} {{ARGS}}

# WipEout HD / Fury (PS3) in RPCS3. It has to be the layer-1 DECRYPTED image -
# the encrypted twin beside it reads as noise and RPCS3 will not boot it; see
# `just ps3iso decrypt` and docs/formats/ps3-disc.md.
launch-hdfury-ps3 image="data/images/hdfury-ps3-eu-dec.iso" *ARGS:
    {{rpcs3_bin}} {{image}} {{ARGS}}

# The same, with no window at all: `--headless` runs the null renderer, so the
# game ticks and nothing is drawn. This is the form a script drives, together
# with the GDB stub on 127.0.0.1:2345 that a stock config.yml already enables -
# see docs/reverse-engineering/rpcs3-debugger.md, whose trap list is the
# difference between a session that works and one that invents its results.
# `--input-config oag` is what makes a virtual pad reachable; without it RPCS3
# binds a keyboard that headless has no window to feed. `just rpcs3-preflight`
# says whether either half is missing. RPCS3 is single-instance: kill a previous
# run before starting this one.
launch-hdfury-ps3-headless image="data/images/hdfury-ps3-eu-dec.iso" *ARGS:
    {{rpcs3_bin}} --headless --no-gui --input-config oag {{image}} {{ARGS}}

# Can a script press a button on this machine? RPCS3 has no input API, so the
# only route is a uinput virtual pad, and two things have to be true for one to
# reach the game: `/dev/uinput` has to be openable (root-only until a udev rule
# says otherwise) and RPCS3 has to have an input profile naming the evdev
# handler (without one it binds a keyboard and swallows every press in silence).
# This checks both and prints the exact command for whichever is wrong - run it
# before blaming the emulator for ignoring input.
rpcs3-preflight:
    uv run --with evdev scripts/rpcs3_pad.py preflight

# Write the input profile `launch-hdfury-ps3-headless` selects. Three lines
# naming the evdev handler and the virtual pad; RPCS3 fills the rest from its
# own defaults.
rpcs3-input-config:
    uv run --with evdev scripts/rpcs3_pad.py install-config

# Create that virtual pad and hold it open, so another terminal can watch RPCS3
# pick it up. The pad must exist BEFORE the emulator starts; RPCS3 binds pads
# when it enumerates devices, and says so with `Evdev device 0 connected`.
rpcs3-pad seconds="30":
    uv run --with evdev scripts/rpcs3_pad.py pad {{seconds}}

# Start the virtual display everything below needs. It listens on TCP and is
# addressed as 127.0.0.1:77, because a sandboxed session may not be able to
# write /tmp/.X11-unix and then the unix socket never appears at all.
rpcs3-display:
    python3 scripts/rpcs3-drive.py display

# Stop that display - always run this when the whole session is done. Unlike
# pcsx2-stop this does not also stop RPCS3 itself: boot/race/shot/capture/
# browse/record already do that on the way out. Only tears down a display
# this tooling started; one that was already there is left alone.
rpcs3-stop:
    python3 scripts/rpcs3-drive.py stop

# Boot WipEout HD and hold it at the Main Menu, on that display, with no window
# on anyone's desktop. `--headless` cannot get here - it stalls inside
# cellGameDataCheck with the null renderer; see
# docs/reverse-engineering/rpcs3-debugger.md.
rpcs3-boot image="data/images/hdfury-ps3-eu-dec.iso" *ARGS:
    uv run --with evdev python3 scripts/rpcs3-drive.py --image {{image}} boot {{ARGS}}

# Watch the boot chain screen by screen and photograph each step - the capture
# that made `oag_hd::frontend::BOOT` a measurement rather than a declaration.
#
#     mv ~/.config/rpcs3/dev_hdd0/home/00000001/savedata/BCES00664-AUTO- /somewhere
#     just rpcs3-bootchain
#     mv /somewhere/BCES00664-AUTO- ~/.config/rpcs3/dev_hdd0/home/00000001/savedata/
#
# **The move is not optional.** With a save present HD skips `FirstPlay`, so the
# chain you watch is seven steps rather than eight and looks complete.
# See docs/formats/hd-frontend.md.
rpcs3-bootchain image="data/images/hdfury-ps3-eu-dec.iso" *ARGS:
    uv run --with evdev python3 scripts/rpcs3-drive.py --image {{image}} bootchain {{ARGS}}

# The whole thing: boot, walk the front end into a race (six taps of cross),
# hold thrust, and screenshot it. Run from the MAIN checkout - `data/` is
# gitignored and absent from a worktree.
rpcs3-race image="data/images/hdfury-ps3-eu-dec.iso" *ARGS:
    uv run --with evdev python3 scripts/rpcs3-drive.py --image {{image}} race --shots {{ARGS}}

# The same, capturing the driven part as video through RPCS3's own recorder.
# HD's HUD is in the frame - lap, position, lap time, shield and km/h - so a
# recording reads as a 30 Hz trace and not just a picture. The file lands in
# ~/.config/rpcs3/recordings/<TITLE_ID>/, a subdirectory per title.
rpcs3-record image="data/images/hdfury-ps3-eu-dec.iso" *ARGS:
    uv run --with evdev python3 scripts/rpcs3-drive.py --image {{image}} record {{ARGS}}

# Read HD's speed readout out of a recording and print it as CSV. Sparse on
# purpose: the HUD is alpha-blended, so an unreadable frame is a gap and never
# a guess. Check the coverage line it prints on stderr before leaning on the
# numbers - about 8 % on Talon's Junction, which is the brightest circuit there
# is. See docs/reverse-engineering/rpcs3-debugger.md.
rpcs3-speed video *ARGS:
    uv run --with numpy python3 scripts/rpcs3_hud.py read {{video}} {{ARGS}}

# Wipeout Pulse (PS2) with no window and no human: Xvfb, PCSX2's OpenGL
# renderer, PINE for memory and savestates, XTEST for buttons. Uses a data path
# of its own under ~/.cache/oag-pcsx2, so the user's ~/.config/PCSX2 is never
# touched. See docs/reverse-engineering/pcsx2-debugger.md, whose trap list is
# the one that costs runs - especially the two silent input gates.
pcsx2-boot image="data/images/pulse-ps2-eu.chd" *ARGS:
    uv run --with python-xlib python3 scripts/pcsx2-drive.py boot --image {{image}} {{ARGS}}

# What the harness thinks is up: the virtual display, the process, PINE, the game.
pcsx2-status:
    python3 scripts/pcsx2-drive.py status

# Tap abstract buttons at the running game: `just pcsx2-press cross start`.
pcsx2-press *BUTTONS:
    uv run --with python-xlib python3 scripts/pcsx2-drive.py press {{BUTTONS}}

# A frame the emulator is actually presenting, at GS resolution, into a file of
# your choosing. Not frame-exact - use `pcsx2-frame` for that.
pcsx2-shot path="/tmp/pcsx2-frame.png":
    uv run --with python-xlib python3 scripts/pcsx2-drive.py shot {{path}}

# THE deterministic capture: load a savestate, step exactly `frames` emulated
# frames with `hold` held down, and grab the paused frame. Pass `--from-state
# <slot>` or it is not repeatable - two runs of
#   just pcsx2-frame /tmp/a.png 30 cross --from-state 2
# produce byte-identical PNGs, without it they do not. Stepping is verified
# against a guest frame counter and costs about 0.35 s a frame, deliberately.
pcsx2-frame path="/tmp/pcsx2-frame.png" frames="30" hold="cross" *ARGS:
    uv run --with python-xlib python3 scripts/pcsx2-drive.py frames {{frames}} --hold {{hold}} --shot {{path}} {{ARGS}}

# Save or load a PCSX2 savestate over PINE, by slot: `just pcsx2-state save 2`.
# The file lands in ~/.cache/oag-pcsx2/PCSX2/sstates/, never in the repository.
pcsx2-state action="save" slot="1":
    python3 scripts/pcsx2-drive.py state {{action}} {{slot}}

# Read EE memory at the Ghidra corpus's own addresses, no rebasing:
#   just pcsx2-read 0x002849e8 8
pcsx2-read addr count="1":
    python3 scripts/pcsx2_pine.py read {{addr}} {{count}}

# Stop the headless emulator AND its virtual display. Always run this when
# you are done; a stale process holds the PINE socket and the next launch
# looks broken, and an untorn-down Xvfb :78 just accumulates. Only stops the
# display if this tooling started it - one already there is left alone.
pcsx2-stop:
    python3 scripts/pcsx2-drive.py stop

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
    uv run --with websocket-client python scripts/psp-trace.py "$@"

# Put the original back on the start line, or send an input script at a
# free-running emulator in real time. `just drive restart`, `just drive drive
# --script F`; see docs/reverse-engineering/ppsspp-debugger.md.
[doc("Restart a race, or send a script at a free-running emulator in real time")]
drive *ARGS:
    uv run --with websocket-client python scripts/psp-drive.py "$@"

# Fly the original round a lap, steering off the track's own spline, and write
# down what it pressed. Needs `oag-trace track` output; see
# docs/tools/oag-trace.md#a-whole-lap-and-where-it-came-from.
[doc("Fly the original round a lap and record the input it took to do it")]
autopilot *ARGS:
    uv run --with websocket-client python scripts/psp-autopilot.py "$@"

# Compare our simulation against a captured trace: first divergent tick, and by
# how much. See docs/tools/oag-trace.md.
trace-compare *ARGS:
    cargo run -q -p oag-trace -- "$@"

# Where a track's speed/weapon pads are, with approach points for `just drive
# place`. See docs/tools/frame-compare.md.
#
#   just pads --before 50 > /tmp/pads.csv
#   just drive place --pads-csv /tmp/pads.csv --pad 3 --before 50 --speed 120
[doc("Dump a track's pad trigger volumes, with approach points for teleports")]
pads *ARGS:
    cargo run -q -p oag-trace -- pads --source {{psp_image}} "$@"

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
    ./scripts/build-ghidra-allegrex.sh "$@"

# Build a patched RPCS3 with working GDB write watchpoints (Z2/z2) - stock
# RPCS3 never implemented them, confirmed against upstream master; see
# docs/reverse-engineering/rpcs3-debugger.md ("A patched build exists").
# Clones into data/tools/rpcs3-watchpoints (gitignored, several GiB).
build-rpcs3-watchpoints *ARGS:
    ./scripts/build-rpcs3-watchpoints.sh "$@"

# Build the Emotion Engine (PS2) processor module against the installed
# Ghidra. Stock Ghidra mis-decodes R5900 code as MIPS Release 6; see
# docs/reverse-engineering/toolchain.md. Needs build-allegrex to have run at
# least once - this borrows its Gradle wrapper.
build-emotionengine *ARGS:
    ./scripts/build-ghidra-emotionengine.sh "$@"

# Build the Ps3GhidraScripts extension against the installed Ghidra, with this
# project's PS3 compiler-spec fix (scripts/ghidra-ps3-language/) added as a
# second language. See docs/reverse-engineering/toolchain.md#ps3.
build-ps3-scripts *ARGS:
    ./scripts/build-ghidra-ps3-scripts.sh "$@"

# Build the ELF/PRX loader for Vita binaries against the installed Ghidra.
# Needs build-allegrex to have run at least once - this borrows its Gradle
# wrapper too, same as build-emotionengine. Installation into Ghidra is
# manual; this only builds the zip.
build-vita-loader-redux *ARGS:
    ./scripts/build-vita-loader-redux.sh "$@"

# Build psvpfsparser, which decrypts a PS Vita PKG's PFS layer given its
# zRIF or klicensee. pkg2zip alone only strips the outer AES-CTR layer - see
# data/README.md#vita-pkgs-decrypt-in-three-steps-dataextractedvita-holds-the-result.
build-psvpfstools *ARGS:
    ./scripts/build-psvpfstools.sh "$@"

# Build GhidraOrbis, the loader/analyzer extension for PS4 binaries, against
# the installed Ghidra. Stock Ghidra already decodes the CPU; this adds the
# SELF wrapper and Orbis-specific ELF program header/dynamic tag types. Needs
# build-allegrex to have run at least once - this borrows its Gradle wrapper
# too, same as build-vita-loader-redux and build-emotionengine. Installation
# into Ghidra is manual; this only builds the zip. See
# docs/reverse-engineering/toolchain.md#ps4.
build-ghidra-orbis *ARGS:
    ./scripts/build-ghidra-orbis.sh "$@"

# Add r2 to the unaffected list of Ghidra's PowerPC 64/32-addr compiler spec,
# which PS3 PPU code needs to decompile correctly - r2 is the TOC pointer and a
# call does not clobber it. Edits the Ghidra install, so it needs sudo and does
# not survive a Ghidra upgrade; `--check` reports whether it is still applied.
# Run `scripts/patch-ghidra-ppc-cspec.sh --help` for the rest. `just
# build-ps3-scripts` plus `scripts/import-ps3-eboot.sh --ps3-cspec` is an
# alternative that does not touch the Ghidra install at all.
patch-ppc-cspec *ARGS:
    ./scripts/patch-ghidra-ppc-cspec.sh "$@"

# Resolve the PSP import stubs from the binary's own NID tables.
#
# `boot`/`out` both default to the USA binary's own path/output, unchanged -
# `data/extracted/psp` is not region-tagged and everything downstream
# (`apply-ghidra-names.py`) still pairs the default `out` with `psp-pulse-usa`
# specifically. Point both at an EU BOOT.BIN and `data/ghidra/psp-imports-eu.tsv`
# once one is extracted, to build the EU counterpart ADR-0048 leaves open:
#
#     just resolve-imports boot=data/extracted/psp-eu/PSP_GAME/SYSDIR/BOOT.BIN \
#         out=data/ghidra/psp-imports-eu.tsv
resolve-imports boot="data/extracted/psp/PSP_GAME/SYSDIR/BOOT.BIN" out="data/ghidra/psp-imports.tsv":
    python3 scripts/resolve-psp-imports.py {{boot}} --modules -o {{out}}

# Recover a Vita title's klicensee from its zRIF - see scripts/zrif-to-klicensee.py.
zrif-to-klicensee *ARGS:
    python3 scripts/zrif-to-klicensee.py "$@"

# Decrypt a Vita retail SELF (eboot.bin/.suprx) to the plain ELF-PRX
# VitaLoaderRedux can actually import - see scripts/vita-self-decrypt.py.
vita-self-decrypt *ARGS:
    python3 scripts/vita-self-decrypt.py "$@"

# Apply every documented symbol name to the open Ghidra program
apply-names *ARGS: resolve-imports
    python3 scripts/apply-ghidra-names.py "$@"

# Report names in the open Ghidra program that names.tsv does not sanction
audit-names *ARGS:
    python3 scripts/audit-ghidra-names.py "$@"

# Export a PSP binary's names.tsv as a PPSSPP .sym symbol map - no Ghidra, no
# PPSSPP needed. Load the result in PPSSPP's Debug menu -> Load symbol map.
# See docs/reverse-engineering/ppsspp-symbol-bridge.md.
export-sym binary:
    python3 scripts/export-ppsspp-sym.py {{binary}}

# Harvest the names PPSSPP's own analysis already knows for a booted PSP
# binary, over its websocket debugger - no GUI, works right after boot. Never
# feeds names.tsv; see docs/reverse-engineering/ppsspp-symbol-bridge.md.
harvest-ppsspp-symbols binary port="47800":
    uv run --with websocket-client scripts/harvest-ppsspp-symbols.py {{binary}} --port {{port}}

# Assert no game content (or a reproduction this project itself writes) is tracked
audit-leakage:
    python3 scripts/check-leakage.py

# Assert the three architecture dependency rules from CLAUDE.md still hold
check-deps:
    python3 scripts/check-dependency-rules.py

# Assert no workspace crate declares a dependency its code never uses, or a
# normal dependency only its tests and examples use (`cargo shear` exits 1)
check-unused-deps:
    cargo shear

# Assert no platform transcendental reaches simulation code - determinism.md's
# rule, which was enforced by review alone until an `acos` sat in `oag-ai` for
# months without anything failing
check-determinism:
    python3 scripts/check-transcendentals.py

# A ratchet on file length: nothing new over 1,000 lines, and nothing already
# over it grows. race.rs reached 11,294 lines before anything measured it
check-size:
    python3 scripts/check-file-size.py

# A ratchet on title-identity comparisons (`title.name == oag_hd::TITLE.name`)
# in generic crates: per-title behaviour is `Title` data, ADR-0058. A file may
# drop, never rise, and a new file may have none
check-title-branching:
    python3 scripts/check-title-branching.py

# A ratchet on generic crates naming a title package by path (`oag_hd::hud::ART`):
# they end up depending on `oag-title` only, ADR-0058. A file may drop, never
# rise, and a new file may have none
check-title-reach:
    python3 scripts/check-title-reach.py

# The names.tsv checks `just apply-names` already makes, minus the Ghidra bridge
# it needs to make them. CI has no Ghidra, so without this nothing reads the file
check-names:
    python3 scripts/check-ghidra-names.py

# docs/overview/status.md's RE-coverage table is generated from names.tsv and
# the evidence pages; a row that lands without regenerating it fails here
check-status:
    python3 scripts/gen-re-coverage.py --check

# Regenerate that table after a names.tsv or evidence-page change
gen-status:
    python3 scripts/gen-re-coverage.py

# docs/ghidra/captures/ is what a Ghidra database held that names.tsv does not:
# labels, plate comments, and the structs and prototypes that turned out not to
# exist. Offline, like check-names - it also catches a captured comment citing a
# docs page nobody wrote, which check-docs never sees. See ADR-0047
check-captures:
    python3 scripts/check-ghidra-captures.py

# Re-snapshot every program in the open Ghidra project. Two steps because only
# the first needs Ghidra: run scripts/ghidra/DumpDatabaseState.java from the
# Script Manager (or the MCP bridge) with a scratch directory as its argument,
# then point this at the same directory. Not part of `just check` - it needs a
# running Ghidra, which CI has not got
capture-ghidra-state raw *ARGS:
    python3 scripts/capture-ghidra-state.py {{raw}} {{ARGS}}

# Assert the open PSP program actually had its relocations applied. Deliberately
# not part of `just check`: it needs a running Ghidra, which CI does not have.
# Three sessions each rediscovered the same empty relocation table by hand
# before anything measured it - see docs/ghidra/workflow.md
check-ghidra-import *ARGS:
    python3 scripts/check-ghidra-import.py "$@"

# Cross-build the Windows oag-game (MinGW, else zig) for wine, without running it:
# RELEASE=1 for a release build. See docs/tools/wine.md
wine-build:
    ./scripts/wine-run.sh build

# Build it, then start it under wine in its own prefix (a window on $DISPLAY):
# `just wine-run --dry-run ...`. See docs/tools/wine.md
wine-run *ARGS:
    ./scripts/wine-run.sh run "$@"

# The determinism tests and reports as Windows binaries under wine; every hash
# must equal the committed reference. See docs/tools/wine.md
wine-check:
    ./scripts/wine-run.sh determinism
