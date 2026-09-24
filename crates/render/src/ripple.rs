//! The Quake's ripple: the road itself rising into a bump that rolls along the
//! track.
//!
//! # What the original does
//!
//! `Quake_UpdateSpan` (`0x0891cab8`, Pulse USA) rewrites the road's own packed
//! vertex positions every frame the wave is alive. Read at instruction level
//! (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, "the ripple
//! itself"), each vertex of a road span moves along the span's own up axis by
//!
//! ```text
//! height(d) = amplitude(age) * (1 + cos(pi * d / half_width(age))) / 2,  |d| < half_width
//! ```
//!
//! where `d` is its distance along the track from the wave's centre - a
//! raised-cosine bump, [`PEAK_AMPLITUDE`] tall at [`RISE_SECONDS`], fading to
//! nothing at [`LIFETIME_SECONDS`], [`half_width`] growing from 25 to 75
//! units. Confidence 88. **The craft does not ride it**: the floor the hover
//! probes cast against is a separate collision soup nothing here touches
//! (confidence 85, same page) - so this is drawing, and lives in the renderer.
//!
//! # What this draws, and what is chosen
//!
//! The original arms span records one at a time and propagates the bump from
//! each into its neighbours. Because every neighbour link is distance-exact to
//! 0.01 units on all 24 circuit files (`oag-vex`'s `quake_ground_truth.rs`),
//! that machine is one bump at one distance drawn into every span it overlaps,
//! and this draws exactly that: [`Ripple`] places each span at a distance round
//! the course once, at load, and every frame displaces whichever spans the bump
//! overlaps. **Chosen, not measured**: the original's update order can leave a
//! span a frame ahead of or behind its neighbours (a 4.5-unit step at 270 units
//! a second); this draws none of that, and whether the original shows it is
//! unmeasured.

use std::ops::Range;

use oag_vex::quake::Span;

use crate::mesh::{BatchPlacement, GpuVertex};

/// The bump's height at its peak, in world units.
///
/// `g_quake_peak_amplitude`, `0x08a88540`: `12.0f`, reached through a
/// relocated `lui`/`lwc1` pair in `Quake_SpanAmplitude` (`0x0891b600`).
/// Confidence 90.
pub const PEAK_AMPLITUDE: f32 = 12.0;

/// Seconds from launch to the peak: a literal in `Quake_SpanAmplitude`.
pub const RISE_SECONDS: f32 = 0.3;

/// Seconds from launch to nothing: the same `5.0` the span's retire test uses.
pub const LIFETIME_SECONDS: f32 = 5.0;

/// Half the bump's width at launch, in world units: `Quake_SpanHalfWidth`
/// (`0x0891b654`) is `age * 50 / 5 + 25`.
pub const HALF_WIDTH_AT_LAUNCH: f32 = 25.0;

/// How much wider the bump gets over [`LIFETIME_SECONDS`] - the `50` in
/// `age * 50 / 5`.
pub const HALF_WIDTH_GROWTH: f32 = 50.0;

/// How far ahead of the firing craft the bump is born, in the direction it
/// travels: `g_quake_launch_lead`, `0x08a7ccac`, `15.0f`, which `Quake_Init`
/// adds to the launch span's position. Confidence 80.
pub const LAUNCH_LEAD: f32 = 15.0;

/// The bump's height at `age` seconds after launch.
///
/// A linear rise to [`PEAK_AMPLITUDE`] over [`RISE_SECONDS`], then a linear fall
/// that reaches zero at [`LIFETIME_SECONDS`]. Zero outside `0..LIFETIME_SECONDS`
/// rather than extrapolated: the original retires every span there.
#[must_use]
pub fn amplitude(age: f32) -> f32 {
    if !(0.0..LIFETIME_SECONDS).contains(&age) {
        0.0
    } else if age < RISE_SECONDS {
        PEAK_AMPLITUDE * age / RISE_SECONDS
    } else {
        PEAK_AMPLITUDE - PEAK_AMPLITUDE * (age - RISE_SECONDS) / (LIFETIME_SECONDS - RISE_SECONDS)
    }
}

/// Half the bump's width at `age` seconds after launch.
#[must_use]
pub fn half_width(age: f32) -> f32 {
    age * HALF_WIDTH_GROWTH / LIFETIME_SECONDS + HALF_WIDTH_AT_LAUNCH
}

/// The raised-cosine profile: how far a vertex `offset` units from the bump's
/// centre rises.
#[must_use]
pub fn height(offset: f32, amplitude: f32, half_width: f32) -> f32 {
    if half_width <= 0.0 || offset.abs() >= half_width {
        return 0.0;
    }
    amplitude * 0.5 * (1.0 + (std::f32::consts::PI * offset / half_width).cos())
}

/// One frame's bump.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wave {
    /// Where its centre is, in the course's own distance.
    pub centre: f32,
    /// [`amplitude`] this frame.
    pub amplitude: f32,
    /// [`half_width`] this frame.
    pub half_width: f32,
}

impl Wave {
    /// The bump of a wave `age` seconds old whose travelling point is at
    /// `progress` along the course, moving in `direction` (`1.0` or `-1.0`).
    ///
    /// `progress` is the simulation's own wave position, which starts at the
    /// firing craft; the bump sits [`LAUNCH_LEAD`] ahead of it, as the
    /// original's launch span does. **The profile is evaluated one tick late on
    /// purpose**: both of the original's helpers are handed `age + dt` after
    /// `age` has already been advanced.
    #[must_use]
    pub fn at(progress: f32, direction: f32, age: f32) -> Self {
        let lagged = age + 1.0 / 60.0;
        Self {
            centre: progress + direction * LAUNCH_LEAD,
            amplitude: amplitude(lagged),
            half_width: half_width(lagged),
        }
    }
}

/// One vertex the ripple moves.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Movable {
    /// Index into the model's vertices.
    index: u32,
    /// Course units from its owner's origin, signed: a span's distance runs
    /// through the course's own mapping, not linearly in its parameter.
    along: f32,
    /// World-space direction it rises along, unnormalised, as the original's
    /// `-lerp(A, B, p)` is.
    up: [f32; 3],
}

/// One span's claim on a batch: where it puts the batch's vertices along the
/// course, and which of them it moves.
#[derive(Debug, Clone, PartialEq)]
struct Owner {
    origin: f32,
    /// The nearest and farthest movable vertex from `origin`.
    reach: (f32, f32),
    /// Sorted by vertex index.
    movable: Vec<Movable>,
}

/// One GE batch the ripple moves, and every span that moves it.
///
/// **Usually one span, but not always, and the difference is visible.** Where
/// paths meet, one batch belongs to two or three span records - six such pairs
/// on `16_Track` - and the original adds their displacements on the same
/// vertices: measured live on PPSSPP, every vertex matches the *sum* over its
/// owners to the short. So owners are summed here, never picked between.
#[derive(Debug, Clone, PartialEq)]
struct PlacedBatch {
    /// The batch's vertices in the model.
    vertices: Range<u32>,
    owners: Vec<Owner>,
}

/// The road batches of one model, placed on the course, and which of them the
/// last frame displaced.
#[derive(Debug, Clone, PartialEq)]
pub struct Ripple {
    /// Sorted by their vertex range, so a vertex finds its batch by search.
    batches: Vec<PlacedBatch>,
    /// The course's full length, for wrapping.
    ring: f32,
    /// Indices into `batches` last written displaced.
    shown: Vec<usize>,
    /// The bump last written.
    wave: Option<Wave>,
}

/// What [`Ripple::build`] found, for a load report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Built {
    /// Spans whose batch is in this model and which the course could place.
    pub placed: usize,
    /// Spans whose batch is in this model but which had no place on the course.
    pub unplaced: usize,
    /// Vertices the ripple can move, counted once per owning span.
    pub vertices: usize,
}

impl Ripple {
    /// Places every span whose batch is in the model `placements` describes.
    ///
    /// `place(i, p)` gives the course distance of span `i`'s parameter `p`, or
    /// `None` for a span with no place; spans of another model are skipped
    /// without asking. `ring` is the course's length.
    #[must_use]
    pub fn build(
        spans: &[Span],
        placements: &[BatchPlacement],
        ring: f32,
        place: impl Fn(usize, f32) -> Option<f32>,
    ) -> (Self, Built) {
        let mut built = Built::default();
        let mut batches: Vec<PlacedBatch> = Vec::new();
        for (i, span) in spans.iter().enumerate() {
            let Some(batch) = placements.iter().find(|p| p.header == span.batch) else {
                continue;
            };
            let Some(origin) = place(i, 0.0) else {
                built.unplaced += 1;
                continue;
            };
            let wrap = |d: f32| {
                if ring > 0.0 {
                    (d - origin + ring * 0.5).rem_euclid(ring) - ring * 0.5
                } else {
                    d - origin
                }
            };
            let movable: Vec<Movable> = span
                .parameters
                .iter()
                .enumerate()
                .filter_map(|(k, p)| {
                    let p = (*p)?;
                    let index = batch.vertices.start + u32::try_from(k).ok()?;
                    (index < batch.vertices.end).then_some(())?;
                    Some(Movable {
                        index,
                        along: wrap(place(i, p)?),
                        up: rotate(&batch.to_world, lerp_negated(span, p)),
                    })
                })
                .collect();
            if movable.is_empty() {
                continue;
            }
            let reach = movable.iter().fold((f32::MAX, f32::MIN), |(lo, hi), m| {
                (lo.min(m.along), hi.max(m.along))
            });
            built.placed += 1;
            built.vertices += movable.len();
            let owner = Owner {
                origin,
                reach,
                movable,
            };
            match batches.iter_mut().find(|b| b.vertices == batch.vertices) {
                Some(shared) => shared.owners.push(owner),
                None => batches.push(PlacedBatch {
                    vertices: batch.vertices.clone(),
                    owners: vec![owner],
                }),
            }
        }
        batches.sort_by_key(|b| b.vertices.start);
        let ripple = Self {
            batches,
            ring,
            shown: Vec::new(),
            wave: None,
        };
        (ripple, built)
    }

    /// How many batches this model's ripple can move.
    #[must_use]
    pub fn len(&self) -> usize {
        self.batches.len()
    }

    /// Whether it can move none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.batches.is_empty()
    }

    /// Signed course distance from `centre` to `at`, wrapped into half a lap
    /// either way.
    fn offset(&self, at: f32, centre: f32) -> f32 {
        if self.ring > 0.0 {
            (at - centre + self.ring * 0.5).rem_euclid(self.ring) - self.ring * 0.5
        } else {
            at - centre
        }
    }

    /// Whether any movable vertex of `owner` is inside the bump.
    fn overlaps(&self, owner: &Owner, wave: &Wave) -> bool {
        let near = self.offset(owner.origin + owner.reach.0, wave.centre);
        let far = near + (owner.reach.1 - owner.reach.0);
        wave.amplitude > 0.0 && far > -wave.half_width && near < wave.half_width
    }

    /// The displacement of `movable` under `wave`.
    fn lift(&self, origin: f32, movable: &Movable, wave: &Wave) -> [f32; 3] {
        let h = height(
            self.offset(origin + movable.along, wave.centre),
            wave.amplitude,
            wave.half_width,
        );
        [movable.up[0] * h, movable.up[1] * h, movable.up[2] * h]
    }

    /// Moves to `wave` - `None` once the Quake is gone - and hands every
    /// vertex range whose picture changed to `upload`, rebuilt from `base`.
    ///
    /// A batch displaced last frame and clear of the bump now is rewritten
    /// undisplaced, which is how the road settles back exactly: the original
    /// cancels its own previous write bit for bit, so the authored position is
    /// the only place a vertex ever returns to.
    ///
    /// `scratch` is refilled per range and owned by the caller.
    pub fn update(
        &mut self,
        wave: Option<Wave>,
        base: &[GpuVertex],
        scratch: &mut Vec<GpuVertex>,
        mut upload: impl FnMut(u32, &[GpuVertex]),
    ) {
        let now: Vec<usize> = match &wave {
            Some(w) => (0..self.batches.len())
                .filter(|&i| self.batches[i].owners.iter().any(|o| self.overlaps(o, w)))
                .collect(),
            None => Vec::new(),
        };
        self.wave = wave;
        let mut dirty: Vec<usize> = self.shown.iter().chain(&now).copied().collect();
        dirty.sort_unstable();
        dirty.dedup();
        for i in dirty {
            let batch = &self.batches[i];
            let range = batch.vertices.start as usize..batch.vertices.end as usize;
            let Some(authored) = base.get(range) else {
                continue;
            };
            scratch.clear();
            scratch.extend_from_slice(authored);
            if let Some(w) = wave.as_ref().filter(|_| now.binary_search(&i).is_ok()) {
                for owner in &batch.owners {
                    for m in &owner.movable {
                        let lift = self.lift(owner.origin, m, w);
                        let v = &mut scratch[(m.index - batch.vertices.start) as usize];
                        for (axis, by) in v.position.iter_mut().zip(lift) {
                            *axis += by;
                        }
                    }
                }
            }
            upload(batch.vertices.start, scratch);
        }
        self.shown = now;
    }

    /// Adds this frame's displacement to `vertex`, the model's vertex `index`,
    /// for a caller that rewrites vertices of its own after [`Self::update`]:
    /// a weapon pad's tint rebuilds its whole node from the authored vertices
    /// every frame, and would otherwise flatten a pad the bump is passing
    /// over.
    pub fn displace(&self, index: u32, vertex: &mut GpuVertex) {
        let Some(wave) = &self.wave else {
            return;
        };
        let at = self.batches.partition_point(|b| b.vertices.end <= index);
        let Some(batch) = self.batches.get(at).filter(|b| b.vertices.contains(&index)) else {
            return;
        };
        for owner in &batch.owners {
            if let Ok(k) = owner.movable.binary_search_by_key(&index, |m| m.index) {
                let lift = self.lift(owner.origin, &owner.movable[k], wave);
                for (axis, by) in vertex.position.iter_mut().zip(lift) {
                    *axis += by;
                }
            }
        }
    }
}

/// `-lerp(down_start, down_end, p)`: the span's up axis at parameter `p`, in
/// the batch's own space.
fn lerp_negated(span: &Span, p: f32) -> [f32; 3] {
    let a = span.down_start;
    let b = span.down_end;
    [
        -(a[0] + (b[0] - a[0]) * p),
        -(a[1] + (b[1] - a[1]) * p),
        -(a[2] + (b[2] - a[2]) * p),
    ]
}

/// A direction through `matrix`'s linear part - row-major, translation in row
/// 3, row vectors, as every `.vex` matrix is (see `vex::transform_point`).
fn rotate(matrix: &[f32; 16], v: [f32; 3]) -> [f32; 3] {
    [
        v[0] * matrix[0] + v[1] * matrix[4] + v[2] * matrix[8],
        v[0] * matrix[1] + v[1] * matrix[5] + v[2] * matrix[9],
        v[0] * matrix[2] + v[1] * matrix[6] + v[2] * matrix[10],
    ]
}

#[cfg(test)]
mod tests;
