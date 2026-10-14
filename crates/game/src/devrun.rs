//! Developer switches for measuring the front end without a person at it: pre-seated
//! people, a timed button script and a once-a-second frame-time line.
//!
//! All three are off by default and none is a setting. They exist because the screens a
//! multiplayer measurement needs (WINDOW LAYOUT after the join screen) take two joined
//! pads to reach, and a measurement over ssh has no hands. Everything here is plain data;
//! the composition root (`session::devrun`) feeds it. **Chosen, not measured**: the
//! placeholder devices, the script syntax and the line's fields.

use oag_gameplay::input::Button;
use oag_input::lobby::Device;
use oag_input::pad::KeyboardId;

/// The first id a placeholder keyboard seat takes. Far above any id the platform hands out
/// (a raw device index), so a real keyboard never meets a placeholder's seat.
pub const PLACEHOLDER_BASE: u32 = 0x4F41_4700;

/// The devices that seat `people` people without a real pad: seat 0 is the anonymous
/// keyboard (so the real keyboard can drive the screens), every further seat a placeholder
/// keyboard that no key ever matches.
///
/// A real pad connecting later takes the next free seat on the join screen the normal way
/// when the grid has one left; with every seat dealt it cannot, until a seat is freed.
#[must_use]
pub fn placeholder_devices(people: usize) -> Vec<Device> {
    (0..people)
        .map(|seat| {
            if seat == 0 {
                Device::Keyboard
            } else {
                Device::Keys(KeyboardId(PLACEHOLDER_BASE + seat as u32))
            }
        })
        .collect()
}

/// What a script step does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// A tap of this button: down for one frame, up on the next.
    Tap(Button),
    /// End the run (the window closes, the log flushes).
    Quit,
}

/// A timed script: `SECONDS:BUTTON` entries, comma separated, e.g. `3:down,3.5:down,4:right,10:quit`.
///
/// Seconds count from the first frame the script is polled. An entry written `+SECONDS:BUTTON`
/// counts from the moment `--preseat` lands instead (from the first frame without it), so a
/// measurement does not depend on how long the boot took; the plain taps that got it through
/// the boot are dropped once it has. `quit` ends the run, so a
/// measurement under `timeout` still flushes its log.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Script {
    /// `(seconds, counted from the anchor, step, done)`, in time order within each base.
    steps: Vec<(f32, bool, Step, bool)>,
}

impl Script {
    /// Parses `text`.
    ///
    /// # Errors
    ///
    /// Names the first entry that is not `[+]SECONDS:BUTTON`, a negative time, or an unknown button.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut steps = Vec::new();
        for entry in text.split(',').map(str::trim).filter(|e| !e.is_empty()) {
            let (at, what) = entry
                .split_once(':')
                .ok_or_else(|| format!("{entry:?} is not SECONDS:BUTTON"))?;
            let (relative, at) = at
                .trim()
                .strip_prefix('+')
                .map_or((false, at.trim()), |rest| (true, rest));
            let at: f32 = at
                .parse()
                .map_err(|_| format!("{entry:?}: {at:?} is not a number of seconds"))?;
            if !at.is_finite() || at < 0.0 {
                return Err(format!("{entry:?}: time must be zero or more"));
            }
            let what = what.trim();
            let step = if what.eq_ignore_ascii_case("quit") {
                Step::Quit
            } else {
                Step::Tap(
                    oag_core::buttons::button_from_name(what)
                        .ok_or_else(|| format!("{entry:?}: {what:?} is not a button name or quit"))?,
                )
            };
            steps.push((at, relative, step, false));
        }
        steps.sort_by(|a, b| a.0.total_cmp(&b.0));
        Ok(Self { steps })
    }

    /// Whether the script has anything to do.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The steps now due, each handed out once: `elapsed` seconds since the first frame, and
    /// `anchored` seconds since `--preseat` landed (`None` before it has, which holds the
    /// `+` entries back).
    pub fn due(&mut self, elapsed: f32, anchored: Option<f32>) -> Vec<Step> {
        let mut out = Vec::new();
        for (at, relative, step, done) in &mut self.steps {
            // A plain tap is meant for the boot (language, logo); once `--preseat` has landed
            // it would hit the layout screen's rows instead, so it is dropped.
            if !*relative && anchored.is_some() && matches!(step, Step::Tap(_)) {
                *done = true;
            }
            let clock = if *relative { anchored } else { Some(elapsed) };
            if !*done && clock.is_some_and(|clock| *at <= clock) {
                *done = true;
                out.push(*step);
            }
        }
        out
    }
}

/// What the once-a-second line reads, all the overlay's own figures.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading<'a> {
    /// The stage's name and, for a menu, the screen over it.
    pub stage: &'a str,
    /// The meter's frames per second.
    pub fps: f32,
    /// The meter's mean frame time, milliseconds.
    pub frame_ms: f32,
    /// The meter's 99th percentile, milliseconds.
    pub p99_ms: f32,
    /// The wall-clock CPU cost, milliseconds, when read.
    pub cpu_ms: Option<f32>,
    /// The scene's GPU cost, milliseconds, when read.
    pub gpu_ms: Option<f32>,
    /// The swapchain present cost, milliseconds, when read.
    pub present_ms: Option<f32>,
    /// The rate the loop is trying to present at.
    pub target_hz: u32,
    /// The window's inner size.
    pub window: (u32, u32),
    /// The surface (swapchain) size.
    pub surface: (u32, u32),
    /// The render extent, when a scene was drawn.
    pub render: Option<(u32, u32)>,
}

/// The line, one `key=value` field after another so a script can split it.
#[must_use]
pub fn format_line(r: &Reading<'_>) -> String {
    let ms = |v: Option<f32>| v.map_or_else(|| "-".to_string(), |v| format!("{v:.2}"));
    let render = r
        .render
        .map_or_else(|| "-".to_string(), |(w, h)| format!("{w}x{h}"));
    format!(
        "frame-log stage={} fps={:.1} frame_ms={:.2} p99_ms={:.2} cpu_ms={} gpu_ms={} present_ms={} target_hz={} window={}x{} surface={}x{} render={}",
        r.stage,
        r.fps,
        r.frame_ms,
        r.p99_ms,
        ms(r.cpu_ms),
        ms(r.gpu_ms),
        ms(r.present_ms),
        r.target_hz,
        r.window.0,
        r.window.1,
        r.surface.0,
        r.surface.1,
        render,
    )
}

/// The three switches a run carries.
#[derive(Debug, Clone, Default)]
pub struct Dev {
    /// `--preseat`: this many people seated and the WINDOW LAYOUT screen opened, once the
    /// menus are up. Taken when it fires.
    pub preseat: Option<usize>,
    /// `--script`.
    pub script: Script,
    /// `--frame-log` or `OAG_FRAME_LOG`.
    pub frame_log: bool,
}

#[cfg(test)]
mod tests;
