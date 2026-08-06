#!/usr/bin/env bash
#
# SessionStart hook for Claude Code on the web.
#
# A cloud session starts from a bare Ubuntu container with rustup on PATH and
# nothing else this repository needs. Without this, the first thing an agent
# tries - `just`, the gate every commit must pass - fails on a missing task
# runner, and `cargo build` fails on a missing system library. This installs
# the four things that gap consists of and warms the build cache.
#
# Deliberately synchronous: the agent's first action in a session is usually to
# build or test, and an async hook would race it.
#
# Idempotent and non-interactive, so `source`ing or re-running it is safe.

set -euo pipefail

# Local checkouts already have a working toolchain; only the cloud container
# needs provisioning. Anyone who wants to run it locally anyway can set
# CLAUDE_CODE_REMOTE=true by hand.
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"

# Everything a SessionStart hook writes to stdout is injected into the session
# context, so progress noise goes to stderr and only the closing summary comes
# back out on the saved copy of stdout.
exec 3>&1 1>&2

step() { printf '\n=== %s ===\n' "$1"; }

as_root() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  else
    sudo -n "$@"
  fi
}

# --- System libraries ------------------------------------------------------
#
# oag-input pulls in gilrs for gamepads, which needs libudev at link time on
# Linux. It is the only non-Rust build dependency in the workspace: wgpu and
# winit resolve their X11/Wayland/Vulkan libraries by dlopen at runtime, so a
# headless container builds and tests the renderer without any of them.
step "system libraries"
if pkg-config --exists libudev; then
  echo "libudev already present"
else
  # Some images carry third-party PPAs this network policy blocks; their index
  # failures are warnings, not a reason to abandon the main archive.
  as_root apt-get update -qq || true
  DEBIAN_FRONTEND=noninteractive as_root apt-get install -y -qq --no-install-recommends \
    pkg-config libudev-dev
fi

# --- Rust toolchain --------------------------------------------------------
#
# rust-toolchain.toml pins the channel and asks for rustfmt and clippy, so any
# cargo invocation inside the repository makes rustup fetch exactly that. Doing
# it here rather than on the agent's first command keeps the sync out of the
# middle of a build log.
step "rust toolchain"
rustup show active-toolchain || true
cargo --version
cargo fmt --version
cargo clippy --version

# --- Cargo-installed tooling -----------------------------------------------
#
# Built from crates.io rather than fetched as release binaries: the egress
# policy for these sessions denies github.com, so `cargo binstall` and the
# get.nexte.st installer both 403. Compiling costs a few minutes once, and the
# container state is cached after this hook completes.
step "cargo tooling"
# `cargo nextest --version` prints a multi-line build report; only the first
# line is a version, and the rest would just be noise in the summary below.
version_of() { "$@" 2>/dev/null | head -n 1; }

if command -v just >/dev/null 2>&1; then
  echo "$(version_of just --version) already installed"
else
  cargo install just --locked
fi

if command -v cargo-nextest >/dev/null 2>&1; then
  echo "$(version_of cargo nextest --version) already installed"
else
  cargo install cargo-nextest --locked
fi

# ~/.cargo/bin is where both land; keep it on PATH for the session even if the
# container's profile does not set it up.
if [ -n "${CLAUDE_ENV_FILE:-}" ]; then
  echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$CLAUDE_ENV_FILE"
fi

# --- Warm the build cache --------------------------------------------------
#
# --all-targets so tests, benches and examples are compiled too; `just lint`
# and `just test` both reuse these artifacts, which is most of what makes the
# first `just` in a session finish in about a minute instead of five.
# Non-fatal: a warm cache is an optimisation, and a session is still usable
# without one.
step "warming the build cache"
cargo fetch --locked || true
cargo build --workspace --all-targets || echo "warm-up build failed; the session is still usable" >&2

# --- Summary ---------------------------------------------------------------
#
# data/ is gitignored and holds user-supplied disc images, so it never travels
# into a cloud session. Saying so up front is cheaper than an agent discovering
# it halfway through a ground-truth test - see the note in CLAUDE.md.
images=0
if [ -d data/images ]; then
  images=$(find data/images -maxdepth 1 -type f ! -name 'README.md' | wc -l)
fi

{
  echo "Environment ready: $(version_of cargo --version), $(version_of just --version), $(version_of cargo nextest --version)."
  echo "Run \`just\` for the full gate (fmt-check + lint + test + check-docs + check-deps)."
  if [ "$images" -eq 0 ]; then
    echo "No disc images under data/images/ - this is expected in a cloud session. Ground-truth tests (\`just test-data\`) and every disc-backed tool (\`just unpack\`, \`just wad\`, \`just view\`) cannot run here; report that rather than skipping the check silently."
  else
    echo "data/images/ holds $images image(s), so disc-backed tools and \`just test-data\` are available."
  fi
} >&3
