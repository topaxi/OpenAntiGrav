//! Flying a ship on a real track: the composition every layer beneath this was
//! built for.
//!
//! Nothing here is a new subsystem. The track spline, the collision soup, the
//! handling parameters, the force law, the keyboard mapping, the mesh pipeline and
//! the chase camera all already exist and are all already tested; this module is
//! the wiring, and it is deliberately thin so that a surprise in the picture is a
//! surprise in one of those layers rather than in here.
//!
//! ```text
//! Data.wad -> track.vex          -> WO Track spline -> spawn pose, ribbon
//!                                -> collision nodes -> oag_physics::CollisionWorld
//!          -> Ship.vex                              -> the drawn model
//!          -> handlingstats.xml                     -> Handling, ChaseParams
//! ```
//!
//! # What is drawn
//!
//! The track's **art meshes** - the map, textured, as a player sees it. 144,351
//! triangles on `16_Track`.
//!
//! `--ribbon` swaps in the **driveable ribbon** instead, a flat band over the
//! spline, and that is a **development view** rather than a style: it is the
//! geometry the simulation actually spawns on and queries, so a picture of
//! ship-plus-ribbon shows directly whether the ship is where the physics thinks it
//! is, where the art meshes are prettier and prove less.
//!
//! **The default used to be the other way round**, and that was right while M4's
//! whole job was verifying a force law against a captured trace. It stopped being
//! right once there was a game to look at: a player launching `just play` was
//! shown a debug view of a spline.
//!
//! # Three things are not recovered, and none is papered over here
//!
//! - ~~**There is no inertia tensor in the ship data**~~, and there is not - but there
//!   is one in the *code*, and it has been read. [`box_inertia`] is now
//!   [`oag_physics::forces::ship_inertia`], the code-literal box `(12, 8, 12)` at a
//!   mass of `0.9` that `Body_SetBoxInertia`'s single call site passes, giving
//!   `(15.6, 21.6, 15.6)` for every craft in the game. The two earlier positions this
//!   module held - "a derived tensor is an invented constant, worse than a visible
//!   oddity", then "build one from `<Misc>`" - are both superseded by a reading, and
//!   the `<Misc>` derivation was `4.4x` out on roll. See [`box_inertia`].
//! - **The magnetic hold is not implemented**, so an inverted ship falls off. That
//!   is expected and documented in `docs/physics/README.md`.
//! - **The camera's `fov` unit is unrecovered.** It is read as degrees and
//!   converted explicitly in [`Race::projection`]; the value read is in the load
//!   report so a reader can judge the assumption.
//!
//! Nothing in this module tunes, scales or corrects a physics value. The four
//! pre-scaled handling fields are scaled exactly once, inside
//! [`oag_gameplay::handling_for`], and grid-slot assignment - which
//! `oag_gameplay::spawn` records as an open question - is not guessed at: one ship
//! starts on the racing line at the beginning of the first path.
//!
//! Two conventions *are* reconciled here, both at the boundary and both scored:
//! [`chase_pos_length`], because the disc's camera offset is signed the other way
//! from the renderer's, and [`MODEL_YAW`], because a `.vex` ship is authored nose
//! along `+Z` while the simulation's forward is `-Z`. Each is one expression with
//! its evidence written next to it.

use anyhow::{Context, Result};
use oag_assets::pulse;
use oag_core::math::frustum::Frustum;
use oag_core::math::{Mat4, Vec3};
use oag_core::{Rng, TickClock, TickRate};
use oag_formats::track::{AiTrack, Sample, StartPosition};
use oag_formats::vex;
use oag_formats::{collision, handling};
use oag_gameplay::{
    InputSnapshot, Pose, Ship, World, collision_world, handling_for, ship_controls,
};
use oag_input::Keyboard;
use oag_physics::{CollisionWorld, Environment, Evaluated, Handling, SpeedClass};
use oag_render::camera::chase::{Chase, ChaseParams, Target};
use oag_render::collision as render_collision;
use oag_render::exhaust::{self, Exhaust, FlareTexture};
use oag_render::mesh::{DrawCall, Model};
use oag_render::mesh_render::Anisotropy;
use oag_render::pvs::{DrawSections, PlacementStats, SectionPadding, UNPLACED, VisibleSet};
use oag_render::sparks::{self, Sparks};
use oag_render::{mesh, mesh_render, track as track_render};

/// The track a race is flown on unless another is named.
///
/// **`16_Track`, and this is a measurement rather than a preference.** The reference
/// scenario every PPSSPP capture uses is Talon's Junction White (see
/// `docs/reverse-engineering/ppsspp-debugger.md`), and the directory that holds it was
/// assumed to be `01_Track` until the recording was checked against the geometry
/// instead of against the name:
///
/// - `Data\Plugins\PI001\Definition.xml` lists `<PI_Track name="16_Track">` **first**,
///   at `soundregister="1"`, with `location="Data\Environments\16_Track"`.
///   `01_Track` is `soundregister="18"`.
/// - Decisively, the capture's own 200 recorded positions were cast against every
///   track on the disc. `16_Track` is the only one that finds geometry under **200 of
///   200** of them, and the mean height it finds is **4.002** against the
///   `4.125 - 0.147 = 3.978` this crate's own spring predicts for a resting ship.
///   Every other track and every sign convention comes out at a few dozen units off
///   or misses outright.
///
/// So a trace comparison run against `01_Track` seeded the original's Talon's Junction
/// position into a different track's collision soup, which is why `grounded` read `0.0`
/// from the first tick while the recording read `1.0` on all 200. That was read as a
/// force-law failure and it was a track-selection one.
///
/// Note the reciprocal fact recorded with it: the same 200 positions sit a fairly
/// constant **~21 units** from `16_Track`'s AI racing line, so `spline_distance` and
/// "on the driveable surface" are not the same measurement for this recording.
pub const DEFAULT_TRACK: &str = r"Data\Environments\16_Track\track.vex";

/// The team whose `handlingstats.xml` and model a race uses by default.
///
/// **Assegai, not Feisar** - chosen to match the reference scenario used for
/// every PPSSPP capture this project has taken (Time Trial, Venom, Talon's
/// Junction White, Assegai; see
/// `docs/reverse-engineering/ppsspp-debugger.md`'s "reference scenario"
/// section), so `just play --race`'s defaults and a captured trace are
/// directly comparable without passing `--team`/`--class` every time.
pub const DEFAULT_TEAM: &str = "Assegai";

/// Turns the ship model's own facing into the body's.
///
/// **A `.vex` ship is authored nose along `+Z`**, and
/// [`oag_physics::Body::forward`] is `-Z`, so a model drawn straight from the body's
/// orientation faces backwards: the chase camera sits behind the body, which is the
/// model's *nose* side, and draws the ship head-on. A half turn about the model's up
/// axis reconciles them.
///
/// Measured rather than assumed, on the default team's `Ship.vex`. Sliced along `z`,
/// the hull is 5.3 units across at the `-Z` end with the full height of the model and
/// 1,175 of its 1,334 vertices, and tapers to 0.9 across at the `+Z` end. The wide,
/// detailed, full-height end of a racing ship is its engine block and the narrow
/// tapering end is its nose; there is no reading of those numbers in which `+Z` is
/// the tail.
///
/// Confidence **85**. The measurement admits one reading and is reproducible from the
/// asset, but it is an inference from the hull's shape: nothing in the executable has
/// been read that states the convention, and only one team's model was measured.
/// Applied here, at the boundary between a gameplay body and a drawn model, and
/// deliberately **not** in `oag_render::mesh` - `oag-view --mesh` shows a model in its
/// own space and must keep doing so.
pub const MODEL_YAW: f32 = std::f32::consts::PI;

/// The viewport shape every camera value on the disc was authored for.
///
/// The PSP renders into 480x272 and nothing else, so an authored field of view is
/// only defined at this aspect ratio. [`Race::projection`] is where that matters.
/// Taken from [`crate::frontend::SCREEN`] rather than written again, because it is
/// the same screen: the front end lays its widgets out in it and the camera was
/// framed for it.
pub const AUTHORED_ASPECT: f32 = crate::frontend::SCREEN.0 / crate::frontend::SCREEN.1;

/// The world's generator seed.
///
/// Fixed and arbitrary: nothing in the recovered force law draws from the
/// generator, so this exists only so the seed is written down in one place for
/// whenever something does. See `crates/core/src/rng.rs`.
pub const SEED: u64 = 1;

/// Seed for the exhaust's flicker, kept distinct from [`SEED`].
///
/// The exhaust draws two numbers per tick and the simulation must not see them:
/// sharing a generator would let the picture change the physics, which is what
/// `docs/architecture/determinism.md` forbids. A separate seed also means the two
/// streams cannot be mistaken for each other when reading a capture.
pub const EXHAUST_SEED: u64 = 0xe8_a5_71_00;

/// Seed for spark spawn parameters, kept distinct from [`SEED`] and
/// [`EXHAUST_SEED`] for the same determinism reason.
pub const SPARKS_SEED: u64 = 0x5_9a_2b_00;

/// The archive entry name of a team's `.vex` model.
///
/// Assembled the way the loader assembles it, with backslashes, which is what the
/// name hash needs.
#[must_use]
pub fn ship_entry_name(team: &str) -> String {
    format!(r"Data\Ships\{team}\Ship.vex")
}

/// Reads and decodes a model's external PS2 texture set, from the archive
/// entry directly before it.
///
/// **Only attempted when the model's own embedded textures are all
/// missing** - a PSP model already has them and this never runs for one; a
/// PS2 model's texture block is empty by design and this is what replaces
/// it. That gate matters beyond efficiency: the "entry before this one" rule
/// is a directory-position heuristic, not a name or a checked format tag, so
/// it must never have the chance to overwrite a model that already decoded
/// correctly on its own.
///
/// See [`pulse::Archives::read_preceding`] for the rule itself and the
/// evidence behind it.
fn ps2_texture_set(
    archives: &mut pulse::Archives,
    entry_name: &str,
) -> Option<Vec<Option<mesh::ModelTexture>>> {
    let blob = archives.read_preceding(entry_name).ok()?;
    mesh::ps2_texture_set(&blob).ok()
}

/// What to load, and which of it to draw.
#[derive(Debug, Clone)]
pub struct Options {
    /// A disc image, or a directory extracted with `oag-unpack`.
    pub source: String,
    /// Archive entry name of the track's `.vex`.
    pub track: String,
    /// Team name, which selects both the handling stats and the model.
    pub team: String,
    /// Speed class the handling parameters are read for.
    pub class: SpeedClass,
    /// Draw the driveable ribbon instead of the track's art meshes.
    ///
    /// **A development view, not a style.** The ribbon is the geometry the
    /// simulation spawns on and queries, so ship-plus-ribbon shows directly
    /// whether the ship is where the physics thinks it is. Default is off, which
    /// means a race draws the map.
    pub ribbon: bool,
    /// Overlay the collision soup - the geometry the physics world is actually
    /// made of - the same view `oag-view --collision` draws, wireframe and
    /// Cage-excluded, on top of whichever track model was chosen above.
    pub collision: bool,
    /// Whether ship and track models draw every child of an authored
    /// `LodGroup`, or only the higher-detail first one. See [`mesh::Lod`].
    pub lod: mesh::Lod,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            source: crate::source::DEFAULT_IMAGE.to_string(),
            track: DEFAULT_TRACK.to_string(),
            team: DEFAULT_TEAM.to_string(),
            class: SpeedClass::Venom,
            ribbon: false,
            collision: false,
            lod: mesh::Lod::Both,
        }
    }
}

/// Everything a race needs with no GPU anywhere in sight.
///
/// Split out from [`Loaded`] so a test can build one by hand, or take one off a
/// disc, and run a whole race against it on a machine with no graphics driver.
#[derive(Debug, Clone)]
pub struct Setup {
    /// The decoded spline graph, as the file has it.
    pub ai: AiTrack,
    /// The spline resampled for locating a ship, and for placing it.
    pub spline: Spline,
    /// The track's own authored grid slot, when it has one.
    ///
    /// `None` is not a broken track: it means this source's file carries no
    /// `Start Position` node, and a ship goes on the spline instead. Every PSP
    /// track has one, so on that path this is always `Some`.
    pub start_position: Option<StartPosition>,
    /// Every collidable triangle of the track.
    pub collision: CollisionWorld,
    /// The force law's parameter set for one team in one speed class.
    pub handling: Handling,
    /// The chase camera's seven values, from `<ExternalCameraFar>`.
    pub chase: ChaseParams,
    /// The `engine_flare` locator, in the ship model's own space.
    ///
    /// `None` means the model carries no `Engine Flare` node, and the exhaust is
    /// then not drawn rather than guessed at. Every team whose `Ship.vex`
    /// resolves by name has **exactly one**, a direct child of `world` - see
    /// `docs/ghidra/functions/psp-pulse/exhaust.md`. One nozzle, centred, not one
    /// per visible engine.
    pub nozzle: Option<Vec3>,
}

/// A [`Setup`] plus the geometry to draw it with.
#[derive(Debug)]
pub struct Loaded {
    /// The simulation half.
    pub setup: Setup,
    /// The HUD's layout, atlas, fonts and strings.
    pub hud: crate::hud::Assets,
    /// What to draw for the track.
    pub track_model: Model,
    /// What to draw for the ship.
    pub ship_model: Model,
    /// The collision soup, if [`Options::collision`] asked for it.
    pub collision_model: Option<Model>,
    /// The track's authored visibility partition, when it decoded.
    ///
    /// `None` when the track declares no `section` nodes - a driveable-ribbon
    /// build has no art meshes to place, and Pure tracks do not use Pulse's
    /// class numbering at all. The first tier is then skipped.
    pub visibility: Option<TrackVisibility>,
    /// The trail ribbon's noise texture off the disc, when it decodes.
    pub noise: Option<FlareTexture>,
    /// The engine-flare texture off the disc, when it decodes.
    ///
    /// `None` falls back to [`Exhaust`]'s procedural glow, and the load report
    /// says so - it is not a silent substitution.
    pub flare: Option<FlareTexture>,
    /// Lines worth printing once, describing what was found.
    pub report: Vec<String>,
}

/// The exhaust sprite's texture, named as a literal string in the executable.
///
/// At `0x08a84c80`, loaded by `Texture_LoadEngineFlare`. Being a literal means the
/// WAD lookup is an exact `wad::hash_name` hit rather than a mined candidate, which
/// is unusual for this project and worth the note.
pub const FLARE_TEXTURE: &str = r"Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip";

/// The trail ribbon's texture, also a literal in the executable.
///
/// At `0x08a889e4`, loaded by `Texture_LoadEngineNoise` into three slots that
/// `Trail_DrawRibbon` indexes per layer. Note the directory case differs from
/// [`FLARE_TEXTURE`]'s - `engineFlare` here, `EngineFlare` there - which does not
/// matter, since the WAD hash is case-insensitive.
pub const NOISE_TEXTURE: &str = r"Data\Tex\engineFlare\Engine_noise.mip";

/// The `engine_flare` locator's position in the ship model's own space.
///
/// A locator class stores a 4x4 in its 64-byte payload exactly as a `Transform`
/// does, which is why [`vex::class_world_transforms`] can compose it with the
/// parent chain. Row 3 is the translation.
///
/// Takes the **first** node if a model somehow had several. Every team checked has
/// exactly one, so this is a total order on a set of size one rather than a policy.
fn engine_flare(ship_blob: &[u8]) -> Option<Vec3> {
    let nodes = vex::nodes(ship_blob).ok()?;
    let m = vex::class_world_transforms(ship_blob, &nodes, vex::CLASS_ENGINE_FLARE)
        .into_iter()
        .next()?;
    Some(Vec3::new(m[12], m[13], m[14]))
}

/// Decodes a `.mip` texture out of the archive set.
///
/// Returns the pixels and a line for the load report, or the reason it could not -
/// **as text, not as `None`**. A missing entry and a blob that does not parse are
/// different problems with different fixes (name mining versus the decoder), and a
/// silent fallback hides which one happened.
fn mip_texture(
    archives: &mut pulse::Archives,
    name: &str,
) -> std::result::Result<(FlareTexture, String), String> {
    let blob = archives
        .read_name(name)
        .map_err(|e| format!("{name}: not in the archive set ({e})"))?;
    let texture = oag_formats::texture::Texture::parse(&blob)
        .map_err(|e| format!("{name}: {} bytes, does not parse ({e})", blob.len()))?;
    let note = format!(
        "{name}: {}x{}, {} mip level(s)",
        texture.width, texture.height, texture.mip_levels
    );
    Ok((
        FlareTexture {
            width: u32::from(texture.width),
            height: u32::from(texture.height),
            rgba: texture.to_rgba(),
        },
        note,
    ))
}

/// Loads a track, a ship and its handling out of a disc image.
///
/// # Errors
///
/// Propagates a missing archive, a missing entry, a `.vex` that does not decode,
/// a `WO Track` payload the spline parser cannot account for, and a
/// `handlingstats.xml` with an attribute missing.
pub fn load(options: &Options) -> Result<Loaded> {
    let mut report = Vec::new();

    // Which archives this source has, rather than which archives a PSP disc has.
    // Every entry name below is the same on both releases - the PS2 build ships
    // `Data\Ships\<Team>\handlingstats.xml` and `Data\Environments\<n>_Track\...`
    // under the names the PSP uses - so the layout is the whole of the
    // difference. See `docs/formats/handling-stats.md`.
    let mut archives = pulse::Archives::open(&options.source)?;
    report.push(archives.layout.describe());

    let spec = archives
        .locate(&options.track)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{} is in none of this source's archives ({})",
                options.track,
                archives.layout.describe()
            )
        })?
        .to_string();

    let (ai, label) = track_render::load(&spec, &options.track)?;
    report.push(format!(
        "{label}: {} path(s), {} junction(s), {} control point(s)",
        ai.paths.len(),
        ai.junctions.len(),
        ai.point_count()
    ));

    let read = |archives: &mut pulse::Archives, name: &str| -> Result<Vec<u8>> {
        archives
            .read_name(name)
            .with_context(|| format!("reading {name} out of {}", archives.layout.describe()))
    };

    // Read again for the collision nodes: `oag_render::track::load` takes an
    // archive rather than bytes, so this blob is decompressed twice. See the
    // wanted-change note in `docs/tools/oag-game.md`. On the PS2 that costs more
    // than it does on the PSP, where nothing is compressed at all: 5,861 of
    // `WADS2.WAD`'s 7,200 entries are LZSS.
    let track_blob = read(&mut archives, &options.track)?;
    let start_position = start_position_of(&track_blob);
    match start_position {
        Some(slot) => report.push(format!(
            "Start Position: {:?} facing {:?}",
            slot.position, slot.forward
        )),
        None => report.push("no Start Position node: spawning on the spline instead".to_string()),
    }
    let nodes =
        collision::from_vex(&track_blob).map_err(|e| anyhow::anyhow!("{}: {e}", options.track))?;
    let collision = collision_world(&nodes);
    report.push(format!(
        "{} collision node(s) -> {} collider(s), {} triangle(s)",
        nodes.len(),
        collision.colliders().len(),
        collision
            .colliders()
            .iter()
            .map(oag_physics::TriangleSoup::triangle_count)
            .sum::<usize>()
    ));

    let stats_name = handling::entry_name(&options.team);
    let stats_blob = read(&mut archives, &stats_name)?;
    let stats =
        handling::from_blob(&stats_blob).map_err(|e| anyhow::anyhow!("{stats_name}: {e}"))?;
    // Nothing is scaled here: the four pre-scaled fields are converted exactly once
    // and this is not the place it happens.
    let handling = handling_for(&stats, options.class);
    let far = stats.external_camera_far;
    let chase = ChaseParams {
        fov: far.fov,
        lookat_height: far.lookat_height,
        lookat_length: far.lookat_length,
        pos_height: far.pos_height,
        pos_length: chase_pos_length(far.pos_length),
        spring_horiz: far.spring_horiz,
        spring_vert: far.spring_vert,
    };
    report.push(format!(
        "{stats_name}: team {:?}, {:?} class, mass {}, ride_height {}",
        stats.team, options.class, handling.physical.mass, handling.antigrav.ride_height
    ));
    report.push(format!(
        "<ExternalCameraFar>: fov {} read as degrees (the unit is unrecovered), \
         pos_length {} on disc -> eye {} up and {} back",
        chase.fov, far.pos_length, chase.pos_height, chase.pos_length
    ));

    let ship_name = ship_entry_name(&options.team);
    let ship_blob = read(&mut archives, &ship_name)?;
    let mut ship_model = mesh::build_with_textures(&ship_name, &ship_blob, None, options.lod)?;
    // The PS2 signature: `Texture` nodes exist (the model wants textures) but
    // every one is missing (its embedded block was empty). Only then is the
    // directory-position heuristic worth trying - see `ps2_texture_set`.
    if !ship_model.textures.is_empty()
        && ship_model.textures.iter().all(Option::is_none)
        && let Some(external) = ps2_texture_set(&mut archives, &ship_name)
    {
        ship_model =
            mesh::build_with_textures(&ship_name, &ship_blob, Some(external), options.lod)?;
    }
    report.push(format!(
        "{ship_name}: {} triangle(s), model centre {:?}, radius {:.2}",
        ship_model.indices.len() / 3,
        ship_model.centre,
        ship_model.radius
    ));

    let track_model = if options.ribbon {
        track_render::build_model(&label, &ai)
    } else {
        let mut track_model =
            mesh::build_with_textures(&options.track, &track_blob, None, options.lod)?;
        // Same PS2 signature and the same directory-position heuristic as the
        // ship above. Checked separately for tracks specifically (not just
        // assumed from the ship result): the entry directly before a track's
        // own `.vex` decodes as a texture set with the same entry count as the
        // model's `Texture` nodes on 27 of the 32 `<n>_Track`/`track_reversed`
        // pairs on the PS2 disc, off by only 1-2 slots (never more) on the
        // rest - see `docs/formats/ps2-texture.md`.
        if !track_model.textures.is_empty()
            && track_model.textures.iter().all(Option::is_none)
            && let Some(external) = ps2_texture_set(&mut archives, &options.track)
        {
            track_model = mesh::build_with_textures(
                &options.track,
                &track_blob,
                Some(external),
                options.lod,
            )?;
        }
        track_model
    };
    report.push(format!(
        "drawing the track's {}: {} triangle(s), radius {:.0}",
        if options.ribbon {
            "driveable ribbon"
        } else {
            "art meshes"
        },
        track_model.indices.len() / 3,
        track_model.radius
    ));

    for model in [&ship_model, &track_model] {
        if let Some(line) = untextured_note(model) {
            report.push(line);
        }
    }

    // The authored PVS. Skipped for a ribbon build, whose geometry is generated
    // from the spline rather than authored, so the section boxes have nothing
    // to say about it.
    let visibility = if options.ribbon {
        None
    } else {
        TrackVisibility::build(&track_model, &track_blob, &ai)
    };
    match &visibility {
        Some(visibility) => report.push(format!(
            "{} authored visibility section(s); {} of {} draw call(s) placed \
             ({:.1}%), {} spanning more than one, {:.1} section(s) each on average",
            visibility.pvs.len(),
            visibility.placement.placed,
            visibility.placement.total(),
            visibility.placement.placed_fraction() * 100.0,
            visibility.placement.spanning,
            visibility.placement.mean_sections(),
        )),
        None if options.ribbon => {}
        None => report.push(
            "no authored visibility sections: PVS culling is unavailable on this track".to_string(),
        ),
    }

    let spline = Spline::from_track(&ai);
    report.push(format!(
        "{} spline sample(s), {} per segment, widest half-width {:.1}",
        spline.len(),
        Spline::STEPS_PER_SEGMENT,
        spline.max_half_width()
    ));

    let collision_model = if options.collision {
        let soup = render_collision::build_model(
            &label,
            &nodes,
            render_collision::Style::Wireframe,
            false,
        );
        report.push(format!(
            "drawing the collision soup: {} triangle(s) as outlines",
            soup.indices.len() / 3
        ));
        Some(soup)
    } else {
        None
    };

    let nozzle = engine_flare(&ship_blob);
    match nozzle {
        Some(at) => report.push(format!(
            "{ship_name}: engine_flare locator at {at:?} in model space"
        )),
        None => report.push(format!(
            "{ship_name}: no Engine Flare node - the exhaust will not be drawn"
        )),
    }

    let noise = match mip_texture(&mut archives, NOISE_TEXTURE) {
        Ok((texture, note)) => {
            report.push(note);
            Some(texture)
        }
        Err(why) => {
            report.push(format!("{why} - the trail falls back to a procedural glow"));
            None
        }
    };

    let flare = match mip_texture(&mut archives, FLARE_TEXTURE) {
        Ok((texture, note)) => {
            report.push(note);
            Some(texture)
        }
        Err(why) => {
            // Reported rather than silently swapped for the placeholder. A
            // stand-in that looks plausible is how a decode failure survives
            // review; see the note on `FlareTexture::placeholder`.
            report.push(format!("{why} - the flare falls back to a procedural glow"));
            None
        }
    };

    let hud = load_hud(&mut archives, &mut report);

    Ok(Loaded {
        setup: Setup {
            ai,
            spline,
            start_position,
            collision,
            handling,
            chase,
            nozzle,
        },
        hud,
        track_model,
        collision_model,
        ship_model,
        visibility,
        flare,
        noise,
        report,
    })
}

/// Which layout a race uses.
///
/// **Time trial, and that is a placeholder rather than a choice.** A race has no
/// mode concept yet - `Options` carries a track, a team and a speed class and
/// nothing that says "single race" or "eliminator" - so there is nothing to select
/// on. Time trial is the smallest layout that carries every widget with a real
/// source, and it is the mode every reference capture in this project was taken
/// in. When modes exist this becomes a lookup; the four other entries are in
/// [`crate::hud::layouts`] already.
const HUD_LAYOUT: &str = crate::hud::layouts::TIME_TRIAL;

/// Reads the HUD's layout, atlas, fonts and strings.
///
/// Every piece degrades on its own and says so. The report matters more here than
/// it looks: a HUD drawn in the 5x7 fallback font looks like a rendering bug, and
/// a silent fallback would send someone looking in the shader.
fn load_hud(archives: &mut pulse::Archives, report: &mut Vec<String>) -> crate::hud::Assets {
    let layout = match archives
        .read_name(HUD_LAYOUT)
        .map_err(|e| e.to_string())
        .and_then(|blob| oag_formats::fexml::expand(&blob).map_err(|e| e.to_string()))
    {
        Ok(xml) => {
            let layout = crate::hud::Layout::from_xml(&xml);
            report.push(format!(
                "HUD {HUD_LAYOUT}: {} sprite(s), {} fill(s), {} label(s), {} model(s)",
                layout.sprites.len(),
                layout.fills.len(),
                layout.labels.len(),
                layout.models.len()
            ));
            for note in &layout.skipped {
                report.push(format!("HUD: skipped {note}"));
            }
            Some(layout)
        }
        Err(why) => {
            report.push(format!("HUD {HUD_LAYOUT} unavailable ({why}); no HUD"));
            None
        }
    };

    // One texture in the sheet, which is what `Sheet::build` is for. Going
    // through the sheet rather than binding the atlas directly means the HUD
    // shares the renderer every other screen uses, `Draw::Sprite` and all.
    let mut sheet = crate::sprite::Sheet::default();
    match archives.read_name(crate::hud::ATLAS) {
        Ok(blob) => {
            let mut notes = Vec::new();
            let built =
                crate::sprite::Sheet::build(&[(crate::hud::ATLAS.to_string(), blob)], &mut notes);
            report.extend(notes);
            if built.get(crate::hud::ATLAS).is_some() {
                report.push(format!(
                    "HUD atlas {}: {}x{} sheet",
                    crate::hud::ATLAS,
                    built.width,
                    built.height
                ));
                sheet = built;
            } else {
                report.push(format!("HUD atlas {} did not decode", crate::hud::ATLAS));
            }
        }
        Err(why) => report.push(format!(
            "HUD atlas {} unavailable ({why})",
            crate::hud::ATLAS
        )),
    }

    let font = hud_font(archives, pulse::names::fonts::HUD, report);
    let small_font = hud_font(archives, pulse::names::fonts::SMALL, report);

    // The HUD's captions are `idstring` keys - `IG_HUD_LAP`, `IG_HUD_BEST` - and
    // without a table `StringTable::get_or_id` falls back to the key itself, which
    // put `ig_hud_lap` on screen where `LAP` belongs. The preferred language is the
    // player's saved one; a race reached through `--race` has no settings to read,
    // so this takes the chain's default rather than threading one through.
    let languages = crate::boot::load_languages(archives, report);
    let strings = crate::boot::load_strings(archives, &languages, None, report);

    crate::hud::Assets {
        layout,
        sheet,
        font,
        small_font,
        strings,
    }
}

/// Reads one `.fnt`, falling back to the built-in glyphs and saying so.
///
/// The same shape as `crate::boot::load_font`, which reads the front end's
/// `Default` font. Not shared with it because that one reaches through
/// `read_front_end_first` for a front-end archive order this path does not have,
/// and threading a strategy through would be more code than the six lines it saves.
fn hud_font(
    archives: &mut pulse::Archives,
    name: &str,
    report: &mut Vec<String>,
) -> crate::font::Atlas {
    match archives
        .read_name(name)
        .map_err(|e| e.to_string())
        .and_then(|blob| oag_formats::fnt::Font::parse(&blob).map_err(|e| e.to_string()))
    {
        Ok(font) => {
            report.push(format!(
                "HUD font {name}: {}x{} atlas, {} glyph(s), line height {}",
                font.width,
                font.height,
                font.glyphs.len(),
                font.line_height
            ));
            crate::font::Atlas::from_font(&font)
        }
        Err(why) => {
            report.push(format!(
                "HUD font {name} unavailable ({why}); drawing with 5x7"
            ));
            crate::font::Atlas::build()
        }
    }
}

/// Says so when a model's texture slots did not all fill, and why.
///
/// **Keyed on empty slots, not on an empty list, and the difference is the whole
/// point.** A model that declares no texture slots at all wanted none - the
/// driveable ribbon `oag_render::track::build_model` generates is one, on either
/// disc - and saying "untextured" about it would blame a decode for geometry that
/// was never textured. What is worth reporting is a model that asked for `n`
/// textures and got fewer, because that is a picture missing something it was
/// authored with.
///
/// **PS2 ships are textured now; PS2 track art is the one place this still
/// fires, and it is reported rather than papered over.** A PS2 model's
/// textures are separate archive entries gathered into a nested WAD of
/// Graphics Synthesizer upload packets, and a `.vex` declares no texture
/// pixels of its own to say which. For ships, `ps2_texture_set` below finds
/// it anyway: the entry directly before a model in the archive's own
/// directory, checked independently against every team on the roster - see
/// `docs/formats/ps2-texture.md`. Track models are a separate, harder case:
/// several typically share one texture set instead of one each, and that
/// grouping is not verified against a known-correct picture the way a ship
/// is, so it is not attempted and `16_Track\track.vex` still draws
/// untextured on PS2.
///
/// The model still draws: `Drawable::draw` binds the white fallback for a draw
/// with no texture slot. So an unresolved model is a correctly-shaped
/// untextured one, and nothing here guesses at a texture set to avoid saying
/// that.
///
/// Deliberately keyed on the model rather than on the platform. A PSP model with a
/// slot that will not decode deserves the same line, and the PS2 disc carries
/// PSP-format batches too - so "which disc is this" is never the right question to
/// ask about one mesh.
/// Where a ship starts a race: the track's authored slot, or the spline.
///
/// The authored [`StartPosition`] wins whenever the track has one, because it is
/// a value recovered off the disc and `spline.start()` is an artifact of how this
/// crate resamples - sample 0 of path 0, which is wherever the exporter happened
/// to begin writing control points. On `16_Track` the two are **188.8 units
/// apart** and the authored slot is the one on the grid.
///
/// What this is **not** is pole. Every track ships one slot and a race grids
/// eight, so this places a ship on the one slot that was authored and says
/// nothing about the other seven. On `16_Track` a time trial in the original
/// starts about 138 units *ahead* of this, on the other side of the centreline -
/// which is what makes the remaining slots worth recovering rather than
/// deriving. See `docs/formats/track.md#start-position`.
///
/// The height comes off the collision geometry rather than out of the slot: see
/// [`Pose::from_start_position`] for the measurement that says the authored `y`
/// is not a ride height. `collision` is cast straight down from well above the
/// slot, and a slot over a hole in the mesh falls back to the authored value.
///
/// `None` only when a track has no authored slot *and* an empty spline, which no
/// real track is.
#[must_use]
fn spawn_pose(
    spline: &Spline,
    start_position: Option<&StartPosition>,
    collision: &CollisionWorld,
    handling: &Handling,
) -> Option<Pose> {
    let height = spawn_height(handling);
    if let Some(slot) = start_position {
        return Some(Pose::from_start_position(
            slot,
            ground_under(collision, slot),
            height,
        ));
    }
    spline
        .start()
        .map(|sample| Pose::from_sample(sample, sample.racing_line, height))
}

/// How far up the track's own drop for a spawn probe starts, and how far it
/// reaches.
///
/// Generous either way on purpose: this is a one-off query at load, and a slot
/// authored a few units under an overhanging piece of track should still find the
/// floor rather than silently falling back.
const SPAWN_PROBE_RISE: f32 = 20.0;
const SPAWN_PROBE_REACH: f32 = 80.0;

/// The world `y` of the collision surface under an authored slot, if there is one.
#[must_use]
fn ground_under(collision: &CollisionWorld, slot: &StartPosition) -> Option<f32> {
    let origin = Vec3::from_array(slot.position) + Vec3::Y * SPAWN_PROBE_RISE;
    let ray = oag_physics::Ray::new(origin, Vec3::NEG_Y, SPAWN_PROBE_REACH);
    oag_physics::Raycaster::raycast(collision, ray, None, false).map(|hit| hit.point.y)
}

/// Reads a track `.vex`'s authored grid slot, if it has one.
///
/// `None` covers three cases a caller treats the same way and none of which is an
/// error: a blob that is not a `.vex` at all, one with no `Start Position` node,
/// and one whose node payload is not the 64 bytes a transform needs. A track with
/// no authored slot is a track a ship goes on the spline of.
#[must_use]
fn start_position_of(blob: &[u8]) -> Option<StartPosition> {
    let nodes = vex::nodes(blob).ok()?;
    let node = nodes
        .iter()
        .find(|node| node.class_id == vex::CLASS_START_POSITION)?;
    oag_formats::track::start_position(blob.get(node.payload())?)
}

#[must_use]
fn untextured_note(model: &Model) -> Option<String> {
    let slots = model.textures.len();
    let decoded = model.textures.iter().filter(|t| t.is_some()).count();
    if slots == 0 || decoded == slots {
        return None;
    }
    Some(format!(
        "{}: {decoded} of {slots} texture slot(s) decoded, drawing the rest \
         untextured. PS2 models keep their textures in separate archive entries \
         and the model-to-texture-set lookup is not recovered; see \
         docs/formats/ps2-texture.md",
        model.label
    ))
}

/// Converts the disc's `pos_length` into the one
/// [`ChaseParams::pos_length`] documents.
///
/// The two conventions differ by a sign, and this is the only place that is
/// reconciled.
///
/// `oag_render::camera::chase` documents `pos_length` as a **positive distance
/// behind** and computes the eye as `position - forward * pos_length`. The disc
/// stores a **signed offset along forward**: every `<ExternalCameraFar>` and
/// `<ExternalCameraClose>` block observed has it negative, and the *close* camera's
/// value is the smaller magnitude of the two, which is what "closer behind the
/// ship" looks like under that reading and is not consistent with any other.
///
/// Passing the file's value through unchanged puts the eye that far **in front** of
/// the ship, aimed at a point `lookat_length` further ahead still, so the ship is
/// behind the camera and nothing of it is drawn. `chase.rs` records that exact
/// mistake as "obvious on screen, so this one is cheap to check once there is a ship
/// to look at" - this is that check, and the negation is its answer.
///
/// Confidence **80**: the geometry admits no other reading and both external blocks
/// of every team agree, but nothing has been compared against the original running,
/// and the fix arguably belongs in `oag-render`'s documented convention rather than
/// here.
#[must_use]
fn chase_pos_length(from_disc: f32) -> f32 {
    -from_disc
}

/// Placing a ship on a track lives in [`oag_gameplay::spawn`], which is where a
/// harness that is not the composition root can reach it: `oag-trace drive` puts
/// a ship on a start line exactly the way a race does and may not depend on this
/// crate. Re-exported so every call site here, and both ground-truth tests, read
/// as they did.
pub use oag_gameplay::spawn::{box_inertia, spawn_height};

/// The spline resampled into a flat table, for locating a ship on the track.
///
/// A `Vec` walked in order rather than any kind of spatial index: the nearest-sample
/// comparison feeds simulation state, so the order it happens in must not be able to
/// vary between runs, and ties go to the earlier sample. See
/// `docs/architecture/determinism.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct Spline {
    samples: Vec<Sample>,
    /// Which path each sample came from, parallel to [`Self::samples`].
    paths: Vec<u16>,
}

impl Spline {
    /// Samples per control-point interval.
    ///
    /// The same four `oag_render::track` draws the ribbon with, so the table and the
    /// picture agree about where the track is.
    pub const STEPS_PER_SEGMENT: usize = 4;

    /// Resamples every path of a decoded spline graph.
    #[must_use]
    pub fn from_track(ai: &AiTrack) -> Self {
        let mut samples = Vec::new();
        let mut paths = Vec::new();
        for (index, path) in ai.paths.iter().enumerate() {
            for segment in 0..path.points.len() {
                for step in 0..Self::STEPS_PER_SEGMENT {
                    let t = step as f32 / Self::STEPS_PER_SEGMENT as f32;
                    if let Some(sample) = path.sample(segment, t) {
                        samples.push(sample);
                        paths.push(index as u16);
                    }
                }
            }
        }
        Self { samples, paths }
    }

    /// How many samples the table holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether the track produced no samples at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// The first sample of the first path: where a ship is put.
    ///
    /// Not a grid slot. How the original assigns one is an open question that
    /// `oag_gameplay::spawn` deliberately leaves alone, so this is the start of the
    /// track and claims nothing more than that.
    #[must_use]
    pub fn start(&self) -> Option<&Sample> {
        self.samples.first()
    }

    /// The sample as [`oag_physics::maglock`] wants it: **lifted**, with its axis.
    ///
    /// Two conversions happen here and both are the format side's business rather
    /// than the simulation's:
    ///
    /// - `Sample::pos` is the *unlifted* disc value and the running game holds the
    ///   lifted one, because `AiTrack_LoadPathPoints` does `pos -= 3.0 * down` at
    ///   load. The hold subtracts that same lift back off to recover the surface
    ///   point, so it has to be handed the lifted form or it lands three units low.
    ///   Blending is linear, so lifting an interpolated sample and interpolating
    ///   lifted points are the same thing.
    /// - `down` is passed **raw**. The hold normalises it where it needs an axis
    ///   and reads it unnormalised where the original does, which is the mag
    ///   probe's direction.
    #[must_use]
    pub fn track_sample(sample: &Sample) -> oag_physics::TrackSample {
        let down = Vec3::from_array(sample.down);
        oag_physics::TrackSample {
            position: Vec3::from_array(sample.pos) - down * oag_formats::track::HOVER_LIFT,
            down,
        }
    }

    /// The nearest sample to `position`: its index, itself, and its distance.
    #[must_use]
    pub fn nearest(&self, position: Vec3) -> Option<(usize, &Sample, f32)> {
        let mut best: Option<(usize, f32)> = None;
        for (index, sample) in self.samples.iter().enumerate() {
            let distance = (Vec3::from_array(sample.pos) - position).length();
            // Strictly nearer, so a tie keeps the earlier sample.
            if best.is_none_or(|(_, previous)| distance < previous) {
                best = Some((index, distance));
            }
        }
        best.map(|(index, distance)| (index, &self.samples[index], distance))
    }

    /// The nearest sample to `position`, searching only `window` samples either
    /// side of `around` in table order.
    ///
    /// **A local search whose failure mode is safe.** [`Self::nearest`] walks
    /// every sample - ~3,400 on a real track - which is affordable once a tick
    /// inside the simulation and not twice more per *frame* on top. The camera
    /// is a chase spring a few units behind the craft, so it is a few samples
    /// away in this table, and a window finds it.
    ///
    /// When it does not - the window straddles a junction, or the camera really
    /// has left the track - the answer is a sample that is too far away, which
    /// the caller turns into [`UNPLACED`] and therefore into *draw everything*.
    /// A local search that misses costs a frame of culling, never a frame of
    /// missing geometry, which is why this is allowed to be approximate.
    #[must_use]
    pub fn nearest_within(
        &self,
        position: Vec3,
        around: usize,
        window: usize,
    ) -> Option<(usize, &Sample, f32)> {
        let last = self.samples.len().checked_sub(1)?;
        let from = around.saturating_sub(window);
        let to = around.saturating_add(window).min(last);
        let mut best: Option<(usize, f32)> = None;
        for index in from..=to {
            let distance = (Vec3::from_array(self.samples[index].pos) - position).length();
            if best.is_none_or(|(_, previous)| distance < previous) {
                best = Some((index, distance));
            }
        }
        best.map(|(index, distance)| (index, &self.samples[index], distance))
    }

    /// Distance from `position` to the nearest sample, or `None` on an empty track.
    ///
    /// Distance to a *sample*, not to the curve: at four samples per segment the two
    /// differ by a fraction of a control-point interval, far below anything worth
    /// asserting on.
    #[must_use]
    pub fn distance_to(&self, position: Vec3) -> Option<f32> {
        self.nearest(position).map(|(_, _, distance)| distance)
    }

    /// The sample at `index`, in table order.
    #[must_use]
    pub fn sample(&self, index: usize) -> Option<&Sample> {
        self.samples.get(index)
    }

    /// Which path the sample at `index` belongs to.
    #[must_use]
    pub fn path_of(&self, index: usize) -> Option<u16> {
        self.paths.get(index).copied()
    }

    /// The widest half-width anywhere on the track, either side.
    ///
    /// A scale for "still roughly on the track" that comes from the track itself
    /// rather than from a number somebody picked.
    #[must_use]
    pub fn max_half_width(&self) -> f32 {
        self.samples.iter().fold(0.0f32, |widest, sample| {
            widest
                .max(sample.half_width_left)
                .max(sample.half_width_right)
        })
    }
}

/// One tick's worth of what the simulation did, for a log or an overlay.
///
/// Everything here is read out of the state *after* a step; nothing in it is an
/// input to the next one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Telemetry {
    /// Ticks elapsed since the race began.
    pub tick: u64,
    /// Where the ship is.
    pub position: Vec3,
    /// How fast it is going, in world units per second.
    pub speed: f32,
    /// Probes in contact over two: `0.0`, `0.5` or `1.0`.
    pub grounded: f32,
    /// Distance to the nearest spline sample.
    pub spline_distance: f32,
    /// Height above that sample along its own up axis. Negative is below the
    /// surface line.
    pub height_above_spline: f32,
}

/// A race in progress: the world, the track it is on, and the camera behind it.
#[derive(Debug)]
pub struct Race {
    /// The simulation state.
    pub world: World,
    collision: CollisionWorld,
    spline: Spline,
    chase_params: ChaseParams,
    camera: Chase,
    dt: f32,
    /// Ticks left before a `Reset` contact can respawn again.
    ///
    /// See [`RESPAWN_COOLDOWN_TICKS`].
    respawn_cooldown: u32,
    /// How many respawns have happened back to back, for [`RESPAWN_GIVE_UP`].
    respawns_in_a_row: u32,
    /// Set once respawning has given up, so the complaint is printed once.
    respawn_disabled: bool,
    /// How many times the ship has been respawned this race, for tests and for
    /// the load report.
    respawns: u32,
    /// The exhaust's animation state, advanced on the simulation tick.
    ///
    /// Here rather than in `World` for the same reason [`Chase`] is: it is
    /// render-only state, so it must not enter a snapshot a replay or a
    /// determinism hash reads. `physics/src/probe.rs` destructures `ShipState`
    /// exhaustively on purpose, and adding a visual field there would move the
    /// pinned hashes for no reason.
    exhaust: Exhaust,
    /// The flicker's generator, deliberately **not** `world.rng`.
    ///
    /// The exhaust draws two random numbers per tick. Taking them from the
    /// simulation's generator would make the picture change what the simulation
    /// does next - the determinism rules exist to prevent exactly that. Seeded, so
    /// a capture at tick *n* is still reproducible.
    exhaust_rng: Rng,
    /// The `engine_flare` locator in model space, when the ship model has one.
    nozzle: Option<Vec3>,
    /// Collision sparks' particle pool, advanced on the simulation tick.
    ///
    /// Here rather than in `World`, for the same reason [`Self::exhaust`] is -
    /// see `oag_render::sparks`'s module doc comment.
    sparks: Sparks,
    /// The sparks' generator, deliberately **not** `world.rng` - see
    /// [`Self::exhaust_rng`].
    sparks_rng: Rng,
    /// Whether `oag_physics::wall::WallResponse::impact` was set on the
    /// previous tick.
    ///
    /// This crate's own edge detector for spark spawning: `impact` stays
    /// `true` for every tick of a sustained scrape, and spawning a burst on
    /// every one of those ticks rather than on the rising edge is exactly the
    /// per-frame-instead-of-per-impact bug
    /// `oag_physics::wall::STUN_PER_CONTACT`'s doc comment already records
    /// costing a session of play-testing.
    sparks_was_impacting: bool,
}

/// Ticks a `Reset` contact is ignored for after a respawn.
///
/// Half a second at 60 Hz. The recovery pose puts the ship on the racing line at
/// hover height, so it should be clear of any trigger immediately - this exists
/// because "should" is an assumption about shipped data, not a property of the
/// code, and the failure it guards is a race that freezes in a respawn loop with
/// no symptom.
pub const RESPAWN_COOLDOWN_TICKS: u32 = 30;

/// Consecutive respawns after which respawning stops and says so.
///
/// A ship that needs five recoveries without ever getting clear is not being
/// recovered, and a visible complaint beats an invisible freeze.
pub const RESPAWN_GIVE_UP: u32 = 5;

impl Race {
    /// Starts a race: one ship, on the racing line, at the start of the spline.
    ///
    /// The mass is copied into the rigid body because the force law reads
    /// `handling.physical.mass` while the integrator reads `body.mass`. They are one
    /// quantity stored twice and keeping them equal is this layer's job -
    /// `oag_physics` deliberately does not do it, so that a mismatch stays visible.
    ///
    /// The inertia comes from [`box_inertia`]; see the module documentation and that
    /// function for why a derived tensor replaced the `(1, 1, 1)` placeholder.
    #[must_use]
    pub fn start(setup: Setup) -> Self {
        let Setup {
            spline,
            start_position,
            collision,
            handling,
            chase,
            nozzle,
            ..
        } = setup;

        let mut world = World::new(SEED);
        let ship = &mut world.ships[0];
        ship.active = true;
        ship.handling = handling;
        ship.physics.body.mass = handling.physical.mass;
        ship.physics.body.inertia = box_inertia();
        if let Some(pose) = spawn_pose(&spline, start_position.as_ref(), &collision, &handling) {
            ship.place_at(pose);
        }
        world.ship_count = 1;

        let camera = Chase::snapped(target_of(&world.ships[0]), &chase);

        Self {
            world,
            collision,
            spline,
            chase_params: chase,
            camera,
            // ADR-0007: 60 Hz, from the clock rather than from a literal, so there
            // is one place the rate is decided.
            dt: TickClock::new(TickRate::DEFAULT).rate().dt(),
            respawn_cooldown: 0,
            respawns_in_a_row: 0,
            respawn_disabled: false,
            respawns: 0,
            // Cold, then snapped on the first tick. A race starts from a standing
            // start with no thrust, so there is nothing to snap *to* here.
            exhaust: Exhaust::new(),
            exhaust_rng: Rng::new(EXHAUST_SEED),
            nozzle,
            sparks: Sparks::new(),
            sparks_rng: Rng::new(SPARKS_SEED),
            // No sync frame to be mid-scrape on, so the first tick's contact -
            // if any - is always read as a fresh impact.
            sparks_was_impacting: false,
        }
    }

    /// How many times a `Reset` contact has respawned the ship this race.
    #[must_use]
    pub fn respawns(&self) -> u32 {
        self.respawns
    }

    /// The fixed timestep, from [`TickRate::DEFAULT`].
    #[must_use]
    pub fn dt(&self) -> f32 {
        self.dt
    }

    /// The player's ship.
    #[must_use]
    pub fn ship(&self) -> &Ship {
        &self.world.ships[0]
    }

    /// The resampled spline, for a caller that wants to measure against it.
    #[must_use]
    pub fn spline(&self) -> &Spline {
        &self.spline
    }

    /// Advances the simulation one fixed tick, and the camera with it.
    ///
    /// The snapshot is mapped through [`oag_gameplay::ship_controls`], which is the
    /// one place "cross is thrust" is written down.
    pub fn tick(&mut self, snapshot: &InputSnapshot) -> Evaluated {
        let controls = ship_controls(snapshot);

        // The two spline samples the magstrip hold reads. In the original these are
        // `AiTrack_LocatePosition`'s two output records on the ship entity; here
        // they are the nearest table entry and the one after it, which is the same
        // "where am I and what is next" pair at this crate's resolution. Off a
        // magstrip nothing consumes them, so a track without one is unaffected.
        let nearest = self
            .spline
            .nearest(self.world.ships[0].physics.body.position);
        let index = nearest.map(|(index, _, _)| index);
        let track_sample = nearest.map(|(_, sample, _)| Spline::track_sample(sample));
        // The neighbour is the next entry in table order, which at the end of a
        // path is the *next path's* first sample rather than the geometric
        // successor through a junction. The original follows the junction graph;
        // this is one sample out of 34,000 per lap and is recorded rather than
        // pretended away.
        let track_sample_next = index
            .and_then(|index| self.spline.sample(index + 1))
            .map(Spline::track_sample);

        let ship = &mut self.world.ships[0];
        if let Some(index) = index {
            // An index into this module's own sample table, which is *not* what
            // `Ship::segment` documents: that field means a per-path control-point
            // segment. Nothing reads it today, and it is written because a locator
            // that keeps no note of where it was is the thing that has to be replaced
            // when the brute-force scan does. Converting the one into the other needs
            // a lap-counting convention nobody has recovered.
            ship.segment = u16::try_from(index).unwrap_or(u16::MAX);
        }

        let env = Environment {
            track_sample,
            track_sample_next,
            ..Environment::default()
        };
        let before = ship.physics.body.position;
        let evaluated = oag_physics::step(
            &mut ship.physics,
            &controls,
            &ship.handling,
            &env,
            &self.collision,
            self.dt,
        );

        self.respawn_cooldown = self.respawn_cooldown.saturating_sub(1);
        if self.reset_zone_touched(&env, before) {
            // `index` is where the ship was *before* this tick moved it, which is
            // as close to "last known good" as this loop can cheaply get.
            self.respawn(index);
        } else if self.respawn_cooldown == 0 {
            // Clear of the trigger with the cooldown expired: whatever run of
            // back-to-back respawns was happening is over.
            self.respawns_in_a_row = 0;
        }

        self.world.tick += 1;
        let target = target_of(&self.world.ships[0]);
        self.camera.advance(target, &self.chase_params, self.dt);

        // Advanced here, on the fixed tick, and not in the frame loop. That is
        // what makes the headless `capture` path - which calls only `tick` -
        // produce the same flare at the same tick count as the window does, and
        // it is the same reason the chase camera is advanced from here.
        let ship = &self.world.ships[0].physics;
        let thrust = ship.thrust;
        let speed = ship.body.linear_velocity.length();
        self.exhaust
            .advance(self.dt, thrust, speed, &mut self.exhaust_rng);

        // One sample per tick, which is what `Trail_Update` does per frame - it
        // takes no `dt` at all. The direction is the nozzle's own backwards axis so
        // a segment keeps the orientation the craft had when it was laid down,
        // rather than swinging with the current pose as the ship turns.
        if let Some(nozzle) = self.nozzle() {
            let back = -self.ship().physics.body.forward();
            self.exhaust.push_trail(nozzle, back);
        }

        // Edge-triggered, not level-triggered - see `Self::sparks_was_impacting`'s
        // doc comment for why a sustained scrape must not spawn a burst every
        // tick.
        let impact_edge = evaluated.wall.impact && !self.sparks_was_impacting;
        self.sparks_was_impacting = evaluated.wall.impact;
        if impact_edge && let Some(contact) = evaluated.wall.resolved {
            // `contact.point` is deliberately the *penetrating* hull sample
            // point, not the wall surface - see its doc comment on
            // `oag_physics::wall::WallContact`, which matches the original's
            // own `Collision_AddContact` exactly. Correct for physics, wrong
            // for a visual: that point sits up to `depth` units inside the
            // opaque wall, so a spark quad centred there is depth-tested away
            // by the wall's own geometry almost every time and reads as
            // nothing spawning. `point + normal * depth` is the same
            // correction `resolve_contact`'s `escape` vector already applies
            // to push the hull back out to the surface.
            let surface = contact.point + contact.normal * contact.depth;
            self.sparks.spawn(
                surface,
                contact.normal,
                evaluated.wall.impact_speed,
                &mut self.sparks_rng,
            );
        }
        // Unconditional, like the exhaust: already-live particles keep ageing
        // even on a tick with no fresh impact.
        self.sparks.advance(self.dt);

        evaluated
    }

    /// Whether this tick ended in contact with `Reset` geometry.
    ///
    /// Suppressed while the cooldown runs and once respawning has given up, so
    /// the two guards live in one place rather than at the call site.
    fn reset_zone_touched(&self, env: &Environment, before: Vec3) -> bool {
        if self.respawn_disabled || self.respawn_cooldown > 0 {
            return false;
        }
        let ship = &self.world.ships[0];
        // The same `env` the force law just ran with, so the self-collider
        // exclusion cannot differ between the two.
        oag_physics::reset::contact(&ship.physics, &ship.handling, env, &self.collision, before)
            .is_some()
    }

    /// Puts the ship back on the track after a `Reset` contact.
    ///
    /// # This pose is a guess, not a reading
    ///
    /// `docs/ghidra/functions/psp-pulse/collision.md` records **that** a `Reset`
    /// contact respawns the ship, at confidence 86. **Where it respawns it is not
    /// recorded anywhere**, and searching the RE tree for it found nothing - which
    /// is itself the finding. So this reuses the initial spawn: the racing line at
    /// [`spawn_height`], on the spline sample nearest where the ship was before
    /// the tick that triggered the reset.
    ///
    /// Confidence **40**. That is deliberately low, and the number matters: this
    /// is a placeholder chosen because it reuses code that is already correct for
    /// the race start, not because anything says the original does it. The likelier
    /// real mechanism is a last-passed checkpoint or track section - [`Ship::segment`]
    /// is the field that would hold it, and nothing populates it meaningfully today.
    /// Whoever recovers that should replace this outright rather than tune it.
    ///
    /// The velocity, orientation and every control state go to zero, because
    /// [`Ship::place_at`] resets the whole physics state and keeps only mass and
    /// inertia. Whether the original preserves any speed through a respawn is also
    /// unrecorded.
    fn respawn(&mut self, sample_index: Option<usize>) {
        let sample = sample_index
            .and_then(|index| self.spline.sample(index))
            .or_else(|| self.spline.start());
        let Some(sample) = sample.copied() else {
            return;
        };

        let ship = &mut self.world.ships[0];
        let height = spawn_height(&ship.handling);
        ship.place_at(Pose::from_sample(&sample, sample.racing_line, height));

        self.respawns += 1;
        self.respawns_in_a_row += 1;
        self.respawn_cooldown = RESPAWN_COOLDOWN_TICKS;

        // Otherwise the ribbon spans the teleport: ten samples of history from
        // wherever the craft fell off, stretched across the track to where it was
        // put back. The camera is snapped for the same reason.
        self.exhaust.clear_trail();

        if self.respawns_in_a_row >= RESPAWN_GIVE_UP {
            self.respawn_disabled = true;
            eprintln!(
                "reset: {} respawns in a row without getting clear, giving up. \
                 The recovery pose is probably inside a Reset volume; see \
                 Race::respawn.",
                self.respawns_in_a_row
            );
        }
    }

    /// What the ship did, as of the last tick.
    #[must_use]
    pub fn telemetry(&self) -> Telemetry {
        let ship = self.ship();
        let position = ship.physics.body.position;
        let (spline_distance, height_above_spline) =
            self.spline
                .nearest(position)
                .map_or((f32::NAN, f32::NAN), |(_, sample, distance)| {
                    let up = (-Vec3::from_array(sample.down)).normalize_or_zero();
                    (distance, (position - Vec3::from_array(sample.pos)).dot(up))
                });

        Telemetry {
            tick: self.world.tick,
            position,
            speed: ship.physics.body.linear_velocity.length(),
            grounded: ship.physics.grounded,
            spline_distance,
            height_above_spline,
        }
    }

    /// Whether the ship is pointing back down the track.
    ///
    /// `dot(craft_forward, sample.tangent) < 0`. **The tangent, not the sample
    /// index** - `HANDOVER.md` records index order as unusable for this, because
    /// `Spline::from_track` concatenates paths in file order rather than travel
    /// order, and on a slow capture most windows step by zero. The dot product has
    /// neither failure mode and reads `+0.9999` against `-0.9999`.
    ///
    /// `false` when the ship is not near the spline at all, which is the safe way
    /// round: a spurious warning is worse than a missing one.
    #[must_use]
    pub fn wrong_way(&self) -> bool {
        let ship = self.ship();
        let Some((_, sample, _)) = self.spline.nearest(ship.physics.body.position) else {
            return false;
        };
        let tangent = Vec3::from_array(sample.tangent).normalize_or_zero();
        ship.physics.body.forward().dot(tangent) < 0.0
    }

    /// What the HUD shows, as of the last tick.
    ///
    /// Separate from [`Telemetry`], which is a log line and carries diagnostics
    /// no player sees. The fields with no source yet are left at their "unknown"
    /// value rather than filled with a plausible number - `lap` and `place` read
    /// zero, and [`crate::hud`] omits a widget rather than claiming a value it does
    /// not have. See `docs/ui/hud.md`.
    #[must_use]
    pub fn readout(&self) -> crate::hud::Readout {
        let ship = self.ship();
        crate::hud::Readout {
            speed_kmh: ship.physics.body.linear_velocity.length()
                * oag_render::exhaust::SPEED_TO_KMH,
            speed_full_kmh: crate::hud::DEFAULT_SPEED_FULL_KMH,
            // The pool the ship started with. Nothing depletes it: there are no
            // weapons, and track-contact damage is unrecovered - so the bar reads
            // full for the whole race, deliberately.
            shield: ship.handling.dimensions.shield,
            shield_max: ship.handling.dimensions.shield,
            // Lap counting is an open M5 question; zero means "unknown" and the
            // widget is omitted. `docs/formats/track.md#where-is-lap-counting`.
            lap: 0,
            laps: 0,
            // One ship on the grid until grid formation is recovered: seven of the
            // eight slots are laid out by unread code.
            place: 0,
            ships: u32::from(self.world.ship_count),
            race_ticks: self.world.tick,
            lap_ticks: self.world.tick,
            best_lap_ticks: None,
            wrong_way: self.wrong_way(),
        }
    }

    /// The exhaust's current animation state.
    #[must_use]
    pub fn exhaust(&self) -> &Exhaust {
        &self.exhaust
    }

    /// The collision sparks' current particle pool.
    #[must_use]
    pub fn sparks(&self) -> &Sparks {
        &self.sparks
    }

    /// The `engine_flare` locator in **world** space, or `None` when the ship
    /// model carries no `Engine Flare` node.
    ///
    /// Composed through [`Race::ship_model_matrix`], which is the only correct
    /// route: the locator is authored in `.vex` model space, so it has to pick up
    /// [`MODEL_YAW`] exactly as the hull's vertices do. A nozzle built from
    /// `body.forward()` instead would be a half-turn out and would sit on the
    /// ship's nose.
    #[must_use]
    pub fn nozzle(&self) -> Option<Vec3> {
        let local = self.nozzle?;
        Some(self.ship_model_matrix().transform_point3(local))
    }

    /// Where the camera is and what it is aimed at, as a view matrix.
    #[must_use]
    pub fn view(&self) -> Mat4 {
        self.camera.view(target_of(self.ship()), &self.chase_params)
    }

    /// The camera's own world position, from the view matrix it produces.
    ///
    /// A view matrix is the inverse of the camera's world transform, so
    /// inverting it back and taking the translation column is the camera's
    /// position. Read out rather than tracked separately so it cannot drift
    /// from what is actually being rendered.
    #[must_use]
    pub fn camera_position(&self) -> Vec3 {
        self.view().inverse().w_axis.truncate()
    }

    /// The authored sections the craft and the camera are in, for PVS culling.
    ///
    /// [`UNPLACED`] means "do not trust this", which the visibility set turns
    /// into *draw everything*. Three things produce it, and all three are
    /// states where culling to a section would be most likely to be visibly
    /// wrong:
    ///
    /// - **The track has no spline**, so nothing can be located at all.
    /// - **The point is off the track**, more than
    ///   [`OFF_TRACK_HALF_WIDTHS`] half-widths from the nearest sample. This is
    ///   the case that matters: a craft that has fallen off, gone airborne over
    ///   a gap or been knocked into scenery is usually still inside some
    ///   authored box and would otherwise get a confidently wrong answer. See
    ///   that constant for why the lookup's own out-of-range fallback does not
    ///   cover this.
    /// - **A respawn is in flight** ([`RESPAWN_COOLDOWN_TICKS`]), where the
    ///   craft teleports and the camera spring is still catching up, so neither
    ///   position describes the shot for several frames.
    ///
    /// The craft and the camera are located independently, so the common case
    /// of a camera swinging off the racing line while the craft is fine
    /// degrades only the camera's half.
    #[must_use]
    pub fn visibility_sections(&self) -> (u8, u8) {
        if self.respawn_cooldown > 0 {
            return (UNPLACED, UNPLACED);
        }
        let ship = self.ship();
        // The simulation already located the craft this tick and left the
        // sample index in `Ship::segment`, so the craft's half is a table read
        // rather than a second scan of the whole track. Doing it again here
        // would cost more per frame than the culling it enables saves.
        // `u16::MAX` is the never-located sentinel, and it is out of range of
        // any real table, so the bounds check covers both.
        let index = usize::from(ship.segment);
        if index >= self.spline.len() {
            return (UNPLACED, UNPLACED);
        }
        let craft = self.section_of(index, ship.physics.body.position);
        // The camera is a chase spring a few units behind, so it is a few
        // samples away in the same table. See `Spline::nearest_within` for why
        // a local search is allowed to miss.
        let camera =
            match self
                .spline
                .nearest_within(self.camera_position(), index, CAMERA_SEARCH_SAMPLES)
            {
                Some((camera_index, _, _)) => self.section_of(camera_index, self.camera_position()),
                None => UNPLACED,
            };
        (craft, camera)
    }

    /// The section of the sample at `index`, or [`UNPLACED`] when `position` is
    /// too far from it to trust. See [`Self::visibility_sections`].
    fn section_of(&self, index: usize, position: Vec3) -> u8 {
        let Some(sample) = self.spline.sample(index) else {
            return UNPLACED;
        };
        let distance = (Vec3::from_array(sample.pos) - position).length();
        let half_width = sample.half_width_left.max(sample.half_width_right);
        if distance > half_width * OFF_TRACK_HALF_WIDTHS {
            return UNPLACED;
        }
        sample.section_id
    }

    /// The projection for a viewport of the given aspect ratio, with the
    /// player's field-of-view setting applied.
    ///
    /// **`<ExternalCameraFar fov>`'s unit is unrecovered.** It is read here as
    /// degrees, and `oag_render::camera::projection` names its parameter
    /// `fov_radians` precisely so that the conversion has to be written at a call
    /// site rather than assumed in a library. The value read from the disc is
    /// printed in the load report, so the assumption is checkable by whoever looks.
    ///
    /// The value is also authored for **one** viewport shape, the PSP's own, which
    /// is the only one the original ever renders into. A window narrower than that
    /// goes through [`oag_render::camera::fit_vertical_fov`], which holds the
    /// horizontal field the authored shape would have had instead of cropping the
    /// sides away; at the PSP's aspect and anything wider it is the authored value
    /// unchanged. That is a presentation choice and is documented as one.
    ///
    /// `setting` is a **percentage of that authored value** and is applied
    /// *before* the fit, so the two compose in the order a reader would expect:
    /// the player widens the field the disc asked for, and the result is then
    /// fitted to whatever shape the window is. At
    /// [`Fov::AUTHORED`](crate::display::Fov::AUTHORED) - the default - nothing
    /// is applied at all and this is the projection every capture under
    /// `data/traces/` was taken with.
    ///
    /// `near` is 1.0 rather than something tiny: track half-widths are tens of
    /// units, so a near plane at 0.01 spends the depth range on nothing and
    /// z-fights in the distance.
    #[must_use]
    pub fn projection(&self, aspect: f32, far: f32, setting: crate::display::Fov) -> Mat4 {
        let authored = setting.apply(self.chase_params.fov.to_radians());
        let fov = oag_render::camera::fit_vertical_fov(authored, AUTHORED_ASPECT, aspect);
        oag_render::camera::projection(fov, aspect, 1.0, far)
    }

    /// Where the ship is and how it is oriented, as the mesh shader's model matrix.
    ///
    /// Rotation and translation only, so the shader's assumption of uniform scale -
    /// which is what lets it rotate normals without an inverse transpose - holds.
    /// The model's own origin is used as it is: no recentring, and the only rotation
    /// composed in is [`MODEL_YAW`], which is a stated finding about the `.vex` and
    /// not a nudge to make the picture look right.
    #[must_use]
    pub fn ship_model_matrix(&self) -> Mat4 {
        let body = &self.ship().physics.body;
        Mat4::from_rotation_translation(body.orientation, body.position)
            * Mat4::from_rotation_y(MODEL_YAW)
    }
}

/// The chase camera's view of a ship.
///
/// Three vectors rather than a ship, because `oag-render` may not see a gameplay
/// type; that is rule 1 of `docs/architecture/workspace-layout.md`.
fn target_of(ship: &Ship) -> Target {
    let body = &ship.physics.body;
    Target {
        position: body.position,
        forward: body.forward(),
        up: body.up(),
    }
}

/// A held-button mask, as one snapshot per tick, through the real keyboard path.
///
/// The headless capture and the ground-truth test both need input with no window.
/// They get it by pressing the **keys** a window would have seen, so there is
/// exactly one mapping from a device to a snapshot and the axes are derived in
/// exactly one place - [`oag_input::Keyboard`]. Deriving them a second time here is
/// how "left" ends up meaning two different things in two files.
#[derive(Debug, Default)]
pub struct HeldButtons {
    keyboard: Keyboard,
}

impl HeldButtons {
    /// Holds every button in `mask` down, and nothing else.
    ///
    /// Buttons with no key bound to them are skipped: a mask naming START in a race
    /// is not a fault condition.
    #[must_use]
    pub fn new(mask: u32) -> Self {
        let mut keyboard = Keyboard::new();
        for index in 0..32u8 {
            if mask & (1u32 << index) == 0 {
                continue;
            }
            if let Some(key) = key_for_button(index) {
                keyboard.set_key(&key, true);
            }
        }
        Self { keyboard }
    }

    /// One tick's snapshot.
    pub fn snapshot(&mut self) -> InputSnapshot {
        self.keyboard.snapshot()
    }
}

/// A key that produces the given abstract button.
///
/// The inverse of [`oag_input::keys::map_key`], for the buttons a race uses. An
/// inverse and not a second mapping: a unit test below asserts every pair
/// round-trips, so this table cannot drift away from the one the window uses.
fn key_for_button(index: u8) -> Option<winit::keyboard::Key> {
    use oag_gameplay::input::button;
    use winit::keyboard::{Key, NamedKey};

    let key = match index {
        button::UP => Key::Named(NamedKey::ArrowUp),
        button::DOWN => Key::Named(NamedKey::ArrowDown),
        button::LEFT => Key::Named(NamedKey::ArrowLeft),
        button::RIGHT => Key::Named(NamedKey::ArrowRight),
        button::CROSS => Key::Character("x".into()),
        button::L => Key::Character("q".into()),
        button::R => Key::Character("e".into()),
        _ => return None,
    };
    Some(key)
}

/// Uniforms shared with `oag-render`'s `mesh.wgsl`.
///
/// Declared here rather than reused because `oag_render::mesh_render` only exposes
/// a writer that computes an *orbit* camera from the model's bounding sphere, which
/// is what a viewer wants and is not something a chase camera can use. The layout is
/// the shader's own, and a test below asserts it against
/// `mesh_render::UNIFORMS_SIZE`, so a change on that side is a failing test rather
/// than a silently wrong picture.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_projection: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    anim_phase: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

/// What one frame's frustum culling did, for the performance overlay.
///
/// Plain counts rather than anything richer: the point is to show the effect
/// of `[graphics] lod` and frustum culling is having, not to profile the
/// renderer.
#[derive(Debug, Clone, Copy, Default)]
pub struct SceneStats {
    pub draws_submitted: u32,
    pub draws_culled: u32,
    pub triangles: u32,
}

impl SceneStats {
    fn add(&mut self, other: Self) {
        self.draws_submitted += other.draws_submitted;
        self.draws_culled += other.draws_culled;
        self.triangles += other.triangles;
    }
}

/// The track's authored visibility partition, and where this track's geometry
/// sits in it.
///
/// Built once at load. Per frame it answers one question - which sections may
/// be drawn - in a handful of array reads. See
/// `docs/architecture/adr/0011-authored-pvs-before-frustum-culling.md`.
#[derive(Debug)]
pub struct TrackVisibility {
    pvs: oag_formats::pvs::TrackPvs,
    padding: SectionPadding,
    sections: DrawSections,
    /// What the association rule managed, for the load report.
    pub placement: PlacementStats,
}

impl TrackVisibility {
    /// Reads the `section` nodes of a track and places its draw calls in them.
    ///
    /// Returns `None` when the track declares no sections at all - which is
    /// every Pure track, since Pure does not share Pulse's class numbering.
    /// The caller then draws with no first tier, exactly as before.
    #[must_use]
    pub fn build(model: &Model, blob: &[u8], ai: &AiTrack) -> Option<Self> {
        let pvs = oag_formats::pvs::TrackPvs::parse(blob).ok()?;
        if pvs.is_empty() {
            return None;
        }
        let (sections, placement) = DrawSections::place(model, &pvs);
        Some(Self {
            pvs,
            padding: SectionPadding::from_track(ai),
            sections,
            placement,
        })
    }

    /// What may be drawn with the craft in `craft` and the camera in `camera`.
    #[must_use]
    fn set(&self, craft: u8, camera: u8) -> VisibleSet {
        VisibleSet::around(&self.pvs, &self.padding, craft, camera)
    }
}

/// How far off the nearest spline sample a point may be and still be trusted to
/// name a section, as a multiple of the track's widest half-width.
///
/// **The conservative path exists because a craft can leave the partition, and
/// being outside it is not the same as looking up an id the track does not
/// have.** A craft that has fallen off, is airborne over a gap, or has been
/// knocked into scenery is usually still inside *some* authored box, so it gets
/// a valid answer that is simply wrong for what the camera is now framing - the
/// all-ones fallback never fires on its own. Culling to a stale section exactly
/// when the craft is somewhere unusual is the most visible way this could fail,
/// so distance to the racing line gates it instead.
///
/// Three half-widths is deliberately loose: it must not trip during ordinary
/// wide cornering, only when the craft is somewhere the authored partition was
/// not drawn around. Widening it costs nothing but a little culling; narrowing
/// it risks pop-in. There is no recovered value to match - the original does
/// not have this problem, because it never culls to a camera.
const OFF_TRACK_HALF_WIDTHS: f32 = 3.0;

/// How many samples either side of the craft's own the camera is looked for in.
///
/// At four samples per control-point interval this is a couple of dozen
/// intervals of track, far more than a chase camera trails by, and about
/// thirty-five times cheaper than the whole-table scan it replaces. Missing is
/// safe - see [`Spline::nearest_within`].
const CAMERA_SEARCH_SAMPLES: usize = 96;

/// Whether `draw` should be submitted, in two tiers.
///
/// **The authored PVS first, the frustum second**, which is the ordering the
/// ADR is about: the mask test is an integer `and` and the frustum test is six
/// plane-versus-sphere evaluations, so the cheap one has to run first for the
/// second tier to see less work. Delegated to `oag_render::pvs` so the ordering
/// lives with the types it operates on rather than being re-established at each
/// call site.
///
/// `set` is `None` when `[graphics] pvs_culling` is off or the track has no
/// sections; `frustum` is `None` per [`Drawable::draw`].
fn visible(
    draw: &DrawCall,
    sections: u64,
    set: Option<&VisibleSet>,
    frustum: Option<&Frustum>,
) -> bool {
    oag_render::pvs::visible(draw, sections, set, frustum)
}

/// One model on the GPU: its pipeline, its geometry and its own uniform buffer.
///
/// Two of these are drawn into one render pass. Separate pipelines rather than one
/// shared between the models, because each `mesh_render::build` creates its own bind
/// group layouts and pairing a bind group with another pipeline's layout is a
/// validation error waiting to happen.
struct Drawable {
    model: Model,
    pipeline: wgpu::RenderPipeline,
    alpha_test_pipeline: wgpu::RenderPipeline,
    blend_pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    uniforms: wgpu::Buffer,
    uniform_bind: wgpu::BindGroup,
    textures: Vec<wgpu::BindGroup>,
}

impl std::fmt::Debug for Drawable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Drawable")
            .field("model", &self.model.label)
            .field("triangles", &(self.model.indices.len() / 3))
            .finish_non_exhaustive()
    }
}

impl Drawable {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: Model,
        format: wgpu::TextureFormat,
        anisotropy: Anisotropy,
    ) -> Result<Self> {
        let mesh_render::Built {
            pipeline,
            alpha_test_pipeline,
            blend_pipeline,
            bind_group: _placeholder,
            vertex_buffer: vertices,
            index_buffer: indices,
            texture_binds: textures,
        } = mesh_render::build(device, queue, &model, format, anisotropy)?;

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("race uniforms"),
            size: mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("race uniforms"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

        Ok(Self {
            model,
            pipeline,
            alpha_test_pipeline,
            blend_pipeline,
            vertices,
            indices,
            uniforms,
            uniform_bind,
            textures,
        })
    }

    fn write(&self, queue: &wgpu::Queue, view_projection: Mat4, model: Mat4, anim_phase: f32) {
        let uniforms = Uniforms {
            view_projection: view_projection.to_cols_array_2d(),
            model: model.to_cols_array_2d(),
            anim_phase,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Draws every list, in pipeline order, and reports what it submitted.
    ///
    /// `frustum` is `None` for the ship and the collision overlay: both draw a
    /// handful of batches next to the camera regardless, where the bookkeeping
    /// would cost more than the culling could ever save, and both carry a
    /// non-identity model matrix the track does not - the [`Bounds`] on a
    /// `DrawCall` are in the *model's own* space, valid to test directly
    /// against a world-space frustum only while that space and world space
    /// are the same transform, which is only true here for the track (see the
    /// `Mat4::IDENTITY` passed to [`Self::write`] at each call site).
    fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        sections: Option<&DrawSections>,
        set: Option<&VisibleSet>,
        frustum: Option<&Frustum>,
    ) -> SceneStats {
        let mut stats = SceneStats::default();
        // A model with no placement table has every draw call unplaced, which
        // the first tier always allows. That is the ship and the collision
        // overlay, and any track whose sections did not decode.
        let empty: &[u64] = &[];
        let (opaque, alpha_tested, transparent) = match sections {
            Some(s) => (&s.opaque[..], &s.alpha_tested[..], &s.transparent[..]),
            None => (empty, empty, empty),
        };
        if self.model.indices.is_empty() {
            return stats;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bind, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        // Slot 0 is the white fallback, so a texture index of n binds slot n + 1.
        for (index, draw) in self.model.draws.iter().enumerate() {
            if !visible(draw, DrawSections::at(opaque, index), set, frustum) {
                stats.draws_culled += 1;
                continue;
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            let slot = draw.texture.map_or(0, |t| t + 1);
            pass.set_bind_group(1, &self.textures[slot.min(self.textures.len() - 1)], &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }

        // Second pipeline, same pass: alpha-tested batches, cutout. See
        // `mesh_render::Built::alpha_test_pipeline`.
        pass.set_pipeline(&self.alpha_test_pipeline);
        for (index, draw) in self.model.alpha_tested_draws.iter().enumerate() {
            if !visible(draw, DrawSections::at(alpha_tested, index), set, frustum) {
                stats.draws_culled += 1;
                continue;
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            let slot = draw.texture.map_or(0, |t| t + 1);
            pass.set_bind_group(1, &self.textures[slot.min(self.textures.len() - 1)], &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }

        // Third pipeline, same pass: transparent batches, blended. See
        // `mesh_render::Built::blend_pipeline`.
        pass.set_pipeline(&self.blend_pipeline);
        for (index, draw) in self.model.transparent_draws.iter().enumerate() {
            if !visible(draw, DrawSections::at(transparent, index), set, frustum) {
                stats.draws_culled += 1;
                continue;
            }
            stats.draws_submitted += 1;
            stats.triangles += (draw.range.end - draw.range.start) / 3;
            let slot = draw.texture.map_or(0, |t| t + 1);
            pass.set_bind_group(1, &self.textures[slot.min(self.textures.len() - 1)], &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
        stats
    }
}

/// The period of the global texture-animation clock, in ticks.
///
/// Every animated surface is a whole number of V sweeps per this period (see
/// `oag_render::mesh::ANIMATED_TEXTURES`), which is what makes the wrap back to
/// phase zero seamless instead of a visible jump: at the wrap every surface is
/// an exact number of full texture heights along, and V wraps too.
///
/// 120 rather than 60 so a surface running at half the blink light's speed can
/// still be expressed as an integer.
const ANIM_PERIOD_TICKS: u64 = 120;

/// The global texture-animation clock's phase for `tick`, 0.0 up to 1.0.
///
/// **Confidence: 85** for the mechanism (a V-axis palette scroll - see
/// `docs/formats/vex.md`, "The animation is authored in the texture, on its V
/// axis"), and a narrower, separately-checkable claim for the rate. A live
/// capture (120 frame-accurate PPSSPP screenshots, pixels sampled at a real
/// light) measured one authored 8-row cycle repeating every 29-31 ticks; the
/// blink texture repeats that 8-row cycle twice across its 16 rows, so one full
/// scroll of all 16 rows is twice that, 60 ticks - which is `BLINK_V_CYCLES`
/// (2.0) sweeps per this 120-tick period, one second at the fixed 60 Hz.
///
/// From `world.tick`, never the wall clock, so a replay of the same tick draws
/// the same frame.
fn anim_phase(tick: u64) -> f32 {
    (tick % ANIM_PERIOD_TICKS) as f32 / ANIM_PERIOD_TICKS as f32
}

/// The track and the ship on the GPU, drawn from a chase camera.
#[derive(Debug)]
pub struct Scene {
    track: Drawable,
    /// The track's authored visibility partition, when it decoded.
    ///
    /// `None` for a track with no `section` nodes - every Pure track - and the
    /// first tier is then skipped entirely rather than approximated.
    visibility: Option<TrackVisibility>,
    ship: Drawable,
    /// The collision soup overlay, present only when `Options::collision` asked
    /// for it. Drawn with the identity transform, same as the track: the
    /// collision geometry is already in world space.
    collision: Option<Drawable>,
    /// The engine flare: the one blended pipeline in the frame.
    ///
    /// `RefCell` because its per-frame upload needs `&mut` while [`Scene::render`]
    /// stays `&self`. That signature is worth keeping: the alternative threads
    /// `&mut` through `RaceStage::render` and `race::capture` for a buffer write
    /// that `queue` already accepts through a shared reference. The borrow is
    /// taken and released inside `render` with nothing re-entrant in between.
    exhaust: std::cell::RefCell<exhaust::Pipeline>,
    /// Collision sparks. `RefCell` for the same reason [`Self::exhaust`] is.
    sparks: std::cell::RefCell<sparks::Pipeline>,
    depth: wgpu::Texture,
    /// Where the far plane goes, from the track's own extent.
    far: f32,
}

impl Scene {
    /// Builds both pipelines and a depth buffer for a viewport of `size`.
    ///
    /// # Errors
    ///
    /// Propagates a pipeline or geometry upload failure from `oag-render`.
    // Three models plus how to draw them (surface format, viewport, texture
    // filtering); a wrapper struct for one call site would name the grouping
    // without clarifying it.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        track_model: Model,
        ship_model: Model,
        collision_model: Option<Model>,
        flare: Option<FlareTexture>,
        noise: Option<FlareTexture>,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        anisotropy: Anisotropy,
        visibility: Option<TrackVisibility>,
    ) -> Result<Self> {
        // The far plane comes from the track's own bounding sphere: a track is
        // hundreds of units across, and a fixed guess would either clip it away or
        // waste the depth range on empty space.
        let far = track_model.radius * 4.0;
        let track = Drawable::new(device, queue, track_model, format, anisotropy)?;
        let ship = Drawable::new(device, queue, ship_model, format, anisotropy)?;
        let collision = collision_model
            .map(|model| Drawable::new(device, queue, model, format, anisotropy))
            .transpose()?;
        // 64 is a stand-in size only, and only when the disc's own texture did not
        // decode; `load` has already reported that when it happens.
        let flare = flare.unwrap_or_else(|| FlareTexture::placeholder(64));
        let noise = noise.unwrap_or_else(|| FlareTexture::placeholder(64));
        let exhaust = std::cell::RefCell::new(exhaust::Pipeline::new(
            device, queue, format, &flare, &noise,
        ));
        let sparks = std::cell::RefCell::new(sparks::Pipeline::new(device, format));

        Ok(Self {
            track,
            visibility,
            ship,
            collision,
            exhaust,
            sparks,
            depth: depth_texture(device, size),
            far,
        })
    }

    /// Rebuilds the depth buffer for a new viewport size.
    ///
    /// A depth attachment whose size does not match the colour attachment is a
    /// validation error, so this is not optional on resize.
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        self.depth = depth_texture(device, size);
    }

    /// Draws one frame into `view`.
    ///
    /// One render pass with one clear: a second pass would either wipe the first's
    /// colour or need its own decision about the depth buffer.
    ///
    /// `cull` is `[graphics] frustum_culling` - off by default, see that
    /// setting's own doc comment for the measurement behind that default.
    ///
    /// Returns what the track's frustum culling did, for the performance
    /// overlay - see [`SceneStats`].
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        race: &Race,
        viewport: (f32, f32, f32, f32),
        fov: crate::display::Fov,
        cull: bool,
        pvs_cull: bool,
        animated_textures: bool,
    ) -> SceneStats {
        let aspect = viewport.2.max(1.0) / viewport.3.max(1.0);
        let view_projection = race.projection(aspect, self.far, fov) * race.view();
        let frustum = cull.then(|| Frustum::from_view_projection(view_projection));
        // Tier one, built once a frame. Both sections come from the authored
        // spline rather than from the section boxes: a control point's
        // `section_id` is what the artists wrote, while a point-in-box test is
        // something this project invented.
        let visible_set = self
            .visibility
            .as_ref()
            .filter(|_| pvs_cull)
            .map(|visibility| {
                let (craft, camera) = race.visibility_sections();
                visibility.set(craft, camera)
            });
        let scroll = anim_phase(race.world.tick);
        // The ship's lights keep scrolling either way: `[graphics]
        // animated_textures` exists to test the *inferred* track entries
        // against a capture, and the ship's behaviour is not inferred.
        let track_scroll = if animated_textures { scroll } else { 0.0 };
        self.track
            .write(queue, view_projection, Mat4::IDENTITY, track_scroll);
        self.ship
            .write(queue, view_projection, race.ship_model_matrix(), scroll);
        if let Some(collision) = &self.collision {
            collision.write(queue, view_projection, Mat4::IDENTITY, track_scroll);
        }

        // The camera's own axes, read out of the view matrix: for a view matrix
        // `V`, world-space right and up are rows 0 and 1 of its rotation part.
        // Building the quad from these is what makes it face the viewer, and it is
        // the whole reason this is world-space rather than the original's
        // post-projection sprite.
        let camera = race.view();
        let right = Vec3::new(camera.x_axis.x, camera.y_axis.x, camera.z_axis.x);
        let up = Vec3::new(camera.x_axis.y, camera.y_axis.y, camera.z_axis.y);
        let (vertices, trail) = match race.nozzle() {
            Some(nozzle) => (
                race.exhaust().vertices(nozzle, right, up),
                race.exhaust().trail_vertices(right, up),
            ),
            // No locator, nothing drawn - rather than a flare at the origin.
            None => (Vec::new(), Vec::new()),
        };
        self.exhaust.borrow_mut().upload(
            queue,
            &view_projection.to_cols_array_2d(),
            &vertices,
            &trail,
        );
        self.sparks.borrow_mut().upload(
            queue,
            &view_projection.to_cols_array_2d(),
            &race.sparks().vertices(right, up),
        );

        let depth_view = self
            .depth
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("race"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Black rather than the near-black blue this used to clear
                    // to. The clear covers the whole attachment and the scene is
                    // then drawn into a sub-rectangle, so this colour is what
                    // `Aspect`'s bars are made of - and a bar has to read as a
                    // bar. The old value was 0.03/0.04/0.06, dark enough that
                    // losing it costs nothing inside the viewport either.
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_viewport(viewport.0, viewport.1, viewport.2, viewport.3, 0.0, 1.0);
        let mut stats = self.track.draw(
            &mut pass,
            self.visibility.as_ref().map(|v| &v.sections),
            visible_set.as_ref(),
            frustum.as_ref(),
        );
        stats.add(self.ship.draw(&mut pass, None, None, None));
        if let Some(collision) = &self.collision {
            stats.add(collision.draw(&mut pass, None, None, None));
        }
        // Last, and that ordering is load-bearing: the flare tests depth but does
        // not write it, so the hull's depth has to already be in the buffer for the
        // flare to be occluded by it. Sparks are the same kind of blended,
        // depth-tested-not-written geometry, so they follow right after for the
        // same reason.
        self.exhaust.borrow().draw(&mut pass);
        self.sparks.borrow().draw(&mut pass);
        stats
    }
}

fn depth_texture(device: &wgpu::Device, size: (u32, u32)) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("race depth"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: mesh_render::DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

/// What a headless capture should do before it draws.
#[derive(Debug, Clone)]
pub struct CaptureOptions {
    /// Where to write the PNG.
    pub path: std::path::PathBuf,
    /// Ticks to advance the simulation first.
    pub ticks: u32,
    /// Buttons held on every one of those ticks.
    pub held: u32,
    /// Image size.
    pub size: (u32, u32),
    /// Print a telemetry line every this many ticks. Zero prints none.
    pub log_every: u32,
    /// The shape to draw at inside the frame, leaving bars.
    ///
    /// A capture is a picture of a window, so it letterboxes the way a window
    /// does. At the default `--size`, which is the PSP's own shape, every value
    /// of this fills the frame and nothing changes.
    pub aspect: crate::display::Aspect,
    /// Anisotropic filtering level for the track and ship textures.
    pub anisotropy: Anisotropy,
    /// Which adapter to draw with, for the same reason `aspect` and `fov` are
    /// here: a capture is only evidence about what a player sees if it was
    /// drawn on the device they see it on. A driver is exactly the kind of
    /// thing a rendering difference gets blamed on, so a capture that quietly
    /// used a different one would be the wrong picture to argue from.
    ///
    /// There is no surface here, so an adapter that could not present is still
    /// eligible - which is the one way this list can be wider than the menu's.
    pub renderer: crate::display::Renderer,
    /// The field-of-view setting, for the same reason `aspect` is here: a
    /// capture should frame what a player at these settings would have seen.
    pub fov: crate::display::Fov,
    /// Whether the view frustum culls before the frame is drawn.
    ///
    /// Honoured rather than forced off, so that the screenshot comparison this
    /// project already claims for `[graphics] frustum_culling` can actually be
    /// run from a capture, and so a report of geometry going missing can be
    /// attributed to a tier rather than guessed at.
    pub frustum_culling: bool,
    /// Whether the authored PVS culls before the frame is drawn.
    ///
    /// Here, and honoured, so that `--screenshot` with `[graphics] pvs_culling`
    /// on and off produces two images to compare. **That comparison is the only
    /// way to show the association rule in `oag_render::pvs` places geometry in
    /// the right sections rather than merely in some section**, and it is the
    /// bar that setting has to clear before it can default on - the same one
    /// frustum culling passed. Frustum culling stays off in a capture either
    /// way, so the two images differ by this tier alone.
    pub pvs_culling: bool,
    /// Whether the inferred trackside texture animations run.
    ///
    /// Honoured for the same reason the two culling tiers are: two captures
    /// differing only by this setting are how `[graphics] animated_textures`
    /// gets checked against the running original, and that check is the whole
    /// reason the setting exists.
    pub animated_textures: bool,
    /// Capture the frame the way a **window** presents it, rather than the
    /// scene the way it is drawn.
    ///
    /// `None` is the ordinary capture: the scene, straight out of the target it
    /// was drawn into, ungraded, at exactly `size`. That is the right default
    /// for a bug report, and it is deliberately not a picture of a window - see
    /// [`crate::upscale`].
    ///
    /// `Some` puts the whole presentation path in the way: the render scale,
    /// the upscaler, the grade and the aspect bars. **This is the only way to
    /// see an upscaler's output at all**, because the ordinary path never
    /// reaches the blit, and it is therefore what a still-frame comparison
    /// between resamplers has to use. It is also, necessarily, an sRGB pipeline
    /// throughout, exactly as a window is.
    pub presented: Option<Presented>,
}

/// [`Presented`] with the scene size worked out.
#[derive(Debug, Clone, Copy)]
struct PresentedState {
    scene_size: (u32, u32),
    presentation: crate::upscale::Presentation,
}

/// The settings a `--presented` capture needs that an ordinary one does not.
#[derive(Debug, Clone, Copy)]
pub struct Presented {
    /// What fraction of the aspect rectangle the scene is drawn at.
    pub render_scale: crate::display::Scale,
    /// The upscaler, its sharpness, and the grade.
    pub presentation: crate::upscale::Presentation,
}

/// Runs a race headless and writes one frame to a PNG.
///
/// Through the same [`Scene`] the window draws, for the same reason
/// [`crate::capture`] goes through the same renderer the front end's window does: a
/// separate capture path would prove nothing about what a player sees.
///
/// # Errors
///
/// Propagates adapter and device creation, pipeline building, the readback map and
/// the file write.
pub fn capture(loaded: Loaded, options: &CaptureOptions) -> Result<()> {
    let (width, height) = options.size;
    let Loaded {
        setup,
        hud,
        track_model,
        ship_model,
        collision_model,
        visibility,
        flare,
        noise,
        ..
    } = loaded;
    let mut race = Race::start(setup);

    let mut held = HeldButtons::new(options.held);
    for _ in 0..options.ticks {
        let snapshot = held.snapshot();
        race.tick(&snapshot);
        if options.log_every > 0 && race.world.tick.is_multiple_of(u64::from(options.log_every)) {
            println!("{}", describe(&race.telemetry()));
        }
    }
    println!(
        "after {} tick(s): {}",
        race.world.tick,
        describe(&race.telemetry())
    );

    let instance = crate::adapter::instance();
    let adapter = crate::adapter::choose(&instance, None, &options.renderer)?.adapter;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("oag-game race offscreen"),
        ..Default::default()
    }))
    .context("requesting the device")?;

    // `Rgba8Unorm` for an ordinary capture: the readback goes straight into a
    // PNG and the front end's own draws already write sRGB values, so a second
    // gamma encode would double-correct. A `--presented` capture is a picture
    // of a *window*, so it takes the window's format and lets the hardware
    // encode on write - which is also what makes the offscreen target's
    // non-sRGB twin, and therefore FSR 1, work at all. See `crate::upscale`.
    //
    // (Whether the *ordinary* path should follow suit is an open question, and
    // it is open because it would also encode authored HUD text colours. See
    // HANDOVER.)
    let format = match options.presented {
        Some(_) => wgpu::TextureFormat::Rgba8UnormSrgb,
        None => wgpu::TextureFormat::Rgba8Unorm,
    };
    // The scene's own size, which presented is the aspect rectangle scaled and
    // otherwise is the whole capture.
    let presented = options.presented.map(|state| PresentedState {
        scene_size: crate::upscale::target_size(
            crate::display::viewport((width, height), options.aspect),
            state.render_scale,
            device.limits().max_texture_dimension_2d,
        ),
        presentation: state.presentation,
    });
    // What the scene - and so its depth buffer - is actually drawn at. A depth
    // attachment whose size does not match the colour one is a validation
    // error, not a bad picture.
    let scene_size = presented.map_or((width, height), |state| state.scene_size);
    let scene = Scene::new(
        &device,
        &queue,
        track_model,
        ship_model,
        collision_model,
        flare,
        noise,
        format,
        scene_size,
        options.anisotropy,
        visibility,
    )?;

    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("race capture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let surface = target.create_view(&wgpu::TextureViewDescriptor::default());
    // Where the scene is drawn. Presented, that is an offscreen target at the
    // render scale which the blit later stretches into the aspect rectangle -
    // the same two-step a window does. Otherwise it is the capture texture
    // itself, and the scene draws into a sub-rectangle of it directly.
    let mut framebuffer = match presented {
        Some(state) => Some(
            crate::upscale::Framebuffer::new(&device, format, state.scene_size)
                .context("building the upscale pipeline")?,
        ),
        None => None,
    };
    let view = match &framebuffer {
        Some(framebuffer) => framebuffer.view().clone(),
        None => surface.clone(),
    };

    // Texture copies want rows aligned to 256 bytes, so the readback buffer is
    // usually wider than the image and needs unpadding.
    let unpadded = width as usize * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = unpadded.div_ceil(align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * height as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("race capture"),
    });
    // Shaped the same way a window is, so a screenshot frames what a player
    // would have seen at that size rather than a differently-cropped picture.
    // Presented, the offscreen target *is* that rectangle and the bars are what
    // the blit clears around it, so the scene fills its target instead.
    let rect = crate::display::viewport((width, height), options.aspect);
    let viewport = match presented {
        Some(state) => (
            0.0,
            0.0,
            state.scene_size.0 as f32,
            state.scene_size.1 as f32,
        ),
        None => rect,
    };
    // Both tiers follow their settings, because two captures differing only by
    // one of them are how that tier gets validated - see
    // `CaptureOptions::frustum_culling` and `CaptureOptions::pvs_culling`.
    scene.render(
        &queue,
        &mut encoder,
        &view,
        &race,
        viewport,
        options.fov,
        options.frustum_culling,
        options.pvs_culling,
        options.animated_textures,
    );

    // The HUD, into the same target. Without this a race screenshot would show
    // the track and no HUD at all, because unlike the front end's capture this
    // path has no `Framebuffer` and so no overlay pass of its own - which would
    // make `--screenshot` useless for the one thing it is most wanted for.
    //
    // Note an ordinary capture's target is `Rgba8Unorm` while a window's is
    // sRGB, and `Renderer::new` forks the sprite sheet's texture format on
    // `format.is_srgb()`. Text and fills go through the R8 coverage atlas and are
    // unaffected; the HUD's art is not. See `docs/ui/hud.md`. A `--presented`
    // capture is sRGB throughout and so takes the window's side of that fork.
    match crate::hud::Overlay::new(&device, &queue, format, &hud) {
        Ok(Some(mut overlay)) => overlay.draw(
            &device,
            &queue,
            &mut encoder,
            &view,
            &race.readout(),
            viewport,
        ),
        Ok(None) => println!("no HUD layout: capturing without one"),
        Err(why) => println!("the HUD overlay did not build ({why}); capturing without one"),
    }
    // The upscaler, the grade and the blit, through exactly the call the
    // window's frame loop makes.
    if let (Some(framebuffer), Some(state)) = (framebuffer.as_mut(), presented) {
        framebuffer.resolve(
            &device,
            &queue,
            &mut encoder,
            &surface,
            rect,
            &state.presentation,
        );
    }

    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .context("waiting for the GPU")?;
    let mapped = slice
        .get_mapped_range()
        .context("mapping the readback buffer")?;
    let mut pixels = Vec::with_capacity(unpadded * height as usize);
    for row in mapped.chunks(padded).take(height as usize) {
        pixels.extend_from_slice(&row[..unpadded]);
    }
    drop(mapped);
    readback.unmap();

    let png = oag_formats::png::encode_rgba(width, height, &pixels);
    std::fs::write(&options.path, png)
        .with_context(|| format!("writing {}", options.path.display()))?;
    println!("wrote {} ({width}x{height})", options.path.display());
    Ok(())
}

/// One telemetry line, for a log or a report.
#[must_use]
pub fn describe(telemetry: &Telemetry) -> String {
    format!(
        "tick {:>5}  speed {:>8.2}  grounded {:>3.1}  spline {:>8.2}  height {:>8.2}  at {:.1}",
        telemetry.tick,
        telemetry.speed,
        telemetry.grounded,
        telemetry.spline_distance,
        telemetry.height_above_spline,
        telemetry.position,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_formats::track;
    use oag_gameplay::input::button;
    use oag_input::keys::map_key;

    /// A model declaring `slots` texture slots of which the first `decoded` filled.
    fn model(slots: usize, decoded: usize) -> Model {
        Model {
            label: "Ship.vex".to_string(),
            vertices: Vec::new(),
            indices: vec![0; 6],
            draws: Vec::new(),
            alpha_tested_draws: Vec::new(),
            transparent_draws: Vec::new(),
            textures: (0..slots)
                .map(|index| {
                    (index < decoded).then(|| mesh::ModelTexture {
                        label: format!("#{index}"),
                        width: 1,
                        height: 1,
                        rgba: vec![255; 4],
                    })
                })
                .collect(),
            centre: [0.0; 3],
            radius: 1.0,
            mesh_count: 1,
        }
    }

    #[test]
    fn anim_phase_starts_at_zero_and_wraps_every_period() {
        assert_eq!(anim_phase(0), 0.0);
        assert!((anim_phase(ANIM_PERIOD_TICKS / 2) - 0.5).abs() < 1e-6);
        assert_eq!(anim_phase(ANIM_PERIOD_TICKS), 0.0);
        assert_eq!(anim_phase(ANIM_PERIOD_TICKS * 3), 0.0);
    }

    /// The blink light kept the exact behaviour it had when the scroll was a
    /// dedicated 60-tick uniform rather than a shared 120-tick clock with a
    /// per-vertex rate: 2 sweeps per 120 ticks is 1 sweep per 60.
    #[test]
    fn the_blink_light_still_sweeps_once_every_sixty_ticks() {
        let sweeps = |tick| anim_phase(tick) * oag_render::mesh::ANIMATED_TEXTURES[0].1;
        assert_eq!(sweeps(0), 0.0);
        assert!((sweeps(30) - 0.5).abs() < 1e-6);
        assert!((sweeps(60) - 1.0).abs() < 1e-6);
        assert!((sweeps(90) - 1.5).abs() < 1e-6);
    }

    /// The PS2 shape, measured: five slots declared and none of them filled.
    #[test]
    fn slots_that_all_failed_to_decode_are_reported_with_the_count() {
        let note = untextured_note(&model(5, 0)).expect("an untextured model is reported");
        assert!(note.contains("Ship.vex"), "{note}");
        assert!(note.contains("0 of 5 texture slot(s)"), "{note}");
        // The reader has to be able to get from the line to the finding, because
        // the line on its own reads like a decode failure and it is not one.
        assert!(note.contains("ps2-texture.md"), "{note}");
    }

    /// A model that got everything it asked for is silent, which is every PSP
    /// model measured: 8 of 8 on the ship, 135 of 135 on the track.
    #[test]
    fn a_fully_textured_model_is_not_reported() {
        assert_eq!(untextured_note(&model(8, 8)), None);
    }

    #[test]
    fn a_partly_textured_model_is_reported_too() {
        let note = untextured_note(&model(8, 3)).expect("a partial decode is worth saying");
        assert!(note.contains("3 of 8 texture slot(s)"), "{note}");
    }

    /// **The regression this rule exists for.** The driveable ribbon is generated
    /// geometry with no texture slots at all, on either disc, and the earlier
    /// "no textures" test reported it on the *PSP* disc - blaming a PS2 texture-set
    /// gap for a mesh that was never textured and never came off a `.vex`.
    #[test]
    fn a_model_that_declares_no_slots_wanted_none_and_is_not_reported() {
        assert_eq!(untextured_note(&model(0, 0)), None);
    }

    /// The inverse table must really be the inverse, or the headless capture and the
    /// window disagree about what a key means and only one of them is ever tested.
    #[test]
    fn every_key_in_the_inverse_table_maps_back_to_its_button() {
        for index in 0..32u8 {
            let Some(key) = key_for_button(index) else {
                continue;
            };
            assert_eq!(
                map_key(&key),
                Some(index),
                "button {index} maps to {key:?}, which maps back to something else"
            );
        }
    }

    /// The buttons a race needs must all be reachable with no window.
    #[test]
    fn thrust_steering_and_both_airbrakes_all_have_a_key() {
        for wanted in [
            button::CROSS,
            button::LEFT,
            button::RIGHT,
            button::UP,
            button::DOWN,
            button::L,
            button::R,
        ] {
            assert!(key_for_button(wanted).is_some(), "no key for {wanted}");
        }
    }

    #[test]
    fn a_held_cross_becomes_thrust() {
        let mut held = HeldButtons::new(1 << button::CROSS);
        let controls = ship_controls(&held.snapshot());
        assert_eq!(controls.thrust, 1.0);
        assert_eq!(controls.steer_x, 0.0);
    }

    #[test]
    fn holding_left_steers_left() {
        let mut held = HeldButtons::new(1 << button::LEFT);
        assert_eq!(ship_controls(&held.snapshot()).steer_x, -1.0);
    }

    /// A mask naming a button no key produces must not panic, and must not leak into
    /// the controls either.
    #[test]
    fn a_button_with_no_key_is_ignored() {
        let mut held = HeldButtons::new(1 << button::START);
        assert_eq!(
            ship_controls(&held.snapshot()),
            oag_physics::ShipControls::default()
        );
    }

    /// The uniform block is `oag-render`'s, not ours. If that side grows a field
    /// this catches it before the picture goes quietly wrong.
    #[test]
    fn the_uniform_block_matches_the_shaders() {
        assert_eq!(
            std::mem::size_of::<Uniforms>() as u64,
            mesh_render::UNIFORMS_SIZE
        );
    }

    /// A straight synthetic path, so the locator and the spawn can be tested with no
    /// disc image. Eight control points along `+x`, twelve units wide either side.
    fn straight_track() -> AiTrack {
        let count = 8usize;
        let paths_at = track::HEADER_LEN + track::RESERVED_LEN;
        let junctions_at = paths_at + track::PATH_LEN;
        let points_at = junctions_at + track::JUNCTION_LEN;
        let mut out = vec![0u8; points_at + count * track::POINT_LEN];
        out[0..4].copy_from_slice(&track::MAGIC.to_le_bytes());
        out[4..8].copy_from_slice(&0x105u32.to_le_bytes());
        out[8..12].copy_from_slice(&1u32.to_le_bytes());
        out[12..16].copy_from_slice(&1u32.to_le_bytes());
        out[paths_at..paths_at + 4].copy_from_slice(&(count as u32).to_le_bytes());
        for k in 0..count {
            let at = points_at + k * track::POINT_LEN;
            let mut put = |off: usize, v: f32| {
                out[at + off..at + off + 4].copy_from_slice(&v.to_le_bytes());
            };
            // Position along +x, tangent +x, down -y, lateral +z.
            put(0x00, k as f32 * 10.0);
            put(0x10, 1.0);
            put(0x24, -1.0);
            put(0x38, 1.0);
            put(0x44, 12.0);
            put(0x48, 12.0);
        }
        track::parse(&out).expect("the synthetic track must parse")
    }

    fn setup(handling: Handling) -> Setup {
        let ai = straight_track();
        let spline = Spline::from_track(&ai);
        Setup {
            ai,
            spline,
            // The synthetic track has no authored slot, which is the fallback
            // path: every assertion below is about a ship placed on the spline.
            start_position: None,
            collision: CollisionWorld::new(),
            handling,
            // Round numbers, chosen to make the geometry readable. None of these
            // claims to be the game's; a real race reads all seven off the disc.
            chase: ChaseParams {
                fov: 60.0,
                lookat_height: 1.0,
                lookat_length: 10.0,
                pos_height: 2.0,
                pos_length: 8.0,
                spring_horiz: 4.0,
                spring_vert: 2.0,
            },
            // A synthetic setup has no ship model, so no locator either. The
            // exhaust still ticks; it just has nowhere to be drawn, which is the
            // same path a model with no `Engine Flare` node takes.
            nozzle: None,
        }
    }

    /// The field-of-view setting has to reach the matrix, and its default has
    /// to leave that matrix exactly as it was before the setting existed - the
    /// ground-truth captures under `data/traces/` were taken against it.
    #[test]
    fn the_field_of_view_setting_widens_the_projection_and_defaults_to_the_authored_one() {
        use crate::display::Fov;

        let race = Race::start(setup(Handling::default()));
        let aspect = AUTHORED_ASPECT;
        let authored = race.projection(aspect, 1000.0, Fov::AUTHORED);

        // The projection's first column scales x by `1 / (tan(fov/2) * aspect)`,
        // so a wider field is a *smaller* number there. Read off the matrix
        // rather than recomputed, which would only restate `Fov::apply`.
        let x_scale = |m: Mat4| m.col(0).x;
        let wide: Fov = "150".parse().expect("parse");
        let narrow: Fov = "75".parse().expect("parse");
        assert!(
            x_scale(race.projection(aspect, 1000.0, wide)) < x_scale(authored),
            "150 % has to show more"
        );
        assert!(
            x_scale(race.projection(aspect, 1000.0, narrow)) > x_scale(authored),
            "75 % has to show less"
        );

        // And the untouched setting is the matrix the fit alone produces.
        let unchanged = oag_render::camera::projection(
            oag_render::camera::fit_vertical_fov(
                race.chase_params.fov.to_radians(),
                AUTHORED_ASPECT,
                aspect,
            ),
            aspect,
            1.0,
            1000.0,
        );
        assert_eq!(authored, unchanged);
    }

    #[test]
    fn the_locator_finds_the_nearest_sample() {
        let spline = Spline::from_track(&straight_track());
        assert!(!spline.is_empty());
        assert_eq!(spline.len(), 8 * Spline::STEPS_PER_SEGMENT);
        assert_eq!(spline.path_of(0), Some(0));

        let start = *spline.start().expect("a first sample");
        let at = Vec3::from_array(start.pos);
        assert!(spline.distance_to(at).expect("a distance") < 1e-3);

        // Straight up from the first sample is still nearest that sample.
        let (index, _, distance) = spline.nearest(at + Vec3::Y * 5.0).expect("a sample");
        assert_eq!(index, 0);
        assert!((distance - 5.0).abs() < 1e-3, "{distance}");
    }

    #[test]
    fn the_widest_half_width_comes_from_the_track() {
        let spline = Spline::from_track(&straight_track());
        assert!((spline.max_half_width() - 12.0).abs() < 1e-3);
    }

    /// A local search finds what a whole-table search would, when the answer is
    /// inside the window.
    #[test]
    fn a_local_search_agrees_with_a_whole_table_one_inside_its_window() {
        let spline = Spline::from_track(&straight_track());
        let target = Vec3::from_array(spline.sample(20).expect("a sample").pos);
        let (whole, _, _) = spline.nearest(target).expect("a sample");
        let (local, _, distance) = spline
            .nearest_within(target, 20, 8)
            .expect("a sample in the window");
        assert_eq!(local, whole);
        assert!(distance < 1e-3, "{distance}");
    }

    /// And when the answer is outside the window it returns something too far
    /// away, which the caller turns into "draw everything" rather than into a
    /// confident wrong section. See `Spline::nearest_within`.
    #[test]
    fn a_local_search_that_misses_reports_a_distance_the_caller_will_reject() {
        let spline = Spline::from_track(&straight_track());
        let far_end = Vec3::from_array(spline.sample(30).expect("a sample").pos);
        let (index, _, distance) = spline
            .nearest_within(far_end, 0, 2)
            .expect("a sample in the window");
        assert!(index <= 2, "the search stayed inside its window");
        assert!(
            distance > 12.0 * OFF_TRACK_HALF_WIDTHS,
            "a miss must be far enough to be rejected, was {distance}"
        );
    }

    /// **The conservative path, which nothing else exercises.** A craft on the
    /// racing line names its section; one that has left the track names
    /// nothing, so the visible set becomes everything.
    #[test]
    fn a_position_off_the_track_refuses_to_name_a_section() {
        let race = Race::start(setup(Handling::default()));
        let spline = race.spline();
        let sample = *spline.sample(4).expect("a sample");
        let on_line = Vec3::from_array(sample.pos);

        assert_eq!(
            race.section_of(4, on_line),
            sample.section_id,
            "a craft on the racing line names its own section"
        );

        // The synthetic track is 12 units wide either side, so the gate is at
        // 36. Checked either side of it rather than at one distance, so the
        // test would fail if the threshold were dropped entirely.
        let half_width = 12.0 * OFF_TRACK_HALF_WIDTHS;
        let lateral = Vec3::from_array(sample.lateral);
        assert_eq!(
            race.section_of(4, on_line + lateral * (half_width - 1.0)),
            sample.section_id,
            "still on the track at just under the gate"
        );
        assert_eq!(
            race.section_of(4, on_line + lateral * (half_width + 1.0)),
            UNPLACED,
            "just past the gate, nothing may be culled on this position's word"
        );
        assert_eq!(
            race.section_of(4, on_line + Vec3::Y * 200.0),
            UNPLACED,
            "far above the track - airborne over a gap, or fallen through"
        );
        assert_eq!(
            race.section_of(9_999, on_line),
            UNPLACED,
            "a sample index the table does not have"
        );
    }

    /// A respawn in flight teleports the craft and leaves the camera spring
    /// catching up, so neither position describes the shot.
    #[test]
    fn a_respawn_in_flight_makes_everything_visible() {
        let mut race = Race::start(setup(Handling::default()));
        race.respawn_cooldown = 1;
        assert_eq!(race.visibility_sections(), (UNPLACED, UNPLACED));

        // And the resulting set really is everything, not merely two unknown
        // ids - this is the property the whole conservative path exists for.
        let pvs = oag_formats::pvs::TrackPvs::empty();
        let padding = SectionPadding::default();
        let (craft, camera) = race.visibility_sections();
        assert!(VisibleSet::around(&pvs, &padding, craft, camera).is_everything());
    }

    /// A parameter set with a real hull, so the reset probes have something to
    /// probe with, and no gravity, so a ship only moves when a test moves it.
    fn hulled_handling() -> Handling {
        Handling {
            physical: oag_physics::params::Physical {
                mass: 1.0,
                ..Default::default()
            },
            antigrav: oag_physics::params::Antigrav {
                ride_height: 8.0,
                ..Default::default()
            },
            dimensions: oag_physics::params::Dimensions {
                width: 2.0,
                height: 1.0,
                length: 4.0,
                ..Default::default()
            },
            ..Handling::ZERO
        }
    }

    /// A large quad, as a collider of one class.
    ///
    /// `axis` picks the plane: 1 is horizontal at height `at`, 0 is vertical at
    /// `x = at`.
    fn plane(
        axis: usize,
        at: f32,
        surface: oag_physics::Surface,
        collider: u32,
    ) -> oag_physics::TriangleSoup {
        let corner = |u: f32, v: f32| {
            let mut p = [0.0f32; 3];
            p[axis] = at;
            let others: Vec<usize> = (0..3).filter(|&k| k != axis).collect();
            p[others[0]] = u;
            p[others[1]] = v;
            p
        };
        oag_physics::TriangleSoup::new(
            vec![
                corner(-500.0, -500.0),
                corner(500.0, -500.0),
                corner(500.0, 500.0),
                corner(-500.0, 500.0),
            ],
            vec![[0, 1, 2], [0, 2, 3]],
            Vec::new(),
            surface,
            collider,
        )
    }

    fn setup_with(handling: Handling, colliders: Vec<oag_physics::TriangleSoup>) -> Setup {
        let mut setup = setup(handling);
        for c in colliders {
            setup.collision.push(c);
        }
        setup
    }

    /// The feature, end to end: a ship that leaves the track and crosses `Reset`
    /// geometry is put back on the spline.
    ///
    /// The reset plane is **horizontal and below**, which is where a real one
    /// sits, and the hull probes are horizontal - so only the swept ray can find
    /// it. That is deliberate: it is the path a falling ship actually takes.
    #[test]
    fn a_ship_that_falls_through_a_reset_plane_is_put_back_on_the_track() {
        let handling = hulled_handling();
        let setup = setup_with(
            handling,
            vec![plane(1, -40.0, oag_physics::Surface::Reset, 0)],
        );
        let mut race = Race::start(setup);
        assert_eq!(race.respawns(), 0);

        // Throw it off the track. No gravity in this fixture, so nothing else can
        // move it and the only thing under test is the reset path.
        {
            let body = &mut race.world.ships[0].physics.body;
            body.position = Vec3::new(20.0, -20.0, 0.0);
            body.linear_velocity = Vec3::new(0.0, -600.0, 0.0);
        }

        for _ in 0..10 {
            race.tick(&InputSnapshot::default());
        }

        assert_eq!(race.respawns(), 1, "the reset plane never triggered");

        let body = race.world.ships[0].physics.body;
        // Back on the track: near a spline sample, above it, and stopped.
        let distance = race.spline().distance_to(body.position).expect("a sample");
        assert!(distance < 20.0, "respawned {distance} from the spline");
        assert!(body.position.y > 0.0, "respawned at {:?}", body.position);
        assert_eq!(body.linear_velocity, Vec3::ZERO);
    }

    /// Nothing but `Reset` geometry may respawn a ship. The same plane in the same
    /// place under a different class must leave the ship where it fell.
    #[test]
    fn falling_through_any_other_class_does_not_respawn() {
        for surface in [
            oag_physics::Surface::Wall,
            oag_physics::Surface::Floor,
            oag_physics::Surface::MagFloor,
        ] {
            let setup = setup_with(hulled_handling(), vec![plane(1, -40.0, surface, 0)]);
            let mut race = Race::start(setup);
            {
                let body = &mut race.world.ships[0].physics.body;
                body.position = Vec3::new(20.0, -20.0, 0.0);
                body.linear_velocity = Vec3::new(0.0, -600.0, 0.0);
            }
            for _ in 0..10 {
                race.tick(&InputSnapshot::default());
            }
            assert_eq!(race.respawns(), 0, "{surface:?} respawned the ship");
        }
    }

    /// **The firehose trap.** A ship held against a wall for many ticks must
    /// spawn one spark burst, on the impact's rising edge, not one burst per
    /// tick of the ensuing scrape - the shape of bug this guards against is
    /// the same one `oag_physics::wall::STUN_PER_CONTACT`'s doc comment
    /// records this crate cost a session of play-testing to, for the
    /// collision stun rather than sparks.
    ///
    /// The plane and approach are copied from
    /// [`falling_through_any_other_class_does_not_respawn`], a known-working
    /// `Surface::Wall` fixture, rather than a fresh vertical wall: getting a
    /// triangle's winding backwards produces a silent "no contact" rather
    /// than a loud failure, and this fixture is already proven to register.
    #[test]
    fn a_sustained_scrape_spawns_sparks_once_not_every_tick() {
        let handling = hulled_handling();
        let setup = setup_with(
            handling,
            vec![plane(1, -40.0, oag_physics::Surface::Wall, 0)],
        );
        let mut race = Race::start(setup);

        // Reset to an inbound approach before every tick, so each one sees a
        // fresh impact rather than the ship bouncing away after the first.
        let push_toward_wall = |race: &mut Race| {
            let body = &mut race.world.ships[0].physics.body;
            body.position = Vec3::new(20.0, -39.7, 0.0);
            body.linear_velocity = Vec3::new(0.0, -50.0, 0.0);
        };

        push_toward_wall(&mut race);
        let evaluated = race.tick(&InputSnapshot::default());
        assert!(
            evaluated.wall.impact,
            "the fixture never reaches the wall - not what this test means to check"
        );
        let after_first_impact = race.sparks().alive_count();
        assert!(after_first_impact > 0, "the first impact spawned no sparks");

        for tick in 0..10 {
            push_toward_wall(&mut race);
            race.tick(&InputSnapshot::default());
            assert_eq!(
                race.sparks().alive_count(),
                after_first_impact,
                "tick {tick} of the same scrape spawned another burst"
            );
        }
    }

    /// The guard against the failure that would otherwise look like a hang: a
    /// recovery pose that is itself inside a reset volume respawns forever.
    ///
    /// Here the trigger is a vertical plane half a unit from the spawn, inside the
    /// hull's own half-width, so every recovery lands straight back in it.
    /// Respawning must stop rather than freeze the race.
    #[test]
    fn a_recovery_pose_inside_a_reset_volume_gives_up_instead_of_looping() {
        let handling = hulled_handling();
        let start = *setup(handling).spline.start().expect("a first sample");
        // Half a unit to the side of wherever the ship is placed, well inside the
        // one-unit hull half-width.
        let beside = Vec3::from_array(start.pos).x + 0.5;
        let setup = setup_with(
            handling,
            vec![plane(0, beside, oag_physics::Surface::Reset, 0)],
        );

        let mut race = Race::start(setup);
        for _ in 0..(RESPAWN_COOLDOWN_TICKS * (RESPAWN_GIVE_UP + 3)) {
            race.tick(&InputSnapshot::default());
        }

        assert_eq!(
            race.respawns(),
            RESPAWN_GIVE_UP,
            "respawning did not stop after {RESPAWN_GIVE_UP} tries"
        );
    }

    /// The cooldown is what stops one contact from respawning on every tick while
    /// the ship is still overlapping the trigger.
    #[test]
    fn a_respawn_is_not_repeated_on_the_very_next_tick() {
        let handling = hulled_handling();
        let start = *setup(handling).spline.start().expect("a first sample");
        let beside = Vec3::from_array(start.pos).x + 0.5;
        let setup = setup_with(
            handling,
            vec![plane(0, beside, oag_physics::Surface::Reset, 0)],
        );

        let mut race = Race::start(setup);
        race.tick(&InputSnapshot::default());
        assert_eq!(race.respawns(), 1);
        for _ in 0..(RESPAWN_COOLDOWN_TICKS - 1) {
            race.tick(&InputSnapshot::default());
            assert_eq!(race.respawns(), 1, "respawned again inside the cooldown");
        }
    }

    /// A ship must arrive on the track with its mass in the body, or the hover
    /// spring and the integrator disagree about how heavy it is.
    #[test]
    fn a_started_race_puts_one_ship_on_the_spline_with_its_mass_set() {
        let handling = Handling {
            physical: oag_physics::params::Physical {
                mass: 7.5,
                ..Default::default()
            },
            antigrav: oag_physics::params::Antigrav {
                // Chosen so the resulting spawn height cannot coincide with
                // `track::HOVER_LIFT`, which the assertion below distinguishes it from.
                ride_height: 8.0,
                ..Default::default()
            },
            pitch: oag_physics::params::Pitch {
                antigrav_height_adjust: 0.5,
                ..Default::default()
            },
            ..Handling::ZERO
        };
        let setup = setup(handling);
        let start = *setup.spline.start().expect("a first sample");
        let race = Race::start(setup);

        assert_eq!(race.world.ship_count, 1);
        assert!(race.ship().active);
        assert_eq!(race.ship().physics.body.mass, 7.5);
        assert_eq!(race.ship().handling.physical.mass, 7.5);
        assert_eq!(race.dt(), 1.0 / 60.0);

        // Lifted off the surface line by the height its own suspension holds it at,
        // which is neither `track::HOVER_LIFT` nor the probe's full reach; see
        // `spawn_height`. This fixture has no gravity, so the sag is zero and the rest
        // height is the target itself: `ride_height` scaled by the measured
        // `TARGET_GLOBAL_SCALE`, with the `antigrav_height_adjust` of 0.5 contributing
        // nothing because `craft+0x74` reads zero in the running game.
        assert_eq!(
            spawn_height(&handling),
            8.0 * oag_physics::hover::TARGET_GLOBAL_SCALE
        );
        assert_ne!(spawn_height(&handling), track::HOVER_LIFT);
        // Well inside the probes' reach, which is what gives the suspension travel.
        assert!(spawn_height(&handling) < handling.antigrav.ride_height);
        let up = (-Vec3::from_array(start.down)).normalize();
        let offset = race.ship().physics.body.position - Vec3::from_array(start.pos);
        assert!(
            (offset.dot(up) - spawn_height(&handling)).abs() < 1e-4,
            "{offset}"
        );
    }

    /// The spawn height is the spring's rest height plus the probes' own drop,
    /// not the spring's target and not the probe's reach.
    ///
    /// Round numbers chosen so the sag is checkable by hand, and **not** a ship's:
    /// two probes give a gradient of `2 * 0.3 * HOVER_K * (normal_gravity +
    /// track_gravity)`, and a grounded craft carries `normal_gravity +
    /// track_gravity` (gravity plus `oag_physics::hover::DOWNFORCE_SCALE`'s
    /// downforce), so the sag is exactly `1.25` whatever the two gravities are.
    /// The centre of mass then sits a probe drop above that. `mass` is
    /// deliberately not 1, to pin that it cancels.
    #[test]
    fn a_ship_spawns_at_the_height_its_own_suspension_holds_it_at() {
        let handling = Handling {
            physical: oag_physics::params::Physical {
                mass: 3.0,
                normal_gravity: 5.0,
                track_gravity: 80.0,
                flight_gravity: 95.0,
            },
            antigrav: oag_physics::params::Antigrav {
                ride_height: 5.5,
                ..Default::default()
            },
            ..Handling::ZERO
        };

        let gradient = 2.0 * 0.3 * oag_physics::hover::HOVER_K * 85.0;
        let target = 5.5 * oag_physics::hover::TARGET_GLOBAL_SCALE;
        let drop = oag_physics::hover::PROBE_DROP_RAW * oag_physics::hover::TARGET_GLOBAL_SCALE;
        let expected = target - 85.0 / gradient + drop;
        assert!(
            (spawn_height(&handling) - expected).abs() < 1e-5,
            "{} was not {expected}",
            spawn_height(&handling)
        );

        // Real travel in **both** directions, which is the property that matters
        // and the one the old geometry could not have. A probe reaches exactly as
        // far as the target (`oag_physics::hover::probe`), so its travel is the
        // whole `target`: the resting probe sits `1.25` up from full compression
        // and `1.25` short of losing the ground. That symmetry is the recovered
        // model's, not a tuning: both numbers are the same sag.
        let probe_rest = spawn_height(&handling) - drop;
        assert!(probe_rest > 0.0 && probe_rest < target);
        assert!(
            (probe_rest - (target - 1.25)).abs() < 1e-5,
            "the probe rests at {probe_rest}, not 1.25 below its {target} target"
        );

        // And mass really does cancel.
        let heavier = Handling {
            physical: oag_physics::params::Physical {
                mass: 50.0,
                ..handling.physical
            },
            ..handling
        };
        assert_eq!(spawn_height(&heavier), spawn_height(&handling));
    }

    /// The tensor is the recovered box and does **not** depend on the ship.
    ///
    /// This test used to assert the opposite: that the tensor was built from
    /// `<Misc width/height/length>` at `<Physical mass>`, with roll the cheapest
    /// axis because the hull is long. Both halves are now refuted -
    /// `Body_SetBoxInertia`'s single call site passes the code literal
    /// `(12, 8, 12)` at a mass of `0.9`, the hull dimensions go to the collider
    /// instead, and the box is square in plan so **pitch and roll are equal** and
    /// yaw is the odd axis out. Kept as a test rather than deleted because "the
    /// inertia varies per craft" is exactly the assumption that would come back.
    #[test]
    fn the_inertia_tensor_is_the_recovered_box_and_is_the_same_for_every_ship() {
        let inertia = box_inertia();

        assert!((inertia.x - 15.6).abs() < 0.01, "pitch was {}", inertia.x);
        assert!((inertia.y - 21.6).abs() < 0.01, "yaw was {}", inertia.y);
        assert!((inertia.z - 15.6).abs() < 0.01, "roll was {}", inertia.z);

        // Square in plan, so the two attitude axes cost the same and yaw is the
        // hardest. A hull-derived tensor could not produce this.
        assert_eq!(inertia.x, inertia.z);
        assert!(inertia.y > inertia.x);
        assert_ne!(inertia, Vec3::ONE);
    }

    /// A ship at rest on flat ground stays at rest.
    ///
    /// This is the assertion whose absence let a ship fall through the floor with no
    /// input held: on the shipped parameters the placeholder inertia tensor turned
    /// single-probe contact into a 38 rad/s pitch oscillator that explicit Euler grew
    /// about 5 % a tick, and the ship inverted and left the world by tick 250 while the
    /// player was not touching the controls. Reverting either the tensor or the spawn
    /// height fails this within two seconds of simulated time.
    ///
    /// A flat floor and no track, deliberately: the same failure reproduced with no
    /// spline, no collision soup and no camera, so this pins the force law rather than
    /// the composition. Ten seconds is long enough that 5 %-per-tick growth would be
    /// astronomically visible.
    #[test]
    fn a_ship_at_rest_on_flat_ground_stays_at_rest() {
        use oag_physics::{Body, ShipControls, ShipState, Surface, TriangleSoup};

        // The shape of the observed parameters. Round numbers, and not a ship's.
        let handling = Handling {
            physical: oag_physics::params::Physical {
                mass: 1.0,
                normal_gravity: 5.0,
                track_gravity: 80.0,
                flight_gravity: 95.0,
            },
            antigrav: oag_physics::params::Antigrav {
                ride_height: 5.5,
                rebound: 0.6,
                landing_rebound: 0.5,
                ..Default::default()
            },
            dimensions: oag_physics::params::Dimensions {
                width: 5.0,
                height: 3.5,
                length: 13.0,
                ..Default::default()
            },
            pitch: oag_physics::params::Pitch {
                pitch_damping: 3.0,
                // Set, and required not to matter: see `hover::target_height`.
                antigrav_height_adjust: 1.0,
                ..Default::default()
            },
            ..Handling::ZERO
        };

        let mut floor = CollisionWorld::new();
        floor.push(TriangleSoup::new(
            vec![
                [-5000.0, 0.0, -5000.0],
                [-5000.0, 0.0, 5000.0],
                [5000.0, 0.0, 5000.0],
                [5000.0, 0.0, -5000.0],
            ],
            vec![[0, 1, 2], [0, 2, 3]],
            Vec::new(),
            Surface::Floor,
            0,
        ));

        let start = spawn_height(&handling);
        let mut state = ShipState {
            body: Body {
                position: Vec3::new(0.0, start, 0.0),
                mass: handling.physical.mass,
                inertia: box_inertia(),
                ..Body::default()
            },
            grounded: 1.0,
            ..ShipState::default()
        };

        let mut worst_tilt = 0.0f32;
        let mut airborne = 0u32;
        for _ in 0..600 {
            let evaluated = oag_physics::step(
                &mut state,
                &ShipControls::default(),
                &handling,
                &Environment::default(),
                &floor,
                1.0 / 60.0,
            );
            if evaluated.hover.contacts == 0 {
                airborne += 1;
            }
            let tilt = state.body.up().dot(Vec3::Y).clamp(-1.0, 1.0).acos();
            worst_tilt = worst_tilt.max(tilt);
        }

        assert_eq!(
            airborne, 0,
            "the ship left the ground on {airborne} tick(s)"
        );
        assert!(
            worst_tilt < 0.01,
            "the ship tilted to {worst_tilt} rad with no input held"
        );
        assert!(
            (state.body.position.y - start).abs() < 0.5,
            "the ship drifted from {start} to {} with no input held",
            state.body.position.y
        );
        assert!(
            state.body.linear_velocity.length() < 1.0,
            "the ship was moving at {} with no input held",
            state.body.linear_velocity.length()
        );
    }

    /// With no collision geometry there is nothing to hover on, so the ship must
    /// fall - and must fall *finitely*. The cheapest possible check that the force
    /// law is really being driven from here.
    #[test]
    fn a_ship_over_nothing_falls_and_stays_finite() {
        let handling = Handling {
            physical: oag_physics::params::Physical {
                mass: 1.0,
                flight_gravity: 10.0,
                normal_gravity: 10.0,
                track_gravity: 10.0,
            },
            ..Handling::ZERO
        };
        let mut race = Race::start(setup(handling));
        let start = race.ship().physics.body.position;

        let mut held = HeldButtons::new(0);
        for _ in 0..60 {
            let snapshot = held.snapshot();
            race.tick(&snapshot);
        }

        let now = race.ship().physics.body.position;
        assert!(now.is_finite(), "{now}");
        assert!(now.y < start.y, "nothing to hover on, so it must fall");
        assert_eq!(race.ship().physics.grounded, 0.0);
        assert_eq!(race.world.tick, 60);
        assert!(race.telemetry().spline_distance.is_finite());
    }
}
