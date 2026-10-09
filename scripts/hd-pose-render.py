#!/usr/bin/env python3
"""Render ours at a matched capture pair's recovered pose, at the pair's own size.

    scripts/hd-pose-render.py data/reference/hd-capture/metropia-bloom 00 out.png [--game target/debug/oag-game]

Same comparison settings as `hd-frame-compare.py` (it imports that script's own
arguments), bloom on, 16:9. Runs under a scratch `XDG_*` tree (`--profile DIR`,
default a temporary one) so no run touches the maintainer's own settings.
"""
import argparse, importlib.util, json, os, pathlib, subprocess, tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("hfc", ROOT / "scripts" / "hd-frame-compare.py")
hfc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(hfc)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("pair_dir")
    ap.add_argument("pose")
    ap.add_argument("out")
    ap.add_argument("--game", default=str(ROOT / "target" / "debug" / "oag-game"))
    ap.add_argument("--profile")
    args = ap.parse_args()
    pair = pathlib.Path(args.pair_dir)
    meta = json.loads((pair / f"{args.pose}.json").read_text())
    from PIL import Image
    width, height = Image.open(pair / f"{args.pose}.png").size
    flags = list(hfc.COMPARISON_ARGS)
    flags[1] = f"{width}x{height}"
    with tempfile.TemporaryDirectory() as tmp:
        cfg = pathlib.Path(args.profile or tmp)
        (cfg / "oag").mkdir(parents=True, exist_ok=True)
        (cfg / "oag" / "settings.toml").write_text(
            '[graphics]\nbloom = true\n\n[display]\naspect = "wide"\n', encoding="utf-8")
        cmd = [args.game, str(hfc.IMAGE), "--race", "--track", hfc.track_arg(meta["track"]),
               "--team", hfc.TEAM, *flags, *hfc.camera_pose_args(meta["camera"]),
               "--no-audio", "--screenshot", args.out]
        env = dict(os.environ, XDG_CONFIG_HOME=str(cfg), XDG_DATA_HOME=str(cfg), XDG_STATE_HOME=str(cfg))
        done = subprocess.run(cmd, capture_output=True, text=True, env=env, cwd=ROOT)
        if done.returncode:
            raise SystemExit(done.stderr[-400:])


if __name__ == "__main__":
    main()
