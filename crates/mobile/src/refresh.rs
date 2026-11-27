//! The refresh rate of the display an Android app is on.
//!
//! winit's Android backend reports no rate, so the activity's `Display.getRefreshRate` is asked
//! over JNI, once a second on a thread of its own. It answers the rate the app's frames are shown
//! at: the active mode's, or the system's override for this app. A change (the player's display
//! setting, an adaptive panel) is followed at the next round; a `DisplayListener` would need a
//! Java class and the APK carries none (`docs/tools/android.md`, "The display's refresh rate").

use std::fmt::Display;

/// What a rate in Hz is in millihertz, or `None` for an answer no display gives (zero, a
/// negative, not a number).
#[must_use]
pub fn millihertz(hz: f32) -> Option<u32> {
    (hz.is_finite() && hz > 0.0)
        .then(|| (f64::from(hz) * 1000.0).round() as u32)
        .filter(|&mhz| mhz > 0)
}

/// One round of the poll: `ask` for the rate in Hz, answered in millihertz. A failure answers
/// `None`, which leaves the last rate standing (and before any, no rate: the caller falls back
/// as it does on a platform that reports none). The first failure is warned, each change logged.
pub fn reading<E: Display>(
    mut ask: impl FnMut() -> Result<f32, E> + Send + 'static,
) -> impl FnMut() -> Option<u32> + Send + 'static {
    let mut warned = false;
    let mut last = None;
    move || match ask() {
        Ok(hz) => {
            let rate = millihertz(hz)?;
            if last.replace(rate) != Some(rate) {
                log::debug!("Android reports {rate} mHz for the app's display");
            }
            Some(rate)
        }
        Err(why) => {
            if !std::mem::replace(&mut warned, true) {
                log::warn!("the display's refresh rate cannot be read: {why}");
            }
            None
        }
    }
}

/// A thread asking Android for the display's rate once a second.
#[cfg(target_os = "android")]
#[derive(Debug)]
pub struct LiveRefresh(oag_thread::poller::Poller);

#[cfg(target_os = "android")]
impl LiveRefresh {
    /// Starts asking; `None` when the activity cannot be reached or no thread starts.
    #[must_use]
    pub fn start() -> Option<Self> {
        let ask = crate::android::display_rate()
            .inspect_err(|why| log::warn!("the display's refresh rate cannot be read: {why}"))
            .ok()?;
        oag_thread::poller::Poller::start(
            "android-refresh",
            std::time::Duration::from_secs(1),
            reading(ask),
        )
        .inspect_err(|why| log::warn!("no thread to read the display's refresh rate: {why}"))
        .ok()
        .map(Self)
    }

    /// The last rate Android gave, `None` before its first answer.
    #[must_use]
    pub fn millihertz(&self) -> Option<u32> {
        self.0.latest()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_present::drs::Target;
    use oag_present::perf::FrameLimit;
    use oag_present::refresh::Display as Output;
    use std::time::Duration;

    /// The limit period and the target a fresh report of `hz` from Android comes to.
    fn follows(hz: f32) -> (Option<Duration>, Option<u32>) {
        let mut output = Output::new();
        output.report(millihertz(hz));
        (
            FrameLimit::FOLLOW_DISPLAY.resolve(output.hz()).period(),
            Target::FOLLOW_DISPLAY.resolve(output.hz()).hz(),
        )
    }

    #[test]
    fn a_phones_rate_reaches_the_frame_limit_and_the_target() {
        // The S21's four modes, as Android spells them.
        for (hz, whole) in [
            (120.000_01, 120),
            (96.000_01, 96),
            (60.000_004, 60),
            (48.000_004, 48),
        ] {
            let (period, target) = follows(hz);
            assert_eq!(target, Some(whole), "{hz}");
            let period = period.expect("a limit").as_secs_f64();
            assert!(
                (period * f64::from(whole) - 1.0).abs() < 1e-3,
                "{hz}: {period}"
            );
        }
    }

    #[test]
    fn an_answer_no_display_gives_is_none_and_falls_back_as_no_report_does() {
        for hz in [0.0, -60.0, f32::NAN, f32::INFINITY, 1e-9] {
            assert_eq!(millihertz(hz), None, "{hz}");
            assert_eq!(follows(hz), follows_nothing(), "{hz}");
        }
        assert_eq!(
            follows_nothing().1,
            Some(Target::FOLLOW_FALLBACK_HZ),
            "no report falls back to 60"
        );
    }

    fn follows_nothing() -> (Option<Duration>, Option<u32>) {
        let nothing = Output::new().hz();
        (
            FrameLimit::FOLLOW_DISPLAY.resolve(nothing).period(),
            Target::FOLLOW_DISPLAY.resolve(nothing).hz(),
        )
    }

    #[test]
    fn a_failed_read_answers_nothing_and_a_later_one_answers_again() {
        let mut answers = [
            Err("no activity"),
            Ok(120.000_01),
            Err("a Java exception"),
            Ok(60.000_004),
        ]
        .into_iter();
        let mut read = reading(move || answers.next().expect("an answer"));
        assert_eq!(read(), None);
        assert_eq!(read(), Some(120_000));
        assert_eq!(read(), None);
        assert_eq!(read(), Some(60_000));
    }
}
