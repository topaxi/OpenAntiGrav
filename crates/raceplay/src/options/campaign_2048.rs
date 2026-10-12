//! What a Wipeout 2048 campaign event's load carries to the race stage.

/// What [`load_event`](crate::load_event) resolved for `RaceStage` to
/// grade the race against, once it finishes - see [`super::Loaded::campaign_2048_event`].
#[derive(Debug, Clone)]
pub struct Campaign2048Progress {
    /// The `SP.xml` instance name - the key
    /// `oag_game::records::Store::record_campaign` persists a medal under,
    /// the same way Pulse/HD's own launch persists one under a cell's name.
    pub name: String,
    /// The event's own pass/elite bars, when it authors them - `None` for a
    /// Speed Lap or a generic template event, which never carries a medal
    /// but still opens whatever names it as a prerequisite once finished.
    /// See `oag_2048::campaign::event_objectives`.
    pub objectives: Option<oag_2048::campaign::EventObjectives>,
    /// What the event's opponents read when they fly the original driver:
    /// its skill, its two AI skills and the level, authored values over the
    /// constructor's defaults. `None` for an Elimination event, whose
    /// opponents keep this project's driver: 2048 hands their throttle to a
    /// law of its own (`0x81199800`) that is unread.
    pub ai: Option<oag_2048::race::EventAi>,
}
