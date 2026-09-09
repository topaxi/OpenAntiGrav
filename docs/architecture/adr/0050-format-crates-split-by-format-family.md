# ADR-0050: format crates split by format family, and why that is not the console axis

## Status

Accepted; extends [ADR-0022](0022-title-packages.md).

## Context

`oag-formats` reached 47 top-level modules and roughly 34,700 non-test lines
across 117 files, plus 52 integration test binaries. Twelve crates link it. The
crate that began as "identification and parsing of asset container formats" now
holds, in one namespace, an archive reader, four texture codecs, a scene graph,
a second scene format for a different console generation, four XML table
parsers, two video containers and an AV1 decoder. Nothing about the name says
which of those a contributor should expect to find, and a new parser has no
obvious home beyond "next to the others".

The change was first proposed as a build-time fix - the belief being that
editing `oag-formats` rebuilt most of the workspace. **That belief was measured
and does not hold.** With a warm cache, `cargo nextest run --workspace --no-run`
after touching `crates/formats/src/lib.rs` takes 89s; after touching
`crates/render/src/lib.rs`, which only `oag-view` and `oag-game` depend on, it
takes 88s. The floor is `oag-game` - 106,842 source lines and 76 integration
test binaries - relinking for a change to *any* upstream crate. In a plain
`cargo build --workspace`, `oag-formats` itself compiles in 0.57s and every
crate downstream of it except `oag-game` finishes inside `oag-game`'s own
window, in parallel. A split of `oag-formats` is worth about one second.

This ADR therefore records a decision made on **legibility**, and records the
measurement so that nobody re-derives the build-time argument and reaches the
wrong conclusion from it.

## Decision

**Split `oag-formats` by format family.** Six crates:

| Crate | Owns |
| --- | --- |
| `oag-formats` *(retained)* | Given a blob off a disc: say what it is, get it out of the archive it lives in, decompress it, and undo the console's byte layout. |
| `oag-texture` | Every pixel format the originals ship, decoded to RGBA8888. |
| `oag-vex` | The `.vex` scene tree and everything authored inside or alongside it. |
| `oag-rcs` | The `RCSMODEL` scene: geometry chunks, the material that shades them, the visibility set that culls them, the shader programs they select. |
| `oag-tables` | Every table the titles author as XML: the numbers that tune a race, not the bytes that draw one. |
| `oag-video` | The containers the originals wrap a bitstream in, plus this project's own cache format. |

A crate that cannot be described in one such sentence is the wrong cut; that
test is the reason there are six and not three or twelve.

**No crate is named for a console, and none may be.** ADR-0022 item 5 says "No
`oag-psp` / `oag-ps2` crates, ever", and this decision does not weaken it.

**ADR-0022 item 1 is unchanged.** "How does this byte stream decode?" is still
answered by the file, from the artifact's own version word:
`vex::classes::{V6, V4, V3}` still selects a schema, and `rcsmodel/psp2` stays
*inside* `rcsmodel` for exactly that reason. No format crate depends on a title
crate; the reverse edge exists, so a forward one would be a cycle.

## Alternatives considered

**Split by console** - `oag-psp`, `oag-ps2`, `oag-ps3`, `oag-vita`. Rejected,
and not on preference: ADR-0022 item 5 forbids it in those words. The
dependency graph rejects it independently, which is worth recording because it
means the ADR and the code agree rather than the ADR merely constraining the
code. `bcn` is shared by `gtf` (PS3) and `gxt` (Vita); `byte_order` exists
precisely because two consoles disagree about endianness; and `gxt` and `pvrtc`
form a real cycle. A console cut would force both private modules into public
API *and* split a cycle across a crate boundary.

The falsifiable test that this cut is not the console axis under another name -
five modules a console cut separates and this one keeps together:

| a console cut separates | this cut lands it |
| --- | --- |
| `kdcol` (2048 collision) | with `collision` (Pulse/Pure) - it reuses its types |
| `shadow_occluder` (Pulse PSP *and* 2048) | one place, `oag-vex` |
| `rcsmodel/psp2` (Vita) | inside `rcsmodel` (PS3), selected by version word |
| `effectsettings` (HD and 2048) | with `handling` (Pulse), in `oag-tables` |
| the PSP, PS2, PS3 and Vita texture codecs | one crate, `oag-texture` |

**Split simulation-facing tables from media.** Tempting, because the
determinism boundary already draws that line: `handling`, `weapons`, `track`,
`collision` reach a committed state hash and a texture never does. Rejected on
the measured graph: `collision`, `track`, `pads`, `pvs`, `fog`, `lighting` and
`sound_emitters` all parse `vex::Node`, so any crate holding them sits on top of
`vex` and the boundary buys nothing. Only the XML cluster is genuinely
`vex`-free, and that cluster is `oag-tables`.

**Keep `oag-formats` as a facade that re-exports the new crates.** Rejected as
an end state. It preserves every import unchanged, which is exactly why it
fails: touching a leaf still invalidates the facade and everything downstream,
and the umbrella name goes on meaning nothing. Each split commit re-points its
consumers directly instead.

**Leave it as one crate.** Genuinely defensible - the build-time argument is
gone, and a 47-module crate that compiles in 0.57s is not hurting anyone's
iteration loop. Rejected because the cost is paid by readers rather than by the
compiler, and this project's second deliverable is that a contributor can
understand the engine from the tree in front of them.

## Consequences

**Good.** Six names that each mean something, so a newly decoded format has an
obvious home or an argued one. `oag-tables` - the crate the simulation reads its
numbers from - ends up with no compilable path to a texture decoder.
`re_rav1d` and the `av1` feature leave the asset-format crate entirely, so the
retained `oag-formats` has exactly one dependency in every feature combination.
`oag-formats` sheds two of its four dev-dependency cycle edges.

**Bad, and worth stating plainly.**

*The build-time win is approximately zero.* Anyone who arrives expecting this to
have made the gate faster will be disappointed, and the honest follow-on is
`oag-game`, which is three times the size of `oag-formats` and is where the time
actually goes.

*Five new silent-failure surfaces.* Each new crate needs its own
`[profile.dev.package.oag-*] opt-level = 2` entry or its decode path drops to
`opt-level = 0` - a regression measured in minutes of `just test-data`, not
seconds, and one that no gate catches. `scripts/check-transcendentals.py`'s
`SCANNED_CRATES` scans `formats` wholesale today, so omitting a new crate
narrows determinism enforcement while the gate stays green. The ~40 rustdoc
intra-doc links naming `oag_formats::` break silently, because `cargo doc` is
not part of `just` and clippy does not see rustdoc lints.

*`oag_vex::vex::Node` stutters.* Module names are kept inside the new crates so
that the move commits stay mechanical and each file's relative `../../../docs/`
links keep resolving. Flattening is possible later, as its own reviewable
change.

*Two placements are arguable rather than derived.* `pob` goes to `oag-vex`
because it is authored content the same scene triggers, though it needs only
`byte_order`; `gxp` goes to `oag-rcs` because a material selects a program,
though `gxt` and `gxp` are sibling SCE containers. Both were decided once. A
future reader is entitled to disagree; they are not entitled to relitigate
without new evidence.

*More manifests.* Six `Cargo.toml` files, six profile entries, six rows in two
crate tables, and a dependency set per consumer that is now three or five names
where it was one.
