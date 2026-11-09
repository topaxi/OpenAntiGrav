//! Resolves a race's Eliminator rule once at load: the title's awards with the opponents' scale
//! for the race's difficulty, and the target a Racebox race starts on. The race carries the
//! result as a plain value and reads no title.
//! `docs/ghidra/functions/ps3-hdfury-eu/eliminator-score.md`.

use oag_tables::race_campaign::Difficulty;
use oag_title::eliminator::EliminatorRule;

/// A race's Eliminator rule, resolved.
pub(crate) struct Resolved {
    /// The awards, `None` for a race won on kills.
    pub(crate) law: Option<oag_race::ScoreLaw>,
    /// The target, `None` for the mode's default.
    pub(crate) target: Option<u32>,
    /// The flip, `None` outside an Eliminator and on a title without one.
    pub(crate) flip: Option<oag_physics::flip::FlipLaw>,
}

/// The race's rule and target, once, at load.
pub(crate) fn of(
    options: &crate::Options,
    title: &oag_title::Title,
    stats: &Option<oag_tables::weapons::ai::WeaponAiStats>,
    archives: &mut oag_assets::Archives,
    report: &mut Vec<String>,
) -> Resolved {
    let rule = title.eliminator.value;
    Resolved {
        // The original's awards all sit behind `mode == 8`.
        law: resolve(rule, stats.as_ref(), options.difficulty)
            .filter(|_| options.mode == oag_race::Mode::Eliminator),
        target: target(options, rule, archives, report),
        // The original's block sits behind the same mode test.
        flip: title
            .eliminator_flip
            .value
            .filter(|_| options.mode == oag_race::Mode::Eliminator)
            .map(|rule| oag_physics::flip::FlipLaw {
                seconds: rule.seconds,
            }),
    }
}

/// The awards `rule` gives at `difficulty`. A table with no `<EliminatorAIStats>` row leaves an
/// opponent's scale at `1.0`.
fn resolve(
    rule: EliminatorRule,
    stats: Option<&oag_tables::weapons::ai::WeaponAiStats>,
    difficulty: oag_ai::Difficulty,
) -> Option<oag_race::ScoreLaw> {
    let EliminatorRule::Score(rule) = rule else {
        return None;
    };
    let tier = match crate::finished_thrust::rung(difficulty) {
        Difficulty::Easy => 0,
        Difficulty::Medium => 1,
        Difficulty::Hard => 2,
    };
    Some(oag_race::ScoreLaw {
        damage: rule.damage,
        kill: rule.kill,
        lap: rule.lap,
        death: rule.death,
        opponent_scale: stats
            .and_then(oag_tables::weapons::ai::WeaponAiStats::eliminator)
            .map_or(1.0, |row| row.score_scale[tier]),
    })
}

/// The target a race starts on: a kill rule's pick, or a score rule's own, a campaign cell's or
/// the Racebox minimum off the disc.
///
/// With no authored minimum a score race has no target at all and says so in the report; a kill
/// count's fallback would end it on the first five points.
fn target(
    options: &crate::Options,
    rule: EliminatorRule,
    archives: &mut oag_assets::Archives,
    report: &mut Vec<String>,
) -> Option<u32> {
    let EliminatorRule::Score(rule) = rule else {
        return options.eliminator_kill_target;
    };
    if options.mode != oag_race::Mode::Eliminator {
        return None;
    }
    if options.eliminator_score_target.is_some() {
        return options.eliminator_score_target;
    }
    let authored = archives
        .read_name(rule.race_setup)
        .ok()
        .and_then(|blob| oag_tables::fexml::text(&blob).ok())
        .and_then(|xml| oag_tables::race_setup::slider(&xml, "Eliminations"))
        .map(|slider| slider.min);
    if authored.is_none() {
        report.push(format!(
            "{}: no Eliminations slider - this Eliminator has no target",
            rule.race_setup
        ));
    }
    Some(authored.unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_title::eliminator::ScoreRule;

    const RULE: ScoreRule = ScoreRule {
        damage: 0.1,
        kill: 20.0,
        lap: 5.0,
        death: 5.0,
        race_setup: "",
    };

    #[test]
    fn a_kill_rule_resolves_to_no_law() {
        let law = resolve(EliminatorRule::Kills, None, oag_ai::Difficulty::Novice);
        assert_eq!(law, None);
    }

    #[test]
    fn a_table_with_no_row_leaves_the_opponents_flat() {
        let law = resolve(EliminatorRule::Score(RULE), None, oag_ai::Difficulty::Elite);
        assert_eq!(law.map(|law| law.opponent_scale), Some(1.0));
    }
}
