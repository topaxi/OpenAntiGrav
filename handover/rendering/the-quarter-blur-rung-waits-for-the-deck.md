---
status: blocked-hardware
titles: [all]
updated: 2026-11-06
summary: "Quarter blur rung and the blur's frame-rate ramp on branch blur-quarter-rung, -31 to -33 % at 4K on the iGPU; the maintainer's Deck runs decide the merge"
difficulty: easy (70)
---

# The quarter blur rung waits for the Deck

Branch `blur-quarter-rung`, **not on `main`**: the maintainer merges it, or not, after measuring it
on the Steam Deck. Evidence: `docs/rendering/motion-blur-half-rung.md` ("The quarter step"),
ADR-0087. The tile skip (the other half of this lane) came out false and landed as a docs note,
`docs/rendering/hires-gpu-levers.md`, lever 2.

## Open

- **Does the step hold a higher extent on the Deck?** On the iGPU at 4K, target 30, it does since
  the 540 lines moved to the ceiling: 5.77 MP against 4.67 to 5.03, two flips. The Deck on HD at
  native 4K is the case to read: the base rung settles at 3264x1836 there.
- **Is none at 30 right?** The ramp (ADR-0087, point 8) is 1 at 60, 0 at 30, both chosen. On the
  iGPU at 4K, target 30, the chain went off once and stayed off: 8.1 MP at 47.8 fps, no blur at
  `high`. A target of 30 means no blur on any machine that needs it. Judge the ramp's two ends and
  its fade on a real panel: nobody has watched it. `SMOOTHING` is derived, the ends are not.
- **Paused frame.** The bit-equality is a unit test; no pixel diff of a paused frame was taken.
  `press start` on a `--drive` script did not pause.
- **Quarter at a fallen extent.** Gathers go under 540 lines there: 270 at the default floor, 135
  at 25 %. Nobody has looked at those.
- **Is the strength cap worth it?** It is 5.3 of the 8.5 ms saved on both titles. The cap on its
  own at half is most of the win and needs no new gather; if the Deck agrees, a cap with no quarter
  step is the smaller change. Not built.
- **A stepped fringe on HD.** The dark underside of a thin wing smears over the background in a
  faint 8 px band, stair-stepped at the 4 px texel (tick 1501). `low`+`half` has it finer. It holds
  still, and takes looking for at native size. Judge it on a real 4K panel.
- **A flip in motion.** Crops and four consecutive frames are otherwise clean. Nobody has watched the 0.25 s
  ease or a quarter flip in motion. `CAP_EASE_SECONDS` is chosen.
- **The quarter reconstruct costs twice as much per fragment as half's.** Not traced; a lower tap
  count at quarter is the first thing to try (`motion_blur.wesl`, `TAP_SPACING_PX`).
- **Row wording.** Under a 2160-line ceiling the row says `quarter` and the frame draws `half`, stated only
  in `menu.toml`'s comment. A player sees no note.

## Next Steps

1. Build one binary from the branch and one from its merge base, on the workstation. The second is
   needed for the race sessions. `<worktree>` is a checkout at each commit, with `data/appimage` a
   real directory (`docs/tools/packaging.md`):

   ```sh
   podman run --rm -v <worktree>:/src -v data/appimage/container:/cache -w /src \
     -e CARGO_HOME=/cache/cargo -e CARGO_TARGET_DIR=/cache/target-quarter \
     -e RUSTFLAGS="-C target-cpu=znver2" oag-appimage-build:bookworm \
     cargo build --profile dist -p oag-game --no-default-features \
       --features extensions,dev-cli,perf-probe
   ```

2. Copy both to `deck@steamdeck:oag-perf/blur-quarter-rung/` as `oag-game-quarter` and
   `oag-game-base`, and `scripts/bench-render.py` to `scripts/` under it. The script finds images
   under its parent's `data/images`. Whole race, offscreen, so vsync and the frame limit are not
   involved; three sizes, two titles, three rungs:

   ```sh
   cd ~/oag-perf/blur-quarter-rung
   mkdir -p data && ln -sfn ~/.local/share/oag/images data/images
   for size in 1280x800 1920x1080 3840x2160; do for title in pulse hd; do
     for rung in "high half" "low half" "high quarter"; do set -- $rung
       ~/oag-perf/deck-run.sh --wait 3600 -- python3 scripts/bench-render.py run \
         --out $PWD/out/$title-$size-$1-$2 --size $size --motion-blur $1 --motion-blur-resolution $2 \
         --binary ./oag-game-quarter $title
     done
   done; done
   ```

   Expected: at 1280x800 and 1920x1080 `high quarter` reads as `high half` (the line rule); at
   3840x2160 about 30 % under it. Read `gpu_blur_us` p50 and `gpu_us`, adapter `RADV VANGOGH`. The
   Deck's 4K rows are good to about 15 % (`docs/rendering/hires-gpu-levers.md`).

3. The race sessions, the one that decides it. Write this to `cfg/oag/settings.toml`, after
   `mkdir -p out cfg/oag`, once per title. Keep `aspect = "free"` for HD and delete it for Pulse,
   which draws the PSP aspect; the profile keys are `Wipeout HD (PS3)` and `Wipeout Pulse (PSP)`:

   ```toml
   [display]
   pause_on_focus_loss = false
   vsync = "off"
   frame_limit = "unlimited"
   aspect = "free"
   [render_profiles."Wipeout HD (PS3)"]
   render_scale = 100
   render_scale_chosen = true
   minimum_resolution = 25
   motion_blur = "high"
   motion_blur_resolution = "full"
   target_fps = "30"
   ```

   Then, with `title` and `image` set for that title, both binaries, three sizes, two runs:

   ```sh
   for bin in base quarter; do for size in "1280 800" "1920 1080" "3840 2160"; do for run in 1 2; do
     set -- $size
     ~/oag-perf/deck-run.sh --wait 3600 -- env RUST_LOG=warn,oag=debug XDG_CONFIG_HOME=$PWD/cfg \
       XDG_DATA_HOME=$PWD/data XDG_STATE_HOME=$PWD/state ENABLE_GAMESCOPE_WSI=0 \
       gamescope --backend headless -W $1 -H $2 -w $1 -h $2 -- \
       ./oag-game-$bin $image --race --autopilot --no-audio --frame-log --seed 1 --script 90:quit \
       2>&1 | tee out/session-$title-$bin-$1-$run.log
   done; done; done
   ```

   Read the median `fps`, `p99_ms`, the mean of `render=` and the `motion blur gathers at` lines.
   Expected: 1280x800 and 1920x1080 the same as base (no quarter under 2160 lines; the Deck at 800p
   is bound by the CPU), HD at 3840x2160 a higher `render=` than base's 3264x1836. A worse picture
   in motion or no extent gained means do not merge the gather; keep the docs.

4. The ramp, with the same binaries and the same `settings.toml`, `motion_blur_resolution =
   "half"`, `target_fps = "off"`, at each size, each title, each steady rate (`frame_limit` takes
   any rate), 40 seconds each. Sessions are vsync off; the limit here is what holds the rate:

   ```sh
   for size in "1280 800" "1920 1080" "3840 2160"; do for rate in 30 40 50; do
     set -- $size
     sed -i "s/^frame_limit = .*/frame_limit = \"$rate\"/" cfg/oag/settings.toml
     ~/oag-perf/deck-run.sh --wait 3600 -- env RUST_LOG=warn,oag=debug XDG_CONFIG_HOME=$PWD/cfg \
       XDG_DATA_HOME=$PWD/data XDG_STATE_HOME=$PWD/state ENABLE_GAMESCOPE_WSI=0 \
       gamescope --backend headless -W $1 -H $2 -w $1 -h $2 -- \
       ./oag-game-quarter $image --race --autopilot --no-audio --frame-log --seed 1 --script 40:quit \
       2>&1 | tee out/ramp-$title-$rate-$1.log
   done; done
   ```

   Read `blur_ramp=` on the `frame-log` lines: expected 0.00 at 30, 0.33 at 40, 0.67 at 50 where the
   rate is held, flat from second to second, and `motion blur ramp:` lines counting the flips: one
   at most. Then, in the same session as step 3 with `frame_limit = "unlimited"` and the target at
   60, the settled `blur_ramp=` and whether the fps spread grows against `oag-game-base`. Finally
   sit at a steady 30, 40 and 50 on the Deck's screen and judge the fade and whether none at 30 is
   right.

5. Decide, and write the result on the page, and in an ADR superseding 0087 if the decision
   changes. A settings key the build rejects logs `ignoring` and keeps its default, so read that
   line before reading a run.
