//! Named GPU timestamps *inside* render and compute passes, so a frame's device
//! time can be split by pass and, inside a pass, by what it drew.
//!
//! A [`crate::timing::PassTimer`] brackets a whole pass; the race pass is one
//! pass holding the sky, the track, its shine, the craft and every effect, so
//! its total says nothing about which of them costs. [`mark`] writes a
//! timestamp between draws, and the spans between consecutive marks are the
//! groups' costs. Inside-pass timestamps are an optional wgpu feature
//! (`TIMESTAMP_QUERY_INSIDE_PASSES`); `mesh_render::optional_features` asks for
//! it only with `perf-probe` on, and [`Marks::new`] declines without it.
//!
//! **Every pass on the queue marks its own start and [`close`]s its end**, so a
//! span never runs across a pass nothing named: a pass's span is its own
//! commands, and the span [`close`] opens ([`BETWEEN`]) is what the queue does
//! from one pass's last command to the next pass's first (stores, MSAA
//! resolves, load-op clears, barriers, copies). The spans of one recording
//! then add up to its first mark to its last, and a bench reads what is left of
//! its wall-clock total as work no encoder timestamp sees (submission, and the
//! queue's staged writes, which run ahead of the encoder).
//!
//! Where the device also offers pipeline statistics, each render-pass span
//! counts its fragment-shader invocations too, which divided by the viewport's
//! pixels is that group's overdraw. A compute pass and a [`BETWEEN`] span count
//! none.
//!
//! Only armed while a bench holds a [`Marks`]: [`mark`] is a no-op otherwise,
//! and the whole module folds away without the feature.

use std::cell::RefCell;

/// The most marks one recording can take. HD's frame takes about 130 (two per
/// pass, sixteen passes of them per craft, plus the race pass's groups).
const CAPACITY: u32 = 512;

/// The label of the span [`close`] opens: from a pass's last command to the
/// next pass's first mark.
pub const BETWEEN: &str = "between passes";

struct Armed {
    queries: wgpu::QuerySet,
    /// One fragment-invocation count per render-pass span, when the device can.
    fragments: Option<wgpu::QuerySet>,
    /// Whether a statistics query is open on the pass being recorded.
    open: bool,
    labels: Vec<&'static str>,
    /// The fragment-count slot each label's span opened, by label index.
    counted: Vec<Option<u32>>,
    /// Statistics slots used so far.
    slots: u32,
    /// Marks past [`CAPACITY`], which wrote nothing.
    dropped: u32,
}

thread_local! {
    /// Per thread, because the queries are the thread's own: a bench arms them
    /// on the thread that then records the pass, and the web build's wgpu
    /// types are not `Send` (docs/tools/web.md, "Threads").
    static ARMED: RefCell<Option<Armed>> = const { RefCell::new(None) };
}

/// A query set [`mark`] writes into while this is alive.
#[derive(Debug)]
pub struct Marks {
    resolved: wgpu::Buffer,
    readback: wgpu::Buffer,
    fragments_resolved: wgpu::Buffer,
    fragments_readback: wgpu::Buffer,
    period: f32,
}

impl Marks {
    /// Arms [`mark`], or `None` when the device cannot timestamp inside a pass.
    #[must_use]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !cfg!(feature = "perf-probe")
            || !device
                .features()
                .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES)
        {
            return None;
        }
        let bytes = u64::from(CAPACITY) * 8;
        let buffer = |label, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: bytes,
                usage,
                mapped_at_creation: false,
            })
        };
        let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("perf marks"),
            ty: wgpu::QueryType::Timestamp,
            count: CAPACITY,
        });
        let fragments = device
            .features()
            .contains(wgpu::Features::PIPELINE_STATISTICS_QUERY)
            .then(|| {
                device.create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some("perf mark fragments"),
                    ty: wgpu::QueryType::PipelineStatistics(
                        wgpu::PipelineStatisticsTypes::FRAGMENT_SHADER_INVOCATIONS,
                    ),
                    count: CAPACITY,
                })
            });
        ARMED.set(Some(Armed {
            queries,
            fragments,
            open: false,
            labels: Vec::new(),
            counted: Vec::new(),
            slots: 0,
            dropped: 0,
        }));
        Some(Self {
            resolved: buffer(
                "perf marks resolved",
                wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            ),
            readback: buffer(
                "perf marks readback",
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            ),
            fragments_resolved: buffer(
                "perf mark fragments resolved",
                wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            ),
            fragments_readback: buffer(
                "perf mark fragments readback",
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            ),
            period: queue.get_timestamp_period(),
        })
    }

    /// Clears the labels for a fresh recording.
    pub fn begin(&self) {
        ARMED.with_borrow_mut(|armed| {
            if let Some(armed) = armed.as_mut() {
                armed.labels.clear();
                armed.counted.clear();
                armed.slots = 0;
                armed.dropped = 0;
                armed.open = false;
            }
        });
    }

    /// Resolves this recording's marks into the readback buffer.
    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        ARMED.with_borrow(|armed| self.resolve_armed(armed.as_ref(), encoder));
    }

    fn resolve_armed(&self, armed: Option<&Armed>, encoder: &mut wgpu::CommandEncoder) {
        let Some(armed) = armed else { return };
        let count = u32::try_from(armed.labels.len()).unwrap_or(CAPACITY);
        if count == 0 {
            return;
        }
        encoder.resolve_query_set(&armed.queries, 0..count, &self.resolved, 0);
        encoder.copy_buffer_to_buffer(&self.resolved, 0, &self.readback, 0, u64::from(count) * 8);
        // Only the slots a render-pass mark opened: a compute or `BETWEEN` span wrote none.
        if let Some(fragments) = &armed.fragments
            && armed.slots > 0
        {
            let slots = armed.slots;
            encoder.resolve_query_set(fragments, 0..slots, &self.fragments_resolved, 0);
            encoder.copy_buffer_to_buffer(
                &self.fragments_resolved,
                0,
                &self.fragments_readback,
                0,
                u64::from(slots) * 8,
            );
        }
    }

    /// The submitted recording's spans - each label's runs from its mark to
    /// the next one - with the fragment invocations in it where the device
    /// counts them. Blocks on the device.
    #[must_use]
    pub fn read(&self, device: &wgpu::Device) -> Vec<Span> {
        let (labels, counted, slots) = ARMED.with_borrow(|armed| {
            armed
                .as_ref()
                .map(|a| (a.labels.clone(), a.counted.clone(), a.slots))
                .unwrap_or_default()
        });
        if labels.len() < 2 {
            return Vec::new();
        }
        let ticks = read_u64s(device, &self.readback, labels.len());
        let fragments = if slots > 0 {
            read_u64s(device, &self.fragments_readback, slots as usize)
        } else {
            Vec::new()
        };
        spans(&labels, &ticks, &counted, &fragments, self.period)
    }
}

/// One span per mark but the last, which only ends the span before it: from
/// that mark's tick to the next, with the fragments of the slot it opened.
fn spans(
    labels: &[&'static str],
    ticks: &[u64],
    counted: &[Option<u32>],
    fragments: &[u64],
    period: f32,
) -> Vec<Span> {
    labels
        .windows(2)
        .zip(ticks.windows(2))
        .zip(counted)
        .map(|((label, tick), slot)| {
            let span = tick[1].saturating_sub(tick[0]) as f64 * f64::from(period);
            Span {
                label: label[0],
                micros: span / 1000.0,
                fragments: slot.and_then(|slot| fragments.get(slot as usize).copied()),
            }
        })
        .collect()
}

/// How many marks the last recording took past [`CAPACITY`] and wrote nothing
/// for, so a frame whose spans come up short says why.
#[must_use]
pub fn dropped() -> u32 {
    ARMED.with_borrow(|armed| armed.as_ref().map_or(0, |a| a.dropped))
}

/// One labelled stretch of a pass.
#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub label: &'static str,
    pub micros: f64,
    /// Fragment-shader invocations, where the device counts them.
    pub fragments: Option<u64>,
}

fn read_u64s(device: &wgpu::Device, buffer: &wgpu::Buffer, count: usize) -> Vec<u64> {
    let slice = buffer.slice(..count as u64 * 8);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    let Ok(view) = slice.get_mapped_range() else {
        return Vec::new();
    };
    let values = bytemuck::cast_slice(&view).to_vec();
    drop(view);
    buffer.unmap();
    values
}

impl Drop for Marks {
    fn drop(&mut self) {
        ARMED.set(None);
    }
}

/// Writes a timestamp named `label` into `pass`, if a [`Marks`] is armed: the
/// start of a span that runs to the next mark. A pass that marks must [`close`]
/// as its last command.
#[inline]
pub fn mark(pass: &mut wgpu::RenderPass<'_>, label: &'static str) {
    if !cfg!(feature = "perf-probe") {
        return;
    }
    ARMED.with_borrow_mut(|armed| {
        if let Some(armed) = armed.as_mut() {
            mark_armed(armed, pass, label, true);
        }
    });
}

/// Ends the span a [`mark`] opened on `pass` and opens a [`BETWEEN`] span that
/// the next pass's first mark ends. A statistics query cannot span two passes,
/// so every marked pass closes before it ends.
#[inline]
pub fn close(pass: &mut wgpu::RenderPass<'_>) {
    if !cfg!(feature = "perf-probe") {
        return;
    }
    ARMED.with_borrow_mut(|armed| {
        if let Some(armed) = armed.as_mut() {
            mark_armed(armed, pass, BETWEEN, false);
        }
    });
}

/// [`mark`] for a compute pass. Counts no fragments.
#[inline]
pub fn mark_compute(pass: &mut wgpu::ComputePass<'_>, label: &'static str) {
    if !cfg!(feature = "perf-probe") {
        return;
    }
    ARMED.with_borrow_mut(|armed| {
        if let Some(armed) = armed.as_mut()
            && let Some(index) = take_index(armed, label, None)
        {
            pass.write_timestamp(&armed.queries, index);
        }
    });
}

/// [`close`] for a compute pass.
#[inline]
pub fn close_compute(pass: &mut wgpu::ComputePass<'_>) {
    mark_compute(pass, BETWEEN);
}

/// The query index for a new mark, or `None` (counted as dropped) when the
/// recording is full.
fn take_index(armed: &mut Armed, label: &'static str, slot: Option<u32>) -> Option<u32> {
    let index = u32::try_from(armed.labels.len()).unwrap_or(CAPACITY);
    if index >= CAPACITY {
        armed.dropped += 1;
        return None;
    }
    armed.labels.push(label);
    armed.counted.push(slot);
    Some(index)
}

/// `count` opens a fragment count for the new span (a [`BETWEEN`] span opens
/// none: it has no draws).
fn mark_armed(
    armed: &mut Armed,
    pass: &mut wgpu::RenderPass<'_>,
    label: &'static str,
    count: bool,
) {
    if armed.labels.len() >= CAPACITY as usize {
        armed.dropped += 1;
        return;
    }
    let mut slot = None;
    if let Some(fragments) = &armed.fragments {
        if armed.open {
            pass.end_pipeline_statistics_query();
            armed.open = false;
        }
        if count {
            pass.begin_pipeline_statistics_query(fragments, armed.slots);
            slot = Some(armed.slots);
            armed.slots += 1;
            armed.open = true;
        }
    }
    if let Some(index) = take_index(armed, label, slot) {
        pass.write_timestamp(&armed.queries, index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_span_runs_to_the_next_mark_and_the_last_mark_opens_none() {
        let labels = ["shadow", BETWEEN, "race", BETWEEN];
        let ticks = [1_000, 3_000, 3_500, 9_500];
        let counted = [Some(0), None, Some(1), None];
        let spans = spans(&labels, &ticks, &counted, &[40, 70], 1.0);
        let read: Vec<_> = spans
            .iter()
            .map(|s| (s.label, s.micros, s.fragments))
            .collect();
        assert_eq!(
            read,
            [
                ("shadow", 2.0, Some(40)),
                (BETWEEN, 0.5, None),
                ("race", 6.0, Some(70)),
            ]
        );
    }

    /// A compute or [`BETWEEN`] span opened no statistics query, so a slot read
    /// back belongs to the next render-pass span, not to the one at its index.
    #[test]
    fn fragments_follow_the_slot_a_span_opened_not_its_index() {
        let spans = spans(
            &["coefficients", "apply", BETWEEN],
            &[0, 1_000, 2_000],
            &[None, Some(0), None],
            &[12],
            1.0,
        );
        assert_eq!(spans[0].fragments, None);
        assert_eq!(spans[1].fragments, Some(12));
    }

    #[test]
    fn nothing_is_dropped_unarmed() {
        assert_eq!(dropped(), 0);
    }
}
