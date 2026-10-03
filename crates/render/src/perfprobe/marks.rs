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
//! Only armed while a bench holds a [`Marks`]: [`mark`] is a no-op otherwise,
//! and the whole module folds away without the feature.

use std::sync::Mutex;

/// The most marks one recording can take.
const CAPACITY: u32 = 64;

struct Armed {
    queries: wgpu::QuerySet,
    labels: Vec<&'static str>,
}

static ARMED: Mutex<Option<Armed>> = Mutex::new(None);

/// A query set [`mark`] writes into while this is alive.
#[derive(Debug)]
pub struct Marks {
    resolved: wgpu::Buffer,
    readback: wgpu::Buffer,
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
        *ARMED.lock().ok()? = Some(Armed {
            queries,
            labels: Vec::new(),
        });
        Some(Self {
            resolved: buffer(
                "perf marks resolved",
                wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            ),
            readback: buffer(
                "perf marks readback",
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            ),
            period: queue.get_timestamp_period(),
        })
    }

    /// Clears the labels for a fresh recording.
    pub fn begin(&self) {
        if let Ok(mut armed) = ARMED.lock()
            && let Some(armed) = armed.as_mut()
        {
            armed.labels.clear();
        }
    }

    /// Resolves this recording's marks into the readback buffer.
    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        let Ok(armed) = ARMED.lock() else { return };
        let Some(armed) = armed.as_ref() else { return };
        let count = u32::try_from(armed.labels.len()).unwrap_or(CAPACITY);
        if count == 0 {
            return;
        }
        encoder.resolve_query_set(&armed.queries, 0..count, &self.resolved, 0);
        encoder.copy_buffer_to_buffer(&self.resolved, 0, &self.readback, 0, u64::from(count) * 8);
    }

    /// The submitted recording's spans, as `(label, microseconds)` - each
    /// label's span runs from its mark to the next one. Blocks on the device.
    #[must_use]
    pub fn read(&self, device: &wgpu::Device) -> Vec<(&'static str, f64)> {
        let labels = match ARMED.lock() {
            Ok(armed) => armed.as_ref().map(|a| a.labels.clone()).unwrap_or_default(),
            Err(_) => return Vec::new(),
        };
        if labels.len() < 2 {
            return Vec::new();
        }
        let slice = self.readback.slice(..labels.len() as u64 * 8);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let Ok(view) = slice.get_mapped_range() else {
            return Vec::new();
        };
        let ticks: Vec<u64> = bytemuck::cast_slice(&view).to_vec();
        drop(view);
        self.readback.unmap();
        labels
            .windows(2)
            .zip(ticks.windows(2))
            .map(|(label, tick)| {
                let span = tick[1].saturating_sub(tick[0]) as f64 * f64::from(self.period);
                (label[0], span / 1000.0)
            })
            .collect()
    }
}

impl Drop for Marks {
    fn drop(&mut self) {
        if let Ok(mut armed) = ARMED.lock() {
            *armed = None;
        }
    }
}

/// Writes a timestamp named `label` into `pass`, if a [`Marks`] is armed.
#[inline]
pub fn mark(pass: &mut wgpu::RenderPass<'_>, label: &'static str) {
    if !cfg!(feature = "perf-probe") {
        return;
    }
    let Ok(mut armed) = ARMED.lock() else { return };
    let Some(armed) = armed.as_mut() else { return };
    let Ok(index) = u32::try_from(armed.labels.len()) else {
        return;
    };
    if index >= CAPACITY {
        return;
    }
    pass.write_timestamp(&armed.queries, index);
    armed.labels.push(label);
}
