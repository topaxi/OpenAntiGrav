//! What the activity stack holds under the stage on screen (ADR-0099).

use crate::app::activity_stack::{ActivityStack, RunId};
use crate::app::race_stage::RaceStage;

use super::{Stage, StageKind};

impl Stage {
    /// The run a stage belongs to on its own: a race's.
    pub(crate) fn own_run(&self) -> Option<RunId> {
        match self {
            Self::Race(stage) => Some(stage.run.id),
            _ => None,
        }
    }
}

impl ActivityStack<Stage> {
    /// How far below the top the race paused under the menus is: a race entry directly under a
    /// menu entry. A finished race held under a replay watch is not one.
    pub(crate) fn paused_race_depth(&self) -> Option<usize> {
        self.find_under(StageKind::Race, StageKind::Menu)
    }

    /// The race paused under the menus.
    pub(crate) fn paused_race(&self) -> Option<&RaceStage> {
        match self.below(self.paused_race_depth()?)? {
            Stage::Race(stage) => Some(stage),
            _ => None,
        }
    }

    pub(crate) fn paused_race_mut(&mut self) -> Option<&mut RaceStage> {
        let depth = self.paused_race_depth()?;
        match self.below_mut(depth)? {
            Stage::Race(stage) => Some(stage),
            _ => None,
        }
    }

    /// Takes the race paused under the menus off the stack, the menus staying on top: the menu
    /// entry is popped, then replaces the race. `None` when no race is paused directly under
    /// the menus on screen.
    pub(crate) fn take_paused_race(&mut self) -> Option<Box<RaceStage>> {
        if self.paused_race_depth() != Some(1) {
            return None;
        }
        let menu = self.pop()?.entry.activity;
        match self.replace(menu, None).entry.activity {
            Stage::Race(stage) => Some(stage),
            _ => None,
        }
    }

    /// `stage` takes the place of everything on the stack, which comes back topmost first: a
    /// pop of every entry above the bottom, then a replace of it.
    pub(crate) fn replace_all(&mut self, stage: Stage, run: Option<RunId>) -> Vec<Stage> {
        let mut gone: Vec<Stage> = std::iter::from_fn(|| self.pop())
            .map(|popped| popped.entry.activity)
            .collect();
        gone.push(self.replace(stage, run).entry.activity);
        gone
    }

    /// Every race under the top: paused under the menus, or finished under a replay watch.
    pub(crate) fn races_under_top_mut(&mut self) -> impl Iterator<Item = &mut RaceStage> {
        let under = self.depth_under_top();
        self.iter_mut().take(under).filter_map(|stage| match stage {
            Stage::Race(stage) => Some(&mut **stage),
            _ => None,
        })
    }

    /// Every race on the stack, the one on screen included.
    pub(crate) fn races_mut(&mut self) -> impl Iterator<Item = &mut RaceStage> {
        self.iter_mut().filter_map(|stage| match stage {
            Stage::Race(stage) => Some(&mut **stage),
            _ => None,
        })
    }
}
