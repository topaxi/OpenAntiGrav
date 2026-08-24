//! Reading a loading screen's own assets off a disc: what each title authors,
//! and which of it this source actually carries.
//!
//! Split out of `loading.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. It is a real
//! seam rather than a cut at a line number: everything here opens archives and
//! resolves strings, and everything left in the parent draws. Nothing here
//! knows a screen coordinate and nothing there opens a file.

use oag_formats::fexml;

use crate::language::StringTable;

/// Everything the screen needs off the disc.
///
/// Read only when `--prefetch` asks for the screen: a default boot never opens
/// either entry, which is what keeps its half-second where it was.
#[derive(Debug)]
pub struct Assets {
    /// The disc's loading tips, in the chosen language and in XML order.
    ///
    /// Empty is an ordinary outcome - a title that authors no wave, a source
    /// whose plugin will not read, or a string table that resolves none of the
    /// ids - and the screen simply draws no tip line.
    pub tips: Vec<String>,
    /// The wave's glow strip.
    ///
    /// Present even on a title that authors no wave, as the stand-in: the
    /// renderer binds this pipeline unconditionally and a zero-sized texture is
    /// a validation error. What decides whether the wave is *drawn* is
    /// [`Self::wave`], not this.
    pub strip: oag_render::loading::GlowStrip,
    /// Whether this title's loading screen has a wave at all.
    ///
    /// `false` on Pure, which authors no loading screen, and on Wipeout HD,
    /// which authors a still instead - and on both the screen draws no wave
    /// rather than Pulse's. See [`oag_title::Loading`].
    pub wave: bool,
    /// Every picture this screen draws, when the title has any. See [`Art`].
    pub art: Option<Art>,
    /// The illustrated feature this screen teaches, when the title has any.
    ///
    /// One of the set, chosen at load: HD rotates through five and this build
    /// picks one for the run rather than cycling, because which one the
    /// original picks is `Feature type == %i` and what drives that is unread.
    /// See [`Feature`].
    pub feature: Option<Feature>,
    /// The word under the screen, resolved out of the disc's string table.
    ///
    /// `None` on a title that names no caption id, or whose table does not
    /// carry the one it names - and the screen falls back to its own heading.
    pub caption: Option<String>,
    /// Lines worth printing once, describing what was found or what was not.
    pub notes: Vec<String>,
}

/// One feature, resolved: its two strings.
///
/// The picture is not here: it shares [`Art`]'s sheet with the screen's chrome,
/// because a [`crate::render::Renderer`] is built with **one** atlas and every
/// `Draw::Sprite` on the screen indexes it.
#[derive(Debug)]
pub struct Feature {
    /// Its name, where the title carries an id for one.
    pub title: Option<String>,
    /// The paragraph under it.
    pub description: String,
}

/// Every picture the screen draws, in one sheet, with where each one sits.
///
/// **One sheet because the renderer takes one.** The illustration and the five
/// marks the original frames this screen with - see
/// [`oag_title::loading::Chrome`] - are stacked together and addressed by the
/// rectangles below, so the whole screen draws from a single atlas.
///
/// A mark that is missing is `None` and simply is not drawn; the layout does
/// not shift to fill the gap, because a rule that moved when its texture failed
/// would make one missing file look like a different screen.
#[derive(Debug)]
pub struct Art {
    /// The stacked sheet.
    pub sheet: crate::sprite::Sheet,
    /// Where the feature illustration sits in it.
    pub illustration: Option<[f32; 4]>,
    /// The marker before the screen's own title.
    pub title_arrow: Option<[f32; 4]>,
    /// The marker before a feature's name.
    pub subtitle_arrow: Option<[f32; 4]>,
    /// The horizontal rules.
    pub rule: Option<[f32; 4]>,
    /// The bracket marks round a region.
    pub corner: Option<[f32; 4]>,
    /// The progression bar's own fill.
    ///
    /// **Loaded and deliberately not drawn.** The original tiles this 8x8 dot
    /// across the bar; `Draw::Sprite` has no repeat mode, so stretching it
    /// gives a blurred smear rather than a pattern - see
    /// [`super::Screen::draw_list`], which draws the trough flat instead. Kept
    /// here because the disc ships it and the day the renderer can repeat a
    /// texture this is the entry to reach for.
    pub dot: Option<[f32; 4]>,
}

impl Default for Assets {
    /// No tips, no feature, no caption, and the authored stand-in strip.
    ///
    /// The same thing [`Assets::load`] falls back to when the disc's own strip
    /// will not decode, so this is a drawable screen rather than an empty
    /// husk - which matters because `--race` carries one it never draws, and a
    /// zero-sized texture would be a validation error the moment anything did.
    ///
    /// `wave: true`, because this is what a caller with no title in hand gets
    /// and the stand-in strip is drawable. A title that says it has no wave
    /// says so through [`Assets::load`].
    fn default() -> Self {
        Self {
            tips: Vec::new(),
            strip: oag_render::loading::GlowStrip::placeholder(
                oag_render::loading::STRIP_SIZE as u32,
            ),
            wave: true,
            art: None,
            feature: None,
            caption: None,
            notes: Vec::new(),
        }
    }
}

impl Assets {
    /// Reads whatever this source's own title authors, degrading rather than
    /// failing.
    ///
    /// **Which entries those are is the title's question**, not this module's.
    /// Until 2026-08-24 this read `oag_pulse::loading`'s two names whatever disc
    /// had booted, on the reasoning that the only other title shipped neither
    /// and so had nothing to put in a table. Wipeout HD ships a full-screen
    /// still and no tips at all, which makes three answers - see
    /// [`oag_title::Loading`], whose module doc carries them.
    ///
    /// `style` names which front-end styling to take the illustrations from -
    /// `HD` or `FURY` on Wipeout HD, which ships every one of them twice. An
    /// unknown or absent name takes the title's first, which is the base game's;
    /// see [`oag_title::loading::FeatureStyle`].
    ///
    /// Nothing here is worth refusing to boot over: without the strip the wave
    /// has nothing to sample, without the tips there is no tip line, without a
    /// feature there is no illustration, and in every case the counts - the
    /// thing this screen exists for on its two long waits - are unaffected. So
    /// every failure is a note, and the caller prints it.
    ///
    /// **The strip falls back to [`oag_render::loading::GlowStrip::placeholder`]
    /// and says so.** That function's own documentation refuses to be a silent
    /// substitute, and the note is what makes it not one. A **feature** takes no
    /// such fallback: a title that ships one and cannot read it draws none,
    /// because a stand-in for an authored illustration is a picture this build
    /// made up.
    #[must_use]
    pub fn load(source: &str, strings: &StringTable, style: Option<&str>) -> Self {
        let mut notes = Vec::new();
        let mut tips = Vec::new();
        let mut strip = None;

        // **Opened as whichever title the source is**, not as Pulse. A Pure disc
        // through `oag_pulse::open` comes back `WrongTitle`, which reads as "you
        // pointed at the wrong disc" when the truth is "this title ships no
        // loading screen".
        let opened = match crate::title::open_source(source, Vec::new()) {
            Ok(opened) => opened,
            Err(e) => {
                notes.push(format!("no loading screen assets from {source}: {e}"));
                return Self {
                    notes,
                    ..Self::default()
                };
            }
        };
        let title = opened.title;
        let mut archives = opened.archives;

        let Some(loading) = title.loading else {
            // Said out loud. A title with no loading screen of its own is a
            // measurement - see `oag_pure`'s own `loading: None` - and a silent
            // fall-through would read exactly like a failed read.
            notes.push(format!(
                "{} authors no loading screen of its own; drawing this build's counts",
                title.name
            ));
            return Self {
                wave: false,
                notes,
                ..Self::default()
            };
        };

        if let Some(wave) = loading.wave {
            match archives
                .read_name(wave.tips)
                .map_err(|e| e.to_string())
                // `text` rather than `expand`: shortening is per file, and
                // the PS2 leaves several of these plain.
                .and_then(|blob| fexml::text(&blob).map_err(|e| e.to_string()))
            {
                Ok(xml) => tips = self::tips(&xml, strings),
                Err(e) => notes.push(format!("{}: {e}", wave.tips)),
            }
            // `read_image`, not `read_name`: the PS2 keeps this strip
            // under the declared name with the extension rewritten to
            // `.pct` (`oag_pulse::ps2_texture_name`), which is why the
            // loading screen had no overlay on that disc.
            match oag_pulse::read_image(&mut archives, wave.glow_strip)
                .map_err(|e| e.to_string())
                .and_then(|blob| {
                    oag_render::loading::GlowStrip::decode(&blob).map_err(|e| format!("{e:#}"))
                }) {
                Ok(decoded) => strip = Some(decoded),
                Err(e) => notes.push(format!("{}: {e}", wave.glow_strip)),
            }
            notes.push(format!("{}: {} loading tip(s)", wave.tips, tips.len()));
        }

        // **One of the set, picked for the whole run rather than cycled.** The
        // original rotates - its `Feature type == %i` says which is up - and
        // what drives that number is unread, so cycling here would be inventing
        // a rhythm. `pick_feature` says how the one is chosen and why it is not
        // arbitrary.
        // **One sheet for the whole screen.** The illustration and the marks
        // the original frames this screen with go into the same atlas, because
        // a renderer is built with one and every `Draw::Sprite` indexes it. See
        // `Art`.
        let chosen = pick_feature(loading.features, style, &mut notes);
        let mut blobs: Vec<(String, Vec<u8>)> = Vec::new();
        let mut read = |entry: &str, notes: &mut Vec<String>| match archives.read_name(entry) {
            Ok(blob) => {
                blobs.push((entry.to_string(), blob));
                true
            }
            Err(e) => {
                notes.push(format!("{entry}: {e}"));
                false
            }
        };
        let illustrated = chosen.is_some_and(|feature| read(feature.image, &mut notes));
        for (_, entry) in loading.chrome {
            read(entry, &mut notes);
        }

        let art = (!blobs.is_empty()).then(|| {
            let mut sheet_notes = Vec::new();
            let sheet = crate::sprite::Sheet::build(&blobs, &mut sheet_notes);
            notes.append(&mut sheet_notes);
            // A sheet that decoded nothing is `Sheet::default` - one
            // transparent texel - and every lookup below misses, which draws
            // the screen with no pictures rather than with one blank one.
            let at = |entry: &str| {
                sheet.get(entry).map(|placed| {
                    [
                        placed.x as f32,
                        placed.y as f32,
                        placed.width as f32,
                        placed.height as f32,
                    ]
                })
            };
            let mark = |want: oag_title::loading::Chrome| {
                loading
                    .chrome
                    .iter()
                    .find(|(kind, _)| *kind == want)
                    .and_then(|(_, entry)| at(entry))
            };
            use oag_title::loading::Chrome;
            Art {
                illustration: chosen
                    .filter(|_| illustrated)
                    .and_then(|feature| at(feature.image)),
                title_arrow: mark(Chrome::TitleArrow),
                subtitle_arrow: mark(Chrome::SubtitleArrow),
                rule: mark(Chrome::Rule),
                corner: mark(Chrome::Corner),
                dot: mark(Chrome::Dot),
                sheet,
            }
        });

        let feature = chosen.and_then(|chosen| {
            Some(Feature {
                // `get` rather than `get_or_id`: a missing title draws no title,
                // where showing the id would put `FE_PILOT_ASSIST` on screen.
                title: chosen
                    .title
                    .and_then(|id| strings.get(id))
                    .map(str::to_string),
                description: strings.get(chosen.description)?.to_string(),
            })
        });

        let caption = loading.caption.and_then(|id| {
            let text = strings.get(id);
            if text.is_none() {
                notes.push(format!("{id}: no such string; the heading stands in"));
            }
            text.map(str::to_string)
        });

        let strip = strip.unwrap_or_else(|| {
            if loading.wave.is_some() {
                notes.push(
                    "the loading wave is drawing an authored stand-in strip, not the disc's own"
                        .to_string(),
                );
            }
            oag_render::loading::GlowStrip::placeholder(oag_render::loading::STRIP_SIZE as u32)
        });

        Self {
            tips,
            strip,
            wave: loading.wave.is_some(),
            art,
            feature,
            caption,
            notes,
        }
    }
}

/// Which feature to illustrate, out of the styles a title ships.
///
/// **The style is chosen, the feature is not.** `style` is a saved setting and
/// names one of the title's own - `HD` or `FURY` on Wipeout HD, which ships
/// every illustration twice. Which *feature* within it is the original's
/// `Feature type == %i`, and what drives that number has not been read - so
/// this takes the one that number was observed at rather than rotating on a
/// rhythm this build made up.
///
/// **Index 2, corroborated twice.** A screenshot of a running Fury race and
/// this project's own RPCS3 capture both show `Pilot_Assist`, and both runs
/// logged `Feature type == 2`; `Pilot_Assist` is third in the order the
/// executable names the ten images in. That is two observations of one value,
/// not a measurement of what selects it - see `docs/formats/hd-loading.md`.
///
/// An unknown style name falls back to the title's first and says so, the same
/// way a saved language this source does not carry falls through.
fn pick_feature<'a>(
    styles: &'a [oag_title::loading::FeatureStyle],
    style: Option<&str>,
    notes: &mut Vec<String>,
) -> Option<&'a oag_title::loading::Feature> {
    let chosen = match style {
        Some(name) => styles
            .iter()
            .find(|style| style.name.eq_ignore_ascii_case(name))
            .or_else(|| {
                let first = styles.first()?;
                notes.push(format!(
                    "front-end style {name:?} is not one this source ships; using {:?}",
                    first.name
                ));
                Some(first)
            }),
        None => styles.first(),
    }?;
    let feature = chosen.features.get(OBSERVED_FEATURE).or_else(|| {
        // A style with fewer than three would be a different disc; answered
        // rather than panicked, because a loading screen is not worth a crash.
        chosen.features.first()
    })?;
    notes.push(format!(
        "loading screen: {} style, {}",
        chosen.name, feature.image
    ));
    Some(feature)
}

/// Which of a style's features this build shows.
///
/// See [`pick_feature`]: `Feature type == 2` is what both observations of the
/// running game logged, and index 2 is `Pilot_Assist` in the order its
/// executable names them.
const OBSERVED_FEATURE: usize = 2;

/// Every `<PI_LoadingScreen>`'s body string, resolved through `strings`.
///
/// The tip is the `<Text StringSrc="MSC_LOAD_...">` child; the `<Title>`'s own
/// `TitleSrc` names the weapon the original's artwork shows and is skipped with
/// it. An id the table does not carry is dropped rather than drawn as its own
/// id: `get_or_id`'s "showing the id beats showing nothing" is right for a
/// screen whose title is missing and wrong for one entry of a rotating list,
/// where the next one along is a better thing to show than `MSC_LOAD_QUAKE`.
#[must_use]
pub fn tips(xml: &str, strings: &StringTable) -> Vec<String> {
    let root = fexml::parse(xml);
    let mut out = Vec::new();
    collect_tips(&root, strings, &mut out);
    out
}

/// Walks the tree for `<PI_LoadingScreen>` at any depth.
///
/// Any depth rather than only the root's children: the file this parses puts
/// them directly under `<Screen name="Top">`, but a front-end XML nests screens
/// inside screens elsewhere and a reader that assumed one level would silently
/// find nothing on the first source that did.
fn collect_tips(node: &fexml::Node, strings: &StringTable, out: &mut Vec<String>) {
    for child in &node.children {
        if child.name.eq_ignore_ascii_case("PI_LoadingScreen") {
            if let Some(text) = child
                .children_named("Text")
                .next()
                .and_then(|text| text.attr("StringSrc"))
                .and_then(|id| strings.get(id))
            {
                out.push(text.to_string());
            }
            continue;
        }
        collect_tips(child, strings, out);
    }
}
