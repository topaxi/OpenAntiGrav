# WebAssembly extensions: a campaign whose races change the craft and the pickups

**Challenged, unresolved:** [progression-challenge.md](progression-challenge.md) lists 25 numbered objections to this draft (5 block a decision) and a smaller first slice; read it beside this page.

**Status: a design draft, nothing built (2026-11-19).** The decision records are
[ADR-0093](../adr/0093-a-custom-campaign-keeps-a-bounded-run-state-and-its-mod-sets-bounded-modifiers-at-race-start.md)
(the run, modifiers, pickups, unlocks; sections 2 to 7) and
[ADR-0094](../adr/0094-a-campaign-run-offers-a-choice-between-races-and-shows-the-mods-stats.md)
(the choice screen and the stats; section 8), both *Proposed*. This page holds the proposed surface of `oag-ext-abi` and
`oag-ext` as Rust signatures, the lifecycle of the data, and the open questions.
It changes no crate: `crates/ext-abi`, `crates/ext` and the mod author skill stay
as they are until the maintainer accepts the ADR. Every number here is **chosen,
not measured** and carries no confidence score.

The motivating example, in generic terms: a custom campaign whose rewards change
the craft and the pickups from race to race. After a race the player picks one of
several rewards; the next race starts with a stronger engine, a bigger shield or a
pickup that now does two things. The campaign keeps going until the player ends
it or starts a new one.

## 1. What exists today, and what this page needs that does not

Read this table first. The README sketches a larger API than is built, and a
reader who takes its WIT for the shipped ABI will design against calls that do
not exist.

| Piece | Built? | Evidence |
| --- | --- | --- |
| Sim world: one `tick` per module per tick, state blob of at most 1,024 bytes, 96 commands a reply | **Built** | `oag_ext_abi::MAX_STATE_BYTES`, `MAX_COMMANDS`, `REVISION` = 6 |
| `oag_wants_race`: asked once at race load, sees a 20-byte `RaceInfo` (title, class, mode, weapons, `RACE_CAMPAIGN` bit, people, craft, four option bytes) | **Built** | `crates/ext-abi/src/race.rs`; [hooks.md](hooks.md#choosing-its-races-oag_wants_race-abi-1-revision-6) |
| A second host-written buffer a guest asks for by export (`oag_track`) | **Built** | `crates/ext-abi/src/track.rs`; the precedent for section 3 |
| Replay header `mods` list with module hash, picks, state bytes | **Built** | `oag_replay::header::ModUse`; [ADR-0091](../adr/0091-a-modded-race-records-its-inputs-and-its-header-names-the-modules.md) |
| `give-shield`, `hold-shield`, `clear-shield`, `give-pickup`, `pickup-roll`, `Weapon::Extension` with `pickup-fire` and `pickup-absorbed` | **Built** | [README](README.md#and-energy-turbo-2026-10-29), "Breakable armour from the Shield" |
| Custom campaigns (`campaigns/<id>.toml`), progress rows `<title slug>/mod:<mod id>/<campaign id>` | **Built** for Pulse, HD, Omega | [custom-campaign.md](../../formats/custom-campaign.md) |
| `race-start` and `race-end` as guest exports | **Not built.** The README's WIT lists them; the ABI has neither | `crates/ext-abi/src/lib.rs` header lists the exports |
| `setup.tune`, and tune bytes in the replay header | **Not built** | ADR-0091 decision 6 |
| The frontend world (`open-mod-screen`, `[[extension.screen]]`, host widgets, the saved blob) | **Not built.** [ADR-0080](../adr/0080-a-wasm-mod-may-own-a-screen-and-save-a-small-blob.md) is *Proposed* | no `[[extension.screen]]` in `crates/mod/src/manifest.rs`; no `open-mod-screen` in `oag_ui::menu` |
| A modded records table (README phase 2) | **Not built** | a modded race files the ordinary per-track `[[record]]` today |
| A network race running a guest | **Not built**; the lobby advertises `NO_MODS` | [hooks.md](hooks.md#picks-sim-and-view), "Online" |

Consequences for this design: capabilities 1, 2 and 3 need only the sim world and
the records store, so they can land before the frontend world. Capabilities 4 and
5 need the frontend world and a narrowing of the "a disc screen gains no mod
behaviour" rule (section 8).

## 2. The data: a run, and its state

A **run** is one playthrough of one custom campaign by one profile. It has:

- `bytes`: the **run state**, opaque to the host, at most `MAX_RUN_BYTES` = 1,024
  (the size of a module's tick state blob, so a guest can copy it whole). The cap
  is on raw bytes.
- `seed`: a `u64` the host draws once when the run is created, so the guest can
  derive reward rolls without an RNG stream (section 4.3).
- `ordinal`: how many events the run has committed.
- `end_owed`: only after a faulted `race-end`, the latched `RunResult` and tick state
  it still owes (about 1.1 KiB), until RETRY or DISCARD settles it (section 5).
- `offer_pending`: a flag, set when a committed `race-end` asked for a choice and the player has not picked.

The host stores it in the records `Store` as a sibling table of `campaign` and
`loyalty`, **keyed like the campaign's progress rows**
(`<title slug>/mod:<mod id>/<campaign id>`), base64 in the TOML, written by
`records::save_in_background`. It is not the ADR-0080 tune: the tune is one blob per
mod and title, owned by a screen; the run is one record per campaign, written by a
race. Two campaigns of one mod have two runs.

### Rejected: reuse the ADR-0080 tune blob

The tune is keyed by mod and title, so a mod with two campaigns would share state
between them, and "start a new run" would have to clear a blob that also holds the
mod's unrelated settings. It is also written only by a screen; the run is written
by a race. Rejected.

### Rejected: let the module keep the run in its tick state blob

The tick blob lives in `RaceSim` and dies with the race. Persisting it would mean
the host saves a snapshot at the finish, and the guest could not tell state it
wants to keep from scratch state. A separate record with its own call is clearer.

## 3. The ABI draft (revision 7)

New **optional** exports, on the pattern of `oag_wants_race` and `oag_track`: a
guest that exports none runs as before, and revision 6 guests are refused by name
as every revision change is.

```rust
// crates/ext-abi, new module `run`. Proposed.

/// The most run-state bytes a campaign may keep. Chosen.
pub const MAX_RUN_BYTES: usize = 1024;
/// Commands a race-start reply may carry: 8 slots x 8 parameters, with room. Chosen.
pub const MAX_START_COMMANDS: usize = 64;

/// Guest exports (core wasm, no imports):
///   oag_run() -> u32          // address of an IO_LEN-byte run buffer in the guest's memory
///   oag_race_start() -> u32   // 0 on success; once per race, after the grid is placed
///   oag_race_end() -> u32     // 0 on success; once, at the finish site, see section 5
///
/// Both calls get a fresh pooled instance, like `oag_wants_race`. The run buffer is a
/// second buffer, like the track region; the tick buffer is untouched.
pub const RUN_ABI_AT: usize = 0;
// Bytes 0..RACE_LEN: a `RaceInfo` (title, class, mode, weapons, campaign flag, people,
// crafts, options), so a guest already knows how to read the first part.
pub const RUN_CAMPAIGN_AT: usize = RACE_LEN;      // u32: channel_key of the campaign id
pub const RUN_EVENT_AT: usize = RUN_CAMPAIGN_AT + 4; // u32: channel_key of the event id
pub const RUN_ORDINAL_AT: usize = RUN_EVENT_AT + 4;  // u32
pub const RUN_SEED_AT: usize = RUN_ORDINAL_AT + 4;   // u64
pub const RUN_ACTIVE_AT: usize = RUN_SEED_AT + 8;    // u32: slots holding a craft
pub const RUN_PEOPLE_AT: usize = RUN_ACTIVE_AT + 4;  // u32: slots flown by a person
pub const RUN_TEAMS_AT: usize = RUN_PEOPLE_AT + 4;   // [u32; MAX_CRAFTS]: channel_key of each team identity
pub const RUN_LEN_AT: usize = RUN_TEAMS_AT + 4 * MAX_CRAFTS; // u32: length of the run state in
pub const RUN_IN_AT: usize = RUN_LEN_AT + 4;         // [u8; MAX_RUN_BYTES]
pub const RUN_RESULT_AT: usize = RUN_IN_AT + MAX_RUN_BYTES; // RunResult, race-end only
pub const RUN_OUT_LEN_AT: usize = RUN_RESULT_AT + RunResult::LEN; // u32, race-end only
pub const RUN_OUT_AT: usize = RUN_OUT_LEN_AT + 4;    // [u8; MAX_RUN_BYTES]: at race-start the guest's initial TICK state
                                                      // (its length is the module's oag_state_len); at race-end the new RUN state
pub const RUN_TICK_AT: usize = RUN_OUT_AT + MAX_RUN_BYTES; // [u8; MAX_STATE_BYTES], race-end only: the tick state latched at the race's end
pub const RUN_REPLY_AT: usize = RUN_TICK_AT + MAX_STATE_BYTES; // count u32, then 12-byte command records
// const _: () = assert!(RUN_REPLY_AT + 4 + COMMAND_LEN * MAX_START_COMMANDS <= IO_LEN); // about 4,000 of 4,096
```

**Why a second buffer, and the 1,024 cap.** The tick buffer is 4,096 bytes with 1,024
reserved for the state blob, so a run state of up to 4 KiB (ADR-0080's cap for a
screen blob) cannot sit beside a race description, an output run state and a reply
inside it. The second buffer fits, tightly: the fixed header is about 120 bytes,
plus the run state in (1,024), the result, the output region (1,024), the final tick
state at race-end (1,024) and 64 x 12 reply bytes come to about 4,000 of the 4,096.
A guest that wants more than 1,024 bytes keeps a compact encoding.

**How the run reaches `tick`.** The tick call never sees the run bytes. `race-start`
reads them and **sets the module's initial tick state** in the output region (its
length is `oag_state_len`, as the manifest checks it); the tick state is already in
the race state and the hash, so a recipe the guest copied there at race start is
what a `pickup-fire` handler reads at tick 100. This is the README's WIT shape
(`race-start ... -> list<u8>`, "returns the initial blob"). At race-end the same
region is the new run state, and the final tick state arrives beside it, so the race
can hand what it counted to the run.

*Rejected: put the run state in the tick buffer's state region.* At `race-start` the
region is the module's own (zeroed) tick state, and the output needs a second region
anyway. *Rejected: a 4 KiB cap.* It needs the buffer to grow past `IO_LEN`, which every
guest has been built against, for a use no example needs.

```rust
/// What the host tells `race-end` on the exit from the race. Fixed layout, little endian.
pub struct RunResult {
    pub slot: u8,          // the person's slot
    pub finished: bool,    // crossed the line (false: wrecked, eliminated, timed out)
    pub place: u8,         // 1-based; 0 when not finished
    pub laps: u8,
    pub tier: u8,          // index into the campaign's medal tiers, 0 = best; 255 = none
    pub rung: u8,          // difficulty rung, as the campaign screens use it
    pub prior_tier: u8,    // the event's best medal before this race (`Store::campaign_medal`); 255 = none
    pub event_index: u8,   // the event's position in the campaign file, so a cleared set is a bitmask
    pub ticks: u32,        // race clock at the finish
    pub score: u32,        // zones reached in Zone, kills in Eliminator, 0 elsewhere
    pub best_lap_ticks: u32,
    pub shield: f32,       // left at the finish
    pub shield_max: f32,
    // 2 bytes spare
}

/// What `race-end` may add to its reply besides new run bytes.
pub const END_OFFER: u32 = 1; // open the run's choice screen before the campaign screen
```

`tier` comes from the host: the medal is evaluated in `oag-game`
(`Launch::record_result`, `crate::campaign::custom`), outside the simulation half.
So `race-end` is called from `oag-game` through a method on the race
(`Race::end_run`, proposed), not from `Race::tick_body`. See section 5.

### New commands

Codes **39 and up**: the gaps at 4, 7 and 15 in `CommandKind` are not reused.
Codes 39 to 41 are valid only in an `oag_race_start` reply, and 42 only in a `tick`
reply (anywhere else is a fault, like `pickup-roll` off slot 0).

| Code | Command | Where valid | Meaning |
| --- | --- | --- | --- |
| 39 | `set-modifier{slot, param, factor}` | race-start reply | scale one handling parameter of that slot's craft (section 4) |
| 40 | `set-pickup-count{slot, weapon, count}` | race-start reply | rockets per press, mines per drop, cannon rounds (section 6) |
| 41 | `set-pickup-duration{slot, weapon, factor}` | race-start reply | how long a Turbo, Shield or Autopilot lasts, for that slot only (section 6) |
| 42 | `give-autopilot{slot}` | tick reply | starts the title's Autopilot for its authored time, as `give-shield` starts the Shield (section 6) |

```rust
// crates/ext-abi: closed vocabularies, `coded!` like Weapon and Cue.
coded! { Param: u16, first 1 {
    Launch,         // engine.accelcap: acceleration from rest (section 4.2)
    TopEnd,         // engine.amount: the plateau, so terminal speed (section 4.2)
    Turning,        // turning.amount
    Airbrake,       // airbrake.amount and airbrake.turn
    Brake,          // brakes.amount (stored negative; a positive factor keeps the sign)
    Grip,           // antigrav.grip_ground and antigrav.grip_air
    ShieldMax,      // dimensions.shield: the maximum and the start
    TurboStrength,  // engine.turbo
} }

/// Valid factor range per parameter, owned by the host. Chosen.
pub const fn modifier_range(param: Param) -> (f32, f32);

pub enum Command { /* the existing variants, and: */
    SetModifier { slot: Slot, param: Param, factor: f32 },
    SetPickupCount { slot: Slot, weapon: Weapon, count: u8 },
    SetPickupDuration { slot: Slot, weapon: Weapon, factor: f32 },  // Turbo, Shield, Autopilot only
    GiveAutopilot { slot: Slot },
}
```

### Events and records

**No new tick event.** `race-start` and `race-end` are calls, not events. A reward
that fires on a press already has `pickup-fire` and `pickup-absorbed`; one that reacts
to a hit has `hit`; the engine raises nothing new for a run. The new **records** are
the run buffer (header, `RunResult`, the output regions) above.

**One new craft flag.** `CRAFT_AUTOPILOT` (bit 5, value 32 in the craft record's flags,
beside `CRAFT_SHIELDED`): the slot's Autopilot is running. Without it a guest that
sent `give-autopilot` could not tell whether one is already running, which matters
because a running Autopilot is not refreshed and a pad grants nothing during it.

### The guest SDK (`oag-ext`)

**Shape (maintainer, 2026-11-20;
[progression-challenge.md](progression-challenge.md), objection 16).** Default methods
on `Extension`, as `wants_race` is, so `export!` (a `macro_rules!` that cannot see a
trait impl) always emits the exports, and an extension that ignores runs compiles and
behaves as before. Reply methods take a craft handle, never a raw slot.

```rust
pub trait Extension {
    type State: Blob;
    /// The run state, read and written whole. `Blob::from_bytes` reads missing bytes as
    /// zero, so a new run is all zeroes; `()` for an extension with no run.
    type Run: Blob = ();
    // tick, wants_race: unchanged.

    /// Once per race (per leg in a tournament), after the grid is placed. Reads the run,
    /// replies with modifiers and pickup parameters, and sets the module's **initial
    /// tick state** (on a later tournament leg it arrives holding the previous leg's
    /// latch). It cannot write the run.
    fn race_start(start: &Start<'_, Self::Run>, state: &mut Self::State, reply: &mut StartReply<'_>) {}
    /// Once, on the exit from a campaign event. `state` is the tick state latched at the
    /// race's end. `run` holds the committed run; what the guest leaves in it is
    /// committed.
    fn race_end(end: &End<'_>, state: &Self::State, run: &mut Self::Run, reply: &mut EndReply<'_>) {}
}

pub struct Start<'a, R> { /* race(), campaign_key(), event_key(), ordinal(), seed(),
                             run() -> &R, crafts() -> impl Iterator<Item = GridCraft> */ }
pub struct End<'a>      { /* race(), keys, ordinal(), seed(), result() -> &RunResult */ }
/// A grid slot the start description handed out: slot, `is_person()`, `team_key()`.
pub struct GridCraft { /* private */ }

pub struct StartReply<'a> { /* private */ }
impl StartReply<'_> {
    /// Clamps to `modifier_range` exactly as the host does; the test harness records a
    /// warning a test can assert on. Err only for what the host faults on: a non-finite
    /// factor, a second command for one craft and parameter, a full reply.
    pub fn modifier(&mut self, craft: &GridCraft, param: Param, factor: f32) -> Result<(), Refused>;
    pub fn pickup_count(&mut self, craft: &GridCraft, weapon: Weapon, count: u8) -> Result<(), Refused>;
    pub fn pickup_duration(&mut self, craft: &GridCraft, weapon: Weapon, factor: f32) -> Result<(), Refused>;
}

pub struct EndReply<'a> { /* private */ }
impl EndReply<'_> {
    pub fn offer(&mut self);                                                   // sets END_OFFER
    pub fn stat(&mut self, string_id: &str, value: u16, max: u16) -> Result<(), Refused>; // ADR-0094, at most 8
    pub fn unlock(&mut self, name: &str) -> Result<(), Refused>;               // section 7.1
}
```

**Clamp in the SDK too.** An out-of-range factor clamps in the SDK as on the host, so
what `cargo test` shows is what a player gets, and the harness's warning list lets a
test assert none were clamped. The first draft's `Err(Refused::Range)` is gone: with a
`let _ =` it dropped the whole reward in silence.

`blob!` serves the run state (`MAX_RUN_BYTES` equals `MAX_STATE_BYTES`, so its size
assert fits); the draft's `run_state!` is dropped. The test harness gains
`oag_ext::test::start::<E>(&info, &run)` and `::end::<E>(&info, &result, &latch, &run)`,
which go through the host's own writer and decoder.

### The frontend world (needs ADR-0080)

```wit
// Proposed additions to frontend-types in README 2.3. A screen opened from a run
// gets the run state and may replace it. A screen opened from a plain menu row is
// unchanged.
record open-context { /* existing fields */, run: option<list<u8>> }
record reply        { /* existing fields */, run: option<list<u8>> }   // replaces `save` for a run screen
```

A run screen's `reply.run` is checked against `MAX_RUN_BYTES`; `save` is refused on
it (one output, one place). Nothing else in ADR-0080 changes.

## 4. Capability 2: bounded per-race handling modifiers

At race start only. The host owns the list of parameters and each range; the
guest names a slot, a parameter and a factor.

### 4.1 The parameters

| `Param` | Field scaled (`oag_physics::Handling`) | Range (chosen) | Notes |
| --- | --- | --- | --- |
| `Launch` | `engine.accelcap` | 0.5 to 2.0, provisional | The cap arm of the force law: acceleration from rest. Refused until its ground truth lands (section 4.2) |
| `TopEnd` | `engine.amount` | 0.5 to 2.0, provisional | The plateau arm: terminal speed, growing about with the square root of the factor. Refused until its ground truth lands (section 4.2). Composes with the per-tick `scale-thrust` (open question 6) |
| `Turning` | `turning.amount` | 0.5 to 2.0 | Yaw authority, no speed factor |
| `Airbrake` | `airbrake.amount`, `airbrake.turn` | 0.5 to 2.0 | The two gains that shape airbrake steering; the other five fields stay |
| `Brake` | `brakes.amount` | 0.5 to 2.0 | Stored negative; a positive factor keeps the sign |
| `Grip` | `antigrav.grip_ground`, `antigrav.grip_air` | 0.5 to 2.0 | |
| `ShieldMax` | `dimensions.shield` | 0.25 to 4.0 | The maximum and the start together ("It is the maximum as well as the start", `params.rs`), so the host sets the slot's pool to the new maximum; `shield-delta`'s bound follows the new maximum |
| `TurboStrength` | `engine.turbo` | 0.5 to 2.0 | Per ship already; the Turbo pickup's boost |

**Not offered, with the reason:** `physical.mass` (two copies, `Dimensions`' and
`Body::mass`, "keeping them equal is the integration layer's job" in
`oag_physics::params`); `engine.falloff` and `engine.gain` (parsed, read by nothing);
`antigrav.ride_height` (the hover probe length and spring target, a camera and
track-clearance risk); anything in `Global` (shared by all teams and classes). A raw
"field index plus factor" command is rejected: the host cannot say what a field
means, so it cannot choose a range, and two engine versions could silently re-point
an index.

**Where a factor is applied.** Once, after the grid is placed and the title's
`Handling` is loaded, and **before `begin_speed_plans`**, multiplied into the slot's
`Ship::handling` (a per-ship copy, so one slot's factor never reaches another). A factor of exactly `1.0` skips the
multiply, so no vanilla bit moves. Two commands for one slot and parameter are a
fault; a slot with no craft is a fault.

**The speed plan (maintainer, 2026-11-20).** The AI, the Autopilot and Pilot Assist
drive a plan learned per hull at race load from that hull's `Handling`
(`crates/raceplay/src/speed_plan.rs`: `HullPlans`, `other_hulls`, `flown_by`). A
modified slot, AI or person, counts as its own hull and **adapts the reference plan**
under the existing 40,000-step budget, the path other hulls already take. A search
from scratch (12 to 24 times a race start's CPU) is not used. Load grows by one
adaptation per distinct modified craft; the plan is close, not optimal
([progression-challenge.md](progression-challenge.md), objection 5).

**Clamp, not fault.** The existing commands fault on an out-of-range value
(README section 1.2: "never a silent clamp"). Here the host clamps to the range and
writes a **named load-report warning** (`mod <id>: TopEnd factor 3.2 for slot 0
clamped to 2.0`), which is not silent. Reason: a fault at `race-start` would turn the
module off for the race, so the player would fly the stock craft with no hint that
their rewards did not apply. A non-finite value is still a fault. This departs from
the established convention, so it is open question 1.

**A fault refuses the launch (maintainer, 2026-11-20).** Any other fault in
`race-start` (a trap, spent fuel, a malformed record, a non-finite factor) refuses the
launch: back to the campaign page with a line naming the mod and the fault. A stock
race never runs in its place, so it never earns a medal in a run campaign
([progression-challenge.md](progression-challenge.md), objection 10; ADR-0077 decision
6). The clamp of an out-of-range factor stays as settled.

**The hash.** `Ship::handling` and `Ship::weapon_counts` are deliberately not
hashed (`oag_gameplay::hash`: "named in the destructure so this is a decision rather
than an omission"). Writing modifiers into them would let a replay run on other run
bytes survive the tick-0 check and drift later. So the host keeps the **applied
record** in `Extensions`:

```rust
// oag-raceplay, `Extensions` state. Proposed.
pub struct SlotMods {
    pub factor: [f32; Param::COUNT],         // 1.0 = untouched
    pub rockets: u8, pub mines: u8, pub cannon_rounds: u8,
    pub duration: [f32; 3],                  // Turbo, Shield, Autopilot (turbo strength is factor[TurboStrength])
}
// `[SlotMods; MAX_CRAFTS]`: cloned with `snapshot_sim`, hashed in `Extensions::hash_into`
// after the `slots.is_empty()` early return, so `VANILLA_FOLD` cannot move.
```

A race with no run module hashes nothing for it. A restored snapshot also restores
`Ship::handling` (it is in `World`), so the record is for the hash, not for
re-applying.

### 4.2 The engine: `Launch` and `TopEnd` (maintainer, 2026-11-20)

The engine law is `T = 2 * min(throttle * amount, 0.5 * speed + accelcap)`
(`oag_physics::engine::engine`; the cap arm at confidence 95 in
[force-balance-ground-truth.md](../../physics/force-balance-ground-truth.md)). From
rest the cap arm binds, so `accelcap` sets the launch; at speed the plateau arm binds,
so `engine.amount` sets terminal speed. The draft's `Thrust` (on `engine.amount`) and
`TopSpeed` (on `accelcap`) had the two the wrong way round
([progression-challenge.md](progression-challenge.md), objection 1). The engine is
therefore two parameters, named for what each does:

- **`Launch`** scales `engine.accelcap`: a quicker getaway and recovery from a wall,
  no change to terminal speed.
- **`TopEnd`** scales `engine.amount`: derived from the law (not measured) on HD
  Feisar, a factor of 1.1 gives about +5 % terminal speed and 2.0 about +43 %, with
  stock acceleration through the stock speed range.

**Ground truth first.** Each is listed and **refused** (a command naming it is a
malformed record) until a held-throttle run on one circuit and class, at factors 1.0
and 2.0, confirms its effect (the shape of `wall_penalty_ground_truth`). The ranges
are provisional until then.

**Not in Zone.** A Zone event's `auto_speed` branch replaces the throttle path and
reads neither field, so both are inert there. Accepted: Zone is its own mode, and a
run campaign's Zone events fly the stock engine, at least for now. A Zone engine
reward, if wanted later, is `scale-thrust`, which acts after the law in every mode.

### 4.3 Rejected alternatives for capability 2

- *A declarative table in the campaign file* (`[[run.modifier]] param = "thrust",
  stat = "engine_level", per_level = 0.1`). Data first is the project's habit for
  race rules. It is rejected as the only path because the shape of a reward is the
  mod's whole point (stacking, caps, trade-offs), and a formula table grows into a
  worse scripting language. It stays a possible addition on top of the guest path
  (open question 7).
- *A per-tick modifier command.* Needs a timer or re-sending, keeps the factor out of
  the applied record, and the brief asks for race start only. `scale-thrust` already
  covers the per-tick thrust case.
- *Replacing `Handling` values.* Contradicts README section 3, "no replacing a recovered
  law or a disc table". Scale factors on the disc's own numbers narrow that row: the
  disc stays the source and the mod gets a bounded multiplier. ADR-0093 says so
  explicitly.

## 5. Capability 1: the lifecycle of the run state

Two bytes-in, one bytes-out, and one commit point: the exit from the race.

| Event | What happens to the run |
| --- | --- |
| Campaign entered, no run | Host creates a run: empty bytes, a drawn `seed`, ordinal 0 |
| NEW RUN (a row on the campaign's page, host-drawn, proposed) | Clears `bytes`, `offer_pending`, ordinal and the seed; clears this campaign's medal rows if the campaign says `new_run = "clear-medals"` (section 7) |
| An event is launched | Host reads the committed bytes into the start description; `race-start` runs; no write |
| The person's craft finishes (or is wrecked or eliminated), or, in Zone, Speed Lap or an abandoned Time Trial, the player leaves | The medal is captured at its site as today, and the race goes on running as today. Nothing is written to the run yet |
| The player **exits the race** (leaves the results for the campaign screens, presses CONTINUE, or, in Zone and Speed Lap, escapes) | `Race::end_run(result)` calls `race-end` once with `prior_tier` and `event_index`. **Commit** at once, in one records save: its output becomes `bytes`, ordinal += 1, and `offer_pending` is set if it asked for an offer. Then the choice screen opens, if there is one (section 8.1) |
| RACE AGAIN on the results page | An exit: `race-end` runs and commits as above, then the event relaunches from the **new** run bytes (after the offer, if one was asked for). Hidden when the campaign sets `[run] race_again = false` |
| A tournament leg ends, not the last | No `race-end`. The tick state latched at the leg's end becomes the next leg's initial tick state; the next leg's `race-start` sees it with the same committed run bytes |
| The player exits after a tournament's last leg | `race-end` once, with the final rank as `place`, the event's medal and the last leg's latch; one ordinal step for the whole event. Quitting midway commits nothing |
| RESTART RACE in the pause menu, before the medal is captured | Nothing was written; the race restarts from the same committed bytes |
| QUIT RACE or escape **before** the finish, in a mode with a lap target | No `race-end`, nothing written, committed bytes untouched |
| `race-end` faults at the exit (a trap, spent fuel, bytes over the cap) | The medal stays. The run bytes and ordinal are untouched; the latch is saved as `end_owed` and a line names the mod. While it is set the campaign's events refuse to launch, and its page offers **RETRY** (call `race-end` on the saved latch again, for example after a mod update; on success it commits as usual) or **DISCARD** (drop the latch, the run stays as it was) |
| The game closes or crashes after the medal is captured, before the exit | The medal is kept, as today; `race-end` never ran, so the run is as it was. A replay of the event then sees the medal as `prior_tier` |
| A replay rebuild, `--from-png`, a demo, an unofficial run, a split screen | `race-end` is not called, nothing is written |

**Why nothing is staged (maintainer, 2026-11-20).** The first draft staged the run
bytes and the medal at the finish, committed them on leaving and discarded them on
RACE AGAIN. That closed RACE AGAIN but not the grid: a cleared event stays enterable,
and each finish rolled a fresh reward
([progression-challenge.md](progression-challenge.md), objections 3 and 6). Instead the
guest gets `prior_tier` (the host has it, `Store::campaign_medal`) and `event_index`,
and decides in one comparison whether a finish earns anything; the host enforces
nothing. The medal is recorded at its site as today, and the run commits on exit, so
both `record_result` sites keep one path for every campaign.

**RACE AGAIN.** It is an exit like any other: `race-end` runs and commits, and the
relaunch starts from the new run, so a retry is a new event of the run, judged by the
mod through `prior_tier`. A campaign that wants no quick retry sets `[run] race_again =
false`, and the results page offers no RACE AGAIN for its events; the player re-enters
from the grid.

**Where an event ends (maintainer, 2026-11-20).** At the two sites that record a
medal today (`Launch::record_result`): the finish site in `Session::frame` for a
lap-count mode, and the escape site in `Session::escape` for Zone, Speed Lap and an
abandoned Time Trial, where leaving is how the mode ends
([progression-challenge.md](progression-challenge.md), objection 2). The race goes on
as it does today: the world keeps stepping under the results, and Zone runs until the
player leaves. The mod's award and end-of-run logic (`race-end`) runs once, when the
player exits the race, not at the finish moment.

**A tournament (maintainer, 2026-11-20)** is one run event over several legs. Each
leg's `race-start` gets the same committed run bytes plus, after the first leg, the
tick state latched at the end of the previous one, so a mod can count across legs
(total kills, a shield that does not refill). `race-end` runs once after the last leg
([progression-challenge.md](progression-challenge.md), objection 9). A per-leg
`race-end`, paying rewards between legs, is a later addition.

**The latch (maintainer, 2026-11-20).** The world keeps stepping under the results,
by a frame-dependent number of ticks, so a state read at the exit would differ between
two machines with the same inputs. On the tick the person's race ends the sim
**latches** the `RunResult` fields and a copy of the module's tick state (plain data
in the race state, no guest call); `race-end` is built from the latch at the exit.
Zone and Speed Lap latch at the leave, which is the player's own moment. A run is then
a function of its races' inputs ([progression-challenge.md](progression-challenge.md),
objection 8).

**Who calls `race-end`.** `oag-game`, on the exit from a race whose medal was
captured, with the tier the medal site evaluated, through `Race::end_run(RunResult) -> Option<RunOut>` in
`oag-raceplay`'s simulation half, which owns the instances. It is not part of
`Race::tick_body`. It is single-player only: it is skipped where
`stage.race.view_state().viewport_count() > 1`, the same guard the result save uses,
and where `race_is_unofficial()` is true.

### 5.1 Replays, ghosts, records, network

- **Replay.** `ModUse` gains an optional `run`: the **input** bytes the race started
  from (not the committed output), the `seed`, the `ordinal`, the campaign and event
  keys, and on a tournament leg after the first the tick state carried in from the
  previous leg (section 5). Base64 in the TOML, capped at `MAX_RUN_BYTES` on the raw bytes, like the
  tune proposal. A rebuild (`Options::recorded_mods`, `apply_recorded`) feeds those to
  `race-start` and never reads the local store; the applied record in section 4.1 is
  hashed from tick 0, so a rebuild on other bytes fails at tick 0. A replay does not
  call `race-end`.
- **Ghosts.** None, as ADR-0091 decision 5 already says: a module changes the run.
- **Records.** A race whose start applied any non-identity modifier, count or scale
  files **no** per-track `[[record]]`. The modded table README phase 2 planned is not
  built, so without this rule a doubled engine files a personal best on a
  leaderboard row (open question 4). Medals are the campaign's own and stay.
  **Loyalty and XP are paid** as for any official race (maintainer, 2026-11-20;
  [progression-challenge.md](progression-challenge.md), objection 11): only the
  per-track record is withheld.
- **Network.** Single-player. The lobby carries no run state, the start description
  has campaign key 0 in a lobby race, and a run module declines it in
  `oag_wants_race`. A campaign shared by a host (all seats on the same bytes) is the
  same question as the per-seat tune (README open question 13).
- **MODS page.** Lists the run's size and offers CLEAR, as for the tune blob. Turning
  the mod off hides the run; uninstalling leaves an orphan the page lists. Whether
  uninstall deletes it is README open question 16.

### 5.2 Rejected alternatives for capability 1

- *Stage the run bytes and the medal, commit on leaving, discard on RACE AGAIN.* The
  first draft. Rejected: it leaves the grid open to farming and sends a `[run]`
  campaign down a second path through both `record_result` sites (above).
- *Let `race-start` write the run state too.* A retry would re-apply a start-time
  write on top of itself. Read-only at start keeps `race-start` a pure function of
  (bytes, seed, ordinal, race).
- *Pass the run state every tick.* The tick state blob already carries what a race
  needs; the run state enters once, as a copy, at `race-start`, and the guest sets its
  initial tick state from it (section 3, "How the run reaches `tick`"). This is the
  ADR-0080 `setup.tune` rule applied to a different owner.
- *A host-typed run state (named integer fields the host understands).* Would let
  the host apply "thrust level 3" itself. Rejected: the host would own a schema the
  mods must fit, and the unit of the brief is "numbers the mod interprets itself".

## 6. Capability 3: pickup parameters

Per slot, at race start. Three kinds of parameter (count, duration, strength), from
what the engine already has.

| Parameter | Weapon | Where it lives today | Range (chosen) | Needs |
| --- | --- | --- | --- | --- |
| `count` | Rocket | `Ship::weapon_counts.rockets`, `1..=MAX_ROCKETS` = 3 (`oag_weapons::counts`) | 1 to the craft's own count (settled 2026-11-20: down only) | Nothing. The machinery exists: 2048 and Omega author `number_of_rockets` per craft; Pulse, Pure and HD default to 3 |
| `count` | Mine | `weapon_counts.mines`, `1..=MAX_MINES` = 5 | 1 to the craft's own count (down only) | Nothing |
| `count` | Cannon | `Held::cannon_rounds`, armed from the race-wide `<Stats rounds>` | 1 to 32 | A per-ship starting-rounds field |
| `Duration` | Turbo, Shield, Autopilot | race-wide `WeaponStats::simple` (`<Stats time>`) | 0.25 to 4.0 | A per-ship scale, read where the pickup starts. Editing the shared table would change every craft, the AI's too |
| strength | Turbo | `Handling::engine.turbo` (per ship) | 0.5 to 2.0 | Named `Param::TurboStrength` and sent with `set-modifier`; no other weapon has a strength to scale |
| strength | Shield | none: the Shield absorbs all damage for its time | n/a | Breakable armour is the mod's own logic (`hit`, `clear-shield`) |

**Counts only go down (maintainer, 2026-11-20).** A `count` is clamped to
`1..=` the craft's own number, the authored per-craft count on 2048 and Omega and
the default elsewhere. Taking a pickup away and giving it back is the intended
use: a campaign can start a craft on one rocket per press and level the pickup up
to two and then three, and the progression ends where today's limit is.

**Hard bounds.** More than three rockets or five mines needs a larger volley array
and mine pool, which are hashed projectile state: an engine change with its own
proof, not a range edit. Damage per weapon is not offered: the weapon tables are
race-wide, a per-owner damage scale touches the `Craft` trait and every projectile
(open question 8).

**Where the overrides live (maintainer, 2026-11-20).** The cannon's starting rounds
and the three duration scales (Turbo, Shield, Autopilot, all kept) are **not** fields
of `Ship` or `World`. They are per-slot overrides in `SlotMods`, the extension state of
section 4.1, which exists (heap-allocated if need be) only when a module is loaded.
Every grant site (pads, the AI, `give-shield`, `give-autopilot`; today
`crates/raceplay/src/weapons.rs`, `field/opponent_weapons.rs`, `field.rs`) goes through
one helper that returns the table's `simple.time` times the slot's override, or the
table's value when there is none. So a race with no module keeps `World`'s size, its
snapshot, its hash and its per-tick work unchanged, and pays at most a "no override"
check at a grant; a modded race may be slower or larger. Measured against the
alternative of three `Ship` fields: +96 bytes of `World` (+0.5 %) and +0.3 % per
snapshot avoided for every vanilla race. Each new `Weapons` field or trigger needs its row in
`docs/gameplay/weapon-lineage.md`, "Checked axes", when it is built; the counts
already have theirs.

### A combined pickup

The mod's own pickup is `Weapon::Extension(kind)`; the engine fires nothing on
press, it raises `pickup-fire{slot}`. A "shield plus autopilot" pickup is the
guest's reply to that event:

1. `give-shield{slot}` (built): the title's Shield for its authored time, not
   refreshing a running one.
2. `give-autopilot{slot}` (**new, code 42**): the title's Autopilot for its authored
   time, mirroring `give-shield`. It follows the Autopilot's recovered rules
   ([autopilot.md](../../ghidra/functions/psp-pulse-usa/autopilot.md)): a running one
   is not refreshed, an absorb cancels it, a fire does not, and a craft whose
   Autopilot runs is granted nothing by a pad. It starts `Ship::autopilot_timer`.
   It runs on any craft, a modified one included, on the slot's adapted plan
   (section 4.1). The maintainer's observation (2026-11-20, not a measurement): the
   pickup Autopilot holds the spline hard and has not been seen to crash, even on a
   poor plan.
3. `clear-pickup{slot}`.

Both durations use the slot's `Duration` scale, so a reward can lengthen the
shield and the autopilot together or separately.

**Several combined rewards.** `ModKind` allows one kind per mod and 32 kinds per
race (craft record codes 15 to 46). A campaign that offers shield+autopilot,
shield+turbo and a triple would need three kinds per mod. *Chosen:* one kind whose
behaviour the run state selects: the guest reads its recipe from its tick state
(set by `race-start` from the run bytes, section 3) and sends `pickup-look` each tick. *Rejected:*
more kinds per mod; it changes what `MAX` means and renumbers every craft record
code a guest may have hard-coded.

**The AI.** Today the AI fires a mod pickup through the Turbo's gate
(`Driver::affords_boost`, "a straight, nobody close ahead"), which is wrong for a
defensive combination. Open question 9.

## 7. The campaign file and the manifest (draft)

ADR-0077 is Accepted and is not edited. A `[run]` table is an optional addition to
`campaign.tosd`, a new minor under ADR-0064's rules; a reader of today reports it as
unread and still offers the campaign.

```toml
[campaign]
id = "long-haul"
# ...

[run]
module = "ext/run.wasm"       # a module the mod lists in [extension.rules]; the only module that sees the run
run_bytes = 256               # the run state's length, at most MAX_RUN_BYTES (1024); the module's tick blob has its own oag_state_len
new_run = "clear-medals"      # "clear-medals" (default) | "keep-medals"
choice_screen = "reward"      # optional: an [[extension.screen]] id, opened after a finish that offered
race_again = true             # false: the results page offers no RACE AGAIN for this campaign's events

# Stats are reported by the guest at race-end (section 8.2); the campaign file declares none.
```

**Binding.** The start description carries the campaign and event keys, and the
`[run]` table names the module. A rules module otherwise "runs in every race the
mod is on for" unless it declines, so the host passes the run to the named module
only, and only in a race launched from this campaign. Another module of the mod, or
the same module in a Single Race, sees an empty run and campaign key 0, and
`oag_wants_race` already lets it decline.

The mod declares `simulation = true`, so every race of the campaign is MODDED. The
campaign page names this, as the MODS page does.

### 7.1 Own unlocks (maintainer, 2026-11-19)

A campaign decides whether Team Selection follows the title's global unlocks or the
campaign's own. The global ones are the title's loyalty and `<Unlock>` rows that
`oag_game::unlock` evaluates against the records `Store` (and `[dev]
unlock_everything` in debug builds).

```toml
[unlocks]
source = "own"            # "global" (default): the title's unlocks, as today

[[unlocks.team]]
team = "Feisar"
open = true               # offered from the start

[[unlocks.team]]
team = "Qirex"
requires = [{ event = "gate-run", medal = "silver" }]   # the ADR-0077 rule shape

[[unlocks.variant]]
team = "Feisar"
variant = "concept1"      # a title's variant/skin name, checked against the disc at load
run_flag = "feisar-concept"   # opened when the run's unlock set holds this name
```

- **Data first.** A row opens a team or a variant from the start, by an event's
  medal (`requires`, evaluated like a cell's `unlock`), or by a name in the run's
  unlock set. A campaign with no module can still define its own unlocks.
- **From the run.** A sim module adds names to the run's unlock set at `race-end`
  with `unlock{name}` (new reply command; the set lives in the run record beside the
  run bytes, at most 64 names of 32 bytes, committed with them). NEW RUN
  clears it; medal-based rows survive a new run, because medals are the campaign's
  progress, not the run's.
- **Scope.** Under `source = "own"` the campaign's events offer only what its rules
  open, whatever the global profile has unlocked. Nothing a campaign opens reaches
  the Race Box, the title's own campaign or another mod's campaign, and a campaign
  never writes the global unlocks.
- **Preset craft** (`craft = { team = ... }` on an event) is unaffected: a preset
  is flown whether or not the campaign's rules have opened it.
- **Validation.** A team or variant the disc does not ship is refused at load by
  name, as a missing team is today (`custom-campaign.md`).

Open: whether `unlock` should also be available to a campaign without `[run]` (a
medal-only campaign does not need it), and whether a variant row may name a mod's
own livery once mods ship liveries.

## 8. Capabilities 4 and 5: the choice screen and mod-defined stats

Decided in [ADR-0094](../adr/0094-a-campaign-run-offers-a-choice-between-races-and-shows-the-mods-stats.md),
split out of ADR-0093 so the run needs no frontend world (maintainer, 2026-11-20).

### 8.1 The choice screen

A mod screen (ADR-0080), named by `[run] choice_screen`; kept over a host-drawn page
with an `oag_run_pick` export (maintainer, 2026-11-20;
[progression-challenge.md](progression-challenge.md), objection 14), so a campaign owns
its reward screen's look. The flow:

1. `race-end` rolls the offers from `seed` and `ordinal` (a pure function; the
   frontend world has no RNG and no determinism, and the roll belongs in sim code)
   and writes them into the run bytes, with `END_OFFER`.
2. The player exits the race. The host **commits first** (section 5: `race-end`'s
   output becomes the committed run bytes), then opens the screen with
   `open-context.run` set to the committed bytes.
3. The screen draws a `choice` or `list` widget from the offers it parses out of the
   run bytes. On accept it returns `reply.run` with the pick applied and `close: true`;
   the host writes that straight over the committed bytes and clears `offer_pending`.
   The pick happens after the commit, and a crash before the pick leaves
   `offer_pending` set.

**Back.** Back writes nothing. `offer_pending` stays set, and the screen re-opens at
the campaign's next launch. If the maintainer allows skipping (open question 5), Back
clears the flag and the offers stay in the run bytes for the guest to ignore or reuse.

**Pointer and pad.** The screen is a host-widget screen, so the pad and the pointer
come from the host widgets (the rule that every screen ships with mouse and touch).
The new part is the host opening it automatically after a finish, which must land
with pointer support in the same change.

**Skipping.** The host records `offer_pending` in the run. While it is set, the
campaign's next launch re-opens the screen first. Whether the player may skip a pick
and take none is open question 5.

*Rejected: the screen returns only the pick, and `race-start` applies it.* It keeps
state transitions in deterministic code. But the stat rows on Ship Select (below)
must reflect the pick before the next race starts, and the run state exists only
after the pick is written. It would also put the pick into the replay header as a
second input.

*Rejected: the screen rolls the offers.* It has no seeded RNG, and a roll made on the
UI side is not a function of the seed the replay carries.

### 8.2 Mod-defined stats

**Reported by the guest (maintainer, 2026-11-20;
[progression-challenge.md](progression-challenge.md), objection 15).** `race-end`'s
reply and the choice screen's reply may carry up to 8 rows:

```rust
// crates/ext-abi, a race-end reply record. Proposed.
pub struct Stat { pub string_id: u32 /* channel_key of the mod's string id */, pub value: u16, pub max: u16 }
pub const MAX_RUN_STATS: usize = 8;
```

The host keeps them beside the run as a display cache and draws them, with the mod's
strings, on pages it owns for the campaign: a stats block on the campaign page, which
also covers an event with a preset craft (question 10). It reads nothing out of the run
bytes, so the byte layout lives in the module alone, and a stale cache shows the last
true values. Rows on a title's Ship Select are a **follow-up**: what is below describes
that panel and the two styles a follow-up would draw there.

**Where the title shows its own bars.** The bars are authored ratings, not derived
from handling. Pulse authors `<FE speed thrust handling shield>` on each team and
draws four bars of ten segments with the number of lit segments beside each
(`oag_raceplay::catalogue::Team::rating`;
[selection-screens.md](../../ui/selection-screens.md), "Team Selection"). HD authors
the same four on each `PI_TeamModel`, in half points, and prints them as tenths
([campaign-screens.md](../../ui/campaign-screens.md), "The ratings are per model").
2048 authors `<FE speed thrust handling shield length firepower description>` on the
plugin's `PI_TeamModel` and draws `RC_SPEED`, `RC_HANDLING`, `RC_HEALTH`,
`RC_FIREPOWER` (`RC_SHIELD` and `RC_THRUST` in the 2048 binary's HD mode;
[team-grid.md](../../ghidra/functions/vita-2048-eu-v104/team-grid.md)). The
authored sets disagree for the same craft (2048's Feisar Fighter is speed 5 in the
plugin and 3 in `HandlingStats`), so a title bar is **never recomputed from scaled
handling**.

**Two display styles (Ship Select follow-up):**

- `show = "row"`: an extra labelled row under the title's bars, in the title's row
  style (name, ten segments, number).
- `show = "bonus"`: a tinted extra share on the title's own bar named by
  `bonus_of`, bounded to the bar's ten segments, after `hud-bar-extra` on the HUD.
  `firepower` and `length` exist on 2048 and Omega only; on another title the stat
  is dropped with a load-report line. The authored base bar is untouched.

**The narrowing (follow-up, not decided).** README section 3 and ADR-0080 say a disc screen never gains
behaviour a mod invented, and Ship Select is a disc screen. The follow-up would narrow
this by one allowance: **a campaign that declares `[[run.stat]]` adds display-only rows and
bonus segments to the stat panel of the Ship Select it launches, for that campaign
only.** No input, no new page, no change to what the screen does. Without the
allowance the stats could live only on a mod screen's canvas, which means the player
sees the craft and the title's bars on one screen and the mod's stats on another.

*Rejected: canvas only.* A screen would have to reproduce the title's bars and pose,
which needs the disc data README open question 15 has not decided a mod may read.

**Preset craft.** A campaign event with `craft = { team = ... }` skips Team Selection
(custom-campaign.md, "Preset craft"). Its stats show in the campaign page's stats
block, as for every event (question 10, settled 2026-11-20).

## 9. A worked example

A campaign "long-haul". The run state is three bytes: engine level, shield level,
and a recipe for the combined pickup. Two offers after each race.

```rust
// Names are illustrative; the shapes are the signatures above.
use oag_ext::{blob, prelude::*};

blob! { struct Haul { version: u8, engine: u8, shield: u8, recipe: u8, offer: [u8; 2], cleared: u64 } }
blob! { struct Tick { recipe: u8 } }   // what the race needs from the run

struct LongHaul;

impl Extension for LongHaul {
    type State = Tick;
    type Run = Haul;

    fn tick(obs: &Observation<'_>, state: &mut Tick, reply: &mut Reply<'_>) {
        for event in obs.events() {
            if let Event::PickupFire { slot } = event {          // the mod's own pickup was pressed
                let Some(craft) = obs.craft(slot) else { continue };
                let _ = reply.give_shield(&craft);
                if state.recipe > 0 {
                    let _ = reply.give_autopilot(&craft);        // new, code 42
                }
                let _ = reply.clear_pickup(&craft);
            }
        }
    }

    fn race_start(s: &Start<'_, Haul>, state: &mut Tick, out: &mut StartReply<'_>) {
        let h = s.run();                                          // all zeroes when the run is new
        state.recipe = h.recipe;                                  // the first tick sees this
        for craft in s.crafts().filter(GridCraft::is_person) {
            // Out of range clamps; an Err here is a bug the author's tests should catch.
            out.modifier(&craft, Param::TopEnd, 1.0 + 0.1 * f32::from(h.engine)).expect("one TopEnd");
            out.modifier(&craft, Param::ShieldMax, 1.0 + 0.25 * f32::from(h.shield)).expect("one ShieldMax");
            out.pickup_duration(&craft, Weapon::Shield, 1.0 + 0.5 * f32::from(h.recipe)).expect("one Shield");
        }
    }

    fn race_end(e: &End<'_>, _latch: &Tick, h: &mut Haul, out: &mut EndReply<'_>) {
        let r = e.result();
        let first_clear = r.prior_tier == 255 && r.tier != 255; // a replay of a cleared event pays nothing
        if first_clear {
            h.cleared |= 1 << r.event_index;
            let roll = splitmix64(e.seed() ^ u64::from(e.ordinal())); // the mod's own pure hash of the run's seed
            let a = (roll % 3) as u8;
            h.offer = [a, (a + 1 + ((roll >> 8) % 2) as u8) % 3];   // two different rewards: 0 engine, 1 shield, 2 recipe
            out.offer();
        }
        let _ = out.stat("STAT_ENGINE", h.engine.into(), 10);
        let _ = out.stat("STAT_SHIELD", h.shield.into(), 10);
    }
}
```

The screen's `event` handler turns accept on offer `n` into `h.engine += 1` (or the
shield or recipe) and returns `reply.run`. The next `race_start` reads it and copies
the recipe into the tick state, which is how `tick` knows it.

## 10. Titles

Custom campaigns exist for Pulse, HD and Omega. Not 2048 or Pure
([custom-campaign.md](../../formats/custom-campaign.md)).

| Capability | Pulse | HD / Fury | Omega | 2048 | Pure |
| --- | --- | --- | --- | --- | --- |
| Run state, modifiers, pickup params | `Handling` is the one shared struct, so a modifier needs no title code | same | same | no custom campaign yet | no custom campaign yet |
| Rocket and mine count | defaults 3 and 5, none authored | same | authored per craft (`number_of_rockets`, `number_of_mines`) | authored per craft | defaults |
| Stat panel rows | four bars per team, ten segments | four bars per `PI_TeamModel`, half points | not read in this lane: Omega's Ship Select is HD's front end, and `team-grid.md` records the 2048 panel on Omega as *checked, applies, not wired* | six authored values, four or six rows drawn | not read |

2048 and Omega share this lineage, so the 2048/Omega check of CLAUDE.md applies when
the build lands: the counts are already authored on both; the panel rows are the
open one. Outcome today for this draft: **checked, applies, not wired** for the
counts; **not checkable until the build** for the panel on Omega.

## 11. Order of building (proposed)

Prerequisite, outside this design: the records file is written atomically (temp
file, then rename), under a general ADR for every profile file (maintainer,
2026-11-20; [progression-challenge.md](progression-challenge.md), objection 7).

Each step is its own merge, with a proof that a race with no run module hashes
exactly as before (`VANILLA_FOLD`).

1. The ABI: `oag_run` and the two exports, `RunResult`, codes 39 to 41, revision 7,
   the SDK `Run` trait and its test harness. No engine meaning yet.
2. The run record in the `Store`, the `[run]` table, `NEW RUN`, the commit on exit, the
   replay `run` field, the no-record rule.
3. `set-modifier` for the settled parameters, `SlotMods` in the hash; a
   disc-backed ground truth in the shape of `energy_turbo_ground_truth` (a +20 %
   `TopEnd`, a +20 % `Launch` and a doubled `ShieldMax` against stock on one circuit).
4. Pickup parameters: counts, then the per-ship duration scales; `give-autopilot`
   with its lineage row.
5. The frontend world (ADR-0080), then the choice screen and the Ship Select
   allowance.

Steps 1 to 4 need no frontend world. A campaign can ship a fixed progression table
chosen by `race-end` alone (the module picks the reward) before step 5 lands.

## 12. Open questions

1. **Clamp or fault at `race-start`?** **Settled (maintainer, 2026-11-19): clamp.**
   An out-of-range factor is clamped to the host's range and named in the load report
   (section 4.1), never a fault.
2. **Who owns NEW RUN?** A host row on the campaign page (this draft) or a mod
   screen action. A host row works with no screen and cannot be hidden by a mod.
3. **Does a failed race commit?** **Settled (maintainer, 2026-11-19): the mod
   decides.** `race-end` gets `finished` and `tier` and chooses whether the run goes
   on; the host imposes no rule. Open only in mechanism: how the mod says "the run is
   over" (a flag in the reply, a status byte the host reads, or the mod's own screen).
4. **A modded records table.** **Settled (maintainer, 2026-11-20): later, outside
   ADR-0093.** The "no `[[record]]` for a modified race" rule drops personal bests
   until a separate modded table (README phase 2) is built; it is already owed in the
   custom race modes thread.
5. **May the player skip the offer?** **Settled (maintainer, 2026-11-19): the mod
   decides**, per offer (skippable, or blocking until a pick); the host offers both.
6. **Do `TopEnd` and `scale-thrust` compose freely?** They are different levers:
   `TopEnd` scales the plateau arm inside the law, `scale-thrust` the finished force
   after it. At the plateau both multiply the force, so the largest effective factor
   is 2.0 x 2.0 = 4.0. The alternative bounds the product by `MAX_THRUST_FACTOR`, so a
   reward would shrink the headroom of a per-tick boost.
7. **A data-first modifier table** (`[[run.modifier]]`) beside the guest path, for
   authors who do not want to write wasm (the project prefers rules as data where
   data can say it).
8. **Per-owner damage scale.** Needs the `Craft` trait and projectile ownership; not
   in this draft.
9. **The AI and a mod pickup.** The Turbo's gate is wrong for a defensive combination.
   Options: the manifest names a gate (`never`, `turbo`, `under-fire`), or the AI never
   holds mod pickups in a run campaign.
10. **Which page shows stats when Team Selection is skipped** (preset craft).
    **Settled (maintainer, 2026-11-20):** the campaign page's stats block, which every
    event has (section 8.2).
11. **Slots beyond the person's.** **Settled (maintainer, 2026-11-19): allowed.**
    `set-modifier` accepts any grid slot, so a campaign may scale the AI's craft too
    (README section 3: a third-party mod is the player's choice). **Settled
    (2026-11-20):** a scaled craft adapts the reference speed plan per slot before the
    plans are built (section 4.1).
12. **`TopSpeed`.** **Settled (maintainer, 2026-11-20):** replaced by `Launch`
    (`accelcap`) and `TopEnd` (`engine.amount`), each gated on its own held-throttle
    ground truth; neither applies in Zone (section 4.2).
13. **The run seed's source.** Drawn from the OS at run creation and stored. A
    campaign that wants the same rolls on every run needs a fixed seed in the `[run]`
    table.
14. **The cap.** 1,024 bytes fits the second buffer and the copy into the tick blob.
    A campaign with a large unlock table would want more.
15. **ADR-0080's fate.** **Settled for ADR-0093 (maintainer, 2026-11-20):** the
    choice screen and the stats are split into ADR-0094, so the run, modifiers and
    pickups no longer wait for the frontend world. ADR-0094 still does, as drafted.
16. **A per-seat run on the wire**, with README open question 13.
