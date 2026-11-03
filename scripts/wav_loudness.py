#!/usr/bin/env python3
"""Integrated loudness (ITU-R BS.1770-4, K-weighted, gated) of 16-bit WAV dumps.

usage: wav_loudness.py file.wav...
Prints LUFS (integrated), short-term max (3 s) and sample peak in dBFS.
Needs numpy and scipy (uv run --with numpy --with scipy).
"""
import sys
import wave

import numpy as np
from scipy.signal import lfilter


def k_filters(fs):
    # BS.1770 stage 1 (high shelf) and stage 2 (RLB high-pass), bilinear designed for any rate.
    f0, g, q = 1681.974450955533, 3.999843853973347, 0.7071752369554196
    k = np.tan(np.pi * f0 / fs)
    vh = 10 ** (g / 20)
    vb = vh ** 0.4996667741545416
    a0 = 1 + k / q + k * k
    shelf_b = [(vh + vb * k / q + k * k) / a0, 2 * (k * k - vh) / a0, (vh - vb * k / q + k * k) / a0]
    shelf_a = [1, 2 * (k * k - 1) / a0, (1 - k / q + k * k) / a0]
    f0, q = 38.13547087602444, 0.5003270373238773
    k = np.tan(np.pi * f0 / fs)
    a0 = 1 + k / q + k * k
    hp_a = [1, 2 * (k * k - 1) / a0, (1 - k / q + k * k) / a0]
    return (shelf_b, shelf_a), ([1, -2, 1], hp_a)


def measure(path):
    w = wave.open(path)
    fs, ch = w.getframerate(), w.getnchannels()
    x = np.frombuffer(w.readframes(w.getnframes()), dtype="<i2").astype(np.float64) / 32768
    x = x.reshape(-1, ch)
    peak = 20 * np.log10(max(np.abs(x).max(), 1e-9))
    (sb, sa), (hb, ha) = k_filters(fs)
    y = np.stack([lfilter(hb, ha, lfilter(sb, sa, x[:, c])) for c in range(ch)], 1)
    blk, hop = int(0.4 * fs), int(0.1 * fs)
    ms = np.array([(y[i:i + blk] ** 2).mean(0).sum() for i in range(0, len(y) - blk + 1, hop)])
    lk = -0.691 + 10 * np.log10(ms + 1e-12)
    keep = ms[lk > -70]
    rel = -0.691 + 10 * np.log10(keep.mean()) - 10
    keep = ms[lk > max(rel, -70)]
    integrated = -0.691 + 10 * np.log10(keep.mean() + 1e-12)
    st = int(3 * fs)
    short = max(-0.691 + 10 * np.log10((y[i:i + st] ** 2).mean(0).sum() + 1e-12) for i in range(0, max(len(y) - st, 1), fs // 2))
    return integrated, short, peak


for p in sys.argv[1:]:
    i, s, pk = measure(p)
    print(f"{p.split('/')[-1]}: integrated {i:.1f} LUFS, short-term max {s:.1f} LUFS, peak {pk:.1f} dBFS")
