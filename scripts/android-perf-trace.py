#!/usr/bin/env python3
"""Summarise a phone race's Perfetto trace (docs/tools/android.md, "Performance on the Galaxy S21").

    adb shell 'perfetto -c - --txt -o /data/misc/perfetto-traces/oag.pftrace' < scripts/android-perf-trace.cfg
    adb pull /data/misc/perfetto-traces/oag.pftrace
    uv run --with perfetto python3 -I scripts/android-perf-trace.py oag.pftrace

Prints JSON: the frame cadence (an app layer's queued buffers, read off the BufferTX counter,
because a NativeActivity gives SurfaceFlinger no frame-timeline token), the app's per-thread CPU
split by core class, the whole-system busy share per core, and the CPU frequency range seen.
"""
import json
import statistics
import sys

from perfetto.trace_processor import TraceProcessor


def percentile(xs, p):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(round(p / 100 * (len(xs) - 1))))]


def main(path):
    tp = TraceProcessor(trace=path)
    query = lambda sql: list(tp.query(sql))
    out = {}
    rows = query(
        """select c.ts ts, c.value v from counter c join counter_track t on c.track_id = t.id
           where t.name like 'BufferTX - org.openantigrav.game%' order by c.ts"""
    )
    edges, prev = [], None
    for r in rows:
        if prev is not None and r.v > prev:
            edges.append(r.ts)
        prev = r.v
    intervals = [(b - a) / 1e6 for a, b in zip(edges, edges[1:])]
    if intervals:
        span = (edges[-1] - edges[0]) / 1e9
        out["frames"] = len(edges)
        out["span_s"] = span
        out["fps"] = len(intervals) / span
        out["median_ms"] = statistics.median(intervals)
        out["mean_ms"] = statistics.mean(intervals)
        out["p95_ms"] = percentile(intervals, 95)
        out["p99_ms"] = percentile(intervals, 99)
        out["worst_ms"] = max(intervals)
        out["over_16.7_pct"] = 100 * sum(x > 16.7 for x in intervals) / len(intervals)
    pid = query("select pid from process where name = 'org.openantigrav.game' limit 1")[0].pid
    bounds = query("select min(ts) a, max(ts + dur) b from sched")[0]
    span = (bounds.b - bounds.a) / 1e9
    threads = query(
        f"""select thread.name n, thread.tid tid, sum(s.dur) / 1e9 sec,
            sum(case when s.cpu < 4 then s.dur else 0 end) / 1e9 little,
            sum(case when s.cpu between 4 and 6 then s.dur else 0 end) / 1e9 mid,
            sum(case when s.cpu = 7 then s.dur else 0 end) / 1e9 big
            from sched s join thread using (utid) join process p using (upid)
            where p.pid = {pid} group by thread.tid order by sec desc limit 14"""
    )
    out["trace_span_s"] = span
    out["threads"] = [
        dict(name=r.n, tid=r.tid, cpu_pct=100 * r.sec / span, little=100 * r.little / span,
             mid=100 * r.mid / span, big=100 * r.big / span)
        for r in threads
    ]
    busy = query(
        "select cpu, sum(dur) / 1e9 sec from sched where utid not in "
        "(select utid from thread where tid = 0) group by cpu order by cpu"
    )
    out["cpu_busy_pct"] = {r.cpu: round(100 * r.sec / span, 1) for r in busy}
    freq = query(
        """select t.name n, avg(c.value) a, min(c.value) mn, max(c.value) mx from counter c
           join counter_track t on c.track_id = t.id where t.name like 'cpu%freq%' group by t.name"""
    )
    out["freq_khz"] = {r.n: dict(avg=r.a, min=r.mn, max=r.mx) for r in freq}
    print(json.dumps(out, indent=1, default=float))


if __name__ == "__main__":
    main(sys.argv[1])
