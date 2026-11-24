#!/usr/bin/env python3
"""Original-vs-ours pairs of Wipeout Pulse (PS2) front-end screens from PCSX2 savestates, environment-checked.

    source <scratch>/env.sh    # OAG_PCSX2_DISPLAY, OAG_PCSX2_DATA, OAG_COMPARE_PLATFORM=pulse-ps2, DISPLAY of a `pcsx2-drive.py boot` instance
    python3 scripts/pcsx2-menu-pair.py pairs --slot 28092 --out D --oag-game target/debug/oag-game

The PS2 counterpart of `psp-menu-pair.py`, on `compare_menu.py`'s route: the same screen, size, language and highlighted row, ours
drawn from the disc's own front-end XML (`--menu-page grid-select`, `cell-select`). The original is a PCSX2 savestate of the screen
(`data/saves/pulse-ps2/menu/`, made by the walk below): the state is loaded into slot 1 of the instance's own savestate folder, the
VM runs on, and PCSX2's own GS screenshot is taken and resized ONCE (Lanczos) to the profile's output, as `pcsx2-pulse-pair.py` does.

The walk that makes a state, from `pcsx2-drive.py boot` (each key through `pcsx2-drive.py press`, a settle of 5-8 s between): the
"continue with this screen mode?" prompt (down, cross answers YES), PRESS START (cross), ENTER YOUR NAME (cross to the fourth letter, right to
CONFIRM, cross), ENTER YOUR TAG (cross, cross, right, cross), the format-memory-card prompt (cross on NO) and Main Menu; then cross for Grid
Selection, cross again for Cell Selection (Single Race, Moa Therma White, AI difficulty Medium), `pcsx2-drive.py state save N` on each.

The screen is read by eye from the frame: PS2 Pulse's front-end state name has no recovered address, so `original.screen_basis` says so.
The selector's pulse and the card title's glow ladder are two counters (`scripts/ps2_pulse.py`): both are read from EE RAM on each side of
the screenshot and placed at the frame the ticker head fit picks. The Selector's phase is passed as `--menu-picker-seconds`; the title's ladder
is recorded but not passed: it runs in RAM and the original's frame shows no step of it (`frontend-pulse.md`, "The ladder does not reach the picture").
"""

import argparse
import datetime
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import compare_env as ce  # noqa: E402
import compare_menu as cm  # noqa: E402
import compare_provenance as cp  # noqa: E402
import ps2_pulse as pp  # noqa: E402
import ps2_ticker as pt  # noqa: E402

_spec = importlib.util.spec_from_file_location("pcsx2_drive", HERE / "pcsx2-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)

SERIAL = "SCES-54748 (F8AE6FF2)"
#: The EU disc's data archive, where `Data\Plugins\PI000\entries.xml` (the English strings) lives.
DATA_WAD = "54748/WADS2.WAD"
#: A frame is grabbed only while its one message's head is this far inside the viewport (in the viewport's own units, 493 wide), so that the
#: few frames between the RAM read and the screenshot neither let the next message in nor take this one out.
HEAD_MARGIN = 40.0
#: The ticker box in the 1280 x 720 pair (the bar between the tag box and the right end, text rows only) the head fit scores.
TICKER_BOX = (210, 628, 1030, 40)

#: The craft's box in the 1280 x 720 `team` pair (the hull and its wings, not the stat panel) the sway fit scores.
HULL_BOX = (100, 200, 500, 320)
#: The length of `ship_FE.vex`'s `Anim Transform` loop, in seconds (359 frames at 60 a second: `docs/ui/selection-screens.md`, "The craft sways"):
#: a phase outside it is the same pose.
SWAY_LOOP = 5.983

#: One pair per entry. `state` is the savestate file under `data/saves/pulse-ps2/menu/`; `page` is ours' `--menu-page`.
WALK = [
    {"id": "grid", "title": "Race Campaign: Grid Selection", "screen": "Grid Selection", "page": "grid-select", "state": "grid-selection.p2s",
     "xml": "CellMode_Definition.xml (Grid Selection, the tier hexes)",
     "selection": {"original": "Grid 1, the first tier", "ours": "Grid 1, the first tier"}},
    {"id": "cell", "title": "Race Campaign: Cell Selection (campaign cell)", "screen": "Cell Selection", "page": "cell-select", "state": "cell-selection.p2s",
     "xml": "CellMode_Definition.xml (Cell Selection)",
     "selection": {"original": "Single Race, Moa Therma White, AI difficulty Medium (the footer)", "ours": "Single Race, Moa Therma White, AI difficulty Medium (the footer)"}},
    {"id": "team", "title": "Ship Select", "screen": "Team Selection", "page": "ship-select", "state": "team-selection.p2s",
     "xml": "Selection_Definition.xml (Team Selection)",
     "selection": {"original": "Assegai, Classic (first craft)", "ours": "Assegai, Classic (first craft)"}},
]
#: The WALK ids whose ours is drawn at the original's two pulse counters (`pulse_flags`): the campaign pages that own a `Selector` and a pulse title.
CLOCKED = ("grid", "cell")
#: `--menu-model-seconds` for ours' hull, per WALK id. PS2 Pulse has no recovered address for the ship node's clock (the PSP's is `node+0x40`),
#: and a loaded state keeps its phase while the VM runs on, so the hull's sway phase is matched by eye/box against the frame: pass `--model-seconds`.


def run(cmd, **kw):
    print("+", " ".join(str(c) for c in cmd), file=sys.stderr)
    return subprocess.run([str(c) for c in cmd], check=True, **kw)


def wait_lone_message(pine, addrs, entries, timeout=90.0):
    """Poll the ticker records (a read is about a millisecond) until one message is on screen well inside the viewport and is a tip ours can pin."""
    deadline = time.monotonic() + timeout
    seen = None
    while time.monotonic() < deadline:
        records = pt.read_records(pine, addrs)
        lone = pt.lone_message(records, HEAD_MARGIN)
        seen = [(r["active"], r["head_x"], r["text"][:30]) for r in records]
        if lone and pt.item_of(lone["text"], entries):
            return lone
        time.sleep(0.05)
    sys.exit("STALLED: no frame with one pinnable ticker message in %.0f s (last read %s)" % (timeout, seen))


def shot_in_bracket(pine, keyboard, addrs, dest, pulse=(None, None), timeout=20.0):
    """PCSX2's GS screenshot (the F8 hotkey) of the running VM, with the ticker read just before the key and again as the file appears.

    A paused VM presents no frame and the shot fails, so the VM runs and the head is only known to lie between the two reads (it moves left 1 a
    frame). Returns `(before, after, pulse_before, pulse_after)`, the two reads and the pulse counters read beside them, or `None` when no shot
    came."""
    before_shots = drive.newest_snapshot()
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        keyboard.focus()
        pulse_before = pp.read_state(pine, *pulse)
        before = pt.read_records(pine, addrs)
        keyboard.tap("F8", after=0.0)
        for _ in range(400):
            latest = drive.newest_snapshot()
            if latest and latest != before_shots and os.path.getsize(latest) > 0:
                after = pt.read_records(pine, addrs)
                pulse_after = pp.read_state(pine, *pulse)
                drive._wait_until_written(latest)
                shutil.move(latest, dest)
                return before, after, pulse_before, pulse_after
            time.sleep(0.01)
    return None


def grab(slot, state_file, dest, settle, oag_wad, source):
    """Load `state_file` into the instance's slot 1, let the VM run `settle` seconds, wait for a frame whose footer holds one pinnable ticker message
    and take PCSX2's GS screenshot of it into `dest`; returns the ticker and tag reads (`docs/tools/compare-page.md`, "PS2 menu pairs: the footer")."""
    states = Path(os.environ["OAG_PCSX2_DATA"]).expanduser() / "PCSX2" / "sstates"
    states.mkdir(parents=True, exist_ok=True)
    shutil.copy(state_file, states / ("%s.01.p2s" % SERIAL))
    pine = drive.Pine(slot=slot)
    keyboard = drive.Keyboard()
    drive.set_paused(pine, keyboard, False)
    pine.loadstate(1)
    time.sleep(settle)
    addrs = pt.find_records(pine)
    if len(addrs) != 2:
        sys.exit("STALLED: %d ticker records found in EE RAM (a screen with a footer shows two)" % len(addrs))
    entries = pt.load_entries(oag_wad, "%s:%s" % (source, DATA_WAD))
    pulse = pp.find(pine)
    print("pulse objects: Selector controller %s, title Text %s" % tuple(("0x%08x" % a) if a else "none" for a in pulse), flush=True)
    wait_lone_message(pine, addrs, entries)
    bracket = shot_in_bracket(pine, keyboard, addrs, dest, pulse)
    if bracket is None:
        sys.exit("no GS screenshot (is the VM paused?)")
    before, after, pulse_before, pulse_after = bracket
    bracket = (before, after)
    # The pair is only the pinned frame when the same one message was alone on screen at both reads.
    lone_before, lone_after = (pt.lone_message(r, HEAD_MARGIN) for r in bracket)
    if not (lone_before and lone_after and lone_before["addr"] == lone_after["addr"] and lone_before["text"] == lone_after["text"]):
        sys.exit("STALLED: the ticker changed across the screenshot (%s then %s); run again" % (before, after))
    item = pt.item_of(lone_before["text"], entries)
    if item is None:
        sys.exit("STALLED: the message %r is not an untried tip, so no item names it; run again" % lone_before["text"])
    ram = pt.read_ram(pine)
    profile = pt.profile_name_tag(ram)
    if profile is None:
        sys.exit("STALLED: no single name/tag profile record in EE RAM")
    return {"item": item, "text": lone_before["text"], "record": lone_before["addr"], "addrs": addrs,
            "head_before": lone_before["head_x"], "head_after": lone_after["head_x"], "name": profile[0], "tag": profile[1], "tag_addr": profile[2],
            "pulse": {"controller": pulse[0], "text": pulse[1], "before": pulse_before, "after": pulse_after}}


def prepare_profile(plat, profile, player):
    """Ours' private profile: the compare settings, `[dev] unlock_everything = false` (the original's fresh profile has the locks) and the stored
    player the original's RAM holds (the footer's tag box draws its tag)."""
    ce.write_ours_settings(plat, profile)
    cm.extra_settings(profile, locked=True, player=player)


def ticker_box_grey(theirs, ours):
    """Mean absolute grey difference of `TICKER_BOX` between two same-size frames."""
    import numpy
    from PIL import Image
    x, y, w, h = TICKER_BOX
    boxes = [numpy.asarray(Image.open(p).convert("L"), dtype=numpy.float64)[y:y + h, x:x + w] for p in (theirs, ours)]
    return float(numpy.abs(boxes[0] - boxes[1]).mean())


def box_grey(theirs, ours, box):
    """Mean absolute grey difference of `box` (x, y, w, h) between two same-size frames, ImageMagick's grey as `compare_env.overlap_score`
    reads it, so the box moves with the pair's own `mean_grey`."""
    x, y, w, h = box
    args = ["magick"]
    for path in (theirs, ours):
        args += ["(", str(path), "-crop", "%dx%d+%d+%d" % (w, h, x, y), "+repage", ")"]
    args += ["-colorspace", "Gray", "-compose", "difference", "-composite", "-format", "%[fx:mean*255]", "info:"]
    return float(subprocess.run(args, capture_output=True, text=True, check=True).stdout)


#: The least the original's hull box may vary (grey standard deviation) and shine (grey mean) for a sway fit to mean anything: a black or wrong GS frame
#: has neither, and every phase would score as well as any other. The real Assegai frame reads a standard deviation of 33.9.
MIN_HULL_STD = 8.0
MIN_HULL_MEAN = 5.0
#: The least the phases' scores may differ (max minus min, grey) for one to be picked: a flat set is a fit of nothing. The real frame's spread is about 17.
MIN_SWAY_SPREAD = 2.0


def box_stats(path, box):
    """`(mean, standard deviation)` of the grey in `box` (x, y, w, h), 0 to 255."""
    x, y, w, h = box
    out = subprocess.run(["magick", str(path), "-crop", "%dx%d+%d+%d" % (w, h, x, y), "+repage", "-colorspace", "Gray", "-format",
                          "%[fx:mean*255] %[fx:standard_deviation*255]", "info:"], capture_output=True, text=True, check=True).stdout.split()
    return float(out[0]), float(out[1])


def fit_sway(args, item, plat, profile, theirs, pdir):
    """The hull's `--menu-model-seconds`: PS2 Pulse has no recovered address for the ship node's clock and a loaded state keeps its phase while the VM
    runs on, so ours is rendered at phases a quarter second apart over the loop, then at the neighbours of the best, and the phase whose `HULL_BOX` lies
    closest to the original's is kept. Returns `(seconds, {seconds: box grey})`.

    Refuses (exit 3, with the reason) when the original's hull box is near flat or near black, and when the phases' scores hardly differ: either is a
    frame the fit cannot read (a black or wrong GS grab), and a phase picked from it would be written into the pair."""
    mean, std = box_stats(theirs, HULL_BOX)
    if std < MIN_HULL_STD or mean < MIN_HULL_MEAN:
        sys.exit("sway fit refused: the original's hull box %s reads mean %.1f, standard deviation %.1f (needs >= %.0f and >= %.0f): a black or wrong frame, "
                 "not a hull; re-grab" % (list(HULL_BOX), mean, std, MIN_HULL_MEAN, MIN_HULL_STD))
    offset = ce.display_offset_record(plat["key"], "menu")
    scores = {}

    def score(seconds):
        seconds = round(seconds % SWAY_LOOP, 3)
        if seconds not in scores:
            png = pdir / ("sway-fit-%s.png" % seconds)
            cm.render_ours(args.oag_game, args.source, plat, item["page"], png, profile, ce.size_text(plat),
                           ("--menu-model-seconds", "%.3f" % seconds))
            if offset:
                ce.apply_display_offset(png, offset)
            scores[seconds] = round(box_grey(theirs, png, HULL_BOX), 3)
            png.unlink()
        return scores[seconds]

    for step in range(int(SWAY_LOOP / 0.25) + 1):
        score(step * 0.25)
    for step in (0.1, 0.05, 0.025):
        best = min(scores, key=scores.get)
        score(best - step)
        score(best + step)
    best = min(scores, key=scores.get)
    spread = max(scores.values()) - min(scores.values())
    if spread < MIN_SWAY_SPREAD:
        sys.exit("sway fit refused: the %d phases score within %.2f grey of each other (needs >= %.1f): the hull moves nothing in ours, or the original's "
                 "box shows no hull; pass --model-seconds to pin a phase by hand" % (len(scores), spread, MIN_SWAY_SPREAD))
    return best, dict(sorted(scores.items()))


def fit_head(args, item, plat, profile, theirs, capture, pdir, extra):
    """The head the screenshot caught, between the two RAM reads: ours is rendered with the message pinned at candidate heads (every third one, then
    the neighbours of the best) and the head whose ticker box lies closest to the original's is kept. Returns `(head, {head: box grey})`."""
    lo, hi = int(capture["head_after"]) - 1, int(capture["head_before"]) + 1
    offset = ce.display_offset_record(plat["key"], "menu")
    scores = {}

    def score(head):
        if head not in scores:
            png = pdir / ("ticker-fit-%d.png" % head)
            cm.render_ours(args.oag_game, args.source, plat, item["page"], png, profile, ce.size_text(plat),
                           extra + ("--menu-ticker", "%s@%d" % (capture["item"], head)))
            if offset:
                ce.apply_display_offset(png, offset)
            scores[head] = round(ticker_box_grey(theirs, png), 3)
            png.unlink()
        return scores[head]

    for head in sorted({*range(lo, hi + 1, 3), hi}):
        score(head)
    best = min(scores, key=scores.get)
    for head in range(best - 2, best + 3):
        if lo <= head <= hi:
            score(head)
    best = min(scores, key=scores.get)
    return best, dict(sorted(scores.items()))


def pulse_flags(pair, capture, head):
    """Ours' `--menu-picker-seconds` for the frame the ticker head fit picked: the counters read before and after the screenshot, placed the
    same fraction of the way as the fitted head is between its two reads (both move once a frame). The title's ladder step is recorded only."""
    pulse = capture["pulse"]
    span = capture["head_before"] - capture["head_after"]
    frac = (capture["head_before"] - head) / span if span > 0 else 0.0
    placed = pp.at_fraction(pulse["before"], pulse["after"], frac)
    flags = ()
    reads = {"before": pulse["before"], "after": pulse["after"], "fraction": round(min(1.0, max(0.0, frac)), 3), "front_end_clock_at_shot": round(placed["clock"], 4)}
    if "phase" in placed:
        seconds = pp.picker_seconds(placed["phase"])
        flags += ("--menu-picker-seconds", "%.3f" % seconds)
        pair.rec["original"]["seconds_since_entry"] = {
            "value": round(seconds, 3), "phase": round(placed["phase"], 4), "controller": "0x%08x" % pulse["controller"], "reads": reads,
            "basis": "not a wall clock: the Selector's phase (float at controller + 0xa0, GridController_UpdateSelectorStamp 0x0018ee30, advanced 2 dt and "
                     "wrapped at 2) read from EE RAM over PINE before and after the screenshot, placed at the fitted ticker head's frame; ours draws phase / 2 s"}
    if "s" in placed:
        pair.rec["original"]["title_glow"] = {
            "value": placed["s"], "rising": bool(placed["up"]), "seconds": round(pp.title_seconds(placed["s"]), 3), "text": "0x%08x" % pulse["text"], "reads": reads,
            "basis": "the card title's ladder byte s (Text + 0xed, Text_Draw 0x001a7fe8: 15 a tick of the front-end clock *(0x002e3640)+0x40 times 20, 0 to "
                     "150 and back) read from EE RAM over PINE before and after the screenshot and stepped to the fitted frame's tick; recorded, not passed to "
                     "ours: the original's title box is the same picture at every step"}
    return flags


def build_pair(args, item, out, plat, profile, raw, captured_at, capture):
    pdir = out / ("menu-" + item["id"])
    pdir.mkdir(parents=True, exist_ok=True)
    pair = ce.Pair(args.platform, pdir)
    pair.rec["meta"] = {"id": "%s-menu-%s" % (args.platform, item["id"]), "title": item["title"], "track": "front end", "craft": "none (a menu)",
                        "note": "Ours is `oag-game --menu-page %s`, drawn from %s. Both sides English, a fresh profile, campaign difficulty Medium." % (item["page"], item["xml"]),
                        "state": "PCSX2 savestate %s, GS screenshot" % item["state"]}
    w, h = plat["output"]
    shutil.copyfile(raw, pdir / "theirs-gs.png")
    size = ce.image_size(pdir / "theirs-gs.png")
    run(["magick", pdir / "theirs-gs.png", "-filter", "Lanczos", "-resize", "%dx%d!" % (w, h), pdir / "resized.png"])
    pair.rec["original"]["emulator"] = dict(plat["emulator"])
    pair.rec["original"]["gs_screenshot_size"] = size
    pair.rec["original"]["resize"] = "one Lanczos resize from %dx%d to %dx%d" % (*size, w, h)
    pair.rec["original"]["captured_at"] = captured_at
    pair.rec["original"]["capture_path"] = "savestate %s loaded, VM running, GS screenshot" % item["state"]
    pair.rec["original"]["selection_basis"] = "read by eye from the frame"
    pair.rec["original"]["screen_basis"] = "read by eye from the frame (no recovered state name for PS2 Pulse)"
    pair.check("GS screenshot aspect equals the output aspect", abs(size[0] / size[1] - w / h) < 0.01, "%dx%d against %dx%d" % (size[0], size[1], w, h))
    player = {"name": capture["name"], "tag": capture["tag"]}
    prepare_profile(plat, profile, player)
    model = ("--menu-model-seconds", "%.3f" % args.model_seconds) if item["id"] == "team" and args.model_seconds is not None else ()
    if item["id"] == "team" and args.model_seconds is None:
        sway, sway_scores = fit_sway(args, item, plat, profile, pdir / "resized.png", pdir)
        model = ("--menu-model-seconds", "%.3f" % sway)
        pair.rec["original"]["ship_sway_fitted"] = {
            "value": sway, "hull_box_mean_grey": sway_scores, "box": list(HULL_BOX), "loop_s": SWAY_LOOP,
            "basis": "fitted: the hull's `Anim Transform` phase whose hull box in ours is closest to the original's (ours rendered at each); the PS2's node clock "
                     "has no recovered address, so this is a fit and not a read"}
    head, scores = fit_head(args, item, plat, profile, pdir / "resized.png", capture, pdir, model)
    clocked = pulse_flags(pair, capture, head) if item["id"] in CLOCKED else ()
    pair.rec["original"]["footer_ticker"] = {
        "basis": "read from EE RAM over PINE with the VM running (scripts/ps2_ticker.py): the message is a rand() draw, so the grab waits for a frame with one tip "
                 "on screen well inside the viewport; a paused VM presents no frame, so the screenshot is bracketed by two reads of the record",
        "item": capture["item"], "text": capture["text"], "record_address": "0x%08x" % capture["record"],
        "records": ["0x%08x" % a for a in capture["addrs"]], "head_read_before_shot": capture["head_before"], "head_read_after_shot": capture["head_after"],
        "head_fitted": {"value": head, "ticker_box_mean_grey": scores, "box": list(TICKER_BOX),
                        "basis": "fitted: the head in the read bracket whose ticker box in ours is closest to the original's (ours rendered at each)"}}
    pair.rec["original"]["player"] = {"name": capture["name"], "tag": capture["tag"], "tag_address": "0x%08x" % capture["tag_addr"],
                                      "basis": "the profile record's name field (10 bytes, space padded, then a NUL) and the 3 characters after it, found in EE RAM by that shape"}
    cp.build(pair, args, "menu", "savestate_restore", item["state"], {"value": None, "unit": None, "basis": "not applicable: a menu is a still"})
    try:
        cm.menu_pair(pair, theirs_png=pdir / "resized.png", oag_game=args.oag_game, image=args.source, page=item["page"], drawn_from="xml",
                     xml_file=item["xml"], screen_seen=item["screen"], screen_wanted=item["screen"], profile_dir=profile,
                     size=ce.size_text(plat), front_end_style="the disc's own", language="English", original_language="English",
                     selection=item["selection"], screen_source="read by eye", player=player,
                     extra_args=model + clocked + ("--menu-ticker", "%s@%d" % (capture["item"], head)))
        pair.finish()
        (pdir / "resized.png").unlink()
    except SystemExit as exc:
        if exc.code == 3:
            return False
        raise
    return True


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("cmd", choices=("pairs",))
    ap.add_argument("--slot", type=int, required=True, help="the instance's PINE slot")
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--platform", default="pulse-ps2")
    ap.add_argument("--source", default=None, help="the disc image ours reads (default: the platform's image)")
    ap.add_argument("--oag-game", default=str(HERE.parent / "target/debug/oag-game"))
    ap.add_argument("--oag-wad", default=None, help="`oag-wad` (default: beside --oag-game)")
    ap.add_argument("--states", type=Path, default=ce.MAIN / "data" / "saves" / "pulse-ps2" / "menu")
    ap.add_argument("--ids", default="", help="only these WALK ids (comma separated)")
    ap.add_argument("--settle", type=float, default=4.0, help="seconds the VM runs after the state loads, before the grab")
    ap.add_argument("--model-seconds", type=float, default=None, help="ours' ship hull's own animation clock for the `team` pair (sway phase)")
    cp.add_arguments(ap)
    args = ap.parse_args()
    if not os.environ.get("OAG_PCSX2_DATA") or "OAG_PCSX2_DISPLAY" not in os.environ:
        sys.exit("source the lane's own env: OAG_PCSX2_DATA and OAG_PCSX2_DISPLAY must be set")
    if os.environ.get("OAG_COMPARE_PLATFORM") != args.platform:
        sys.exit("start PCSX2 with OAG_COMPARE_PLATFORM=%s so its ini carries the profile's settings" % args.platform)
    if not os.environ.get("XDG_CONFIG_HOME", "").startswith(str(ce.MAIN / "data" / "scratch")):
        sys.exit("XDG_CONFIG_HOME must be a directory under data/scratch/<lane>/: oag-game persists settings there")
    plat = ce.platform(args.platform)
    args.source = args.source or str(ce.MAIN / plat["image"])
    args.oag_wad = args.oag_wad or str(Path(args.oag_game).with_name("oag-wad"))
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    profile = Path(os.environ["XDG_CONFIG_HOME"]).parent
    refused = []
    for item in WALK:
        if args.ids and item["id"] not in args.ids.split(","):
            continue
        raw = out / ("raw-%s.png" % item["id"])
        capture = grab(args.slot, args.states / item["state"], raw, args.settle, args.oag_wad, args.source)
        at = datetime.datetime.now().astimezone().isoformat(timespec="seconds")
        if not build_pair(args, item, out, plat, profile, raw, at, capture):
            refused.append(item["id"])
        else:
            print("pair", item["id"], "ok")
    json.dump({"refused": refused}, open(out / "refused.json", "w"))
    return 3 if refused else 0


if __name__ == "__main__":
    sys.exit(main())
