#!/usr/bin/env python3
"""Sweeps Wipeout HD's sixteen circuits and measures how bright each frame is.

The harness behind `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "HD's
emissive glow reaches the lit path" - and it is committed for one reason: the
2026-08-20 brightness figures this subsystem was tuned against **cannot be
reproduced**, because the harness that produced them lived in a scratch
directory. Two independent numbers in that investigation turned out to be
unrecoverable. This file is the fix for that class of loss, not a convenience.

    scripts/hd-glow-sweep.py reference          # the rpcs3 grabs, trimmed
    scripts/hd-glow-sweep.py sweep --bloom on   # our sixteen, bloom on
    scripts/hd-glow-sweep.py sweep --bloom both --ticks 0 --label fixed

The metric is `scripts/clipped-white.py`'s - share of pixels with R, G and B
all at least 250. **Mean luminance does not discriminate** and is why the
2026-08-20 defect survived review; it is printed alongside as context and
never as the verdict.

# Why `reference` trims

**`data/reference/hd-capture/talons/00.png` is a 1600x1200 canvas holding a
1278x718 frame at +322+24, and the rest of it is black.** Measured whole it
reads 3.850 %; measured over its own picture it reads **8.056 %**. The
"3.85 %" that this project carried as the *lower* bound of its reference band
was 47.8 % picture and 52.2 % padding, and the true figure is the highest of
the two grabs the band was built from rather than the lowest. `talons/01.png`
and the four under `flare-size/` are padded the same way; every other grab in
that tree is a clean 1278x718.

So `reference` trims a black border off every grab before measuring, which is
a no-op on the unpadded ones. **Do not compare a padded canvas against one of
our captures** - the aggregate is a share, and padding moves it by the padding
fraction alone.

# Which bloom

`[graphics] bloom` defaults to `false` and reaches HD as of `9f6a1be3`, so
every measurement has to name which side of it it was taken on. This writes
its own `settings.toml` into a scratch `XDG_CONFIG_HOME` rather than reading
or touching the player's, so a sweep never depends on, and never edits, the
configuration of the machine it runs on.

# Needs

`magick` on PATH, a built `oag-game` (`cargo build --release -p oag-game`),
and the decrypted PS3 image under `data/images/`. `data/` is gitignored, so
`rg` and `fd` return nothing under it with exit code 0 - use `ls`/`find`.
"""

import argparse
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
IMAGE = ROOT / "data" / "images" / "hdfury-ps3-eu-dec.iso"
GAME = ROOT / "target" / "release" / "oag-game"

# The sixteen environment directories holding a `track.vex`, in archive order.
# Mirrors `oag_hd::layout::ENVIRONMENTS`; kept as a literal here so the sweep
# runs against a checkout with nothing built but the binary.
ENVIRONMENTS = [
    "amphiseum",
    "modesto_heights",
    "talons_junction",
    "tech_de_ra",
    "zone_1",
    "zone_2",
    "zone_3",
    "zone_4",
    "01_vineta_k",
    "02_track",
    "03_track",
    "04_chenghou_project",
    "05_ubermall",
    "10_sebenco_climb",
    "12_sol_2",
    "15_anulpha_pass",
]


def measure(path):
    """Clipped-white percent and mean luminance, via `clipped-white.py`.

    Shelled rather than imported: that script is another lane's file and its
    module name is not importable anyway (a hyphen). Its stdout is
    `  9.860 %  mean 0.5131  (1175040 px)  <path>`.
    """
    out = subprocess.run(
        [sys.executable, str(ROOT / "scripts" / "clipped-white.py"), str(path)],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.split()
    return float(out[0]), float(out[3])


def trimmed(path, into):
    """A copy of `path` with any black border removed, for measuring.

    `-fuzz 1%` rather than an exact match because a lossless grab of a letter-
    boxed frame still carries a row or two of near-black at the seam. A grab
    with no padding trims to itself.
    """
    out = into / (path.parent.name + "-" + path.stem + ".png")
    subprocess.run(
        [
            "magick",
            str(path),
            "-bordercolor",
            "black",
            "-fuzz",
            "1%",
            "-trim",
            "+repage",
            str(out),
        ],
        check=True,
    )
    return out


def reference(args):
    """Measures every rpcs3 race grab under `data/reference/hd-capture/`."""
    root = ROOT / "data" / "reference" / "hd-capture"
    if not root.is_dir():
        raise SystemExit(f"{root} is missing - see data/README.md")
    # The race grabs only: the `menus*`, `racebox` and `flare-size` trees hold
    # front-end screens and a calibration grid, which are not race frames and
    # have no business in a band a race frame is read against.
    grabs = sorted(
        p
        for p in root.glob("*/*.png")
        if p.stem.isdigit() and not p.parent.name.startswith(("menus", "racebox"))
    )
    if not grabs:
        raise SystemExit(f"no numbered grabs under {root}")
    with tempfile.TemporaryDirectory() as tmp:
        into = pathlib.Path(tmp)
        rows = []
        for grab in grabs:
            size = subprocess.run(
                ["magick", "identify", "-format", "%wx%h", str(grab)],
                check=True,
                capture_output=True,
                text=True,
            ).stdout
            cut = trimmed(grab, into)
            cut_size = subprocess.run(
                ["magick", "identify", "-format", "%wx%h", str(cut)],
                check=True,
                capture_output=True,
                text=True,
            ).stdout
            clipped, mean = measure(cut)
            rows.append((grab.relative_to(root), size, cut_size, clipped, mean))
    print("| grab | canvas | picture | clipped white | mean |")
    print("| --- | --- | --- | --- | --- |")
    for name, size, cut, clipped, mean in rows:
        pad = " **padded**" if size != cut else ""
        print(f"| `{name}` | {size}{pad} | {cut} | {clipped:.3f} % | {mean:.4f} |")
    shares = [r[3] for r in rows]
    print(f"\n{len(rows)} grab(s): {min(shares):.3f} % to {max(shares):.3f} %")


def capture(env, bloom, args, into):
    """One `--screenshot` run, returning its PNG path or None if it failed."""
    cfg = into / f"cfg-{bloom}"
    (cfg / "oag").mkdir(parents=True, exist_ok=True)
    (cfg / "oag" / "settings.toml").write_text(
        f"[graphics]\nbloom = {'true' if bloom == 'on' else 'false'}\n",
        encoding="utf-8",
    )
    shot = into / f"{env}-{bloom}.png"
    env_vars = dict(os.environ, XDG_CONFIG_HOME=str(cfg))
    cmd = [
        str(GAME),
        str(IMAGE),
        "--race",
        "--track",
        f"/data/environments/{env}/track.vex",
        "--ticks",
        str(args.ticks),
        "--size",
        args.size,
        "--no-audio",
        "--screenshot",
        str(shot),
    ]
    if args.autopilot:
        cmd.append("--autopilot")
    done = subprocess.run(cmd, env=env_vars, capture_output=True, text=True)
    if done.returncode != 0 or not shot.exists():
        tail = (done.stderr or done.stdout).strip().splitlines()[-1:]
        print(f"  {env} ({bloom}): FAILED {' '.join(tail)}", file=sys.stderr)
        return None
    return shot


def sweep(args):
    """Captures each environment at the requested bloom setting(s)."""
    if not GAME.exists():
        raise SystemExit(f"{GAME} is missing - cargo build --release -p oag-game")
    if not IMAGE.exists():
        raise SystemExit(f"{IMAGE} is missing - see data/README.md")
    sides = ["on", "off"] if args.bloom == "both" else [args.bloom]
    envs = args.environments or ENVIRONMENTS
    keep = pathlib.Path(args.keep) if args.keep else None
    if keep:
        keep.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        into = pathlib.Path(tmp)
        rows = []
        for env in envs:
            row = {"env": env}
            for bloom in sides:
                shot = capture(env, bloom, args, into)
                row[bloom] = measure(shot) if shot else None
                if shot and keep:
                    shutil.copy2(shot, keep / f"{args.label}-{env}-{bloom}.png")
            rows.append(row)
    head = "| circuit |" + "".join(f" bloom {s} |" for s in sides)
    print(f"\n{args.label}: --ticks {args.ticks}, {args.size}"
          f"{', autopilot' if args.autopilot else ''}")
    print(head)
    print("| --- |" + " --- |" * len(sides))
    for row in rows:
        cells = "".join(
            f" {row[s][0]:.3f} % |" if row[s] else " - |" for s in sides
        )
        print(f"| `{row['env']}` |{cells}")
    for side in sides:
        got = [r[side][0] for r in rows if r[side]]
        if got:
            print(f"\nbloom {side}: {len(got)} circuit(s), "
                  f"{min(got):.3f} % to {max(got):.3f} %, "
                  f"median {sorted(got)[len(got) // 2]:.3f} %")


def main():
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    sub = parser.add_subparsers(dest="command", required=True)

    ref = sub.add_parser("reference", help="measure the rpcs3 grabs, trimmed")
    ref.set_defaults(func=reference)

    run = sub.add_parser("sweep", help="capture and measure our own frames")
    run.add_argument("--bloom", choices=["on", "off", "both"], default="both")
    run.add_argument("--ticks", type=int, default=0)
    run.add_argument("--size", default="1440x816")
    run.add_argument("--autopilot", action="store_true")
    run.add_argument("--label", default="ours")
    run.add_argument("--keep", help="directory to copy every capture into")
    run.add_argument("environments", nargs="*")
    run.set_defaults(func=sweep)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
