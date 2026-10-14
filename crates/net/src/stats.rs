//! What a network race reports about itself, as plain data for the performance
//! overlay and the log.
//!
//! Two layers, both cheap. [`PathStats`] is the transport's own cumulative
//! counters, read through [`crate::link::Link::path`] once a second (QUIC's
//! smoothed round trip, bytes and datagrams on the wire, lost packets, the
//! congestion window). [`NetStats`] is one second of those differenced, joined to
//! the client's own counts ([`crate::client::Stats`]) and to the time an input
//! takes to come back confirmed. Nothing here allocates per packet: the client
//! counts into the `Stats` it already keeps, and a second's worth is assembled
//! once a second by [`Collector::second`].
//!
//! **The shim sits above QUIC.** Under `--net-shim` the transport's round trip and
//! loss read loopback, because the delay and the drops are added on this end, over
//! the socket. So [`NetStats::confirm_ms`] (an input sent, its tick confirmed
//! back, measured by this client) is the number that moves with the shim, and
//! [`NetStats::shim`] carries the configured figures so a reader never takes them
//! for a real network's.
//!
//! Chosen, not measured: [`SAMPLE_STEPS`] and [`HISTORY`], the sparkline's
//! resolution.

#[cfg(feature = "client")]
use std::collections::VecDeque;

use web_time::Duration;

#[cfg(feature = "client")]
use crate::client::Stats;
use crate::link::Conditions;

/// One transport's cumulative counters, from [`crate::link::Link::path`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PathStats {
    /// QUIC's smoothed round trip.
    pub rtt: Option<Duration>,
    /// The congestion window, in bytes.
    pub cwnd: Option<u64>,
    /// Bytes put on the wire, all packets.
    pub tx_bytes: u64,
    /// Bytes taken off the wire.
    pub rx_bytes: u64,
    /// UDP datagrams sent.
    pub tx_datagrams: u64,
    /// UDP datagrams received.
    pub rx_datagrams: u64,
    /// QUIC packets sent.
    pub sent_packets: u64,
    /// QUIC packets declared lost.
    pub lost_packets: u64,
    /// The shim wrapped around the link, if any.
    pub shim: Option<ShimStats>,
}

/// A [`crate::link::Shim`]'s conditions and what it has dropped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShimStats {
    /// What it was configured to do. Chosen, not measured.
    pub conditions: Conditions,
    /// Datagrams it has dropped, both directions.
    pub dropped: u64,
}

/// One second of a network race, as the overlay shows it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NetStats {
    /// QUIC's smoothed round trip in milliseconds, `None` on a link without one.
    pub rtt_ms: Option<f32>,
    /// Mean time from sending a tick's input to that tick coming back confirmed,
    /// in milliseconds: the round trip plus the server's input buffer.
    pub confirm_ms: Option<f32>,
    /// The mean change between consecutive [`Self::confirm_ms`] samples (the
    /// RFC 3550 interarrival jitter's shape).
    pub jitter_ms: Option<f32>,
    /// Bytes per second off the wire.
    pub rx_bytes_s: f32,
    /// Bytes per second onto the wire.
    pub tx_bytes_s: f32,
    /// Datagrams per second received.
    pub rx_datagrams_s: f32,
    /// Datagrams per second sent.
    pub tx_datagrams_s: f32,
    /// Percent of QUIC packets declared lost this second, `None` when none were sent.
    pub loss_percent: Option<f32>,
    /// The congestion window in bytes.
    pub cwnd: Option<u64>,
    /// The ticks of this client's input the server holds beyond the tick it is about
    /// to run (the server's input buffer depth for this seat).
    pub lead: i16,
    /// Present tick less confirmed tick.
    pub window: u32,
    /// The tick the confirmed (server) state is at.
    pub server_tick: u32,
    /// The tick the present is at.
    pub local_tick: u32,
    /// Corrections this second.
    pub corrections: u32,
    /// Ticks replayed by them.
    pub replayed: u32,
    /// The largest distance a correction moved a craft this second.
    pub largest_pop: f32,
    /// The largest prediction error this second.
    pub largest_error: f32,
    /// Hash matches since the race began.
    pub hash_ok: u64,
    /// Hash mismatches since the race began. Always a bug or different content.
    pub hash_bad: u64,
    /// Steps the client waited this second.
    pub stalls: u32,
    /// Resends it asked for this second.
    pub resends: u32,
    /// Ticks run extra and skipped to steer the lead this second.
    pub sped_up: u32,
    /// See [`Self::sped_up`].
    pub slowed_down: u32,
    /// The shim, when `--net-shim` is on.
    pub shim: Option<ShimStats>,
}

/// Ticks of the frame clock per sparkline column: 100 ms at 60 Hz. Chosen.
pub const SAMPLE_STEPS: u32 = 6;
/// Sparkline columns: with [`SAMPLE_STEPS`] twelve seconds. Chosen.
pub const HISTORY: usize = 120;

#[cfg(feature = "client")]
/// Differences the transport's counters and keeps the hash totals and the
/// sparkline between seconds.
#[derive(Debug, Default)]
pub struct Collector {
    previous: Option<PathStats>,
    hash_ok: u64,
    hash_bad: u64,
    history: VecDeque<f32>,
    steps: u32,
    sum_ms: f32,
    count: u32,
    held_ms: f32,
}

#[cfg(feature = "client")]
impl Collector {
    /// An empty collector.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds one step's counts, for the sparkline: every [`SAMPLE_STEPS`] steps a
    /// column is added, the mean confirm time of the samples since, or the last
    /// column's value when none arrived (so a quiet stretch is a flat line rather
    /// than a gap).
    pub fn step(&mut self, stats: &Stats) {
        self.sum_ms += stats.confirm_sum_ms;
        self.count += stats.confirm_count;
        self.steps += 1;
        if self.steps < SAMPLE_STEPS {
            return;
        }
        if self.count > 0 {
            self.held_ms = self.sum_ms / self.count as f32;
        }
        self.history.push_back(self.held_ms);
        if self.history.len() > HISTORY {
            self.history.pop_front();
        }
        self.steps = 0;
        self.sum_ms = 0.0;
        self.count = 0;
    }

    /// The sparkline, oldest first, in milliseconds.
    pub fn history(&self) -> impl Iterator<Item = f32> + '_ {
        self.history.iter().copied()
    }

    /// One second: `sum` is what [`Client::take_stats`](crate::client::Client::take_stats)
    /// returned added over the second, `path` the transport's counters now,
    /// `seconds` how long it was, and the two ticks come from the caller.
    pub fn second(
        &mut self,
        sum: &Stats,
        path: Option<PathStats>,
        seconds: f32,
        server_tick: u32,
        local_tick: u32,
    ) -> NetStats {
        self.hash_ok += u64::from(sum.hash_matches);
        self.hash_bad += u64::from(sum.hash_mismatches);
        let seconds = seconds.max(1e-3);
        let mut out = NetStats {
            lead: sum.lead,
            window: sum.window,
            server_tick,
            local_tick,
            corrections: sum.corrections,
            replayed: sum.replayed,
            largest_pop: sum.largest_pop,
            largest_error: sum.largest_error,
            hash_ok: self.hash_ok,
            hash_bad: self.hash_bad,
            stalls: sum.stalls,
            resends: sum.resends,
            sped_up: sum.sped_up,
            slowed_down: sum.slowed_down,
            ..NetStats::default()
        };
        if sum.confirm_count > 0 {
            out.confirm_ms = Some(sum.confirm_sum_ms / sum.confirm_count as f32);
        }
        if sum.jitter_count > 0 {
            out.jitter_ms = Some(sum.jitter_sum_ms / sum.jitter_count as f32);
        }
        if let Some(now) = path {
            out.rtt_ms = now.rtt.map(|rtt| rtt.as_secs_f32() * 1000.0);
            out.cwnd = now.cwnd;
            out.shim = now.shim;
            if let Some(before) = self.previous {
                let per_second = |a: u64, b: u64| a.saturating_sub(b) as f32 / seconds;
                out.rx_bytes_s = per_second(now.rx_bytes, before.rx_bytes);
                out.tx_bytes_s = per_second(now.tx_bytes, before.tx_bytes);
                out.rx_datagrams_s = per_second(now.rx_datagrams, before.rx_datagrams);
                out.tx_datagrams_s = per_second(now.tx_datagrams, before.tx_datagrams);
                let sent = now.sent_packets.saturating_sub(before.sent_packets);
                let lost = now.lost_packets.saturating_sub(before.lost_packets);
                if sent > 0 {
                    out.loss_percent = Some(lost as f32 * 100.0 / sent as f32);
                }
            }
            self.previous = Some(now);
        }
        out
    }
}

#[cfg(all(test, feature = "client"))]
mod tests {
    use super::*;

    fn path(rx: u64, lost: u64, sent: u64) -> PathStats {
        PathStats {
            rtt: Some(Duration::from_millis(40)),
            rx_bytes: rx,
            sent_packets: sent,
            lost_packets: lost,
            ..PathStats::default()
        }
    }

    #[test]
    fn a_second_is_the_difference_of_the_counters() {
        let mut c = Collector::new();
        let first = c.second(&Stats::default(), Some(path(1_000, 0, 100)), 1.0, 0, 0);
        assert_eq!(first.rx_bytes_s, 0.0, "no earlier reading to difference");
        assert_eq!(first.rtt_ms, Some(40.0));
        let next = c.second(&Stats::default(), Some(path(4_000, 2, 200)), 2.0, 0, 0);
        assert_eq!(next.rx_bytes_s, 1_500.0);
        assert_eq!(next.loss_percent, Some(2.0));
    }

    #[test]
    fn hash_totals_run_across_seconds_and_confirm_is_a_mean() {
        let mut c = Collector::new();
        let s = Stats {
            hash_matches: 60,
            hash_mismatches: 1,
            confirm_sum_ms: 300.0,
            confirm_count: 3,
            ..Stats::default()
        };
        c.second(&s, None, 1.0, 0, 0);
        let out = c.second(&s, None, 1.0, 0, 0);
        assert_eq!((out.hash_ok, out.hash_bad), (120, 2));
        assert_eq!(out.confirm_ms, Some(100.0));
        assert_eq!(out.rtt_ms, None);
    }

    #[test]
    fn the_sparkline_adds_a_column_per_sample_and_holds_through_a_gap() {
        let mut c = Collector::new();
        let s = Stats {
            confirm_sum_ms: 50.0,
            confirm_count: 1,
            ..Stats::default()
        };
        c.step(&s);
        for _ in 1..SAMPLE_STEPS {
            c.step(&Stats::default());
        }
        for _ in 0..SAMPLE_STEPS {
            c.step(&Stats::default());
        }
        assert_eq!(c.history().collect::<Vec<_>>(), vec![50.0, 50.0]);
    }
}
