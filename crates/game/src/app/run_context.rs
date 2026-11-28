//! What one launched run carries across its stages (ADR-0099, decision 1).
//!
//! A run is what one launch started and every relaunch of it continues: a single race and its
//! RACE AGAINs, a campaign cell, an attract demo, a benchmark. `Session::run` holds the run being
//! launched; the race build moves it into the `RaceStage`, and RACE AGAIN, NEXT RACE and RESTART
//! relaunch from the race's copy (`docs/architecture/navigation-map.md`, "Single-slot holders").

use oag_raceplay as race;

/// The launch slots of one run.
#[derive(Default, Clone)]
pub(crate) struct RunContext {
    /// The race options as the launch saw them, before `Session::launch_race`'s per-launch
    /// refresh (seed, language, render profile, mods), which every relaunch runs again. `None`
    /// until the launch reaches `launch_race`, which takes them from `Session::race_options`.
    /// A demo, a benchmark and every relaunch set them beforehand, so the menus' own
    /// `race_options` stay the player's.
    pub(crate) options: Option<race::Options>,
    /// The campaign cell this run races. The mirror of `DAT_08b30ffc`,
    /// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "current campaign cell"; not
    /// carried through `race::Options` because nothing in `oag_race`/`oag_gameplay` may know a
    /// campaign exists. `None` on every run but a campaign launch.
    pub(crate) campaign_cell: Option<oag_tables::race_campaign::Cell>,
    /// A mod campaign event's extras for [`Self::campaign_cell`] (ADR-0077): where its medal is
    /// kept, and its AI. `None` for the disc's own.
    pub(crate) campaign_custom: Option<crate::campaign::custom::Launch>,
    /// [`Self::campaign_cell`]'s own difficulty rung, as `Cell Selection` confirmed it. Read by
    /// the medal evaluation and the skill level of the launch.
    pub(crate) campaign_difficulty: Option<oag_tables::race_campaign::Difficulty>,
    /// [`Self::campaign_cell`]'s own resolved AI skill scale (`AI_ResolveSkillScale` against
    /// the rung and the first leg's `stats.xml`), resolved once at the cell's launch and applied
    /// when the race stage is built. `None` whenever [`Self::campaign_difficulty`] is, or the
    /// resolution fell through.
    pub(crate) campaign_ai_skill_scale: Option<f32>,
    /// The downloadable campaign a 2048 tier cell belongs to: its craft fly instead of the
    /// player's pick. Taken by `Session::launch_campaign_race`.
    pub(crate) campaign_add_on: Option<&'static oag_title::AddOn>,
    /// What a mod campaign event replaced in the menus' `race_options`, put back when the menus
    /// return (`Session::restore_campaign_home`) or the next cell launches.
    pub(crate) campaign_home: Option<CampaignHome>,
    /// The Wipeout 2048 campaign event this run starts, by its `SP.xml` name; taken by the
    /// load (`oag_raceplay::load_event`). `None` is an ordinary race.
    pub(crate) pending_event: Option<String>,
    /// The card that event was launched from, which the race load draws behind its box.
    pub(crate) pending_card: Option<oag_ui::frontend::LaunchedCard>,
}

/// The race options a mod campaign event replaces: a cross-disc event's
/// sources, an event's AI level.
#[derive(Clone)]
pub(crate) struct CampaignHome {
    pub(crate) source: String,
    pub(crate) craft_source: Option<String>,
    pub(crate) difficulty: oag_ai::Difficulty,
}

impl crate::app::session::Session {
    /// The options of the race on screen, or else of the one being launched: its run's
    /// snapshot, else the menus' own (a watched recording takes no snapshot).
    pub(crate) fn race_options_now(&self) -> Option<&race::Options> {
        let run = match &self.stage {
            crate::app::stage::Stage::Race(stage) => Some(&stage.run),
            _ => self.run.as_ref(),
        };
        run.and_then(|run| run.options.as_ref())
            .or(self.race_options.as_ref())
    }

    /// The campaign cell of the run being launched.
    pub(crate) fn launch_cell(&self) -> Option<&oag_tables::race_campaign::Cell> {
        self.run.as_ref().and_then(|run| run.campaign_cell.as_ref())
    }

    /// The run being launched, started empty when there is none.
    pub(crate) fn run_mut(&mut self) -> &mut RunContext {
        self.run.get_or_insert_default()
    }

    /// RACE AGAIN and NEXT RACE: the run of the race on screen becomes the run being launched,
    /// as it launched. No race on screen leaves no run.
    pub(crate) fn rerun_race_on_screen(&mut self) {
        self.run = match &self.stage {
            crate::app::stage::Stage::Race(stage) => Some(stage.run.clone()),
            _ => None,
        };
    }

    /// Ends the run being launched before a new one starts: what a mod campaign event replaced
    /// in the menus' options goes back, and the run is returned.
    pub(crate) fn end_run(&mut self) -> Option<RunContext> {
        self.restore_campaign_home();
        self.run.take()
    }
}
