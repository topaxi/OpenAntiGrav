#!/usr/bin/env python3
"""Per frame of a phone race, when `QueuePresentKHR` returns against when the frame's GPU fence signals.

    uv run --with perfetto --with pandas python3 -I scripts/android-fence-timeline.py TRACE [--by-window]

TRACE is a Perfetto trace with the `gfx` atrace category (scripts/android-perf-trace.cfg has it).
For each frame: when `QueuePresentKHR` starts and returns on the game's frame thread, and when
that frame's GPU completion fence (the one `queueBuffer` hands over) signals, read off the game
process's FenceMonitor thread ("GPU completion"). A present that returns before its own fence
signals means the CPU is a frame ahead of the GPU (docs/tools/android.md, "What the frame thread
waits for").

--by-window splits the same medians by the `OAG_SKIP_PASSES: window N on|off` lines a toggle run prints
(`oag_gpu::skip`), which the trace carries under its `android.log` source; frames within 0.5 s of a window
edge are left out (docs/tools/android.md, "Freezing the frame's writes")."""
import re, statistics as st, sys
by_window = "--by-window" in sys.argv[2:]
from perfetto.trace_processor import TraceProcessor
tp = TraceProcessor(trace=sys.argv[1])
q = lambda s: tp.query(s).as_pandas_dataframe()
proc = q("select upid, pid from process where name like 'org.openantigrav%'")
upid = int(proc.upid[0])
main = int(q(f"select utid from thread where upid={upid} and name='android_main'").utid[0])
fm = q(f"select utid from thread where upid={upid} and name='GPU completion'")
fmu = int(fm.utid[0])
pres = q(f"""select s.ts, s.dur from slice s join thread_track tt on s.track_id=tt.id
             where tt.utid={main} and s.name='QueuePresentKHR' order by s.ts""")
traced = q(f"""select s.ts, s.name from slice s join thread_track tt on s.track_id=tt.id
              where tt.utid={main} and s.name like 'Trace GPU completion fence%' order by s.ts""")
waits = q(f"""select s.ts, s.dur, s.name from slice s join thread_track tt on s.track_id=tt.id
             where tt.utid={fmu} and s.name like 'waiting for GPU completion%' order by s.ts""")
sig = {int(re.findall(r'\d+', n)[-1]): ts + d for ts, d, n in zip(waits.ts, waits.dur, waits.name)}
subs = q(f"""select s.ts, s.dur from slice s join thread_track tt on s.track_id=tt.id
             where tt.utid={main} and s.name='QueueSubmit' order by s.ts""")
rows = []
for ts, name in zip(traced.ts, traced.name):
    n = int(re.findall(r'\d+', name)[-1])
    p = pres[(pres.ts <= ts) & (pres.ts + pres.dur >= ts)]
    if n in sig and len(p):
        p0, pd_ = int(p.ts.iloc[0]), int(p.dur.iloc[0])
        rows.append((p0, pd_, sig[n]))
print("frames matched:", len(rows))
if by_window:
    logs = q("select ts, msg from android_logs where msg like 'OAG_SKIP_PASSES: window%' order by ts")
    edges = [(int(t), m.split()[-1]) for t, m in zip(logs.ts, logs.msg)]
    if not edges:
        sys.exit("no OAG_SKIP_PASSES window lines: was the trace taken with the android.log source on a toggle run?")
    arms = {"off": [], "on": []}
    for i in range(1, len(rows) - 1):
        p0 = rows[i][0]
        if any(abs(p0 - e) < 5e8 for e, _ in edges):
            continue
        arm = None
        for e, name in edges:
            if e <= p0:
                arm = name
        if arm is not None:
            arms[arm].append(i)
    for arm, label in (("off", "switch off"), ("on", "switch on")):
        idx = arms[arm]
        if not idx:
            print(label, "no frames")
            continue
        m = lambda v: round(st.median(v), 2)
        print(f"{label}: n={len(idx)} frame {m([(rows[i+1][0]-rows[i][0])/1e6 for i in idx])}"
              f" present {m([rows[i][1]/1e6 for i in idx])}"
              f" previous-fence->present-returns {m([((rows[i][0]+rows[i][1])-rows[i-1][2])/1e6 for i in idx])}"
              f" fence period {m([(rows[i+1][2]-rows[i][2])/1e6 for i in idx])}")
    sys.exit(0)
ret_to_sig = [(s - (p0 + d)) / 1e6 for p0, d, s in rows]
pdur = [d / 1e6 for _, d, _ in rows]
period = [(rows[i + 1][2] - rows[i][2]) / 1e6 for i in range(len(rows) - 1)]
prev_sig_to_ret = [((rows[i][0] + rows[i][1]) - rows[i - 1][2]) / 1e6 for i in range(1, len(rows))]
med = lambda v: round(st.median(v), 2)
print("QueuePresentKHR duration ms, median", med(pdur))
print("present returns -> THIS frame's GPU fence signals, ms, median", med(ret_to_sig),
      "(share of frames whose fence signals after present returned:", round(sum(x > 0 for x in ret_to_sig) / len(ret_to_sig), 3), ")")
print("PREVIOUS frame's fence signals -> this present returns, ms, median", med(prev_sig_to_ret))
print("fence-to-fence period ms, median", med(period))
frame = [(rows[i + 1][0] - rows[i][0]) / 1e6 for i in range(len(rows) - 1)]
print("present-to-present (frame) ms, median", med(frame))
for r in rows[100:106]:
    p0, d, s = r
    print(f"  present {p0/1e6:.2f} +{d/1e6:.2f} -> returns {(p0+d)/1e6:.2f}; own fence signals {s/1e6:.2f}")
