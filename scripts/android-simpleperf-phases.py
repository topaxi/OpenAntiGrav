#!/usr/bin/env python3
"""A thread's wall time by top-level frame phase from a `simpleperf record -g --trace-offcpu`.

    python3 -I scripts/android-simpleperf-phases.py PERF_DATA BINARY_CACHE [THREAD]

BINARY_CACHE is what the NDK's `binary_cache_builder.py -i PERF_DATA -lib <dir of the unstripped
liboag_android.so>` builds. Prints the thread's sampled time (simpleperf --trace-offcpu,
so every sample's period is wall time) by top-level phase, on-CPU and off-CPU, plus the top
self-time functions of its on-CPU samples."""
import collections, sys
import glob, os
sys.path.insert(0, sorted(glob.glob(os.path.join(os.environ.get("ANDROID_HOME", "/opt/android-sdk"), "ndk", "*", "simpleperf")))[-1])
from simpleperf_report_lib import ReportLib

PHASES = [  # first marker found walking from the root (outermost) frame inwards wins, in this order of specificity
    ("present (Queue::present)", ["wgpu::api::queue::Queue>::present"]),
    ("acquire (get_current_texture)", ["get_current_texture"]),
    ("encode (CommandEncoder::finish)", ["CommandEncoder>::finish"]),
    ("submit (Queue::submit)", ["wgpu::api::queue::Queue>::submit"]),
    ("timestamp readback and DRS", ["read_timing_and_feed_drs", "PassTimer>::drain", "drain_labelled"]),
    ("sim ticks (Race::tick)", ["oag_raceplay::Race>::tick"]),
    ("draw-list build (prepare_view, record)", ["Scene>::prepare_view", "Recorder>::record", "draw_lists"]),
    ("HUD", ["draw_window_hud", "hud_views::draw"]),
    ("pass recording (Scene::render and rest of RaceStage::render)", ["RaceStage>::render", "Scene>::render"]),
    ("queue.write_buffer (outside the above)", ["Queue>::write_buffer"]),
]
OFF = "[kernel.kallsyms][+ffffffc01101e71c]"

lib = ReportLib()
lib.SetRecordFile(sys.argv[1])
lib.SetSymfs(sys.argv[2])
thread = sys.argv[3] if len(sys.argv) > 3 else "android_main"
lib.ShowIpForUnknownSymbol()
tot = collections.Counter(); on = collections.Counter(); self_on = collections.Counter()
total = 0
while True:
    s = lib.GetNextSample()
    if s is None:
        break
    if s.thread_comm != thread:
        continue
    period = s.period
    sym = lib.GetSymbolOfCurrentSample()
    chain = lib.GetCallChainOfCurrentSample()
    names = [sym.symbol_name] + [chain.entries[i].symbol.symbol_name for i in range(chain.nr)]
    off = sym.symbol_name == OFF or "kallsyms" in sym.dso_name and any("futex" in n or "sem_wait" in n or "poll" in n for n in names)
    total += period
    phase = "other"
    for label, keys in PHASES:
        if any(k in n for n in names for k in keys):
            phase = label
            break
    tot[phase] += period
    if not off:
        on[phase] += period
        self_on[sym.symbol_name] += period
print(f"thread {thread}: {total/1e9:.2f} s sampled")
for phase, t in tot.most_common():
    print(f"  {phase:62s} {100*t/total:5.1f} %  (on-CPU {100*on[phase]/total:5.1f} %)")
ontot = sum(on.values())
print(f"  on-CPU total {100*ontot/total:.1f} %")
print("top 15 self time, on-CPU only (share of the thread's on-CPU time):")
for name, t in self_on.most_common(15):
    print(f"  {100*t/ontot:5.2f} %  {name[:150]}")
