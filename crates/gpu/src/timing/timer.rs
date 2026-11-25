//! Asking the GPU how long one render pass took, without waiting for it.
//!
//! [`Timing`](super::Timing) says whether an adapter can be asked at all; this
//! is the asking. It exists for dynamic resolution
//! ([dynamic-resolution.md](../../../../docs/rendering/dynamic-resolution.md)),
//! whose whole difficulty is that the answer arrives late: a timestamp pair is
//! resolved on the GPU and copied back to the CPU, so the reading for a frame
//! lands a frame or more after the frame it describes.
//!
//! Three properties follow from that, and they are what this type is for:
//!
//! - **Every reading names its own frame.** [`Reading::frame`] is the frame
//!   index handed to [`PassTimer::begin`], carried through the round trip
//!   rather than assumed to be "the last one". A controller that assumed
//!   would credit a cheap frame's cost to an expensive one and step the wrong
//!   way; the caller here can also discard a reading from before a load, which
//!   a controller reading only the newest value cannot.
//! - **Nothing blocks.** [`PassTimer::read`] polls and returns what has
//!   arrived. Waiting on the map would drain the pipeline and destroy the
//!   measurement it was taken to make - which is exactly what
//!   `timing/tests.rs`'s probe does, correctly, because a test has nothing to
//!   pace.
//! - **A frame with no free slot is not measured.** With every slot still
//!   waiting on its readback, [`PassTimer::begin`] takes none and
//!   [`PassTimer::writes`] hands the pass `None`. A skipped measurement is a
//!   gap in the samples; a reused slot would be a wrong number in them.
//!
//! A ring made with [`PassTimer::with_passes`] times **many passes a frame**, each under a
//! label the pass site names: see [`stamp`], [`PassReading`] and the "Many passes" section of
//! [`PassTimer`].
//!
//! The one-pair rings measure **one pass**, not a frame. Bracketing an encoder's whole span
//! would need `TIMESTAMP_QUERY_INSIDE_ENCODERS`, which is not
//! WebGPU-portable, and would have to know which pass is last - which varies
//! with the bloom, motion-blur and HD-chain settings. Timing each pass and
//! summing the readings needs neither, and is what lets the scene-resolution
//! post-processes join the measurement one at a time as they gain a viewport.

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// How many frames may be measured before one has to come back.
///
/// Three would do for the frames a driver usually keeps in flight; four is one
/// spare, so an ordinary hitch costs a *late* reading rather than a missing
/// one. Each slot is two timestamps and two 16-byte buffers, so the whole ring
/// is smaller than a single 8x8 texture.
const SLOTS: usize = 4;

/// Which end of a pair [`PassTimer::half_writes`] is asking for.
///
/// A two-variant enum rather than a `bool`, because `half_writes(true)` at a
/// call site says nothing about which end true is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Half {
    /// The opening timestamp: put this on the chain's first pass.
    Begin,
    /// The closing timestamp: put this on the chain's last pass.
    End,
}

/// One timestamp pair: two `u64`s, which is also the smallest useful buffer.
const PAIR_BYTES: u64 = 16;

/// What one slot is doing.
///
/// Split from the GPU objects and kept in [`Ring`] so the whole "which frame
/// does this reading belong to" question is a plain state machine with no
/// device in it - see this module's tests, which run without a GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    /// Nothing in it.
    Free,
    /// Claimed by a frame whose pass has not been resolved yet.
    Writing(u64),
    /// Resolved and copied into the readback buffer by an encoder that has not
    /// necessarily been submitted yet, so the map cannot be started.
    Copied(u64),
    /// Mapping, or mapped and waiting to be read.
    InFlight(u64),
}

/// Which frame is in which slot, with no GPU in it.
#[derive(Debug)]
struct Ring {
    slots: [Slot; SLOTS],
    /// Where the next scan for a free slot starts, so slots are used round
    /// robin rather than always reaching for the first one - which keeps a
    /// readback the longest possible time to land in.
    next: usize,
}

impl Ring {
    fn new() -> Self {
        Self {
            slots: [Slot::Free; SLOTS],
            next: 0,
        }
    }

    /// The slot `frame` will be timed in, or `None` when every slot is still
    /// waiting on a reading.
    fn claim(&mut self, frame: u64) -> Option<usize> {
        let free = (0..SLOTS)
            .map(|offset| (self.next + offset) % SLOTS)
            .find(|&slot| self.slots[slot] == Slot::Free)?;
        self.slots[free] = Slot::Writing(frame);
        self.next = (free + 1) % SLOTS;
        Some(free)
    }

    /// Give a claimed slot back unwritten.
    ///
    /// Only legal from `Slot::Writing`: a slot that has been resolved or is
    /// mapping has a readback in flight and is not the caller's to reclaim.
    /// See [`PassTimer::abandon`] for the case this exists for.
    fn release(&mut self, slot: usize) {
        if matches!(self.slots[slot], Slot::Writing(_)) {
            self.slots[slot] = Slot::Free;
        }
    }

    /// The slot's queries have been resolved into its readback buffer.
    fn copied(&mut self, slot: usize) {
        if let Slot::Writing(frame) = self.slots[slot] {
            self.slots[slot] = Slot::Copied(frame);
        }
    }

    /// Whether `slot`'s copy is submitted and its map can now start.
    fn is_copied(&self, slot: usize) -> bool {
        matches!(self.slots[slot], Slot::Copied(_))
    }

    /// The map for `slot` has been started.
    fn mapping(&mut self, slot: usize) {
        if let Slot::Copied(frame) = self.slots[slot] {
            self.slots[slot] = Slot::InFlight(frame);
        }
    }

    /// The oldest slot whose map has landed, judged by `ready`.
    ///
    /// **Oldest of the *ready* ones, not the oldest outright.** A backlog
    /// drains in the order the frames happened, which is what the ordinary
    /// case wants; but a map that never completes must not hold up the slots
    /// behind it, or one failure takes the ring with it - four of them and
    /// nothing is ever measured again. The reading names its own frame either
    /// way, so a caller is never misled by the one case where this hands back
    /// a later frame first.
    fn ready(&self, ready: impl Fn(usize) -> bool) -> Option<(usize, u64)> {
        (0..SLOTS)
            .filter_map(|slot| match self.slots[slot] {
                Slot::InFlight(frame) if ready(slot) => Some((slot, frame)),
                _ => None,
            })
            .min_by_key(|&(_, frame)| frame)
    }

    /// The slot's reading has been taken and the slot is reusable.
    fn free(&mut self, slot: usize) {
        self.slots[slot] = Slot::Free;
    }
}

/// One pass's cost, and the frame it was measured on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    /// The frame index handed to [`PassTimer::begin`], **not** the frame this
    /// arrived on. The gap between the two is the latency a controller's
    /// cooldown has to cover.
    pub frame: u64,
    /// How long the pass took on the GPU, in seconds - the same unit
    /// `perf::Meter` is fed, so the two are directly comparable.
    pub seconds: f32,
}

/// A ring of timestamp pairs: one pass timed per frame, read back late.
///
/// Held by whatever owns the frame loop, never by a renderer: it is per-window
/// state with a wall-clock flavour, exactly as `perf::Meter` is, and nothing
/// the simulation reads comes back out of it.
#[derive(Debug)]
pub struct PassTimer {
    queries: wgpu::QuerySet,
    /// Timestamp pairs a slot holds: one for a ring that times one pass or chain, more for one
    /// made by [`PassTimer::with_passes`].
    pairs: usize,
    /// Whether this ring's pairs are handed out by [`stamp`] under labels, rather than being
    /// the one pair [`PassTimer::writes`] and its siblings name.
    labelled: bool,
    /// Per slot, the labels of the pairs the frame in it used, in pair order: what turns a
    /// reading back into "this much for that pass" when it lands frames later. Kept per slot
    /// and captured when the slot is resolved, because the set of passes a frame encodes
    /// changes from frame to frame (a glass pass on some circuits, a photo effect when it is
    /// on), so one table for the ring would label a late reading with a later frame's passes.
    used: Vec<Vec<&'static str>>,
    /// Per slot, labels that overflowed the slot's pairs: one pass of them went untimed, so any
    /// sum for them would be a part of the truth and none is reported.
    dropped: Vec<Vec<&'static str>>,
    /// One pair of buffers per slot rather than one buffer with a stride.
    /// `resolve_query_set`'s destination offset has a 256-byte alignment and a
    /// timestamp pair is 16 bytes, so a shared buffer would be mostly padding
    /// and arithmetic; four tiny buffers are neither.
    buffers: Vec<Buffers>,
    ring: Ring,
    /// Nanoseconds per timestamp tick, from `Queue::get_timestamp_period`.
    ///
    /// **Read every run and never assumed**: the two adapters in this
    /// project's development machine differ by 52x, so a constant here would
    /// be a 52x error on one of them.
    period: f32,
    /// The slot this frame is being written into, between [`Self::begin`] and
    /// [`Self::resolve`].
    writing: Option<usize>,
}

#[derive(Debug)]
struct Buffers {
    resolved: wgpu::Buffer,
    readback: wgpu::Buffer,
    /// Set by the map callback, cleared when the reading is taken. An atomic
    /// rather than a poll of the buffer's own state because the callback is
    /// the only thing that knows, and `get_mapped_range`'s error does not
    /// distinguish "not yet" from "never".
    mapped: Arc<AtomicBool>,
}

impl PassTimer {
    /// How many readings [`Self::drain`] can hand back at once - the ring's
    /// size, and the size of the array a caller lends it.
    pub const SLOTS: usize = SLOTS;

    /// A timer, or `None` on a device that cannot be asked.
    ///
    /// Takes the **device** rather than the adapter, deliberately: an adapter
    /// that advertises `TIMESTAMP_QUERY` says nothing about whether the device
    /// was actually requested with it, and using a feature that was not
    /// requested is a validation error rather than a degraded picture.
    /// [`Timing`](super::Timing) is the probe that decides what to request;
    /// this checks what came back.
    ///
    /// Also `None` on a nonsensical tick period. Zero would make every
    /// measured cost zero, which reads as "the frame was free" rather than as
    /// "the clock is broken" - and a controller fed zeroes would raise the
    /// resolution until it dropped frames.
    ///
    /// Silent either way, as everything else in this crate is: whether a run
    /// got a timer is the caller's line to write, and the caller is the one
    /// that knows what it wanted the timer for.
    #[must_use]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        Self::make(device, queue, 1, false)
    }

    /// A timer for **many passes a frame**, each timed under the label its site gives
    /// [`stamp`], with room for `pairs` of them in a frame. `None` on a device that cannot be
    /// asked, as [`Self::new`] is.
    ///
    /// # Many passes
    ///
    /// [`Self::begin`] claims a slot as for any ring, and [`Self::arm`] then makes it the
    /// calling thread's current frame: until [`Self::resolve`] or [`Self::abandon`], every
    /// [`stamp`] call on that thread hands the pass about to be encoded the next timestamp
    /// pair of the slot. The pass sites are spread over six crates and a pass site knows its
    /// own label and nothing about frames, so a handout through a thread-local (as
    /// `perfprobe::marks` does for its inside-pass marks) is what keeps a site to one call.
    /// **A pass encoded on another thread is not timed**: a split-screen view's worker records
    /// into its own encoder and finds nothing armed. [`Self::resolve`] copies out only the
    /// pairs that were handed out, so a pair no pass wrote is never read.
    ///
    /// [`Self::drain_labelled`] sums a frame's passes by label. A label whose passes did not
    /// all fit in `pairs` reports nothing for that frame, not the part that fitted.
    #[must_use]
    pub fn with_passes(device: &wgpu::Device, queue: &wgpu::Queue, pairs: usize) -> Option<Self> {
        Self::make(device, queue, pairs.max(1), true)
    }

    fn make(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pairs: usize,
        labelled: bool,
    ) -> Option<Self> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let period = queue.get_timestamp_period();
        if !period.is_finite() || period <= 0.0 {
            return None;
        }
        let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("pass timer"),
            ty: wgpu::QueryType::Timestamp,
            count: u32::try_from(SLOTS * 2 * pairs).expect("a query set's worth of pairs"),
        });
        let buffers = (0..SLOTS)
            .map(|slot| Buffers {
                resolved: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("pass timer resolved {slot}")),
                    size: PAIR_BYTES * pairs as u64,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                readback: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("pass timer readback {slot}")),
                    size: PAIR_BYTES * pairs as u64,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                mapped: Arc::new(AtomicBool::new(false)),
            })
            .collect();
        Some(Self {
            queries,
            pairs,
            labelled,
            used: vec![Vec::new(); SLOTS],
            dropped: vec![Vec::new(); SLOTS],
            buffers,
            ring: Ring::new(),
            period,
            writing: None,
        })
    }

    /// Take a slot for `frame`, if one is free.
    ///
    /// Called before the pass is encoded and paired with exactly one
    /// [`Self::resolve`]: a slot claimed and never resolved holds a frame
    /// index that will never come back, and after four of them nothing is
    /// measured again.
    ///
    /// **That is a caller error, and it is a different thing from a readback
    /// that never lands.** The second is the GPU's to do and costs one slot
    /// ([`Ring::ready`] steps over it); this one is a missing call, and there
    /// is nothing the ring can do about a slot it was never told to let go of.
    /// The two sit next to each other in the tests for exactly that reason.
    pub fn begin(&mut self, frame: u64) {
        self.writing = self.ring.claim(frame);
        if self.labelled {
            self.arm();
        }
    }

    /// The first query of `slot`'s pairs.
    fn base(&self, slot: usize) -> u32 {
        u32::try_from(slot * 2 * self.pairs).expect("a query set's worth of pairs")
    }

    /// Makes the claimed slot the calling thread's current frame for [`stamp`], or clears the
    /// thread's current frame when nothing was claimed, so a frame that found the ring full
    /// times nothing rather than writing into the last frame's slot.
    fn arm(&self) {
        let armed = self.writing.map(|slot| Armed {
            queries: self.queries.clone(),
            base: self.base(slot),
            capacity: self.pairs,
            labels: Vec::new(),
            dropped: Vec::new(),
        });
        ARMED.with(|cell| *cell.borrow_mut() = armed);
    }

    /// What to put in the pass descriptor's `timestamp_writes` this frame.
    ///
    /// `None` when [`Self::begin`] found no free slot, which a pass descriptor
    /// takes as "do not time this one" with no branch at the call site.
    #[must_use]
    pub fn writes(&self) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let slot = self.writing?;
        let base = self.base(slot);
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.queries,
            beginning_of_pass_write_index: Some(base),
            end_of_pass_write_index: Some(base + 1),
        })
    }

    /// Half a pair, for bracketing a **chain** of passes rather than one pass.
    ///
    /// `Half::Begin` writes only the opening timestamp and `Half::End` only the
    /// closing one, so a caller can put the first on the chain's first pass and
    /// the second on its last and get one reading spanning all of them. That is
    /// what `oag_post::hd_bloom` needs: its ladder is a loop of render
    /// passes whose length is a runtime decision inside the chain, so
    /// [`Self::writes`] on any single one of them would measure a fraction of
    /// the cost and there is no `TIMESTAMP_QUERY_INSIDE_ENCODERS` to bracket
    /// the encoder with.
    ///
    /// **Both halves are required.** wgpu allows either index to be `None`
    /// independently, which is what makes this expressible at all; what it
    /// does not do is notice that a slot only ever got its opening write. Such
    /// a slot resolves to a closing timestamp that was never written - an
    /// unspecified value, not zero - so a chain that can encode *no* passes
    /// must not have claimed a slot in the first place. See
    /// [`Self::abandon`].
    #[must_use]
    pub fn half_writes(&self, half: Half) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let slot = self.writing?;
        let base = self.base(slot);
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.queries,
            beginning_of_pass_write_index: matches!(half, Half::Begin).then_some(base),
            end_of_pass_write_index: matches!(half, Half::End).then_some(base + 1),
        })
    }

    /// Give back a slot claimed for a chain that turned out to encode nothing.
    ///
    /// **The escape hatch [`Self::begin`]'s contract needs once a claim can be
    /// made before the caller knows whether there will be a pass.** A claim is
    /// paired with exactly one [`Self::resolve`], and four claims that never
    /// resolve end measurement for the run; a caller that has to decide "is
    /// this chain empty" *after* claiming needs a way to un-claim rather than a
    /// reason to guess beforehand.
    ///
    /// Returns the slot to `Free` rather than to any in-flight state, because
    /// nothing was ever written into it: there is no readback to wait for and
    /// no frame index worth carrying.
    pub fn abandon(&mut self) {
        if self.labelled {
            ARMED.with(|cell| cell.borrow_mut().take());
        }
        if let Some(slot) = self.writing.take() {
            self.ring.release(slot);
        }
    }

    /// The same pair, for a **compute** pass.
    ///
    /// A separate method rather than a generic one because wgpu spells the two
    /// descriptors as different types - `ComputePassTimestampWrites` and
    /// `RenderPassTimestampWrites` - with no conversion between them, even
    /// though a slot is a slot and this hands back the same two indices into
    /// the same query set. Written for FSR 3.1, which since ADR-0045 encodes
    /// its eight dispatches as two compute passes and takes one of these per
    /// ring for each; see `oag_post::fsr3::Fsr3::render` and
    /// `oag_post::fsr3::ChainTimestamps`.
    #[must_use]
    pub fn compute_writes(&self) -> Option<wgpu::ComputePassTimestampWrites<'_>> {
        let slot = self.writing?;
        let base = self.base(slot);
        Some(wgpu::ComputePassTimestampWrites {
            query_set: &self.queries,
            beginning_of_pass_write_index: Some(base),
            end_of_pass_write_index: Some(base + 1),
        })
    }

    /// Copy this frame's pair out of the query set, into the same encoder the
    /// pass was recorded in.
    ///
    /// A no-op when nothing was claimed. Must come after the pass is encoded
    /// and before the encoder is submitted.
    pub fn resolve(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let Some(slot) = self.writing.take() else {
            return;
        };
        let pairs = if self.labelled {
            let armed = ARMED.with(|cell| cell.borrow_mut().take());
            let Some(armed) = armed.filter(|armed| !armed.labels.is_empty()) else {
                // No pass was stamped: nothing was written, so nothing is resolved.
                self.ring.release(slot);
                return;
            };
            let pairs = armed.labels.len();
            self.used[slot] = armed.labels;
            self.dropped[slot] = armed.dropped;
            pairs
        } else {
            1
        };
        let base = self.base(slot);
        let bytes = PAIR_BYTES * pairs as u64;
        let buffers = &self.buffers[slot];
        let count = u32::try_from(pairs * 2).expect("a query set's worth of pairs");
        encoder.resolve_query_set(&self.queries, base..base + count, &buffers.resolved, 0);
        encoder.copy_buffer_to_buffer(&buffers.resolved, 0, &buffers.readback, 0, bytes);
        self.ring.copied(slot);
    }

    /// Everything that has come back from a ring made by [`Self::with_passes`], oldest frame
    /// first, one [`PassReading`] per label per frame, into `out` (cleared first).
    ///
    /// The same round trip as [`Self::drain`], so the same rule for what a caller may rely on:
    /// every reading names its own frame, and nothing here waits.
    pub fn drain_labelled(&mut self, device: &wgpu::Device, out: &mut Vec<PassReading>) {
        out.clear();
        self.start_maps();
        let _ = device.poll(wgpu::PollType::Poll);
        while let Some((slot, frame)) = self.ready() {
            self.take_labelled(slot, frame, out);
        }
    }

    /// A slot's pairs, summed by label, into `out`; frees the slot.
    fn take_labelled(&mut self, slot: usize, frame: u64, out: &mut Vec<PassReading>) {
        let labels = std::mem::take(&mut self.used[slot]);
        let dropped = std::mem::take(&mut self.dropped[slot]);
        let buffers = &self.buffers[slot];
        buffers.mapped.store(false, Ordering::Release);
        let mut ticks = Vec::with_capacity(labels.len());
        {
            let slice = buffers.readback.slice(..);
            if let Ok(mapped) = slice.get_mapped_range() {
                let at = |offset: usize| {
                    u64::from_le_bytes(
                        <[u8; 8]>::try_from(&mapped[offset..offset + 8]).expect("eight bytes"),
                    )
                };
                for pair in 0..labels.len() {
                    ticks.push((at(pair * 16), at(pair * 16 + 8)));
                }
            }
        }
        buffers.readback.unmap();
        self.ring.free(slot);
        if ticks.len() != labels.len() {
            return;
        }
        for (label, seconds) in sum_by_label(&labels, &dropped, &ticks, self.period) {
            out.push(PassReading {
                frame,
                label,
                seconds,
            });
        }
    }

    /// Whatever has come back, at most one reading per call.
    ///
    /// **Call after the queue submit**, which is what makes it safe to start a
    /// map on the slot that frame just wrote: a map is asked for on a copy
    /// that has been submitted, and the callback fires when the GPU is done
    /// with it.
    ///
    /// One per call rather than a batch, so a caller that wants exactly one
    /// number a frame gets it with no buffer to hand over. A backlog drains
    /// oldest first, one a frame, and each item still names its own frame - so
    /// a caller loses ordering information by ignoring [`Reading::frame`],
    /// not by calling this once. **A caller pairing this ring's readings
    /// against another ring's wants [`Self::drain`] instead**, for the reason
    /// its documentation gives.
    pub fn read(&mut self, device: &wgpu::Device) -> Option<Reading> {
        self.start_maps();
        // Non-blocking, unlike the probe in `timing/tests.rs`: waiting here
        // would drain the pipeline this is measuring.
        let _ = device.poll(wgpu::PollType::Poll);
        let (slot, frame) = self.ready()?;
        self.take(slot, frame)
    }

    /// Everything that has come back, oldest first, into `out`.
    ///
    /// The same round trip as [`Self::read`] with one difference that matters
    /// to a caller reading **several rings for one frame**: this takes every
    /// reading that is ready, not the oldest one. `read`'s one-a-call shape
    /// lets a ring fall permanently one frame behind its neighbours - the
    /// map callbacks for one submission fire in whichever `poll` happens to
    /// run after the GPU finishes it, and a frame loop polls once per ring,
    /// so a completion landing between two of those polls hands the second
    /// ring a reading the first ring will only see next frame. From then on
    /// the first ring holds two ready readings, returns the older, and its
    /// newest is exactly one frame behind the other ring's on *every* call;
    /// nothing short of a frame with no arrival at all resyncs them. A caller
    /// that only trusts readings naming the same frame then never sees a
    /// match again. Taking everything that is ready keeps a ring's backlog
    /// at zero, so two rings can disagree by at most the one frame the race
    /// itself costs, and are back in step on the next.
    ///
    /// `out` is caller-owned and fixed-size so that this allocates nothing
    /// per frame; a ring holds at most [`Self::SLOTS`] readings. Unfilled
    /// entries are left `None`, and the filled ones are contiguous from the
    /// front.
    pub fn drain(&mut self, device: &wgpu::Device, out: &mut [Option<Reading>; SLOTS]) {
        self.start_maps();
        let _ = device.poll(wgpu::PollType::Poll);
        for entry in out.iter_mut() {
            *entry = None;
        }
        let mut count = 0;
        // Bounded by the ring, whatever `ready` does: every iteration frees
        // a slot, so this ends within `SLOTS` rounds.
        while let Some((slot, frame)) = self.ready() {
            let reading = self.take(slot, frame);
            if let Some(reading) = reading {
                out[count] = Some(reading);
                count += 1;
            }
        }
    }

    /// Start a map on every slot whose copy has been submitted.
    fn start_maps(&mut self) {
        for slot in 0..SLOTS {
            if !self.ring.is_copied(slot) {
                continue;
            }
            let buffers = &self.buffers[slot];
            let mapped = Arc::clone(&buffers.mapped);
            buffers
                .readback
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    // A failed map leaves the flag clear and the slot is
                    // never read - one sample lost rather than a wrong one
                    // reported. It costs the ring that slot for the rest of
                    // the run, which is why `Ring::ready` takes the oldest
                    // *ready* slot rather than the oldest one: a slot stuck
                    // like this must not hold up the ones behind it.
                    if result.is_ok() {
                        mapped.store(true, Ordering::Release);
                    }
                });
            self.ring.mapping(slot);
        }
    }

    /// The oldest slot whose map has landed.
    fn ready(&self) -> Option<(usize, u64)> {
        self.ring
            .ready(|slot| self.buffers[slot].mapped.load(Ordering::Acquire))
    }

    /// Take the reading out of a slot [`Self::ready`] returned, and free it.
    fn take(&mut self, slot: usize, frame: u64) -> Option<Reading> {
        let buffers = &self.buffers[slot];
        buffers.mapped.store(false, Ordering::Release);
        let ticks = {
            let slice = buffers.readback.slice(..);
            let Ok(mapped) = slice.get_mapped_range() else {
                // Mapped-then-unreadable should not happen; free the slot
                // rather than keeping it out of circulation for ever.
                self.ring.free(slot);
                return None;
            };
            let at = |offset: usize| {
                u64::from_le_bytes(
                    <[u8; 8]>::try_from(&mapped[offset..offset + 8]).expect("eight bytes"),
                )
            };
            (at(0), at(8))
        };
        buffers.readback.unmap();
        self.ring.free(slot);
        // Discarded rather than asserted on: a wrapped or unordered pair comes
        // back as an enormous unsigned difference and would read as a
        // catastrophically slow frame, which is exactly the reading that would
        // drive a controller to the floor. `timing/tests.rs` asserts the
        // property holds on this machine's adapters; production drops the
        // sample instead, and the caller sees one frame with no reading.
        let (begin, end) = ticks;
        if end < begin {
            return None;
        }
        let seconds = (end - begin) as f64 * f64::from(self.period) / 1e9;
        Some(Reading {
            frame,
            seconds: seconds as f32,
        })
    }
}

/// One label's cost in one frame, from a ring made by [`PassTimer::with_passes`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassReading {
    /// The frame index handed to [`PassTimer::begin`], as [`Reading::frame`] is.
    pub frame: u64,
    /// What the pass site called itself in [`stamp`].
    pub label: &'static str,
    /// The GPU time of every pass that frame stamped with the label, summed, in seconds.
    pub seconds: f32,
}

/// A frame's pairs summed by label, in the order each label first appeared.
///
/// **A label that has a pair with an end before its beginning, or that is in `dropped`, is left
/// out whole**: a wrapped pair reads as an enormous duration and a label that lost a pass to a
/// full slot is a part of its cost, and both are worse than a missing row.
fn sum_by_label(
    labels: &[&'static str],
    dropped: &[&'static str],
    ticks: &[(u64, u64)],
    period: f32,
) -> Vec<(&'static str, f32)> {
    let mut sums: Vec<(&'static str, Option<f64>)> = Vec::new();
    for (&label, &(begin, end)) in labels.iter().zip(ticks) {
        let seconds = (end >= begin).then(|| (end - begin) as f64 * f64::from(period) / 1e9);
        match sums.iter_mut().find(|(known, _)| *known == label) {
            Some((_, total)) => *total = total.zip(seconds).map(|(a, b)| a + b),
            None => sums.push((label, seconds)),
        }
    }
    sums.into_iter()
        .filter(|(label, _)| !dropped.contains(label))
        .filter_map(|(label, total)| Some((label, total? as f32)))
        .collect()
}

/// The frame a ring made by [`PassTimer::with_passes`] has armed on this thread.
struct Armed {
    queries: wgpu::QuerySet,
    /// The first query of the slot the frame is in.
    base: u32,
    /// Pairs the slot holds.
    capacity: usize,
    /// The label of each pair handed out, in pair order.
    labels: Vec<&'static str>,
    /// Labels a pass of which found the slot full.
    dropped: Vec<&'static str>,
}

thread_local! {
    /// Per thread, because the frame loop arms it on the thread that then encodes the passes
    /// (and the web build's wgpu types are not `Send`, `docs/tools/web.md`, "Threads"). A pass
    /// recorded on another thread finds it empty and is not timed.
    static ARMED: RefCell<Option<Armed>> = const { RefCell::new(None) };
}

/// A timestamp pair for one pass, on the frame this thread has armed: a clone of the ring's
/// query set and the two indices, owned so it outlives the lookup and a pass descriptor can
/// borrow from it.
#[derive(Debug)]
pub struct Stamp {
    queries: wgpu::QuerySet,
    begin: u32,
}

impl Stamp {
    /// For a render pass's `timestamp_writes`.
    #[must_use]
    pub fn render_writes(&self) -> wgpu::RenderPassTimestampWrites<'_> {
        wgpu::RenderPassTimestampWrites {
            query_set: &self.queries,
            beginning_of_pass_write_index: Some(self.begin),
            end_of_pass_write_index: Some(self.begin + 1),
        }
    }

    /// For a compute pass's `timestamp_writes`.
    #[must_use]
    pub fn compute_writes(&self) -> wgpu::ComputePassTimestampWrites<'_> {
        wgpu::ComputePassTimestampWrites {
            query_set: &self.queries,
            beginning_of_pass_write_index: Some(self.begin),
            end_of_pass_write_index: Some(self.begin + 1),
        }
    }
}

/// The next timestamp pair of the frame armed on this thread, for a pass about to be encoded
/// under `label`, or `None` when no frame is armed, the device cannot be asked, or the slot is
/// full.
///
/// **Call it immediately before the pass it times, and use the result**: a pair handed out and
/// not written is read back all the same, and an unwritten pair is unspecified, not zero.
/// Several passes may share a label (a shadow map per layer): their times are summed. A pass
/// with its own `timestamp_writes` (a chain a [`PassTimer`] already brackets) takes none from
/// here.
#[must_use]
pub fn stamp(label: &'static str) -> Option<Stamp> {
    ARMED.with(|cell| {
        let mut armed = cell.borrow_mut();
        let armed = armed.as_mut()?;
        if armed.labels.len() >= armed.capacity {
            if !armed.dropped.contains(&label) {
                armed.dropped.push(label);
            }
            return None;
        }
        let begin = armed.base
            + u32::try_from(armed.labels.len() * 2).expect("a query set's worth of pairs");
        armed.labels.push(label);
        Some(Stamp {
            queries: armed.queries.clone(),
            begin,
        })
    })
}

#[cfg(test)]
mod tests;
