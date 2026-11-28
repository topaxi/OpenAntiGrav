//! What the window shows, as a stack of activities (ADR-0099, decision 3).
//!
//! The top entry is drawn, ticked and receives input; every entry under it is alive and paused.
//! Four exits write the stack and nothing else does: [`ActivityStack::push`],
//! [`ActivityStack::pop`], [`ActivityStack::replace`] and [`ActivityStack::unwind_to`]. Each hands
//! back what it removed rather than dropping it in place, so the caller decides where a race
//! scene is released (`crate::app::race_build::drop_off_thread`). Telling the entry underneath
//! it is resumed is the session's (`Session::resumed`).
//!
//! An entry names the run it belongs to by [`RunId`], not by owning it: a race owns its
//! `RunContext` (`RaceStage::run`, ADR-0099 decision 2), and a pause menu pushed over it belongs
//! to the same run. A popped entry's run has ended when no entry left on the stack names it
//! ([`Popped::run_ended`]).

/// One launched run, the same across every relaunch of it (`RunContext::id`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RunId(u64);

impl Default for RunId {
    /// A run never seen before.
    fn default() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

/// What can sit on the stack: something with a kind to unwind to.
pub(crate) trait Activity {
    type Kind: Copy + PartialEq;
    fn kind(&self) -> Self::Kind;
}

/// One activity and the run it belongs to, if any.
pub(crate) struct Entry<T> {
    pub(crate) activity: T,
    pub(crate) run: Option<RunId>,
}

/// An entry taken off the stack, and whether its run went with it.
pub(crate) struct Popped<T> {
    pub(crate) entry: Entry<T>,
    /// The entry named a run and no entry left on the stack does.
    pub(crate) run_ended: bool,
}

/// Never empty: the bottom entry is only ever replaced.
pub(crate) struct ActivityStack<T> {
    entries: Vec<Entry<T>>,
}

impl<T: Activity> ActivityStack<T> {
    pub(crate) fn new(first: T, run: Option<RunId>) -> Self {
        Self {
            entries: vec![Entry {
                activity: first,
                run,
            }],
        }
    }

    fn top_entry(&self) -> &Entry<T> {
        self.entries.last().expect("the stack is never empty")
    }

    /// What is on screen.
    pub(crate) fn top(&self) -> &T {
        &self.top_entry().activity
    }

    pub(crate) fn top_mut(&mut self) -> &mut T {
        &mut self
            .entries
            .last_mut()
            .expect("the stack is never empty")
            .activity
    }

    /// The run the top entry belongs to.
    pub(crate) fn top_run(&self) -> Option<RunId> {
        self.top_entry().run
    }

    /// How many entries there are, the top included.
    #[cfg(test)]
    pub(crate) fn depth(&self) -> usize {
        self.entries.len()
    }

    /// How many entries are under the top.
    pub(crate) fn depth_under_top(&self) -> usize {
        self.entries.len() - 1
    }

    /// The entry `depth` below the top (0 is the top).
    pub(crate) fn below(&self, depth: usize) -> Option<&T> {
        let index = self.entries.len().checked_sub(depth + 1)?;
        Some(&self.entries[index].activity)
    }

    pub(crate) fn below_mut(&mut self, depth: usize) -> Option<&mut T> {
        let index = self.entries.len().checked_sub(depth + 1)?;
        Some(&mut self.entries[index].activity)
    }

    /// Every activity, bottom first.
    #[cfg(test)]
    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().map(|entry| &entry.activity)
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.entries.iter_mut().map(|entry| &mut entry.activity)
    }

    /// The topmost activity of `kind` whose entry is directly under one of `over`, with its
    /// depth below the top.
    pub(crate) fn find_under(&self, kind: T::Kind, over: T::Kind) -> Option<usize> {
        (1..self.entries.len()).find_map(|depth| {
            let under = self.below(depth)?;
            let above = self.below(depth - 1)?;
            (under.kind() == kind && above.kind() == over).then_some(depth)
        })
    }

    fn names(&self, run: RunId) -> bool {
        self.entries.iter().any(|entry| entry.run == Some(run))
    }

    /// A new activity over the top, which stays alive and paused.
    pub(crate) fn push(&mut self, activity: T, run: Option<RunId>) {
        self.entries.push(Entry { activity, run });
    }

    /// Back to the entry underneath. `None` on the bottom entry, which is only ever replaced.
    pub(crate) fn pop(&mut self) -> Option<Popped<T>> {
        if self.entries.len() < 2 {
            return None;
        }
        let entry = self.entries.pop()?;
        let run_ended = entry.run.is_some_and(|run| !self.names(run));
        Some(Popped { entry, run_ended })
    }

    /// The top entry ends and `activity` takes its place. Its run ended unless the new entry
    /// or one underneath names it.
    pub(crate) fn replace(&mut self, activity: T, run: Option<RunId>) -> Popped<T> {
        let top = self.entries.last_mut().expect("the stack is never empty");
        let entry = std::mem::replace(top, Entry { activity, run });
        let run_ended = entry.run.is_some_and(|run| !self.names(run));
        Popped { entry, run_ended }
    }

    /// Pops until an entry of `kind` is on top, topmost first in the result. Nothing is popped
    /// when no entry is of that kind.
    pub(crate) fn unwind_to(&mut self, kind: T::Kind) -> Vec<Popped<T>> {
        let Some(depth) = (0..self.entries.len())
            .find(|&depth| self.below(depth).is_some_and(|a| a.kind() == kind))
        else {
            return Vec::new();
        };
        (0..depth).filter_map(|_| self.pop()).collect()
    }
}

/// Whether coming back to a race re-arms its result: only one still running. A finished race's
/// result was saved when it finished, and a pop back to it must not record it twice (ADR-0099,
/// decision 4).
pub(crate) fn rearms_on_resume(finished: bool) -> bool {
    !finished
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Kind {
        Menu,
        Race,
        Loading,
    }

    #[derive(Debug, PartialEq)]
    struct Fake(Kind, u8);

    impl Activity for Fake {
        type Kind = Kind;
        fn kind(&self) -> Kind {
            self.0
        }
    }

    fn tops(stack: &ActivityStack<Fake>) -> Vec<u8> {
        stack.iter().map(|fake| fake.1).collect()
    }

    #[test]
    fn a_push_keeps_the_entry_underneath_and_a_pop_comes_back_to_it() {
        let mut stack = ActivityStack::new(Fake(Kind::Race, 1), None);
        stack.push(Fake(Kind::Menu, 2), None);
        assert_eq!(stack.top(), &Fake(Kind::Menu, 2));
        assert_eq!(stack.below(1), Some(&Fake(Kind::Race, 1)));
        let popped = stack.pop().expect("two entries");
        assert_eq!(popped.entry.activity, Fake(Kind::Menu, 2));
        assert_eq!(stack.top(), &Fake(Kind::Race, 1));
        assert_eq!(stack.depth(), 1);
    }

    #[test]
    fn the_bottom_entry_cannot_be_popped_only_replaced() {
        let mut stack = ActivityStack::new(Fake(Kind::Menu, 1), None);
        assert!(stack.pop().is_none());
        let old = stack.replace(Fake(Kind::Loading, 2), None);
        assert_eq!(old.entry.activity, Fake(Kind::Menu, 1));
        assert_eq!(tops(&stack), [2]);
    }

    #[test]
    fn unwind_pops_to_the_topmost_entry_of_a_kind_and_leaves_it_alone_when_there_is_none() {
        let mut stack = ActivityStack::new(Fake(Kind::Menu, 1), None);
        stack.push(Fake(Kind::Race, 2), None);
        stack.push(Fake(Kind::Menu, 3), None);
        stack.push(Fake(Kind::Loading, 4), None);
        assert!(stack.unwind_to(Kind::Loading).is_empty(), "already on top");
        let popped: Vec<u8> = stack
            .unwind_to(Kind::Race)
            .into_iter()
            .map(|p| p.entry.activity.1)
            .collect();
        assert_eq!(popped, [4, 3]);
        assert_eq!(tops(&stack), [1, 2]);
        let mut lone = ActivityStack::new(Fake(Kind::Race, 1), None);
        assert!(lone.unwind_to(Kind::Menu).is_empty());
        assert_eq!(tops(&lone), [1]);
    }

    #[test]
    fn a_run_ends_with_the_last_entry_that_names_it() {
        let (race, watch) = (RunId::default(), RunId::default());
        assert_ne!(race, watch);
        let mut stack = ActivityStack::new(Fake(Kind::Race, 1), Some(race));
        // A pause menu belongs to the race's run: popping it leaves the run.
        stack.push(Fake(Kind::Menu, 2), Some(race));
        assert!(!stack.pop().expect("pause menu").run_ended);
        // A watch over the finished race is a run of its own: popping it ends that one.
        stack.push(Fake(Kind::Loading, 3), Some(watch));
        let replaced = stack.replace(Fake(Kind::Race, 4), Some(watch));
        assert!(!replaced.run_ended, "the load became the watch's race");
        let popped = stack.pop().expect("watch");
        assert!(popped.run_ended);
        assert_eq!(popped.entry.run, Some(watch));
        assert_eq!(stack.top_run(), Some(race));
        // The race replaced by the menus ends its run.
        assert!(stack.replace(Fake(Kind::Menu, 5), None).run_ended);
        // An entry with no run ends none.
        stack.push(Fake(Kind::Loading, 6), None);
        assert!(!stack.pop().expect("load").run_ended);
    }

    #[test]
    fn find_under_names_a_race_held_directly_under_the_menus() {
        let mut stack = ActivityStack::new(Fake(Kind::Race, 1), None);
        assert_eq!(stack.find_under(Kind::Race, Kind::Menu), None);
        stack.push(Fake(Kind::Menu, 2), None);
        assert_eq!(stack.find_under(Kind::Race, Kind::Menu), Some(1));
        stack.push(Fake(Kind::Loading, 3), None);
        assert_eq!(stack.find_under(Kind::Race, Kind::Menu), Some(2));
        let mut finished = ActivityStack::new(Fake(Kind::Race, 1), None);
        finished.push(Fake(Kind::Loading, 2), None);
        assert_eq!(finished.find_under(Kind::Race, Kind::Menu), None);
    }

    #[test]
    fn a_pop_back_to_a_finished_race_does_not_re_arm_its_result() {
        assert!(rearms_on_resume(false), "a paused race keeps racing");
        assert!(!rearms_on_resume(true), "a finished leg is recorded once");
    }
}
