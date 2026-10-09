//! Named GPU timestamps *inside* a render pass, so one pass's device time can
//! be split by what it drew.
//!
//! A [`crate::timing::PassTimer`] brackets a whole pass; the race pass is one
//! pass holding the sky, the track, its shine, the craft and every effect, so
//! its total says nothing about which of them costs. [`mark`] writes a
//! timestamp between draws, and the spans between consecutive marks are the
//! groups' costs. Inside-pass timestamps are an optional wgpu feature
//! (`TIMESTAMP_QUERY_INSIDE_PASSES`); `mesh_render::optional_features` asks for
//! it only with `perf-probe` on, and [`Marks::new`] declines without it.
//!
//! Where the device also offers pipeline statistics, each span counts its
//! fragment-shader invocations too, which divided by the viewport's pixels is
//! that group's overdraw.
//!
//! Only armed while a bench holds a [`Marks`]: [`mark`] is a no-op otherwise,
//! and the whole module folds away without the feature.

use std::cell::RefCell;

/// The most marks one recording can take.
const CAPACITY: u32 = 64;

struct Armed {
    queries: wgpu::QuerySet,
    /// One fragment-invocation count per span, when the device can.
    fragments: Option<wgpu::QuerySet>,
    /// Whether a statistics query is open on the pass being recorded.
    open: bool,
    labels: Vec<&'static str>,
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
        // The last mark closes the last span and opens none.
        if let Some(fragments) = &armed.fragments
            && count > 1
        {
            let spans = count - 1;
            encoder.resolve_query_set(fragments, 0..spans, &self.fragments_resolved, 0);
            encoder.copy_buffer_to_buffer(
                &self.fragments_resolved,
                0,
                &self.fragments_readback,
                0,
                u64::from(spans) * 8,
            );
        }
    }

    /// The submitted recording's spans - each label's runs from its mark to
    /// the next one - with the fragment invocations in it where the device
    /// counts them. Blocks on the device.
    #[must_use]
    pub fn read(&self, device: &wgpu::Device) -> Vec<Span> {
        let (labels, counted) = ARMED.with_borrow(|armed| {
            armed
                .as_ref()
                .map(|a| (a.labels.clone(), a.fragments.is_some()))
                .unwrap_or_default()
        });
        if labels.len() < 2 {
            return Vec::new();
        }
        let ticks = read_u64s(device, &self.readback, labels.len());
        let fragments = if counted {
            read_u64s(device, &self.fragments_readback, labels.len() - 1)
        } else {
            Vec::new()
        };
        labels
            .windows(2)
            .zip(ticks.windows(2))
            .enumerate()
            .map(|(i, (label, tick))| {
                let span = tick[1].saturating_sub(tick[0]) as f64 * f64::from(self.period);
                Span {
                    label: label[0],
                    micros: span / 1000.0,
                    fragments: fragments.get(i).copied(),
                }
            })
            .collect()
    }
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

/// Writes a timestamp named `label` into `pass`, if a [`Marks`] is armed.
#[inline]
pub fn mark(pass: &mut wgpu::RenderPass<'_>, label: &'static str) {
    if !cfg!(feature = "perf-probe") {
        return;
    }
    ARMED.with_borrow_mut(|armed| {
        if let Some(armed) = armed.as_mut() {
            mark_armed(armed, pass, label);
        }
    });
}

fn mark_armed(armed: &mut Armed, pass: &mut wgpu::RenderPass<'_>, label: &'static str) {
    let Ok(index) = u32::try_from(armed.labels.len()) else {
        return;
    };
    if index >= CAPACITY {
        return;
    }
    if let Some(fragments) = &armed.fragments {
        if armed.open {
            pass.end_pipeline_statistics_query();
        }
        // `end` closes the last span; every other mark opens one.
        armed.open = label != "end";
        if armed.open {
            pass.begin_pipeline_statistics_query(fragments, index);
        }
    }
    pass.write_timestamp(&armed.queries, index);
    armed.labels.push(label);
}

/// Ends the fragment count a [`mark`] opened on `pass`, for a pass that closes
/// before the next mark: a statistics query cannot span two passes, so a
/// chain of passes marks each one's start and closes each one's end.
#[inline]
pub fn close(pass: &mut wgpu::RenderPass<'_>) {
    if !cfg!(feature = "perf-probe") {
        return;
    }
    ARMED.with_borrow_mut(|armed| {
        if let Some(armed) = armed.as_mut()
            && armed.fragments.is_some()
            && armed.open
        {
            pass.end_pipeline_statistics_query();
            armed.open = false;
        }
    });
}
