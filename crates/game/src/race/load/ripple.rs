//! Places the circuit's Quake road spans on the course, so the renderer can
//! ripple the road where the simulation's wave is.
//!
//! A span knows where it lies as a fraction of its own AiTrack path
//! (`oag_vex::quake::Span::t_start`); the wave knows where it is as a distance
//! round [`Course`]. This is the one place that holds both, so the mapping is
//! built here, once, and handed to `oag_render::ripple` as plain numbers.
//!
//! # How a vertex is placed
//!
//! - **On a path the lap drives**, through its own `t`: a vertex at parameter
//!   `p` of a span sits at `t = t_start + p * (t_end - t_start)` of the path,
//!   and `t` is the path's control points' authored `progress`, normalised
//!   over the path - the quantity `Quake_Init` computes for the firing craft.
//!   Each control point's `t` is matched to the ring sample `Course` took for
//!   it, and a vertex between two is interpolated. **Every vertex goes through
//!   that one function of `t`**, whichever span owns it, which is what keeps
//!   neighbouring spans seamless: the table's own links agree with `t` to 0.01
//!   units (`oag-vex`'s `quake_ground_truth.rs`), so two spans meeting at the
//!   same `t` meet at the same distance. Mapping each span linearly between
//!   its own two ends instead left steps of up to 26 units at seams inside long
//!   curved spans.
//! - **On a path the lap does not drive** - the other branch of a split on
//!   05, 07 and 14 - through the span table's own links from spans already
//!   placed, linearly in the span's own length. The course has no distance of
//!   its own for a branch it does not walk, so this is the only placement the
//!   data offers. **Chosen, not measured**: whether both branches ripple
//!   together in play is unobserved.
//!
//! `quake_ripple_ground_truth.rs` holds the result against where each vertex
//! actually is on the course, and against every seam in the table.

use oag_race::course::Course;
use oag_render::mesh::{self, BatchPlacement, Model};
use oag_render::ripple::{Built, Ripple};
use oag_vex::quake::{self, Span};
use oag_vex::track::AiTrack;
use oag_vex::vex;

/// One ripple per model the spans can land in. Every `None` is a model with no
/// span in it, or a race with no ripple at all.
#[derive(Debug, Clone, Default)]
pub struct Ripples {
    /// The road.
    pub track: Option<Ripple>,
    /// The speedup pads - `16_Track` sets 51 of its spans on them.
    pub pads: Option<Ripple>,
    /// The weapon pads - 27 on `16_Track`.
    pub weapon_pads: Option<Ripple>,
}

/// Where every span of one circuit sits on its course.
#[derive(Debug, Clone)]
pub struct SpanPlaces {
    ring: f32,
    profiles: Vec<Profile>,
    places: Vec<Option<Place>>,
}

#[derive(Debug, Clone, Copy)]
enum Place {
    /// On a driven path: `profiles[profile]` maps its `t` to distance.
    Driven {
        profile: usize,
        t_start: f32,
        t_end: f32,
    },
    /// Off the ring, placed through a link: linear in world units.
    Linked { origin: f32, length: f32 },
}

impl SpanPlaces {
    /// Places `spans` on `course`.
    #[must_use]
    pub fn new(spans: &[Span], ai: &AiTrack, course: &Course) -> Self {
        let mut profiles: Vec<Profile> = course
            .path_order()
            .into_iter()
            .filter_map(|path| Profile::of(ai, course, path))
            .collect();
        // **A path's `t = 1` is the next path's `t = 0`.** The table's links
        // across a junction say so - the last span of one path ends exactly
        // where the first of the next begins - but the course keeps sampling
        // one more segment of the path past its last control point before the
        // next path's first sample, about 8 units. Pinning the last knot to the
        // next path's start takes that step out of the seam.
        let ring = course.length();
        let starts: Vec<f32> = profiles.iter().map(|p| p.start).collect();
        for (k, profile) in profiles.iter_mut().enumerate() {
            let next = starts[(k + 1) % starts.len()];
            let whole = if starts.len() == 1 {
                ring
            } else {
                (next - profile.start).rem_euclid(ring)
            };
            if let Some(last) = profile.knots.last_mut() {
                last.1 = whole;
            }
        }
        let places = spans
            .iter()
            .map(|span| {
                let profile = profiles
                    .iter()
                    .position(|p| i32::from(p.path) == i32::from(span.path))?;
                Some(Place::Driven {
                    profile,
                    t_start: span.t_start,
                    t_end: span.t_end,
                })
            })
            .collect();
        let mut out = Self {
            ring: course.length(),
            profiles,
            places,
        };
        out.place_through_links(spans);
        out
    }

    /// The branch the lap does not drive, through the links, until nothing new
    /// is placed. Ascending order and first placement wins, so the result
    /// depends on nothing but the table.
    fn place_through_links(&mut self, spans: &[Span]) {
        loop {
            let mut placed_any = false;
            for (i, span) in spans.iter().enumerate() {
                if self.places[i].is_none() || span.length <= 0.0 {
                    continue;
                }
                let forward = span.forward.iter().flatten().map(|&(n, at)| (n, at));
                let backward = span.backward.iter().flatten().map(|&(n, gap)| {
                    (
                        n,
                        span.length + gap - spans.get(usize::from(n)).map_or(0.0, |s| s.length),
                    )
                });
                for (neighbour, at) in forward.chain(backward).collect::<Vec<_>>() {
                    let n = usize::from(neighbour);
                    if n >= spans.len() || self.places[n].is_some() {
                        continue;
                    }
                    let Some(origin) = self.distance(i, at / span.length) else {
                        continue;
                    };
                    self.places[n] = Some(Place::Linked {
                        origin,
                        length: spans[n].length,
                    });
                    placed_any = true;
                }
            }
            if !placed_any {
                break;
            }
        }
    }

    /// Course distance of span `span`'s parameter `p`, or `None` for a span
    /// with no place. `p` may run past `0..=1`, as a link's does.
    #[must_use]
    pub fn distance(&self, span: usize, p: f32) -> Option<f32> {
        let at = match (*self.places.get(span)?)? {
            Place::Driven {
                profile,
                t_start,
                t_end,
            } => {
                let profile = &self.profiles[profile];
                profile.start + profile.distance(t_start + p * (t_end - t_start))
            }
            Place::Linked { origin, length } => origin + p * length,
        };
        Some(at.rem_euclid(self.ring))
    }

    /// How many spans have no place at all.
    #[must_use]
    pub fn unplaced(&self) -> usize {
        self.places.iter().filter(|p| p.is_none()).count()
    }
}

/// Builds the ripples for a Pulse PSP circuit, reporting what it placed.
///
/// A circuit with no `Quake` node, no closed course or a table that will not
/// read gets no ripple and says so - the road stays still rather than moving
/// somewhere invented.
pub(super) fn build(
    track_blob: &[u8],
    ai: &AiTrack,
    course: Option<&Course>,
    models: [Option<&Model>; 3],
    report: &mut Vec<String>,
) -> Ripples {
    let spans = match read_spans(track_blob) {
        Ok(spans) => spans,
        Err(why) => {
            report.push(format!("quake ripple: {why}; the road will not ripple"));
            return Ripples::default();
        }
    };
    let Some(course) = course else {
        report.push("quake ripple: no closed course to place the road spans on".into());
        return Ripples::default();
    };
    let places = SpanPlaces::new(&spans, ai, course);
    let picks: [fn(vex::classes::Classes) -> Option<u32>; 3] =
        [|c| c.mesh, |c| c.speedup_pad, |c| c.weapon_pad];
    let mut built = [None, None, None];
    let mut totals = Built::default();
    for ((slot, model), pick) in built.iter_mut().zip(models).zip(picks) {
        let Some(model) = model else {
            continue;
        };
        let placements: Vec<BatchPlacement> = match mesh::batch_placements(track_blob, pick) {
            Ok(p) => p,
            Err(why) => {
                report.push(format!("quake ripple: {why:#}"));
                continue;
            }
        };
        // A model this walk does not describe - one rebuilt from elsewhere -
        // would ripple the wrong vertices, so it gets none.
        let walked = placements.last().map_or(0, |p| p.vertices.end as usize);
        if walked != model.vertices.len() {
            report.push(format!(
                "quake ripple: {} holds {} vertices where its batches walk to {walked}; not rippled",
                model.label,
                model.vertices.len()
            ));
            continue;
        }
        let (ripple, counts) = Ripple::build(&spans, &placements, course.length(), |i, p| {
            places.distance(i, p)
        });
        totals.placed += counts.placed;
        totals.unplaced += counts.unplaced;
        totals.vertices += counts.vertices;
        *slot = (!ripple.is_empty()).then_some(ripple);
    }
    report.push(format!(
        "quake ripple: {} of {} road span(s) placed on the course ({} unplaced), {} vertices can move",
        totals.placed,
        spans.len(),
        totals.unplaced,
        totals.vertices
    ));
    let [track, pads, weapon_pads] = built;
    Ripples {
        track,
        pads,
        weapon_pads,
    }
}

/// The circuit's span table, or why there is none.
fn read_spans(track_blob: &[u8]) -> Result<Vec<Span>, String> {
    let classes = vex::classes_of(track_blob).map_err(|e| e.to_string())?;
    let class = classes
        .quake
        .ok_or("no Quake class id for this .vex version")?;
    let nodes = vex::nodes(track_blob).map_err(|e| e.to_string())?;
    let node = vex::nodes_by_class(&nodes, class)
        .next()
        .ok_or("the circuit authors no Quake node")?;
    quake::spans(track_blob, node).map_err(|e| e.to_string())
}

/// One driven path's `t` against the course's own distance along it.
///
/// Each control point's `t` is matched to the ring sample `Course` took at
/// the start of its segment: `Course::from_track` samples every path at
/// [`Course::STEPS_PER_SEGMENT`] a control point, contiguously and without
/// gaps.
#[derive(Debug, Clone)]
struct Profile {
    path: u16,
    /// Course distance of the path's first ring sample.
    start: f32,
    /// `(t, distance from start)`, one per control point, both rising.
    knots: Vec<(f32, f32)>,
}

impl Profile {
    fn of(ai: &AiTrack, course: &Course, path: u16) -> Option<Self> {
        let points = &ai.paths.get(usize::from(path))?.points;
        let first = (0..course.len()).find(|&i| course.path_of(i) == Some(path))?;
        let start = course.progress_at(first)?;
        // Progress wraps from just under 1.0 to 0.0 once round the circuit,
        // which can fall inside a path; unwrapped, it only rises.
        let mut lift = 0.0f32;
        let mut previous = f32::MIN;
        let unwrapped: Vec<f32> = points
            .iter()
            .map(|p| {
                if p.progress + lift < previous - 0.5 {
                    lift += 1.0;
                }
                previous = p.progress + lift;
                previous
            })
            .collect();
        let (&low, &high) = (unwrapped.first()?, unwrapped.last()?);
        if high <= low || unwrapped.len() < 2 {
            return None;
        }
        let ring = course.length();
        let knots = unwrapped
            .iter()
            .enumerate()
            .filter_map(|(k, &progress)| {
                let at = course.progress_at(first + k * Course::STEPS_PER_SEGMENT)?;
                Some((
                    (progress - low) / (high - low),
                    (at - start).rem_euclid(ring),
                ))
            })
            .collect();
        Some(Self { path, start, knots })
    }

    /// Course distance from the path's start to `t`: linear between control
    /// points, and continued along the end segments' own slope past either
    /// end, which is where a link onto the next path lands.
    fn distance(&self, t: f32) -> f32 {
        let n = self.knots.len();
        let at = self.knots.partition_point(|&(k, _)| k <= t).clamp(1, n - 1);
        let (t0, d0) = self.knots[at - 1];
        let (t1, d1) = self.knots[at];
        if t1 > t0 {
            d0 + (d1 - d0) * (t - t0) / (t1 - t0)
        } else {
            d0
        }
    }
}
