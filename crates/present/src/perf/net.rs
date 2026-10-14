//! The network block of the `Dev` overlay: what a network race reports about
//! itself, drawn under the frame-time panel.
//!
//! Plain data in, [`Draw`]s out, the same shape as [`super::SceneStats`]: this
//! crate knows no transport, so the composition root maps `oag_net`'s
//! `NetStats` into a [`Readout`]. Values are the per-second figures the log line
//! prints, so the panel and the log agree by construction, and only the sparkline
//! moves faster.
//!
//! Layout is chosen, not measured. Rows are short because the grid is 272 high and
//! the frame-time and cost panels already use most of a corner.

use oag_ui::frontend::{Align, Draw};

use super::{BAD, LINE, PAD, PANEL, TEXT, WARN};

/// The panel's width in grid units: 30 glyphs of the overlay font. Chosen.
pub const WIDTH: f32 = 192.0;
/// The sparkline's height. Chosen.
const SPARK_H: f32 = 22.0;
/// The sparkline's scale ceiling floor, milliseconds: a quiet link is not blown up
/// to fill the box. Chosen.
const SPARK_FLOOR_MS: f32 = 50.0;
const SPARK: [f32; 4] = [0.35, 0.86, 0.45, 0.9];

/// One second of a network race, as the overlay reads it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Readout {
    /// QUIC's smoothed round trip, milliseconds.
    pub rtt_ms: Option<f32>,
    /// Input sent to tick confirmed, milliseconds.
    pub confirm_ms: Option<f32>,
    /// Change between consecutive confirm times, milliseconds.
    pub jitter_ms: Option<f32>,
    /// Bytes per second received.
    pub rx_bytes_s: f32,
    /// Bytes per second sent.
    pub tx_bytes_s: f32,
    /// Datagrams per second received.
    pub rx_datagrams_s: f32,
    /// Datagrams per second sent.
    pub tx_datagrams_s: f32,
    /// Percent of QUIC packets lost.
    pub loss_percent: Option<f32>,
    /// Congestion window, bytes.
    pub cwnd: Option<u64>,
    /// The server's input buffer depth for this seat, ticks.
    pub lead: i16,
    /// Present tick less confirmed tick.
    pub window: u32,
    /// The confirmed (server) tick.
    pub server_tick: u32,
    /// The present tick.
    pub local_tick: u32,
    /// Corrections this second.
    pub corrections: u32,
    /// Ticks replayed this second.
    pub replayed: u32,
    /// Largest correction this second, world units.
    pub largest_pop: f32,
    /// Largest prediction error this second, world units.
    pub largest_error: f32,
    /// Hash matches so far.
    pub hash_ok: u64,
    /// Hash mismatches so far; any is a bug.
    pub hash_bad: u64,
    /// Stalled steps this second.
    pub stalls: u32,
    /// Resends asked for this second.
    pub resends: u32,
    /// `--net-shim`: configured round trip in ms, loss percent and datagrams dropped.
    pub shim: Option<(f32, f32, u64)>,
    /// The confirm-time sparkline, oldest first, milliseconds.
    pub history: Vec<f32>,
}

fn kib(bytes_s: f32) -> String {
    format!("{:.1}", bytes_s / 1000.0)
}

fn ms(value: Option<f32>) -> String {
    value.map_or_else(|| "-".to_string(), |v| format!("{v:.0}"))
}

impl Readout {
    /// The sparkline's full height in milliseconds: its tallest column, never under
    /// [`SPARK_FLOOR_MS`].
    fn ceiling(&self) -> f32 {
        self.history.iter().copied().fold(SPARK_FLOOR_MS, f32::max)
    }

    /// The text rows, each with its colour.
    #[must_use]
    pub fn rows(&self) -> Vec<(String, [f32; 4])> {
        let mut rows = vec![
            (
                format!(
                    "NET RTT {} MS  CONF {}",
                    self.rtt_ms
                        .map_or_else(|| "-".to_string(), |v| format!("{v:.1}")),
                    ms(self.confirm_ms)
                ),
                TEXT,
            ),
            (
                format!(
                    "JITTER {} MS  LOSS {}",
                    self.jitter_ms
                        .map_or_else(|| "-".to_string(), |v| format!("{v:.1}")),
                    self.loss_percent
                        .map_or_else(|| "-".to_string(), |v| format!("{v:.1}%"))
                ),
                TEXT,
            ),
            (
                format!(
                    "RX {} KB/S {:.0} D/S",
                    kib(self.rx_bytes_s),
                    self.rx_datagrams_s
                ),
                TEXT,
            ),
            (
                format!(
                    "TX {} KB/S {:.0} D/S",
                    kib(self.tx_bytes_s),
                    self.tx_datagrams_s
                ),
                TEXT,
            ),
            (
                format!(
                    "LEAD {}  WIN {}  CWND {}",
                    self.lead,
                    self.window,
                    self.cwnd
                        .map_or_else(|| "-".to_string(), |c| format!("{}K", c / 1024))
                ),
                TEXT,
            ),
            (
                format!("TICK SRV {} / ME {}", self.server_tick, self.local_tick),
                TEXT,
            ),
            (
                format!(
                    "CORR {} REPLAY {} POP {:.2}",
                    self.corrections, self.replayed, self.largest_pop
                ),
                if self.corrections > 0 { WARN } else { TEXT },
            ),
            (
                format!(
                    "ERR {:.2}  STALL {} RESEND {}",
                    self.largest_error, self.stalls, self.resends
                ),
                if self.stalls > 0 { WARN } else { TEXT },
            ),
            (
                format!("HASH OK {} BAD {}", self.hash_ok, self.hash_bad),
                if self.hash_bad > 0 { BAD } else { TEXT },
            ),
        ];
        rows.push((format!("CONF GRAPH 0-{:.0} MS", self.ceiling()), TEXT));
        if let Some((rtt, loss, dropped)) = self.shim {
            rows.push((format!("SHIM {rtt:.0}MS {loss:.0}% DROP {dropped}"), WARN));
            rows.push(("SHIM IS A TEST CONDITION".to_string(), WARN));
        }
        rows
    }
}

/// The panel at the right edge `right`, from `top`, and its height so a caller
/// can stack something under it. Empty `Vec` and zero when `readout` is `None`.
#[must_use]
pub fn draw_list(readout: Option<&Readout>, right: f32, top: f32) -> (Vec<Draw>, f32) {
    let Some(readout) = readout else {
        return (Vec::new(), 0.0);
    };
    let rows = readout.rows();
    let text_h = rows.len() as f32 * LINE;
    let height = PAD * 2.0 + text_h + SPARK_H + PAD;
    let left = right - WIDTH;
    let mut out = vec![Draw::Fill {
        rect: [left, top, WIDTH, height],
        color: PANEL,
    }];
    for (row, (text, color)) in rows.into_iter().enumerate() {
        out.push(Draw::Text {
            x: left + PAD,
            y: top + PAD + row as f32 * LINE,
            scale: 1.0,
            color,
            border: None,
            align: Align::Left,
            text,
            wrap_width: None,
        });
    }
    let bottom = top + height - PAD;
    let ceiling = readout.ceiling();
    let columns = (WIDTH - PAD * 2.0) as usize;
    let skip = readout.history.len().saturating_sub(columns);
    for (column, value) in readout.history.iter().skip(skip).enumerate() {
        let h = (value / ceiling).clamp(0.0, 1.0) * SPARK_H;
        if h <= 0.0 {
            continue;
        }
        out.push(Draw::Fill {
            rect: [left + PAD + column as f32, bottom - h, 1.0, h],
            color: SPARK,
        });
    }
    (out, height)
}

#[cfg(test)]
mod tests {
    use super::super::{CpuCost, GpuCost, Meter, Overlay, WINDOW};
    use super::*;

    fn texts(list: &[Draw]) -> Vec<(&str, [f32; 4])> {
        list.iter()
            .filter_map(|d| match d {
                Draw::Text { text, color, .. } => Some((text.as_str(), *color)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn no_readout_draws_nothing() {
        let (list, height) = draw_list(None, 480.0, 0.0);
        assert!(list.is_empty());
        assert_eq!(height, 0.0);
    }

    #[test]
    fn the_rows_carry_the_figures_and_a_bad_hash_is_red() {
        let readout = Readout {
            rtt_ms: Some(0.4),
            confirm_ms: Some(118.0),
            hash_ok: 600,
            hash_bad: 2,
            ..Readout::default()
        };
        let (list, height) = draw_list(Some(&readout), 480.0, 0.0);
        assert!(height > 0.0);
        let rows = texts(&list);
        assert!(
            rows.iter()
                .any(|(t, _)| t.contains("RTT 0.4 MS") && t.contains("CONF 118"))
        );
        let hash = rows
            .iter()
            .find(|(t, _)| t.starts_with("HASH"))
            .expect("hash row");
        assert_eq!(hash.1, BAD);
        let clean = Readout {
            hash_bad: 0,
            ..readout
        };
        let (list, _) = draw_list(Some(&clean), 480.0, 0.0);
        let rows = texts(&list);
        assert_eq!(
            rows.iter().find(|(t, _)| t.starts_with("HASH")).unwrap().1,
            TEXT
        );
    }

    #[test]
    fn the_shim_is_labelled_as_a_test_condition() {
        let readout = Readout {
            shim: Some((100.0, 2.0, 14)),
            ..Readout::default()
        };
        let rows = readout.rows();
        assert!(rows.iter().any(|(t, _)| t == "SHIM 100MS 2% DROP 14"));
        assert!(rows.iter().any(|(t, _)| t.contains("TEST CONDITION")));
        assert!(
            Readout::default()
                .rows()
                .iter()
                .all(|(t, _)| !t.starts_with("SHIM"))
        );
    }

    #[test]
    fn the_sparkline_draws_a_column_per_sample() {
        let readout = Readout {
            history: vec![10.0, 20.0, 30.0],
            ..Readout::default()
        };
        let (with, _) = draw_list(Some(&readout), 480.0, 0.0);
        let (without, _) = draw_list(Some(&Readout::default()), 480.0, 0.0);
        assert_eq!(with.len(), without.len() + 3);
    }

    #[test]
    fn the_dev_overlay_stacks_the_block_and_the_other_modes_leave_it_out() {
        let mut meter = Meter::new();
        for _ in 0..WINDOW {
            meter.record(1.0 / 60.0);
        }
        let readout = Readout::default();
        let draw = |mode, net| {
            super::super::draw_list(
                &meter,
                mode,
                60,
                None,
                None,
                None,
                None,
                GpuCost::default(),
                CpuCost::default(),
                net,
                480.0,
            )
        };
        let dev = draw(Overlay::Dev, Some(&readout));
        assert!(texts(&dev).iter().any(|(t, _)| t.starts_with("HASH")));
        assert!(
            !texts(&draw(Overlay::Dev, None))
                .iter()
                .any(|(t, _)| t.starts_with("HASH"))
        );
        assert!(
            !texts(&draw(Overlay::Pacing, Some(&readout)))
                .iter()
                .any(|(t, _)| t.starts_with("HASH"))
        );
    }
}
