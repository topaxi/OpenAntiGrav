//! [`RaceSim`]: the half of a race that decides what happens.
//!
//! What [`RaceView`](super::RaceView) left behind, and the point of leaving it:
//! the world, the collision broadphase, the track's spline and racing line, the
//! opponents' drivers, the pad broadphases, the recovery dwells and the weapon
//! table. **No field here is an `oag_render` type**, and the boundary is now the
//! type rather than a convention, so a renderer handle cannot be added to the
//! simulation by accident the way it could when all ninety-seven sat in one
//! struct.
//!
//! # What it is not yet
//!
//! This is not a crate, and two fields are why. `cues` and its two companions
//! are `oag_sound::sfx` types - plain data (a `Cue` and a slot index), but
//! declared inside a module that reaches `oag_audio`, so they would have to move
//! down first. Everything else is `oag_gameplay`, `oag_physics`, `oag_ai`,
//! `oag_race`, `oag_tables`, `oag_vex`, `oag_title` or a primitive, and would
//! travel as-is.
//!
//! # The hash
//!
//! [`RaceSim::state_hash`] lives here rather than on `Race` for the reason the
//! split exists: a hash over the simulation should not be *able* to reach the
//! camera. See [`hash`](super::hash).

use super::*;

/// The half of a race that decides what happens next.
///
/// Reached as `Race::sim`. Holds no renderer, audio or windowing type - see the
/// module doc comment for the two that stand between this and a crate of its
/// own.
///
/// # Why it is `Clone`
///
/// A reconciliation restores a whole simulation, not a selection from one.
/// [`Race::snapshot_sim`] and [`Race::restore_sim`] are that pair, and
/// `race/tests/reconcile.rs` is what they were added for.
///
/// **Whole-struct `Clone` rather than a hand-written snapshot type,
/// deliberately.** [`Self::state_hash`] is not an oracle for whether a
/// snapshot is complete: [`Self::rolls_armed`], [`Self::rolls_spent`],
/// [`Self::wall_contact_ticks`], [`Self::wall_damage`],
/// [`Self::contact_cue_cooldown`] and the three per-tick output accumulators
/// are all outside it, each for its own recorded reason. A snapshot type that
/// forgot one of them would pass a hash-equality test anyway, and the field it
/// dropped would surface as a diverging race thousands of ticks later. `Clone`
/// cannot forget a field, so the completeness is structural rather than
/// asserted.
///
/// It copies the track with the race - the collision soup, the spline, the
/// racing line, both pad lists - none of which any tick mutates. That is
/// wasteful for a snapshot taken every tick and it is not what this is
/// measured on yet; the handover thread records the split as the optimization
/// to reach for when a reconcile budget exists.
#[derive(Debug, Clone)]
pub struct RaceSim {
    /// The simulation state.
    pub world: World,
    /// The finished player's thrust table and skill; `None` off the four
    /// Pulse classes. See [`super::finished_thrust`].
    pub(super) finished_thrust: Option<super::finished_thrust::FinishedThrust>,
    pub(super) collision: CollisionWorld,
    pub(super) spline: Spline,
    /// The authored racing line, as the opponents' drivers want it.
    ///
    /// Built once from [`Self::spline`] and **index-parallel to [`Self::ai_order`]**,
    /// so a driver's place on the line is also its place in that table and no
    /// craft pays for two searches. Track data rather than world state, which is
    /// why it is here and the drivers themselves are on the ships. See
    /// [`racing_line`] and `docs/gameplay/ai.md`.
    pub(super) racing_line: oag_ai::Line,
    /// Which spline sample each racing-line index is, in lap order.
    ///
    /// The identity permutation on nine of the disc's twelve circuits. See
    /// [`ai_order`], and [`Race::ai_sample`] for the lookup itself.
    pub(super) ai_order: Vec<u32>,
    /// One AI line per way round a fork, beside the ring's - see
    /// [`super::routes`]. Empty on a circuit with no fork.
    pub(super) routes: Vec<super::routes::RouteLine>,
    /// The speed plan the opponents' drivers follow, built at the start from
    /// this line, this collision and the field's handling, or `None` when the
    /// race has no opponents or the plan did not verify clean. See
    /// [`Race::field_speed_plan`] and `oag_ai::plan`.
    pub(super) speed_plan: Option<oag_ai::SpeedPlan>,
    /// What the opponents' drivers are flown with. One set for the whole field:
    /// per-craft variation is the skill work, and this is the basic driver.
    pub(super) ai_tuning: oag_ai::Tuning,
    /// Which pilot each grid slot is flying.
    ///
    /// Not world state: it is read-only for the whole race and drawn from the
    /// race seed, so it reproduces without being carried. Slot 0 is the
    /// player's, and is read only while [`Race::flown_for_the_player`] is -
    /// by [`Race::set_ai_pilot`] (`--autopilot-pilot`) or the Autopilot
    /// pickup - otherwise inert.
    pub(super) ai_pilots: [oag_ai::Pilot; oag_gameplay::MAX_SHIPS],
    /// [`Race::autopilot_controls`]'s own tuning, for `--autopilot-skill`.
    ///
    /// `None` outside that flag - every ordinary race, and every existing
    /// `Race::set_autopilot(true)` call - which is what lets
    /// [`Race::autopilot_controls`] fall back to [`Self::ai_tuning`], the
    /// field's own. See [`Race::set_autopilot_tuning`] for why this is a
    /// second field rather than a call to [`Race::set_ai_tuning`].
    pub(super) autopilot_tuning: Option<oag_ai::Tuning>,
    /// The lap counter's ring, or `None` on a track whose chain does not close.
    pub(super) course: Option<Course>,
    /// Zone mode's three numbers, off the disc. `None` outside Zone mode, and on
    /// a source whose `handlingstats.xml` carries no `<Global><Zone/>`.
    pub(super) zone: Option<oag_tables::handling::Zone>,
    pub(super) dt: f32,
    /// Ticks left before a `Reset` contact can respawn each craft again.
    ///
    /// See [`RESPAWN_COOLDOWN_TICKS`].
    ///
    /// **Per slot, and all four of these are, which is not tidiness.** One
    /// shared set of counters would let an opponent stuck in a corner exhaust
    /// [`RESPAWN_GIVE_UP`] and switch off the *player's* recovery, and a
    /// cooldown armed by one craft would strand another that fell off in the
    /// same half second. Indexed by ship slot, like [`RaceView::exhaust`].
    pub(super) respawn_cooldown: [u32; oag_gameplay::MAX_SHIPS],
    /// How many respawns each craft has had back to back, for
    /// [`RESPAWN_GIVE_UP`].
    pub(super) respawns_in_a_row: [u32; oag_gameplay::MAX_SHIPS],
    /// Set once respawning has given up on a craft, so the complaint is printed
    /// once.
    pub(super) respawn_disabled: [bool; oag_gameplay::MAX_SHIPS],
    /// How many times each craft has been respawned this race, for tests and
    /// for the load report.
    pub(super) respawns: [u32; oag_gameplay::MAX_SHIPS],
    /// Which trigger fired the most recent respawn of each craft, `None` before
    /// the first. Bookkeeping outside `state_hash`, read by survey tests through
    /// [`Race::last_respawn_cause_of`].
    pub(super) last_respawn_cause: [Option<respawn::RespawnCause>; oag_gameplay::MAX_SHIPS],
    /// Whether this race runs with weapons - [`Setup::weapons_on`], resolved.
    /// Read through [`Self::damage_rules`] so contact and weapon damage agree
    /// with the pads the load kept.
    pub(super) weapons_on: bool,
    /// The kill count that ends an Eliminator event - see
    /// [`Setup::eliminator_kill_target`].
    pub(super) eliminator_kill_target: u32,
    /// The slot whose weapon last got through to each craft, `None` while
    /// nobody's has. Eliminator-only bookkeeping - see `crate::eliminator`.
    pub(super) last_damager: [Option<u8>; oag_gameplay::MAX_SHIPS],
    /// The tick, plus one, on which a weapon hit last got through to each craft,
    /// or `0` for never. It is what tells [`Self::last_damager`]'s credit apart
    /// from a death by wall: see `crate::eliminator`. Hashed.
    pub(super) last_weapon_hit: [u64; oag_gameplay::MAX_SHIPS],
    /// Seconds left before an Eliminated craft returns to the race, once it
    /// has reached that state - **the Eliminator only**, since a craft
    /// destroyed in any other mode stays down; see `crate::eliminator`. Hashed: a
    /// craft one tick out on its respawn is one tick out on everything after.
    pub(super) respawn_delay: [f32; oag_gameplay::MAX_SHIPS],
    /// Seconds until `cont_elim` is raised for each wrecked opponent, `0.0`
    /// while the craft is not down and `-1.0` once raised for this wreck.
    /// Bookkeeping for a cue, which is an output: outside the state hash. See
    /// `Race::tick_wreck_voice`.
    pub(super) wreck_voice: [f32; oag_gameplay::MAX_SHIPS],
    /// How many barrel rolls each craft has armed this race, and what they
    /// cost it.
    ///
    /// **Bookkeeping, not simulation state**, which is why it is here rather
    /// than on `Ship`: nothing reads it back into the race and it is outside
    /// `Race::state_hash` deliberately. It exists because the deviation our AI
    /// carries - opponents that barrel-roll, which the original's never do -
    /// has to be *measurable* on the disc's own circuits, and the thing that
    /// would say it had gone wrong is a tier rolling itself down to single-digit
    /// shield. See `crates/game/tests/ai_roll_ground_truth.rs`.
    pub(super) rolls_armed: [u32; oag_gameplay::MAX_SHIPS],
    /// The shield those rolls cost, in pool units.
    pub(super) rolls_spent: [f32; oag_gameplay::MAX_SHIPS],
    /// How many ticks each opponent has spent **in contact with a wall**.
    ///
    /// Kept here, and outside [`Race::state_hash`], for exactly the reason
    /// [`Self::rolls_armed`] is: bookkeeping, not state the race reads back. It
    /// exists because "an Ace should lap without wall contact" is the standard
    /// the AI is measured against and nothing in the harness could see wall
    /// contact at all - see `crates/game/tests/ai_clean_lap_board.rs`.
    ///
    /// **A contact counted here is a contact with a non-hoverable surface by
    /// construction**, which is what makes this a wall count rather than a
    /// count of everything the hull touches. `oag_physics::wall::responds`
    /// excludes `Floor` and `MagFloor` - the hover spring owns those - so a
    /// `WallResponse` contact can only be `Surface::Wall` or `Surface::Reset`.
    /// That is a surface-*tag* test, and it is strictly stronger than the
    /// `|n.up|` near zero geometric proxy a by-hand reading had to use.
    ///
    /// Counts `WallResponse::contacts`, **not**
    /// `oag_physics::ShipState::wall_contact_prev`. The latter is `impact`, an
    /// *inbound* test (`normal_speed < 0.0`), and a craft grinding along a wall
    /// stops being inbound long before it stops being in contact.
    /// [`Self::wall_inbound_ticks`] is that inbound subset, kept beside this so
    /// the two can be compared rather than confused.
    pub(super) wall_contact_ticks: [u32; oag_gameplay::MAX_SHIPS],
    /// The inbound subset of [`Self::wall_contact_ticks`]: ticks on which
    /// `WallResponse::impact` was set - a crash rather than a scrape.
    pub(super) wall_inbound_ticks: [u32; oag_gameplay::MAX_SHIPS],
    /// What those contacts **charged** each opponent, in shield-pool units.
    ///
    /// **Derived from the quantity physics charges, not differenced off the
    /// pool.** `oag_physics::damage::contact_damage(impulse_sum, rules)` is the
    /// whole of the wall's damage term, so summing it per tick gives the wall's
    /// share with barrel-roll charge, shield pads and weapons structurally
    /// excluded - the same argument [`Self::rolls_spent`] makes for its own
    /// figure, and the reason a shield delta could never separate them.
    ///
    /// `WallResponse::impulse_sum`'s own doc warns it is derived state a caller
    /// is told not to build on. That warning is about using it as *physics
    /// input*; this is the diagnostic use it is summed for, and
    /// `oag_physics::damage::wall` charges off the identical value.
    ///
    /// It is what the wall charged, which is not always what the pool lost: a
    /// craft at zero shield is charged the same and loses nothing, and
    /// `damage::subtract` clamps. Read it beside the end-of-run pool, never as
    /// a substitute for it.
    ///
    /// **[`Self::wall_racing_ticks`]'s gate stops a *wreck* accruing, not a
    /// live craft at zero shield**, and the two are different states: a craft
    /// that has emptied its pool but not yet been destroyed is still `Racing`,
    /// still driven, and still charged here for every wall it finds. That is
    /// why a row can read well over the pool - 162.56 against 95.00 on
    /// `01_Track` at PHANTOM - and why any total taken across rows should cap
    /// each row at its own `Dimensions::shield` first. Uncapped, the
    /// clean-Ace board's own totals overstate a tuning improvement by five
    /// percentage points.
    pub(super) wall_damage: [f32; oag_gameplay::MAX_SHIPS],
    /// How many ticks each opponent was **still racing** - the denominator the
    /// three counters above are measured over.
    ///
    /// A row that was wrecked at tick 4,000 has its walls counted over 4,000
    /// ticks and a row that finished has them counted over 18,000, and reading
    /// the two against each other without this is how a wreck resting on a wall
    /// reads as the worst driver on the board.
    pub(super) wall_racing_ticks: [u32; oag_gameplay::MAX_SHIPS],
    /// How many consecutive ticks each craft has spent away from the track.
    ///
    /// **The two halves of the grid measure "away" differently and share this
    /// counter.** An opponent's distance is to the sample its own driver believes
    /// it is on, over [`RESCUE_TICKS`]; the player's is the true distance to the
    /// nearest spline sample, over [`PLAYER_RESCUE_TICKS`]. Nobody steers the player's
    /// craft for them, so there is no believed index to compare against - see
    /// [`Race::lost_off_the_track`].
    pub(super) lost_ticks: [u32; oag_gameplay::MAX_SHIPS],
    /// How many consecutive ticks each opponent has spent stopped while asking to
    /// move. See [`STALL_TICKS`].
    ///
    /// A **second** dwell rather than a widening of [`Self::lost_ticks`], because
    /// the two measure different failures and share only their response: one
    /// craft has left the circuit, the other is still on it and going nowhere.
    /// Slot 0's entry is never written, for the reason [`Self::lost_ticks`] gives.
    pub(super) stalled_ticks: [u32; oag_gameplay::MAX_SHIPS],
    /// [`RESCUE_HALF_WIDTHS`] in track units, resolved once against this
    /// circuit's widest half-width rather than folded over the sample table
    /// every tick.
    pub(super) rescue_distance: f32,
    /// [`PLAYER_RESCUE_HALF_WIDTHS`] in track units, resolved once the same way
    /// [`Self::rescue_distance`] is.
    ///
    /// Zero on a track whose samples author no width at all, which switches the
    /// player's rescue off rather than making every position "off the track":
    /// the threshold is a scale read from the circuit, and a circuit that states
    /// no scale has not stated one.
    pub(super) player_rescue_distance: f32,
    /// The last spline sample the player was within [`Self::player_rescue_distance`]
    /// of, and therefore where [`Race::respawn`] puts them back.
    ///
    /// **Latched rather than reconstructed at the respawn.** By the time the
    /// dwell expires the craft is hundreds of units below the circuit, where the
    /// nearest sample can belong to a different part of it - recovering there
    /// would silently teleport the player across the lap counter.
    /// `docs/gameplay/ai.md` records the same trap on the opponents' side under
    /// "the rescue index, reconstructed backwards".
    ///
    /// Seeded at [`Race::start`] from where the craft is placed, not at zero, for
    /// the reason `Self::set_autopilot` gives about `Driver::index`: on a circuit
    /// whose grid sits at sample 2,791, zero is a different piece of track.
    pub(super) last_on_track: u32,
    /// This title's own zone-to-speed-class ladder, straight out of
    /// [`Setup::zone_stages`] - what [`RaceView::class_announcer`] fires against.
    pub(super) zone_stages: Option<&'static oag_title::ZoneStages>,
    /// Whether the race voices its start - see [`Setup::countdown_voice`].
    pub(super) countdown_voice: bool,
    /// The grid hover the craft carries - see [`Setup::launch_hover`].
    pub(super) launch_hover: Option<&'static oag_title::launch_hover::LaunchHover>,
    /// The per-class grounded-gravity scale - see [`Setup::class_gravity_scale`].
    pub(super) class_gravity_scale: f32,
    /// The launch boost's parameters - see [`Setup::start_boost`].
    pub(super) start_boost: Option<oag_physics::launch::StartBoost>,
    /// The track's speed-pad trigger volumes - see [`Setup::speedup_pads`].
    pub(super) speedup_pads: Vec<oag_vex::pads::PadVolume>,
    /// Distance from the ship to each pad, one entry per pad, in track units.
    ///
    /// The original's `pad+0x1d0`, reimplemented as a broadphase rather than as
    /// the latch `docs/formats/track.md` used to guess it was: `Pad_SweptTest`
    /// (`0x0888686c`) subtracts how far the craft moved from each entry every
    /// tick and only runs the real containment test on entries that reach zero,
    /// then stores the freshly measured distance back. A ship 900 units from a pad
    /// moving 2 units a tick is skipped for 450 ticks for the cost of one
    /// subtraction.
    ///
    /// **One row per racer**, as the original keeps: a broadphase cache is a
    /// statement about where *a* craft is, and eight craft sharing one row would
    /// skip pads for each other.
    pub(super) pad_distance: [Vec<f32>; MAX_SHIPS],
    /// Which pad the ship was inside last tick, the original's `craft+0x1d0`.
    ///
    /// `None` outside every pad. Only a *change* of value counts as entering a new
    /// pad, which is what gates the Zone score - standing still on one pad does
    /// not pay repeatedly. One per racer.
    pub(super) pad_current: [Option<usize>; MAX_SHIPS],
    /// The track's weapon-pad trigger volumes - see [`Setup::weapon_pads`].
    ///
    /// **Empty unless the mode arms them.** A weapons-off race in the original
    /// does not skip the trigger, it zeroes the list's own count
    /// (`World_CollectNodeLists`), and emptying this reproduces that at the same
    /// layer rather than adding a mode test to every tick.
    pub(super) weapon_pads: Vec<oag_vex::pads::PadVolume>,
    /// Distance from the ship to each weapon pad. The speed pads'
    /// [`Self::pad_distance`], one class over, and the same broadphase.
    pub(super) weapon_pad_distance: [Vec<f32>; MAX_SHIPS],
    /// Which weapon pad the ship was inside last tick.
    ///
    /// The edge this changes on is what grants a pickup, exactly as
    /// [`Self::pad_current`]'s edge is what pays the Zone score. Two pads
    /// overlapping on one tick is one entry. One per racer.
    pub(super) weapon_pad_current: [Option<usize>; MAX_SHIPS],
    /// Seconds each weapon pad has left before it can be triggered again.
    ///
    /// The original's `pad+0x1a0`, stamped by `WeaponPads_TestCraft` with
    /// `<WeaponPad refresh_time>` and counted back down by
    /// `WeaponPad_UpdateRefreshTimer` (`0x0892c034`) - see
    /// [`oag_tables::handling::WeaponPad`]. Per pad rather than per craft,
    /// which is what makes it a property of the track rather than of the racer.
    ///
    /// **Deliberately outside the determinism hash**, unlike
    /// `ShipState::turbo_timer`. It is genuine simulation state and a replay of
    /// a single race would need it; it is not hashed because the hash covers
    /// `ShipState` and the tick, and widening that is a change to the gate
    /// rather than a change to this feature. Recorded here so the gap is a known
    /// one - see `docs/gameplay/pickups.md`.
    pub(super) weapon_pad_refresh_left: Vec<f32>,
    /// How long a stamped weapon pad stays inert - see
    /// [`Setup::weapon_pad_refresh`].
    pub(super) weapon_pad_refresh: f32,
    /// Each weapon pad's nearest sample on [`Self::racing_line`] and its
    /// offset across it - see `field::pad_seek::line_positions`.
    pub(super) weapon_pad_line: Vec<Option<(u32, f32)>>,
    /// Which opponents steer for a weapon pad - see [`super::PadSeeking`].
    pub(super) pad_seeking: super::PadSeeking,
    /// Which rule fires an opponent's forward weapon - see [`super::FireLaw`].
    pub(super) fire_law: super::FireLaw,
    /// The odds the original's fire law reads - see [`Setup::weapon_ai`].
    pub(super) weapon_ai: Option<oag_tables::weapons::ai::WeaponAiStats>,
    /// This race's weapon table - see [`Setup::weapons`].
    pub(super) weapons: Option<oag_tables::weapons::WeaponStats>,
    /// Restricts what a `Weapon Pad` hands out - see
    /// [`Setup::allowed_weapons`]. Empty is no restriction.
    pub(super) allowed_weapons: Vec<oag_tables::weapons::Weapon>,
    /// The speed class, which indexes the pickup odds - see [`Setup::class`].
    ///
    /// The disc's own spelling, so a title whose ladder is not Pulse's - Pure,
    /// with `VECTOR` - selects its own row.
    pub(super) class: String,
    /// Which control scheme maps the snapshot. `[controls] scheme`.
    ///
    /// On `Race` and not on the input layer because the schemes differ in which
    /// *gesture* a sideshift takes, and the gesture is read by the simulation
    /// out of `ShipControls` - so this is what decides which of
    /// `ship_controls`' two field groups gets filled. See
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    pub(super) scheme: ControlScheme,
    /// Where the ship was at the end of last tick, for the swept test.
    ///
    /// `None` on the first tick, which is the original's own "no previous
    /// position" case in `Pads_TestCraft` and takes the single-point path. One
    /// per racer.
    pub(super) pad_previous_position: [Option<Vec3>; MAX_SHIPS],
    /// Whether slot 0 is being driven by its own [`oag_ai::Driver`] instead of
    /// by the input snapshot. See [`Race::set_autopilot`].
    pub(super) autopilot: bool,
    /// One-shot sound cues this tick asked for, awaiting a drain.
    ///
    /// **A per-tick output, never state** - the shape ADR-0018 requires, and
    /// deliberately absent from [`Self::state_hash`]: a race that made no sound
    /// and one that made every sound must hash alike. See [`Race::drain_cues`].
    pub(super) cues: Vec<oag_sound::sfx::CueEvent>,
    /// Zone milestone numbers reached this tick, awaiting a drain.
    ///
    /// Kept apart from [`Self::cues`] rather than folded into [`Cue`](oag_sound::sfx::Cue):
    /// the milestone ladder is per-title data
    /// ([`oag_title::ZoneAnnouncer`]), not one of the engine's own fixed,
    /// statically-named cues, so there is no `Cue` variant for it to be. Same
    /// per-tick-output shape as `cues` and the same exclusion from
    /// [`Self::state_hash`].
    pub(super) announcements: Vec<u16>,
    /// Speed-class stages reached this tick, awaiting a drain. Same shape as
    /// [`Self::announcements`]; kept apart because the two ladders are
    /// separate title axes that happen to raise on related but distinct
    /// edges - see [`Race::push_class_announcement`].
    pub(super) class_announcements: Vec<u32>,
    /// Seconds before a wall contact may raise a sound cue again.
    ///
    /// Its own timer rather than [`RaceView::sparks_cooldown`], because a shielded
    /// contact raises `ABSORB` and ignites no sparks - so that timer would
    /// never re-arm. Render-side state, and out of the hash for the same
    /// reason. See [`Race::tick`], where the two are set side by side.
    pub(super) contact_cue_cooldown: [f32; oag_gameplay::MAX_SHIPS],
}

impl RaceSim {
    /// The mode's damage rules with this race's own weapons switch applied -
    /// what every damage and weapon-hit site asks instead of
    /// `oag_gameplay::damage_rules(mode)` alone, which knows only the mode.
    pub(super) fn damage_rules(&self) -> oag_physics::DamageRules {
        oag_physics::DamageRules {
            weapons: self.weapons_on,
            ..oag_gameplay::damage_rules(self.world.mode())
        }
    }
}
