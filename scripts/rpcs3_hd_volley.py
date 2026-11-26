"""What a Detonator grab's cannon did and when, from reads at the pauses the shot script already takes.

The picture is several ticks older than its own pause, and the game sees a pad press ticks after it is sent. So each
pause keeps the mines' draw words and the cannon's counters and rounds (`rpcs3_hd_field.cannon_state`); a round's
age gives the lap clock of the tick that fired it, and its launch speed the craft's speed on that tick
(`docs/tools/compare-page.md`, "The volley clocks and the picture-clock reads"). No pause is added for these reads.
"""

import rpcs3_hd_field as field

#: A volley's two rounds share a launch tick; volleys are 0.1 s (6 ticks) apart.
SAME_TICK_S = 0.5 / 60


def launches(samples):
    """`[(launch_clock_s, craft_speed)]`, one per volley, earliest first, from `samples` (`{"race_clock_s", "rounds"}`).

    A round's launch clock is the sample's lap clock less its age. A retired round keeps its last age, which makes that
    difference grow with later samples, so each round object counts its earliest."""
    first = {}
    for sample in samples:
        for r in sample["rounds"]:
            at = sample["race_clock_s"] - r["age_s"]
            if r["address"] not in first or at < first[r["address"]][0]:
                first[r["address"]] = (at, (r["launch_speed"] - 500.0) / 3.6)
    volleys = []
    for at, speed in sorted(first.values()):
        if not volleys or at - volleys[-1][0] > SAME_TICK_S:
            volleys.append((at, speed))
    return volleys


def fire_record(fire, samples):
    """`fire` with what the rounds say: every volley's lap clock (the tick before the frame that fired it, as the round's
    age counts), the first one's, and how long after the harness's send it is. That is the press latency only to a few
    ticks: the pauses bracket the game's first frame with the trigger held later than this clock."""
    volleys = launches(samples)
    out = dict(fire)
    if volleys:
        out["first_launch_clock_s"] = volleys[0][0]
        out["craft_speed"] = volleys[0][1]
        out["volley_after_send_ticks"] = (volleys[0][0] - fire["press_clock_before_s"]) * 60
        out["volley_clocks_s"] = [at for at, _ in volleys]
    return out


class Recorder:
    """Reads the mines and the cannon at a pause that is already taken; `mine_reads` and `samples` grow."""

    def __init__(self, session, ship):
        self.mem = field.Mem(session.open_mem())
        self.ship = ship
        self.mines = [m["address"] for m in field.detonator_mines(self.mem)]
        self.pool = field.pulse_bullets(self.mem)
        self.mine_reads = []
        self.samples = []
        print("recorder: %d mines, %d rounds in the pool" % (len(self.mines), len(self.pool)), flush=True)

    def sample(self, clock):
        """Call while the game is paused at lap clock `clock`."""
        words = {str(a): field.mine_state(self.mem.rd(a, field.MINE_OBJECT_BYTES)) for a in self.mines}
        self.mine_reads.append({"race_clock_s": clock, "mines": words})
        self.samples.append(dict(field.cannon_state(self.mem, self.ship, self.pool), race_clock_s=clock))


def self_test():
    sample = lambda clock, ages, held: {"race_clock_s": clock, "held_frames": held, "rounds": [
        {"address": n, "age_s": a, "launch_speed": 874.0} for n, a in ages]}
    samples = [sample(1.9, [], 0), sample(2.0, [(1, 4 / 60), (2, 4 / 60)], 4),
               sample(2.1, [(1, 10 / 60), (2, 10 / 60), (3, 4 / 60), (4, 4 / 60)], 10),
               sample(2.3, [(1, 10 / 60), (2, 22 / 60), (3, 16 / 60), (4, 16 / 60)], 10)]
    got = launches(samples)
    assert [round(at, 4) for at, _ in got] == [round(2.0 - 4 / 60, 4), round(2.1 - 4 / 60, 4)], got
    assert abs(got[0][1] - 374 / 3.6) < 1e-9, got
    rec = fire_record({"press_clock_before_s": 1.87}, samples)
    assert abs(rec["volley_after_send_ticks"] - (2.0 - 4 / 60 - 1.87) * 60) < 1e-6, rec
    assert [round(at, 4) for at in rec["volley_clocks_s"]] == [round(at, 4) for at, _ in got], rec
    assert fire_record({"press_clock_before_s": 1.87}, [sample(1.9, [], 0)]) == {"press_clock_before_s": 1.87}


if __name__ == "__main__":
    self_test()
    print("ok")
