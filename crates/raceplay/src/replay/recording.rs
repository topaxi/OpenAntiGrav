use oag_gameplay::PlayerInputs;
use oag_replay::header::GhostInfo;
use oag_replay::{GhostLap, Header, Pose, PoseRecorder, Recorder, Replay};

use crate::*;

/// The recording and the ghost, both optional.
#[derive(Debug, Default)]
pub(crate) struct ReplayState {
    pub(super) recording: Option<Recording>,
    ghost: Option<Ghost>,
    /// The player's `Ghost Ship Visible` is off: the ghost is still raced and
    /// recorded, but [`Race::ghost_pose`] draws nothing.
    ghost_hidden: bool,
    /// The launch request this race was loaded with, which a recording of it carries in its
    /// header ([`Header::setup`]) so the file can rebuild the race.
    request: Option<oag_replay::header::Setup>,
}

impl ReplayState {
    /// No recording, no ghost, and the request a recording will carry.
    pub(crate) fn requested(request: Option<oag_replay::header::Setup>) -> Self {
        Self {
            request,
            ..Self::default()
        }
    }
}

/// A recording in progress.
#[derive(Debug)]
pub(super) struct Recording {
    pub(super) recorder: Recorder,
    /// Every craft on the grid, refined as each segment closes: keys, not a pose a tick
    /// (`oag_replay::track`).
    track: PoseRecorder,
    /// The player's pose at world ticks `base..base + poses.len()`.
    ///
    /// Trimmed to the lap being driven at every line crossing, so an hour of
    /// Speed Lap holds one lap of poses rather than an hour of them.
    poses: Vec<Pose>,
    /// The world tick `poses[0]` was taken at.
    base: u64,
    /// The player's lap and lap-start tick as of the last tick, to see an
    /// edge in.
    last_lap: u32,
    last_start: Option<u64>,
    /// The quickest lap this run has driven, and where it sat.
    best: Option<(GhostInfo, GhostLap)>,
    /// Whether [`Self::best`] changed since the last
    /// [`Race::take_new_best_ghost`].
    best_unsaved: bool,
}

/// The lap being raced against.
#[derive(Debug, Clone)]
pub struct Ghost {
    /// Its pose track.
    pub lap: GhostLap,
    /// Whose hull it is drawn as.
    pub team: String,
}

impl Race {
    /// Starts recording this race: every tick's inputs, a hash a second, and
    /// the player's laps as ghost candidates.
    ///
    /// Called before the first tick - the recording's first hash is the
    /// state *before* it, which is what lets a replay handed the wrong race
    /// be refused at tick zero. `header` carries the key and whatever else
    /// the caller needs to rebuild this race; its slots are replaced by the
    /// slots a person flies here.
    ///
    /// A MODDED race ([`Self::is_modded`]) records nothing: a simulation mod's race
    /// has no replay in this phase (`docs/architecture/wasm-extensions/README.md` 4.4).
    pub fn start_recording(&mut self, mut header: Header) {
        if self.is_modded() {
            log::info!("replay: not recorded, a simulation mod runs in this race");
            return;
        }
        header.slots = self
            .sim
            .world
            .human_slots()
            .filter_map(|slot| u8::try_from(slot).ok())
            .collect();
        header.tick_rate = TickRate::DEFAULT.hz();
        header.setup = self.replay.request.clone();
        let recorder = Recorder::new(header, self.sim.state_hash());
        let standing = self.player_standing();
        let mut track = PoseRecorder::new(usize::from(self.ship_count()));
        track.push(&self.grid_poses());
        self.replay.recording = Some(Recording {
            recorder,
            track,
            poses: vec![self.player_pose()],
            base: self.sim.world.tick,
            last_lap: standing.lap,
            last_start: standing.lap_start_tick,
            best: None,
            best_unsaved: false,
        });
    }

    /// Stops recording and drops the ghost: a race that plays a recording back is not a run, so
    /// it keeps no lap, and nothing it does can be written as a ghost or a replay.
    pub fn stop_recording(&mut self) {
        self.replay.recording = None;
        self.replay.ghost = None;
    }

    /// Whether this race is being recorded.
    #[must_use]
    pub fn recording(&self) -> bool {
        self.replay.recording.is_some()
    }

    /// Races `ghost` from the player's next lap clock reading on.
    ///
    /// The lap this run itself sets, once one beats it, replaces it - see
    /// [`Self::record_tick`].
    pub fn set_ghost(&mut self, ghost: Ghost) {
        self.replay.ghost = Some(ghost);
    }

    /// Whether the ghost ship is drawn (Pulse's `Ghost Ship Visible`). Hidden, it is still
    /// raced, recorded and replaced by a quicker lap; only [`Self::ghost_pose`] stops
    /// reporting it, so no drawable and no fade is built. Display state, so the frame
    /// loop may set it every frame.
    pub fn set_ghost_visible(&mut self, visible: bool) {
        self.replay.ghost_hidden = !visible;
    }

    /// Whether the ghost is drawn ([`Self::set_ghost_visible`]).
    #[must_use]
    pub fn ghost_visible(&self) -> bool {
        !self.replay.ghost_hidden
    }

    /// The ghost being raced, if any.
    #[must_use]
    pub fn ghost(&self) -> Option<&Ghost> {
        self.replay.ghost.as_ref()
    }

    /// Where the ghost is now and where it was a tick ago, or `None` when
    /// there is no ghost or it is not on the circuit: the player's lap clock
    /// has not started, or the ghost has already finished its lap.
    ///
    /// **Read at the player's lap clock**, which is what makes the two start
    /// together: [`oag_race::Standing::lap_ticks`] counts from the same line
    /// crossing the ghost's own `poses[0]` was taken at.
    #[must_use]
    pub fn ghost_pose(&self) -> Option<(Pose, Pose)> {
        let ghost = self.replay.ghost.as_ref()?;
        if self.finished() || self.replay.ghost_hidden {
            return None;
        }
        let ticks = self.player_standing().lap_ticks(self.sim.world.tick)?;
        let now = ghost.lap.pose_at(ticks)?;
        let before = ticks
            .checked_sub(1)
            .and_then(|previous| ghost.lap.pose_at(previous))
            .unwrap_or(now);
        Some((now, before))
    }

    /// The model matrix a ghost pose draws a hull at: [`model_matrix_of`]'s
    /// own composition, from a stored pose instead of a live craft.
    #[must_use]
    pub fn ghost_model_matrix(pose: &Pose) -> Mat4 {
        Mat4::from_rotation_translation(pose.rotation, pose.position)
            * Mat4::from_rotation_y(MODEL_YAW)
            * Mat4::from_scale(Vec3::splat(oag_fx::exhaust::CRAFT_ROW_SCALE))
    }

    /// The best lap this run has set since the last call, as a replay ready
    /// to save: the inputs and hashes up to the end of that lap, and its
    /// poses. `None` when no lap has improved on it since.
    pub fn take_new_best_ghost(&mut self) -> Option<Replay> {
        let recording = self.replay.recording.as_mut()?;
        if !recording.best_unsaved {
            return None;
        }
        recording.best_unsaved = false;
        let (info, lap) = recording.best.clone()?;
        let mut replay = recording.recorder.replay(Some(lap));
        replay.truncate(info.start_tick + u64::from(info.lap_ticks));
        replay.header.ghost = Some(info);
        Some(replay)
    }

    /// The whole recording so far, with this run's best lap as its ghost and every craft's pose
    /// track. Valid at any tick: the open segment is closed in the copy, so a race that ends by
    /// finish, quit or restart writes the same kind of file.
    #[must_use]
    pub fn recorded(&self) -> Option<Replay> {
        let recording = self.replay.recording.as_ref()?;
        let mut replay = recording
            .recorder
            .replay(recording.best.as_ref().map(|(_, lap)| lap.clone()));
        replay.pose_track = recording.track.track();
        replay.header.ghost = recording.best.as_ref().map(|(info, _)| *info);
        Some(replay)
    }

    /// The player's craft as a pose: its position, and its orientation with
    /// the barrel roll and the steering lean folded in exactly as [`model_matrix_of`]
    /// folds them.
    fn player_pose(&self) -> Pose {
        self.craft_pose(self.player_slot())
    }

    /// Grid slot `slot`'s craft as the pose it is drawn at: its position, and its orientation
    /// with the barrel roll and the steering lean folded in. What the pose track records for
    /// every craft and the ghost for the player's.
    #[must_use]
    pub fn craft_pose(&self, slot: usize) -> Pose {
        let ship = &self.sim.world.ships[slot];
        let roll = oag_render::roll::display_rotation(
            Vec3::NEG_Z,
            ship.physics.roll_phase,
            ship.physics.steer_lean,
        );
        Pose {
            position: ship.physics.body.position,
            rotation: ship.physics.body.orientation * roll,
        }
    }

    /// Every craft on the grid, in slot order.
    fn grid_poses(&self) -> Vec<Pose> {
        (0..usize::from(self.ship_count()))
            .map(|slot| self.craft_pose(slot))
            .collect()
    }

    /// Everything the recording and the ghost do after a tick has run: the
    /// last thing [`Race::tick`] calls.
    pub(crate) fn record_tick(&mut self, inputs: &PlayerInputs) {
        let Some(recording) = self.replay.recording.as_mut() else {
            return;
        };
        let sim = &self.sim;
        recording.recorder.record(inputs, || sim.state_hash());

        let pose = self.player_pose();
        let grid = self.grid_poses();
        let standing = *self.player_standing();
        let tick = self.sim.world.tick;
        let Some(recording) = self.replay.recording.as_mut() else {
            return;
        };
        recording.poses.push(pose);
        recording.track.push(&grid);

        let crossed = standing.lap_start_tick != recording.last_start;
        let completed = crossed && standing.lap > recording.last_lap;
        if completed
            && let (Some(start), Some(end)) = (recording.last_start, standing.lap_start_tick)
            && let Some(lap) = recording.slice(start, end)
        {
            let info = GhostInfo {
                lap: recording.last_lap,
                start_tick: start,
                lap_ticks: lap.lap_ticks,
            };
            if lap.beats(recording.best.as_ref().map(|(_, best)| best)) {
                recording.best = Some((info, lap.clone()));
                recording.best_unsaved = true;
            }
            // Raced from the next lap on when it beats the ghost too - see
            // ADR-0055, "it improves within a run". Drawn as the player's own
            // hull from then on, since it is the player's lap.
            if lap.beats(self.replay.ghost.as_ref().map(|g| &g.lap)) {
                self.replay.ghost = Some(Ghost {
                    lap,
                    team: recording.recorder.header().team.clone(),
                });
            }
        }
        let Some(recording) = self.replay.recording.as_mut() else {
            return;
        };
        if crossed && let Some(start) = standing.lap_start_tick {
            recording.trim_before(start);
        }
        recording.last_lap = standing.lap;
        recording.last_start = standing.lap_start_tick;
        debug_assert_eq!(recording.base + recording.poses.len() as u64, tick + 1);
    }
}

impl Recording {
    /// The poses of world ticks `start..=end`, as a lap of `end - start`
    /// ticks, or `None` when any of them is no longer held.
    fn slice(&self, start: u64, end: u64) -> Option<GhostLap> {
        let from = usize::try_from(start.checked_sub(self.base)?).ok()?;
        let to = usize::try_from(end.checked_sub(self.base)?).ok()?;
        let poses = self.poses.get(from..=to)?.to_vec();
        Some(GhostLap {
            lap_ticks: u32::try_from(end - start).ok()?,
            poses,
        })
    }

    /// Forgets every pose before world tick `start`.
    fn trim_before(&mut self, start: u64) {
        let Some(drop) = start
            .checked_sub(self.base)
            .and_then(|n| usize::try_from(n).ok())
        else {
            return;
        };
        let drop = drop.min(self.poses.len());
        self.poses.drain(..drop);
        self.base += drop as u64;
    }
}

/// What a headless capture does with ghosts: race one read from a file, and
/// write the run's best lap to another. `oag-game`'s `--ghost` and
/// `--record-ghost` fill this.
#[derive(Debug, Clone, Default)]
pub struct GhostCapture {
    /// The replay file whose ghost lap to race.
    pub race: Option<std::path::PathBuf>,
    /// Where to write the run's best lap, and the header to record under.
    pub record: Option<(std::path::PathBuf, Header)>,
}

impl GhostCapture {
    /// Before the first tick: loads the ghost to race and starts recording.
    ///
    /// # Errors
    ///
    /// A ghost file that will not read or carries no lap - named on the
    /// command line, so a failure is the caller's to see, not to skip.
    pub fn arm(&self, race: &mut Race) -> Result<()> {
        if let Some(path) = &self.race {
            let bytes =
                std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
            let read = Replay::read(&bytes)
                .with_context(|| format!("reading the ghost in {}", path.display()))?;
            if !read.schema_header {
                log::debug!(
                    "{} has no toml-schema header (an older file); loaded as it is",
                    path.display()
                );
            }
            let replay = read.replay;
            let lap = replay
                .ghost
                .with_context(|| format!("{} carries no ghost lap", path.display()))?;
            println!(
                "racing the ghost in {}: {} ticks, {}",
                path.display(),
                lap.lap_ticks,
                replay.header.team
            );
            race.set_ghost(Ghost {
                lap,
                team: replay.header.team,
            });
        }
        if let Some((_, header)) = &self.record {
            race.start_recording(header.clone());
        }
        Ok(())
    }

    /// After the last tick: writes the run's best lap, if it set one.
    ///
    /// # Errors
    ///
    /// A file that cannot be written.
    pub fn finish(&self, race: &mut Race) -> Result<()> {
        if self.race.is_some() {
            match race.ghost_pose() {
                Some((pose, _)) => {
                    let distance = (race.ship().physics.body.position - pose.position).length();
                    let fade = oag_render::ghost::fade(distance);
                    println!(
                        "ghost: {distance:.1} units from the player, weight {:.3}, glow {}",
                        fade.weight, fade.glow
                    );
                }
                None => println!("ghost: not on the circuit at this tick"),
            }
        }
        let Some((path, _)) = &self.record else {
            return Ok(());
        };
        let Some(replay) = race.take_new_best_ghost() else {
            println!("--record-ghost: no lap completed, nothing written");
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, replay.to_bytes())
            .with_context(|| format!("writing {}", path.display()))?;
        println!(
            "--record-ghost: a lap of {} ticks written to {}",
            replay.ghost.as_ref().map_or(0, |lap| lap.lap_ticks),
            path.display()
        );
        Ok(())
    }
}
