//! [`load_event`]: [`load`] with a Wipeout 2048 campaign event resolved onto
//! [`Options`] first, instead of the caller naming a track/class/mode by
//! hand.
//!
//! **A second entry point, not a new [`Options`] field.** [`Options`] is
//! built exhaustively - every field named, no `..Default::default()` - at
//! every call site in this workspace, dozens of ground-truth tests among
//! them; a new required field would have touched every one of them for a
//! feature only the 2048 campaign needs. This overlays an event on top of an
//! already-built [`Options`] instead, through the same
//! [`Options::track`]/[`Options::class`]/[`Options::mode`]/
//! [`Options::laps_override`] fields a caller already sets by hand - see
//! `oag_2048::campaign` for the event schema this reads and
//! `docs/formats/2048-campaign.md` for the census.

use super::*;

/// [`load`], after resolving `event_name` against `data/xml/SP.xml`'s own
/// campaign and overriding `options.track`/`class`/`mode`/`laps_override`
/// with what the event authors.
///
/// # What is overridden, and what is not
///
/// - **Track**: [`oag_2048::campaign::track_vex_entry`] off the event's own
///   `M_TRACKDEF`. `Options::track` stays as the caller left it (usually
///   `None`) for a `Zone` event, which authors no track reference at all -
///   `load` then falls to the title's own default the way it always does,
///   and Zone's own [`oag_title::ZoneCircuit::SameCircuit`] resolution
///   applies from there. See `oag_tables::mjolnir::campaign::Event::track`'s
///   own doc comment.
/// - **Class**: [`oag_2048::campaign::EClass`] off `M_SPEEDCLASS`, spelled
///   the way `handlingstats.xml` spells it (`"FLASH"`, `"SUPERPHANTOM"`,
///   ...). Left as the caller's own choice for a `Zone` event, which
///   authors none.
/// - **Mode**: [`oag_2048::campaign::engine_mode`]'s token, parsed through
///   [`Mode::from_name`]. Every one of the four event kinds this schema
///   carries resolves to a real mode - see that function's own doc comment
///   for why `Elimination` is not refused.
/// - **Laps**: [`Options::laps_override`] only for a lap count of `1` or
///   more. `laps == Some(0)` is Speed Lap's own sentinel (see
///   `oag_tables::mjolnir::campaign::typedef::RACE_A`'s doc comment) and
///   must **not** become `laps_override: Some(0)` - a zero-lap race - so it
///   is left `None` and [`Mode::laps_target`] answers for the `speed_lap`
///   mode instead, the same way every non-campaign Speed Lap already
///   resolves its lap count.
/// - Everything else on [`Options`] (source, DLC, team, difficulty, ...) is
///   the caller's, untouched.
///
/// # Errors
///
/// - `options.source` does not open as Wipeout 2048 - this campaign is
///   2048's own, not shared with any other title.
/// - `event_name` does not match any `SP.xml` instance.
/// - The event's own `M_TRACKDEF`/`M_SPEEDCLASS` reference does not resolve
///   inside the same document (a dangling reference, not observed on the
///   real file but not assumed away either).
/// - Everything [`load`] itself can return, once the override is applied.
pub fn load_event(options: &Options, event_name: &str) -> Result<Loaded> {
    let mut archives = oag_2048::open(&options.source).with_context(|| {
        format!(
            "opening {:?} as Wipeout 2048 - --event only resolves against this title's own campaign",
            options.source
        )
    })?;

    let bytes = archives
        .read_name(oag_2048::campaign::SP_XML)
        .with_context(|| format!("reading {} for --event", oag_2048::campaign::SP_XML))?;
    let text = String::from_utf8(bytes)
        .with_context(|| format!("{} is not valid UTF-8", oag_2048::campaign::SP_XML))?;
    let doc = oag_2048::campaign::parse(&text);

    let instance = doc.instance_named(event_name).ok_or_else(|| {
        anyhow::anyhow!(
            "{event_name:?} names no instance in {}",
            oag_2048::campaign::SP_XML
        )
    })?;
    let event = oag_2048::campaign::Event::from_instance(instance).ok_or_else(|| {
        anyhow::anyhow!(
            "{event_name:?} is instance typedef {}, not one of the four event shapes {} reads",
            instance.typedef_id,
            oag_2048::campaign::SP_XML
        )
    })?;

    let mode_name = oag_2048::campaign::engine_mode(&event).ok_or_else(|| {
        anyhow::anyhow!(
            "{event_name:?} is a {:?} event, and this engine has no mode for that kind yet",
            event.kind
        )
    })?;
    let mode = Mode::from_name(mode_name)
        .ok_or_else(|| anyhow::anyhow!("oag_2048::campaign::engine_mode returned {mode_name:?}, which oag_race::Mode does not recognise"))?;

    let mut resolved = options.clone();
    resolved.mode = mode;

    if let Some(track_ref) = event.track {
        let track = oag_2048::campaign::track_for(&doc, track_ref).ok_or_else(|| {
            anyhow::anyhow!(
                "{event_name:?}'s M_TRACKDEF ({}) does not resolve inside {}",
                track_ref.instance_id,
                oag_2048::campaign::SP_XML
            )
        })?;
        resolved.track = Some(oag_2048::campaign::track_vex_entry(&track));
    }

    if let Some(class) = event
        .speed_class
        .and_then(oag_2048::campaign::EClass::from_ordinal)
    {
        resolved.class = class.as_str().to_string();
    }

    // `Some(0)` is Speed Lap's own sentinel, not a zero-lap race - see this
    // function's own doc comment and `Options::laps_override`'s.
    resolved.laps_override = event.laps.filter(|&laps| laps > 0);

    load(&resolved)
}
