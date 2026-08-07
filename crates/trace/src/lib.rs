//! Comparing our simulation against a trace captured from the original.
//!
//! This is the reading half of M3's verification harness. The writing half is
//! `scripts/psp-trace.py`, which breaks in `Ship_UpdateCraft` once per tick under
//! PPSSPP's websocket debugger and writes a CSV of the craft and its rigid body;
//! what that CSV must contain, and what this crate must do with it, is
//! `docs/reverse-engineering/verification-protocol.md`.
//!
//! ```text
//!  PPSSPP + psp-trace.py -> recorded.csv
//!                              |
//!                       replay (our physics, the recording's dt and input)
//!                              |
//!                          simulated
//!                              |
//!                          compare  ->  first divergent tick, and by how much
//! ```
//!
//! # The output is not pass or fail
//!
//! It is the **first tick at which the two diverge and the magnitude**, plus a
//! per-field trend, because a run that tracks the original for 400 ticks and then
//! drifts has a different bug from one that is wrong at tick 1, and an error that
//! grows every tick is systematic even while it is inside tolerance.
//!
//! # Layout
//!
//! | Module | What it is |
//! | --- | --- |
//! | [`mod@trace`] | The CSV format, exactly as the capture script writes it |
//! | [`mod@script`] | The committed input-script format both sides are driven by |
//! | [`mod@replay`] | Driving `oag-gameplay` and `oag-physics` over a recording's scenario |
//! | [`mod@plan`] | Finding an input script that drives our simulation through a gate |
//! | [`mod@compare`] | The tolerance table, the first divergence, and the trends |
//!
//! # Traces are not committed; scripts are
//!
//! A trace is *derived game data* - it comes off a disc, through an emulator, and
//! must never be committed. An input script is the opposite: a list of button
//! names somebody chose, containing nothing that came from the original, and it
//! is committed precisely so that a capture and a replay months apart can be
//! driven by the same authored intent. See [`mod@script`].
//!
//! # What this crate deliberately does not do
//!
//! It does not adjust, rescale or reinterpret a recorded value on the way in. The
//! two conventions that *have* to be reconciled - which way row 0 of the basis
//! points, and that `Body`'s forward is `-Z` - are both in [`mod@replay`], both
//! documented with their evidence, and the first is a **switch** rather than a
//! constant, because it is precisely the kind of finding a comparison exists to
//! settle.
//!
//! Traces are derived game data. They live under `data/traces/`, which
//! `.gitignore` covers, and are never committed - so nothing here can be a CI
//! test, and every test in this crate runs against a hand-authored fixture whose
//! shape matches the capture script's header.

pub mod compare;
pub mod plan;
pub mod replay;
pub mod script;
pub mod trace;

pub use compare::{Comparison, Divergence, Field, Tolerances, compare};
pub use replay::{Options, replay};
pub use script::Script;
pub use trace::{Frame, Trace};
