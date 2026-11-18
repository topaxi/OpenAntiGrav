//! Where the view phase's report lines go among the sim phase's.
//!
//! A load reads the simulation's half and then the picture's, but its report reads in
//! the order a single pass would have written it: the lines a view step writes sit
//! between the sim lines written before and after it. The sim phase marks each place
//! a view step stood ([`Seams::mark`]); the view phase opens the same places in the
//! same order ([`ViewReport::at`]) and writes its lines there; [`ViewReport::weave`]
//! puts the two together.

/// One place in a load where view steps stand between sim steps, in load order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seam {
    /// A foreign track's drawn model, after its track inputs.
    TrackArt,
    /// The camera rigs, after the weapon tables.
    Cameras,
    /// The player's paint, before the grid's teams are drawn.
    Looks,
    /// The grid's liveries, after its teams are drawn.
    Liveries,
    /// The circuit's drawn scene, after the opponents' tuning.
    Scene,
    /// The collision overlay, after the spline and its cage.
    CollisionModel,
    /// Effects, sound, the HUD and the trail textures, after the launch boost.
    Effects,
    /// The cockpit static, the destroy cameras and the flyby, after the simulation modules.
    Flyby,
    /// Everything drawn after the craft's weapon nodes, to the end of the load.
    Finish,
}

/// Where in one phase's report each [`Seam`] falls.
#[derive(Debug, Default)]
pub struct Seams {
    at: Vec<(Seam, usize)>,
}

impl Seams {
    /// Marks `seam` at the end of the report so far.
    pub fn mark(&mut self, seam: Seam, report: &[String]) {
        self.at.push((seam, report.len()));
    }
}

/// The whole report: `sim`'s lines with each seam's `view` lines at its place.
///
/// `sim_seams` marks where each seam falls among `sim`; `view_seams` where each seam's own
/// lines begin among `view`, the view phase marking a seam before it writes them.
///
/// # Panics
///
/// When the two phases did not mark the same seams in the same order, or the view phase
/// wrote a line before its first seam: a load whose report would not read as one pass.
#[cfg(feature = "view")]
#[must_use]
pub fn weave(
    sim: Vec<String>,
    sim_seams: &Seams,
    view: Vec<String>,
    view_seams: &Seams,
) -> Vec<String> {
    let order = |seams: &Seams| seams.at.iter().map(|(seam, _)| *seam).collect::<Vec<_>>();
    assert_eq!(
        order(sim_seams),
        order(view_seams),
        "the view phase opened other seams than the sim phase marked"
    );
    assert!(
        view_seams.at.first().is_none_or(|(_, start)| *start == 0),
        "the view phase wrote a line before its first seam"
    );
    let mut woven = Vec::with_capacity(sim.len() + view.len());
    let mut sim = sim.into_iter();
    let mut view = view.into_iter();
    let mut taken = 0;
    for (k, (_, at)) in sim_seams.at.iter().enumerate() {
        woven.extend(sim.by_ref().take(at - taken));
        taken = *at;
        let begin = view_seams.at[k].1;
        let end = view_seams
            .at
            .get(k + 1)
            .map_or(usize::MAX, |(_, start)| *start);
        woven.extend(view.by_ref().take(end - begin));
    }
    woven.extend(sim);
    woven.extend(view);
    woven
}

#[cfg(all(test, feature = "view"))]
mod tests {
    use super::*;

    fn lines(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    #[test]
    fn each_seams_view_lines_land_where_the_sim_phase_marked_it() {
        let (mut sim_seams, mut view_seams) = (Seams::default(), Seams::default());
        let mut sim = lines(&["s1"]);
        sim_seams.mark(Seam::Cameras, &sim);
        sim.extend(lines(&["s2", "s3"]));
        sim_seams.mark(Seam::Looks, &sim);
        sim_seams.mark(Seam::Liveries, &sim);
        sim.extend(lines(&["s4"]));
        let mut view = Vec::new();
        view_seams.mark(Seam::Cameras, &view);
        view.extend(lines(&["v1", "v2"]));
        view_seams.mark(Seam::Looks, &view);
        view_seams.mark(Seam::Liveries, &view);
        view.push("v3".to_string());
        assert_eq!(
            weave(sim, &sim_seams, view, &view_seams),
            lines(&["s1", "v1", "v2", "s2", "s3", "v3", "s4"])
        );
    }

    #[test]
    #[should_panic(expected = "opened other seams")]
    fn a_seam_the_sim_phase_never_marked_is_refused() {
        let mut view_seams = Seams::default();
        view_seams.mark(Seam::Scene, &[]);
        let _ = weave(Vec::new(), &Seams::default(), Vec::new(), &view_seams);
    }
}
