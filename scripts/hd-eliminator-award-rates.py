"""An Eliminator score CSV's opponent points per craft-lap and per craft-minute by award kind, pooled
over files (the columns of `scripts/rpcs3-hd-eliminator-score.py`, or the `OAG_SWEEP` sweep in
`crates/raceplay/tests/hd_eliminator_hud_ground_truth.rs`).

`PLAYER`: the person's craft (7 on the original's captures, 0 in ours). `SCALE`: the opponents' score
scale (1.15 Novice, 1.25 Skilled, 1.4 Elite)."""
import csv, sys, os
from collections import defaultdict
PLAYER = int(os.environ.get('PLAYER', '7')); SCALE = float(os.environ.get('SCALE', '1.15'))
tot = defaultdict(float); laps = 0; minutes = 0.0
for path in sys.argv[1:]:
    prev = {}; t0 = None; t = 0
    for r in csv.DictReader(open(path)):
        c = int(r['craft']); t = float(r['t']); t0 = t if t0 is None else t0
        if c == PLAYER: continue
        s, k, lap = float(r['score']), int(r['kills']), int(r['lap'])
        if c in prev:
            ps, pk, pl = prev[c]; ds = s - ps
            if lap > pl: tot['lap'] += 5*SCALE*(lap-pl); ds -= 5*SCALE*(lap-pl); laps += lap-pl
            if k > pk: tot['kill'] += 20*SCALE*(k-pk); ds -= 20*SCALE*(k-pk)
            if ds < -1e-3 and (abs(ds+5) < .02 or abs(ds+min(ps,5)) < .02): tot['death'] += ds; ds = 0
            if abs(ds) > 1e-3: tot['damage'] += ds
        prev[c] = (s, k, lap)
    minutes += len(prev) * (t - t0) / 60
row = lambda k: f"{tot[k]/max(laps,1):6.2f} /lap {tot[k]/minutes:6.2f} /min"
print(f"craft-laps {laps} craft-min {minutes:.0f} | " + " | ".join(f"{k} {row(k)}" for k in ['damage','kill','death','lap']) + f" | all {sum(tot.values())/max(laps,1):.2f}/lap {sum(tot.values())/minutes:.2f}/min")
