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
| [0013](0013-anti-aliasing-architecture.md) | Separate anti-aliasing by class, and gate spatial passes against the upscaler by render scale | Accepted; its row shape superseded by [ADR-0041](0041-one-row-for-what-resolves-the-frame.md) |
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
| [0026](0026-hd-authored-lighting-is-linear.md) | Wipeout HD's authored lighting is computed in linear light | Accepted; narrows [ADR-0020](0020-gamma-authoritative-colour-space.md) |
| [0027](0027-three-mix-buses.md) | Three mix buses and a master row, split on the original's own lines | Accepted; supersedes [ADR-0018](0018-audio-mixer-architecture.md)'s two-bus item |
| [0028](0028-camera-motion-blur-first.md) | Ship camera-reprojection motion blur first, under the strength row the full design specified | Accepted as the stepping stone; technique superseded by [ADR-0030](0030-velocity-buffer-motion-blur.md) |
| [0029](0029-primer-capture-and-craft-focus-mask.md) | A capture renders a primer frame, and the craft are masked out of the camera blur | Accepted; the focus mask superseded by [ADR-0030](0030-velocity-buffer-motion-blur.md), the primer capture and cap stand |
| [0030](0030-velocity-buffer-motion-blur.md) | The velocity buffer lands - measured per-draw motion replaces camera reprojection and the focus mask | Accepted; supersedes [ADR-0028](0028-camera-motion-blur-first.md)'s technique and [ADR-0029](0029-primer-capture-and-craft-focus-mask.md)'s mask |
| [0031](0031-wait-briefly-for-the-mixer-lock.md) | The audio callback waits out a held mixer lock, and ramps the buffer it still loses | Superseded by [ADR-0032](0032-render-ahead-into-a-ring.md); its ramps stand |
| [0032](0032-render-ahead-into-a-ring.md) | A thread renders ahead into a lock-free ring and the callback only copies out | Accepted; supersedes [ADR-0031](0031-wait-briefly-for-the-mixer-lock.md) and [ADR-0018](0018-audio-mixer-architecture.md)'s `Output` shape |
| [0033](0033-external-key-material-for-decryption.md) | External key material lives under `data/keys/`, gitignored, and its absence is never an error | Accepted |
| [0034](0034-a-race-may-open-two-titles-at-once.md) | A race may open two titles at once; nothing dispatches through either | Accepted; clarifies [ADR-0022](0022-title-packages.md) |
| [0035](0035-a-craft-pick-may-fall-back-to-a-title-that-reships-the-same-roster.md) | A Race Remix craft pick may fall back to a title that reships the same roster | Accepted |
| [0036](0036-ui-composites-at-presentation-resolution.md) | The UI composites at presentation resolution | Accepted; its seam item superseded by [ADR-0038](0038-a-stage-with-no-scene-draws-at-presentation-resolution.md) |
| [0037](0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md) | Dynamic resolution varies a viewport, not an allocation | Accepted |
| [0038](0038-a-stage-with-no-scene-draws-at-presentation-resolution.md) | A stage with no scene draws entirely at presentation resolution | Accepted; supersedes [ADR-0036](0036-ui-composites-at-presentation-resolution.md)'s seam item |
| [0039](0039-camera-jitter-post-multiplies-onto-the-view-projection.md) | Camera jitter post-multiplies onto the view-projection, after the frustum and the motion snapshot | Accepted |
| [0040](0040-the-dynamic-resolution-budget-is-a-share-of-a-frame.md) | The dynamic-resolution budget is a share of a frame, not a measured frame | Accepted; its budget formula superseded by [ADR-0042](0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md) |
| [0041](0041-one-row-for-what-resolves-the-frame.md) | One row for what resolves the frame, and MSAA on an axis of its own | Accepted; supersedes [ADR-0013](0013-anti-aliasing-architecture.md)'s row shape |
| [0042](0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md) | The dynamic-resolution budget subtracts what it can measure | Accepted; supersedes [ADR-0040](0040-the-dynamic-resolution-budget-is-a-share-of-a-frame.md)'s budget formula |
| [0043](0043-hd-bloom-joins-the-scalable-budget.md) | `hd_bloom` joins the scalable budget | Accepted; extends ADR-0042 |
| [0044](0044-the-residual-is-a-learned-upper-bound.md) | The residual is a learned upper bound, not a constant | Accepted; extends ADR-0042 and ADR-0043 |
| [0045](0045-fsr3-splits-into-a-scaled-and-a-presented-reading.md) | FSR 3.1 is timed in two halves, and only one of them is fixed | Accepted; extends ADR-0042 |
| [0046](0046-test-referenced-traces-are-tracked-in-git.md) | A trace a test names is tracked in git | Accepted; narrows ADR-0006 for one file class |
| [0047](0047-database-state-beyond-names-is-captured-not-replayed.md) | Database state beyond names is captured, not replayed | Accepted; extends ADR-0005 |
| [0048](0048-eu-is-the-psp-pulse-re-target-of-record.md) | `psp-pulse-eu` is the Ghidra target of record; `psp-pulse-usa` corroborates | Accepted |
| [0049](0049-race-records-are-a-chosen-schema-captured-outside-the-tick.md) | Race records are a chosen schema, captured outside the tick, at two sites | Accepted |
| [0050](0050-format-crates-split-by-format-family.md) | Format crates split by format family, and why that is not the console axis | Accepted; extends ADR-0022 |
| [0051](0051-the-composition-root-splits-below-itself.md) | The composition root splits below itself, and the build-time argument does not survive a second time | Accepted; extends ADR-0050 |
| [0052](0052-world-and-race-tick-widen-to-n-players.md) | `World` and `Race::tick` widen to N players ahead of split screen, multi-window or network play | Accepted; extends ADR-0003 |
| [0053](0053-screen-filters-are-loadable-wgsl-after-the-composite.md) | Screen filters are loadable WGSL files, run after the UI composites and before the grade, per title | Accepted; extends ADR-0036 and ADR-0041 |
| [0054](0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md) | A touch front end is a second axis on `FrontEnd`, not a `MenuSkin` variant | Accepted; extends ADR-0022 |
| [0055](0055-replays-are-inputs-and-a-ghost-is-poses.md) | Replays are inputs and hashes, and a ghost is drawn from poses | Accepted; opens M7, creates `oag-replay` |
| [0056](0056-a-render-side-port-of-the-particle-generator.md) | A render-side port of the original's particle generator | Accepted; `oag_fx::ranrot`, render-only |
| [0057](0057-the-workspace-after-the-2026-10-05-splits.md) | The workspace after the 2026-10-05 splits | Accepted; extends ADR-0051, supersedes ADR-0050's `.pob` placement |
| [0058](0058-per-title-behaviour-is-title-data-with-provenance.md) | Per-title behaviour is `Title` data with provenance, never a title comparison | Accepted; narrows [ADR-0022](0022-title-packages.md) item 4, extends [ADR-0025](0025-a-boot-chain-carries-its-provenance.md) |

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
