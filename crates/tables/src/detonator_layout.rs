//! `DetonatorLayouts/PatternFile.xml` and `GamePlayValues.xml`: what Detonator lays on a circuit.
//!
//! Two global files, the same on every circuit and, once line endings are dropped, on HD, 2048
//! and Omega (`docs/ghidra/functions/ps3-hdfury-eu/detonator-placement.md`, "Where the data
//! comes from"). [`PatternFile`] is a list of groups of clusters; [`GamePlayValues`] carries the
//! per-stage mine counts and the numbers the placement reads.
//!
//! ```text
//! <Pattern stageToAppearFrom>
//!   <Cluster1 numOfObjects distanceBetweenObjects type percentageAcrossTrack
//!             angleUp angleRight distanceFromOriginalPosition object1Type .. object10Type/>
//!   <Cluster2 .../>
//! </Pattern>                                                   x18 on the disc
//! <GamePlayValues RemoveMinesEachStage ... MineCountStage1 .. MineCountStage14/>
//! ```
//!
//! **A missing or unparseable attribute is the original's own zero**, not an error: the loader
//! clears a record and overwrites what the file names, reading integers as `atoi` and floats as
//! `atof` (`Detonator_LoadPatternFile`, `0x00063510`). [`GamePlayValues::default`] is the static
//! block the executable ships before the file overwrites it (`g_DetonatorGamePlayValues`).

use crate::fexml;

/// Most objects one cluster lists (`object1Type`..`object10Type`).
pub const OBJECTS_PER_CLUSTER: usize = 10;

/// Stages that author a mine count.
pub const STAGES: usize = 14;

/// One `<ClusterN>`: a run of objects down the track in one lane.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Cluster {
    /// `numOfObjects`.
    pub num_objects: i32,
    /// `distanceBetweenObjects`: track entries between objects of the run.
    pub distance_between_objects: i32,
    /// `type`; the placement lays a cluster only when it is `0`.
    pub kind: i32,
    /// `percentageAcrossTrack`: parsed as a float, kept truncated, as the original stores it;
    /// the lane offset from the centre lane.
    pub percentage_across_track: i32,
    /// `angleUp`. Parsed; the placement never passes it on.
    pub angle_up: f32,
    /// `angleRight`. Parsed; the placement never passes it on.
    pub angle_right: f32,
    /// `distanceFromOriginalPosition`: track entries from the group's base position.
    pub distance_from_original_position: i32,
    /// `object1Type`..`object10Type`: `0` a mine, `1` a bomb.
    pub object_types: [i32; OBJECTS_PER_CLUSTER],
}

/// One `<Pattern>`: a group of clusters that may appear from a stage on.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    /// `stageToAppearFrom`, `1` when the file names none.
    pub stage_to_appear_from: i32,
    /// The group's children in document order.
    pub clusters: Vec<Cluster>,
}

/// Every `<Pattern>` of `PatternFile.xml`, in document order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PatternFile {
    /// The groups.
    pub patterns: Vec<Pattern>,
}

/// C `atoi`: leading whitespace, an optional sign, digits; anything else is `0`.
fn atoi(text: &str) -> i32 {
    let text = text.trim_start();
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let end = digits
        .bytes()
        .position(|b| !b.is_ascii_digit())
        .unwrap_or(digits.len());
    let magnitude = digits[..end].parse::<i64>().unwrap_or(0);
    let signed = if negative { -magnitude } else { magnitude };
    i32::try_from(signed).unwrap_or(0)
}

/// C `atof`, for the plain decimals these files author; anything else is `0.0`.
fn atof(text: &str) -> f32 {
    text.trim()
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
        .unwrap_or(0.0)
}

fn int(node: &fexml::Node, name: &str) -> i32 {
    node.attr(name).map_or(0, atoi)
}

fn float(node: &fexml::Node, name: &str) -> f32 {
    node.attr(name).map_or(0.0, atof)
}

fn cluster(node: &fexml::Node) -> Cluster {
    let mut object_types = [0; OBJECTS_PER_CLUSTER];
    for (slot, value) in object_types.iter_mut().enumerate() {
        *value = int(node, &format!("object{}Type", slot + 1));
    }
    Cluster {
        num_objects: int(node, "numOfObjects"),
        distance_between_objects: int(node, "distanceBetweenObjects"),
        kind: int(node, "type"),
        percentage_across_track: float(node, "percentageAcrossTrack").trunc() as i32,
        angle_up: float(node, "angleUp"),
        angle_right: float(node, "angleRight"),
        distance_from_original_position: int(node, "distanceFromOriginalPosition"),
        object_types,
    }
}

/// Reads `PatternFile.xml`. A document with no `<Pattern>` yields no groups.
#[must_use]
pub fn parse_patterns(xml: &str) -> PatternFile {
    let root = fexml::parse(xml);
    let patterns = root
        .children_named("Pattern")
        .map(|group| Pattern {
            stage_to_appear_from: group.attr("stageToAppearFrom").map_or(1, atoi),
            clusters: group.children.iter().map(cluster).collect(),
        })
        .collect();
    PatternFile { patterns }
}

/// `GamePlayValues.xml`: the placement's constants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GamePlayValues {
    /// `RemoveMinesEachStage`.
    pub remove_mines_each_stage: bool,
    /// `DoMusicResponsePreviousStageMines`.
    pub do_music_response_previous_stage_mines: bool,
    /// `DistanceOffsetBetweenMines`: the lane width a lane offset multiplies.
    pub distance_offset_between_mines: f32,
    /// `NumStepsToIgnoreAfterJump`: track entries a jump blanks out.
    pub num_steps_to_ignore_after_jump: i32,
    /// `MineCountsRepresentMaxTotalToAppear`: a stage's count is a total that includes the mines
    /// still alive, not a number to add.
    pub mine_counts_represent_max_total_to_appear: bool,
    /// `EnergyRechargePerStage`.
    pub energy_recharge_per_stage: i32,
    /// `PointsForChainReactionMines`.
    pub points_for_chain_reaction_mines: i32,
    /// `BombChainReactionDistance`. The original parses it and drops it.
    pub bomb_chain_reaction_distance: f32,
    /// `CannonLockoutTimeFromManualReload`.
    pub cannon_lockout_time_from_manual_reload: f32,
    /// `CannonLockoutTimeRunOutBullets`.
    pub cannon_lockout_time_run_out_bullets: f32,
    /// `MineCountStage1`..`MineCountStage14`.
    pub mine_count_stage: [i32; STAGES],
    /// `MinesScaleToMusic`: laid mines grow and shrink with the music. No shipped file names
    /// it, so a disc's own layout scales; a layout that sets `0` holds its mines at their
    /// authored size (`docs/gameplay/detonator-pulse.md`, "Mine size").
    pub mines_scale_to_music: bool,
}

impl Default for GamePlayValues {
    /// The executable's static block: what a title keeps for every attribute its file omits
    /// (`g_DetonatorGamePlayValues`, `0x008c1478`). The three values the original drops or keeps
    /// elsewhere have no static default read here and are `0`.
    fn default() -> Self {
        let mut mine_count_stage = [0; STAGES];
        for (stage, count) in mine_count_stage.iter_mut().enumerate() {
            *count = 70 + 10 * stage as i32;
        }
        Self {
            remove_mines_each_stage: false,
            do_music_response_previous_stage_mines: false,
            distance_offset_between_mines: 3.5,
            num_steps_to_ignore_after_jump: 20,
            mine_counts_represent_max_total_to_appear: true,
            energy_recharge_per_stage: 5,
            points_for_chain_reaction_mines: 100,
            bomb_chain_reaction_distance: 0.0,
            cannon_lockout_time_from_manual_reload: 0.0,
            cannon_lockout_time_run_out_bullets: 0.0,
            mine_count_stage,
            mines_scale_to_music: true,
        }
    }
}

impl GamePlayValues {
    /// The mine count the stage `stage` (1-based, `1..=14`) aims for, or `None` outside it.
    #[must_use]
    pub fn mine_count(&self, stage: usize) -> Option<i32> {
        stage
            .checked_sub(1)
            .and_then(|index| self.mine_count_stage.get(index))
            .copied()
    }
}

/// `DetonatorScoring.xml`: what a shot pays
/// (`docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, "The score").
///
/// ```text
/// <DetonatorScoring>
///   <Bullet multiplier new_mine_score old_mine_score bomb_hit_score bomb_score bomb_increment/>
///   <Emp new_mine_score old_mine_score bomb_score/>
/// </DetonatorScoring>
/// ```
///
/// A missing attribute is the original's zero, as in the layout files.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Scoring {
    /// `Bullet multiplier`: what a live mine's multiplier grows by per live mine shot.
    pub multiplier: f32,
    /// `Bullet new_mine_score`: a live mine, before the multiplier.
    pub new_mine_score: i32,
    /// `Bullet old_mine_score`: a dark mine.
    pub old_mine_score: i32,
    /// `Bullet bomb_hit_score`: a bomb hit that does not destroy it. Nothing calls its adder.
    pub bomb_hit_score: i32,
    /// `Bullet bomb_score`: the first bomb the cannon destroys.
    pub bomb_score: i32,
    /// `Bullet bomb_increment`: added per bomb already destroyed.
    pub bomb_increment: i32,
    /// `Emp new_mine_score`.
    pub emp_new_mine_score: i32,
    /// `Emp old_mine_score`.
    pub emp_old_mine_score: i32,
    /// `Emp bomb_score`.
    pub emp_bomb_score: i32,
}

/// Reads `DetonatorScoring.xml`.
#[must_use]
pub fn parse_scoring(xml: &str) -> Scoring {
    let document = fexml::parse(xml);
    let root = document
        .children_named("DetonatorScoring")
        .next()
        .unwrap_or(&document);
    let mut scoring = Scoring::default();
    if let Some(bullet) = root.children_named("Bullet").next() {
        scoring.multiplier = float(bullet, "multiplier");
        scoring.new_mine_score = int(bullet, "new_mine_score");
        scoring.old_mine_score = int(bullet, "old_mine_score");
        scoring.bomb_hit_score = int(bullet, "bomb_hit_score");
        scoring.bomb_score = int(bullet, "bomb_score");
        scoring.bomb_increment = int(bullet, "bomb_increment");
    }
    if let Some(emp) = root.children_named("Emp").next() {
        scoring.emp_new_mine_score = int(emp, "new_mine_score");
        scoring.emp_old_mine_score = int(emp, "old_mine_score");
        scoring.emp_bomb_score = int(emp, "bomb_score");
    }
    scoring
}

fn flag(node: &fexml::Node, name: &str, otherwise: bool) -> bool {
    node.attr(name).map_or(otherwise, |text| atoi(text) != 0)
}

/// Reads `GamePlayValues.xml`: the attributes the file names replace
/// [`GamePlayValues::default`]'s, the rest keep it.
#[must_use]
pub fn parse_values(xml: &str) -> GamePlayValues {
    let root = fexml::parse(xml);
    let mut values = GamePlayValues::default();
    let Some(node) = root.children_named("GamePlayValues").next() else {
        return values;
    };
    values.remove_mines_each_stage =
        flag(node, "RemoveMinesEachStage", values.remove_mines_each_stage);
    values.do_music_response_previous_stage_mines = flag(
        node,
        "DoMusicResponsePreviousStageMines",
        values.do_music_response_previous_stage_mines,
    );
    if node.attr("DistanceOffsetBetweenMines").is_some() {
        values.distance_offset_between_mines = float(node, "DistanceOffsetBetweenMines");
    }
    if node.attr("NumStepsToIgnoreAfterJump").is_some() {
        values.num_steps_to_ignore_after_jump = int(node, "NumStepsToIgnoreAfterJump");
    }
    values.mine_counts_represent_max_total_to_appear = flag(
        node,
        "MineCountsRepresentMaxTotalToAppear",
        values.mine_counts_represent_max_total_to_appear,
    );
    if node.attr("EnergyRechargePerStage").is_some() {
        values.energy_recharge_per_stage = int(node, "EnergyRechargePerStage");
    }
    if node.attr("PointsForChainReactionMines").is_some() {
        values.points_for_chain_reaction_mines = int(node, "PointsForChainReactionMines");
    }
    if node.attr("BombChainReactionDistance").is_some() {
        values.bomb_chain_reaction_distance = float(node, "BombChainReactionDistance");
    }
    if node.attr("CannonLockoutTimeFromManualReload").is_some() {
        values.cannon_lockout_time_from_manual_reload =
            float(node, "CannonLockoutTimeFromManualReload");
    }
    if node.attr("CannonLockoutTimeRunOutBullets").is_some() {
        values.cannon_lockout_time_run_out_bullets = float(node, "CannonLockoutTimeRunOutBullets");
    }
    values.mines_scale_to_music = flag(node, "MinesScaleToMusic", values.mines_scale_to_music);
    for (index, count) in values.mine_count_stage.iter_mut().enumerate() {
        if let Some(text) = node.attr(&format!("MineCountStage{}", index + 1)) {
            *count = atoi(text);
        }
    }
    values
}

#[cfg(test)]
mod tests;
