//! What each separately-encoded pass cost, by the label its site gave it.
//!
//! The rows of [`super::GpuCost`] that come from [`oag_gpu::timing::PassTimer::with_passes`]:
//! the glass and sea target, the shadow passes, the probe blend, the post passes and the HUD.
//! `oag-gpu` knows a pass only by the `&'static str` its site passes; this module is what
//! names a row on the overlay and in what order.

/// The longest a label list [`PassRows`] holds. More than the build has passes; a label past it
/// is dropped from the overlay and still counted by [`PassRows::total`] of what fitted.
pub const MAX_ROWS: usize = 24;

/// Frames a pass's mean is taken over.
const WINDOW: usize = 120;

/// How a pass's label becomes a row: the label's prefix and the row's name, in the order the
/// overlay draws them. A label that starts with none of them is drawn after these under its
/// own text.
///
/// Prefixes rather than whole labels because the shared full-screen pass helper stamps with
/// each pass's own label (the PSP bloom's lanes are several), and a row is a *group* of
/// passes. The PS2 prefix comes before the PSP one it contains.
const ROWS: &[(&str, &str)] = &[
    ("glass", "GLASS"),
    ("advert card", "ADVERT CARDS"),
    ("psys distort", "DISTORT"),
    ("shadow map", "SHADOW MAP"),
    ("self shadow", "SELF SHADOW"),
    ("shadow occlusion", "OCCLUSION"),
    ("probe blend", "PROBE BLEND"),
    ("lens flare", "LENS FLARE"),
    ("ps2 bloom", "PS2 BLOOM"),
    ("bloom", "PSP BLOOM"),
    ("smaa", "SMAA"),
    ("fxaa", "FXAA"),
    ("fsr1", "FSR1"),
    ("fsr3 clear", "FSR3 CLEAR"),
    ("hd zoom", "HD ZOOM"),
    ("omega", "OMEGA TONEMAP"),
    ("wo composite", "WO COMPOSITE"),
    ("photo fx", "PHOTO FX"),
    ("composite", "COMPOSITE"),
    ("blit", "BLIT"),
    ("backdrop", "BACKDROP"),
    ("menu scene", "MENU SCENE"),
    ("hud", "HUD AND UI"),
];

/// The row a label belongs to: its position in the overlay's order and its name, or `None` for a
/// label no row claims.
fn row_of(label: &str) -> Option<(usize, &'static str)> {
    ROWS.iter()
        .enumerate()
        .find(|(_, (prefix, _))| label.starts_with(prefix))
        .map(|(index, (_, name))| (index, *name))
}

/// Whether two labels are one row.
fn same_row(a: &'static str, b: &'static str) -> bool {
    row_of(a).map_or(a, |(_, name)| name) == row_of(b).map_or(b, |(_, name)| name)
}

/// One row's two readings, in seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Reading {
    /// The mean over every frame of the window, a frame the pass did not run in counting zero.
    per_frame: f32,
    /// The mean over only the frames the pass ran in; zero when it ran in none of them.
    when_run: f32,
}

/// One frame's passes by row, in seconds: what [`super::GpuCost`] carries.
///
/// Fixed-size so `GpuCost` stays `Copy`. Rows are by **row name**, so two labels of one row
/// are one entry.
///
/// Each row holds two means. The **per-frame** one counts a frame without the pass as zero, so
/// [`Self::total`] is what the passes cost a frame on average and `frame - total` stays honest;
/// [`Self::ordered`] reads it too, for the frame log. The **when-run** one is what a pass costs
/// on a frame it runs in, for the overlay ([`Self::ordered_when_run`]): a pass that runs a
/// second in every few would otherwise read as a fraction of its cost.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PassRows {
    rows: [Option<(&'static str, Reading)>; MAX_ROWS],
}

impl PassRows {
    /// Adds `seconds` to the row `label` belongs to, naming a label no row claims after itself.
    /// A pass added is a pass that ran, so both of the row's means take `seconds`.
    pub fn add(&mut self, label: &'static str, seconds: f32) {
        self.insert(
            label,
            Reading {
                per_frame: seconds,
                when_run: seconds,
            },
        );
    }

    fn insert(&mut self, label: &'static str, reading: Reading) {
        let name = row_of(label).map_or(label, |(_, name)| name);
        for slot in &mut self.rows {
            match slot {
                Some((known, total)) if *known == name => {
                    total.per_frame += reading.per_frame;
                    total.when_run += reading.when_run;
                    return;
                }
                None => {
                    *slot = Some((name, reading));
                    return;
                }
                Some(_) => {}
            }
        }
    }

    /// Whether no row has a reading.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows[0].is_none()
    }

    /// Every row's per-frame seconds added up: what [`super::GpuCost::residual_ms`] takes off the
    /// frame. Never the when-run means, which would take a pass's cost off frames it did not
    /// run in.
    #[must_use]
    pub fn total(&self) -> f32 {
        self.rows
            .iter()
            .flatten()
            .map(|(_, reading)| reading.per_frame)
            .sum()
    }

    /// The rows in the overlay's order: the known ones as [`ROWS`] lists them, then any other by
    /// name. Each is `(name, per-frame seconds)`, which is what the frame log's `pass_ms` prints.
    #[must_use]
    pub fn ordered(&self) -> Vec<(&'static str, f32)> {
        self.sorted(|reading| reading.per_frame)
    }

    /// The same rows and order as [`Self::ordered`], each with its mean over the frames it ran
    /// in, which is what the overlay shows. A pass that ran in none of the window reads zero,
    /// which the overlay's hide threshold leaves off.
    #[must_use]
    pub fn ordered_when_run(&self) -> Vec<(&'static str, f32)> {
        self.sorted(|reading| reading.when_run)
    }

    fn sorted(&self, seconds: impl Fn(&Reading) -> f32) -> Vec<(&'static str, f32)> {
        let mut rows: Vec<(usize, &'static str, f32)> = self
            .rows
            .iter()
            .flatten()
            .map(|(name, reading)| {
                let index = ROWS
                    .iter()
                    .position(|(_, row)| row == name)
                    .unwrap_or(ROWS.len());
                (index, *name, seconds(reading))
            })
            .collect();
        rows.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
        rows.into_iter()
            .map(|(_, name, seconds)| (name, seconds))
            .collect()
    }
}

/// One label's last [`WINDOW`] frames, zeros included.
///
/// Not a [`super::Meter`]: that drops a zero, and a pass that did not run on a frame cost the
/// frame nothing, which is what keeps `frame - timed` honest on a frame without it.
#[derive(Debug, Clone)]
struct Window {
    label: &'static str,
    samples: [f32; WINDOW],
    len: usize,
    next: usize,
}

impl Window {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            samples: [0.0; WINDOW],
            len: 0,
            next: 0,
        }
    }

    fn push(&mut self, seconds: f32) {
        self.samples[self.next] = seconds;
        self.next = (self.next + 1) % WINDOW;
        self.len = (self.len + 1).min(WINDOW);
    }

    fn mean(&self) -> f32 {
        self.samples[..self.len.min(WINDOW)].iter().sum::<f32>() / self.len.max(1) as f32
    }

    /// The sample `back` frames before the newest, `None` past what has been recorded.
    fn back(&self, back: usize) -> Option<f32> {
        (back < self.len).then(|| self.samples[(self.next + WINDOW - 1 - back) % WINDOW])
    }
}

/// The mean over only the frames any of `labels` ran in, zero when none did.
///
/// A row can be several labels (the PSP bloom's lanes), so a frame counts once if any of them
/// ran, and its cost is theirs added: the windows are aligned on the newest frame, which every
/// one of them records each frame. A sample is a frame the pass ran in when it is above zero,
/// since a reading that is not is never recorded as one (`record_passes`).
fn mean_when_run(labels: &[&Window]) -> f32 {
    let longest = labels.iter().map(|window| window.len).max().unwrap_or(0);
    let (mut sum, mut ran) = (0.0, 0_u32);
    for back in 0..longest {
        let frame: f32 = labels
            .iter()
            .filter_map(|window| window.back(back))
            .filter(|seconds| *seconds > 0.0)
            .sum();
        if frame > 0.0 {
            sum += frame;
            ran += 1;
        }
    }
    if ran == 0 { 0.0 } else { sum / ran as f32 }
}

/// The overlay's and the frame log's per-pass means: each row over every frame of the window, and
/// over the frames it ran in.
#[derive(Debug, Clone, Default)]
pub struct PassCosts {
    windows: Vec<Window>,
    /// The newest frame whose readings are no longer wanted.
    floor: u64,
}

impl PassCosts {
    /// An empty set, which reports nothing until a frame has been recorded.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// One whole frame's readings, `(label, seconds)`: every label that has ever been seen and
    /// is not in `readings` records zero for this frame, because the pass was not encoded.
    ///
    /// **Whole frames only**: a frame's readings recorded a label at a time would put a zero
    /// against the labels not yet recorded. A frame at or before the floor is dropped.
    pub fn record_frame(&mut self, frame: u64, readings: &[(&'static str, f32)]) {
        if frame <= self.floor {
            return;
        }
        for &(label, _) in readings {
            if !self.windows.iter().any(|window| window.label == label) {
                self.windows.push(Window::new(label));
            }
        }
        for window in &mut self.windows {
            let seconds = readings
                .iter()
                .find(|(label, _)| *label == window.label)
                .map_or(0.0, |&(_, seconds)| seconds);
            window.push(seconds);
        }
    }

    /// A load or stall on `frame`: forget what was recorded and refuse every reading up to it.
    pub fn stalled(&mut self, frame: u64) {
        self.windows.clear();
        self.floor = self.floor.max(frame);
    }

    /// Every row's two means, in seconds, for [`super::GpuCost`]: over every frame of the window
    /// and over only the frames the pass ran in ([`PassRows`]). Empty before the first frame.
    #[must_use]
    pub fn rows(&self) -> PassRows {
        let mut rows = PassRows::default();
        let mut grouped: Vec<&Window> = Vec::new();
        for (at, window) in self.windows.iter().enumerate() {
            if self.windows[..at]
                .iter()
                .any(|earlier| same_row(earlier.label, window.label))
            {
                continue;
            }
            grouped.clear();
            grouped.extend(
                self.windows[at..]
                    .iter()
                    .filter(|other| same_row(other.label, window.label)),
            );
            rows.insert(
                window.label,
                Reading {
                    per_frame: grouped.iter().map(|window| window.mean()).sum(),
                    when_run: mean_when_run(&grouped),
                },
            );
        }
        rows
    }
}
