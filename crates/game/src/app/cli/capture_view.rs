//! The clock, the camera and the picture of a capture.

/// The clock, the camera and the picture of a capture.
#[derive(clap::Args, Debug)]
pub(crate) struct CaptureViewArgs {
    /// Seconds added to the world meshes' texture-animation clock (scrolling surfaces, flip-book signs, the neon
    /// strips), which is the race's own tick count over 60 otherwise. The original feeds them a clock that is the
    /// HUD clock plus a per-state constant (13.6 to 24.3 s on the 24 Pulse PSP grid states; inferred to be the time
    /// since the circuit loaded, flyby and countdown);
    /// `scripts/psp-world-anim-clock.py` reads that offset off a state. Unlike `--anim-seconds` it leaves the GO
    /// board and the HUD on their own clocks.
    #[arg(long, value_name = "SECONDS", allow_negative_numbers = true)]
    pub(crate) world_clock_offset: Option<f32>,

    /// Show the start gantry's board at this animation time instead of the one the race clock gives.
    /// **A capture aid**: the original's GO board loops over a window of its own node's time, and a
    /// pair is only comparable when ours is put at the phase that node held (`+0xc0`, read off RAM).
    /// Unlike `--anim-seconds` it leaves every other clock alone.
    #[arg(long, value_name = "SECONDS")]
    pub(crate) gantry_seconds: Option<f32>,

    /// Draw the engine flame surfaces at this clock instead of the world clock. **A capture aid**:
    /// HD's flame scrolls its noise by `2 * time` with `time` the frame context's boot clock (engine
    /// parameter slot 0), which a pair records as `original.flame_clock_s`
    /// (`docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`, "The flame's clock at a pair's grab").
    /// Unlike `--anim-seconds` it leaves every other clock alone.
    #[arg(long, value_name = "SECONDS")]
    pub(crate) flame_clock: Option<f32>,

    /// Which tick of `--pose-from` to take the pose off.
    #[arg(long, default_value_t = 0, requires = "pose_from")]
    pub(crate) pose_tick: u64,

    /// With `--pose-from`: ignore the capture's camera columns and keep the
    /// chase camera.
    #[arg(long, requires = "pose_from")]
    pub(crate) no_camera: bool,

    /// With `--pose-from`: render the recorded camera at this fov instead of
    /// the disc's authored value, in **vertical degrees** - the authored
    /// value's own unit (confidence 94). With `--no-camera` it sets the field
    /// the chase camera draws with (`Options::fov_override`).
    ///
    /// It was the calibration knob for settling that unit. It is now a
    /// *measurement instrument* rather than a leftover: `place_at` resets the
    /// body, so a posed craft has zero velocity and never receives the
    /// original's speed-dependent widen. **Every matched-pose comparison needs
    /// this flag**, set to `60 + 0.075 * dot(fwd, vel)` for the captured tick.
    /// See `docs/rendering/projection-vs-the-original.md`.
    #[arg(long)]
    pub(crate) camera_fov: Option<f32>,

    /// Leave out a Pulse hull's `0x2000` environment-mapped extra pass
    /// (`oag_render::shine`): the headless way to measure what the pass adds,
    /// by rendering the same frame with and without it.
    #[arg(long)]
    pub(crate) no_hull_shine: bool,

    /// Leave out a Pulse circuit's own extra pass: its `*_shinemap` batches
    /// under their chrome map, which `oag_render::shine::build_track` draws.
    #[arg(long)]
    pub(crate) no_track_shine: bool,

    /// Render from a camera given outright, as nine comma-separated numbers:
    /// `eye_x,eye_y,eye_z,fwd_x,fwd_y,fwd_z,up_x,up_y,up_z`.
    ///
    /// **The consumer of an RPCS3 capture.** `scripts/rpcs3-drive.py capture`
    /// pairs a screenshot with the `viewProj` the original's own shaders were
    /// fed, and `scripts/ps3_pose.py` decomposes that matrix into exactly these
    /// nine numbers plus a field of view. Passing them here puts this renderer
    /// at the original's camera, so the two frames can be laid over each other
    /// instead of eyeballed at approximately the same place.
    ///
    /// `--camera-fov` is what carries the tenth number, and a matched-pose
    /// comparison wants both. Unlike the other pose flags this one needs no
    /// `--pose-from`: a captured camera is a complete statement on its own.
    #[arg(long, value_name = "EYE,FWD,UP", conflicts_with = "pose_from")]
    pub(crate) camera_pose: Option<String>,

    /// With `--screenshot`: draw the frame with photo mode's picture controls, as
    /// `exposure=N,saturation=N,speed_blur=N` and, for depth of field, `aperture=N,focus=N`
    /// (the panel's `0..=100` values; a missing one is where the panel opens it).
    ///
    /// What a photo-mode shot's `[picture]` text names, so `--from-png` and a bug report's
    /// `reproduce.flags` draw the graded or blurred picture. One frame: a motion blur needs the
    /// race's past and is not drawn.
    #[arg(long, value_name = "CONTROLS")]
    pub(crate) picture: Option<String>,

    /// Draw with the profile a photo mode shot is made with (`RenderProfile::for_a_photo`): set
    /// by `--from-png` on a photo, so its re-render is not at the render scale or through the
    /// upscaler of the profile it runs in.
    #[arg(skip)]
    pub(crate) photo_profile: bool,
}
