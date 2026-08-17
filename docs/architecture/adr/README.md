# Architecture decision records

An ADR records a decision that was hard to make and would be expensive to
revisit: the context at the time, what was chosen, what was rejected, and what
it costs.

ADRs are **immutable**. When a decision changes, write a new ADR that supersedes
the old one, and mark the old one superseded. The point is that a reader can
reconstruct why the codebase is the way it is, including the parts that turned
out to be wrong.

| ADR | Title | Status |
| --- | --- | --- |
| [0001](0001-rust-workspace-and-wgpu.md) | Rust workspace and wgpu | Accepted |
| [0002](0002-determinism-model.md) | Determinism model | Accepted |
| [0003](0003-no-ecs.md) | Plain data-oriented world, no ECS | Accepted |
| [0004](0004-asset-pipeline.md) | Load from originals, normalise per platform | Accepted |
| [0005](0005-ghidra-conventions.md) | Ghidra naming and documentation conventions | Accepted |
| [0006](0006-no-copyrighted-content.md) | No copyrighted content in the repository | Accepted |
| [0007](0007-fixed-timestep-vs-original.md) | Keep a fixed timestep, and diverge from the original | Accepted |
| [0008](0008-av1-movie-cache.md) | Cache movies as lossless AV1, and decode them in process | Accepted |
| [0009](0009-multi-game-fanout.md) | Fan out to other games by verified layer, not by date | Accepted; the no-abstraction-at-n=1 item superseded by [ADR-0022](0022-title-packages.md) |
| [0010](0010-movie-decode-thread.md) | Decode movie frames on a worker thread, one per movie | Accepted |
| [0011](0011-authored-pvs-before-frustum-culling.md) | Cull with the track's authored PVS, before the view frustum | Accepted; placement and padding rules superseded by [ADR-0014](0014-authored-section-placement.md) |
| [0012](0012-wgsl-upscalers-not-native-fidelityfx.md) | Port the FSR upscalers to WGSL rather than driving the native SDK | Accepted |
| [0013](0013-anti-aliasing-architecture.md) | Separate anti-aliasing by class, and gate spatial passes against the upscaler by render scale | Accepted |
| [0014](0014-authored-section-placement.md) | Place draw calls by their authored section group, and pad with bits, not masks | Accepted; union rule refined by [ADR-0015](0015-ordered-visible-set-union.md) |
| [0015](0015-ordered-visible-set-union.md) | Order the visible-set union, and never rejoin authored alternatives | Accepted |
| [0016](0016-platform-native-h264-decode.md) | Platform-native H.264 decode as an accelerated path, on Linux first | Superseded by [ADR-0017](0017-gstreamer-native-video.md) |
| [0017](0017-gstreamer-native-video.md) | GStreamer for platform-native H.264 decode, not a hand-rolled `libva` decoder | Accepted |
| [0018](0018-audio-mixer-architecture.md) | Own the mixer, and treat cues as a per-tick output | Accepted |
| [0019](0019-atrac3plus-out-of-process.md) | Decode ATRAC3+ out of process, into the movie cache | Accepted |
| [0020](0020-gamma-authoritative-colour-space.md) | Gamma is the authoritative colour space; nothing linearises | Accepted |
| [0021](0021-region-independent-dlc.md) | Mount downloadable content independently of region | Accepted |
| [0022](0022-title-packages.md) | Title packages, and title as a data axis rather than a code axis | Accepted; supersedes [ADR-0009](0009-multi-game-fanout.md) item 3; item 4 partly superseded by [ADR-0023](0023-boot-sequence-as-title-data.md) |
| [0023](0023-boot-sequence-as-title-data.md) | The boot sequence is a measured table per title | Accepted; supersedes [ADR-0022](0022-title-packages.md) item 4 in part |
| [0024](0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md) | Decode in process where a light library exists; `ffmpeg` is the last resort | Accepted; narrows [ADR-0019](0019-atrac3plus-out-of-process.md) to ATRAC3+ |
| [0025](0025-a-boot-chain-carries-its-provenance.md) | A boot chain carries its provenance: measured, or only declared | Accepted; narrows [ADR-0023](0023-boot-sequence-as-title-data.md) |

## Format

```markdown
# ADR-NNNN: Title

## Status
Accepted | Superseded by ADR-MMMM

## Context
What was true when this was decided.

## Decision
What was chosen.

## Alternatives considered
What was rejected, and why.

## Consequences
What this costs, including the bad parts.
```

The "Consequences" section must be honest about downsides. An ADR that only
lists benefits is advocacy, not a record.
