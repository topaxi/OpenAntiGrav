//! A string-keyed hierarchical state machine, shaped like the original's.
//!
//! The original is **not** a switch on an integer. `StateMachine_TransitionTo`
//! (`0x0889123c`) takes a state *name* as a string and queries are `strcmp`
//! against the current name; 31 call sites fire transitions that way. Screens
//! nest, and a child is named `"Parent->Child"`: the intro state's `OnEnter`
//! caches `"DevPubRedirect"` and `"Intro Screen->IntroMovie1"` by name. See
//! `docs/ghidra/functions/psp-pulse-usa/main-loop.md`.
//!
//! Keeping that shape has a practical payoff beyond fidelity: transitions
//! recovered from the executable and from the front-end XML can be written down
//! verbatim, and a test can assert on the exact sequence of names.
//!
//! Nothing here knows about rendering, input or time.

use std::collections::HashMap;

/// The separator between a parent state and its child, as the original spells
/// it in its own string literals.
pub const SEPARATOR: &str = "->";

/// One state: a name, a parent, and nothing else.
#[derive(Debug, Clone)]
struct State {
    name: String,
    parent: Option<usize>,
}

/// What a transition did, in the order it happened.
///
/// Exits come before entries, and both are ordered outermost-last on the way
/// out and outermost-first on the way in. That ordering is load-bearing: the
/// Movie widget's teardown is a *consequence* of the intro state exiting, not a
/// peer of the transition, so anything reacting to these events has to see them
/// in the same order the original does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A state was left. Carries its full name.
    Exit(String),
    /// A state was entered. Carries its full name.
    Enter(String),
}

/// A hierarchical state machine keyed by name.
#[derive(Debug, Default)]
pub struct StateMachine {
    states: Vec<State>,
    by_name: HashMap<String, usize>,
    current: Option<usize>,
    /// A fired transition waiting to be applied.
    pending: Option<String>,
    /// Every state entered since construction, for tests and `--trace`.
    history: Vec<String>,
}

impl StateMachine {
    /// An empty machine.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a state, creating its ancestors if they do not exist yet.
    ///
    /// `"Intro Screen->IntroMovie1"` registers `"Intro Screen"` as well, which
    /// is what makes the original's own literals usable as-is.
    pub fn register(&mut self, path: &str) -> usize {
        if let Some(&index) = self.by_name.get(path) {
            return index;
        }

        let parent = path.rfind(SEPARATOR).map(|at| self.register(&path[..at]));

        let index = self.states.len();
        self.states.push(State {
            name: path.to_string(),
            parent,
        });
        self.by_name.insert(path.to_string(), index);
        index
    }

    /// Registers several states at once.
    pub fn register_all<'a>(&mut self, paths: impl IntoIterator<Item = &'a str>) {
        for path in paths {
            self.register(path);
        }
    }

    /// Whether a state with this name is registered.
    #[must_use]
    pub fn contains(&self, path: &str) -> bool {
        self.by_name.contains_key(path)
    }

    /// The current state's full name.
    #[must_use]
    pub fn current(&self) -> Option<&str> {
        self.current.map(|i| self.states[i].name.as_str())
    }

    /// The current state's own name, without its ancestors.
    #[must_use]
    pub fn current_leaf(&self) -> Option<&str> {
        self.current().map(leaf)
    }

    /// Whether the current state has exactly this full name.
    ///
    /// This is the original's own query: a `strcmp` against the current name.
    #[must_use]
    pub fn is(&self, path: &str) -> bool {
        self.current() == Some(path)
    }

    /// Whether the current state is `path` or nested inside it.
    #[must_use]
    pub fn is_in(&self, path: &str) -> bool {
        let Some(&target) = self.by_name.get(path) else {
            return false;
        };
        self.ancestry(self.current).contains(&target)
    }

    /// Queues a transition to `path`.
    ///
    /// Firing does not transition: the original's states fire from inside their
    /// own update, and tearing the state down underneath the code that fired
    /// would be a use-after-free. [`apply`](Self::apply) does the work, once,
    /// at the end of the frame.
    ///
    /// A second fire in the same frame replaces the first, which matches
    /// "queued transition" semantics and keeps the machine from ever owing two
    /// transitions at once.
    pub fn fire(&mut self, path: &str) {
        self.pending = Some(path.to_string());
    }

    /// Whether a transition is queued.
    #[must_use]
    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Applies the queued transition, if any, and reports what it did.
    ///
    /// An unregistered target is ignored rather than fatal: the original stores
    /// transition targets as strings and a name that resolves to nothing is a
    /// no-op there too.
    pub fn apply(&mut self) -> Vec<Event> {
        let Some(target) = self.pending.take() else {
            return Vec::new();
        };
        self.transition_to(&target)
    }

    /// Transitions immediately, bypassing the queue. For boot only.
    pub fn transition_to(&mut self, path: &str) -> Vec<Event> {
        let Some(&target) = self.by_name.get(path) else {
            return Vec::new();
        };
        if self.current == Some(target) {
            return Vec::new();
        }

        let from = self.ancestry(self.current);
        let to = self.ancestry(Some(target));

        // Everything both paths share stays entered. Only the divergent tails
        // exit and enter, which is what makes a nested screen able to swap a
        // child without rebuilding its parent.
        let shared = from
            .iter()
            .zip(to.iter())
            .take_while(|(a, b)| a == b)
            .count();

        let mut events = Vec::new();
        for &index in from[shared..].iter().rev() {
            events.push(Event::Exit(self.states[index].name.clone()));
        }
        for &index in &to[shared..] {
            events.push(Event::Enter(self.states[index].name.clone()));
            self.history.push(self.states[index].name.clone());
        }

        self.current = Some(target);
        events
    }

    /// Every state entered so far, oldest first.
    #[must_use]
    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// Ancestors of `index`, outermost first, including `index` itself.
    fn ancestry(&self, index: Option<usize>) -> Vec<usize> {
        let mut chain = Vec::new();
        let mut at = index;
        while let Some(i) = at {
            chain.push(i);
            at = self.states[i].parent;
        }
        chain.reverse();
        chain
    }
}

/// The last component of a `"Parent->Child"` name.
#[must_use]
pub fn leaf(path: &str) -> &str {
    path.rsplit(SEPARATOR).next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine() -> StateMachine {
        let mut sm = StateMachine::new();
        sm.register_all([
            "Intro Screen",
            "Intro Screen->IntroMovie1",
            "Language Selection",
            "LogoFMV",
            "Launch Game",
        ]);
        sm
    }

    #[test]
    fn registering_a_child_creates_its_parent() {
        let sm = machine();
        assert!(sm.contains("Intro Screen"));
        assert!(sm.contains("Intro Screen->IntroMovie1"));
    }

    #[test]
    fn entering_a_child_enters_its_parent_first() {
        let mut sm = machine();
        assert_eq!(
            sm.transition_to("Intro Screen->IntroMovie1"),
            vec![
                Event::Enter("Intro Screen".into()),
                Event::Enter("Intro Screen->IntroMovie1".into()),
            ]
        );
        assert_eq!(sm.current(), Some("Intro Screen->IntroMovie1"));
        assert_eq!(sm.current_leaf(), Some("IntroMovie1"));
    }

    #[test]
    fn leaving_a_child_exits_it_innermost_first() {
        let mut sm = machine();
        sm.transition_to("Intro Screen->IntroMovie1");
        assert_eq!(
            sm.transition_to("Language Selection"),
            vec![
                Event::Exit("Intro Screen->IntroMovie1".into()),
                Event::Exit("Intro Screen".into()),
                Event::Enter("Language Selection".into()),
            ]
        );
    }

    #[test]
    fn a_shared_parent_is_not_re_entered() {
        let mut sm = StateMachine::new();
        sm.register_all(["Top->A", "Top->B"]);
        sm.transition_to("Top->A");
        assert_eq!(
            sm.transition_to("Top->B"),
            vec![Event::Exit("Top->A".into()), Event::Enter("Top->B".into())]
        );
    }

    #[test]
    fn firing_queues_and_apply_performs() {
        let mut sm = machine();
        sm.fire("Language Selection");
        assert!(sm.has_pending());
        assert_eq!(sm.current(), None, "firing alone must not transition");
        assert_eq!(sm.apply(), vec![Event::Enter("Language Selection".into())]);
        assert!(!sm.has_pending());
        assert_eq!(sm.apply(), Vec::new(), "applying twice is a no-op");
    }

    #[test]
    fn an_unregistered_target_is_ignored() {
        let mut sm = machine();
        sm.transition_to("Language Selection");
        sm.fire("Reticulating Splines");
        assert_eq!(sm.apply(), Vec::new());
        assert_eq!(sm.current(), Some("Language Selection"));
    }

    #[test]
    fn transitioning_to_the_current_state_does_nothing() {
        let mut sm = machine();
        sm.transition_to("Launch Game");
        assert_eq!(sm.transition_to("Launch Game"), Vec::new());
    }

    #[test]
    fn queries_are_by_name() {
        let mut sm = machine();
        sm.transition_to("Intro Screen->IntroMovie1");
        assert!(sm.is("Intro Screen->IntroMovie1"));
        assert!(!sm.is("Intro Screen"));
        assert!(sm.is_in("Intro Screen"));
        assert!(!sm.is_in("Language Selection"));
    }

    #[test]
    fn history_records_every_state_entered() {
        let mut sm = machine();
        sm.transition_to("Intro Screen->IntroMovie1");
        sm.transition_to("Language Selection");
        sm.transition_to("Launch Game");
        assert_eq!(
            sm.history(),
            [
                "Intro Screen",
                "Intro Screen->IntroMovie1",
                "Language Selection",
                "Launch Game"
            ]
        );
    }

    #[test]
    fn leaf_takes_the_last_component() {
        assert_eq!(leaf("Intro Screen->IntroMovie1"), "IntroMovie1");
        assert_eq!(leaf("Language Selection"), "Language Selection");
    }
}
