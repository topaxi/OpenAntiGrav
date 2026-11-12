//! Reading a loading screen's own assets off a disc: what each title authors,
//! and which of it this source actually carries.
//!
//! Split out of `loading.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. It is a real
//! seam rather than a cut at a line number: everything here opens archives and
//! resolves strings, and everything left in the parent draws. Nothing here
//! knows a screen coordinate and nothing there opens a file.

use oag_tables::fexml;

use oag_ui::language::StringTable;

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
    /// The four colours this source's own front end tints the screen with, when
    /// the title names them. See [`oag_title::loading::Palette`].
    pub palette: Option<Palette>,
    /// The front-end style whose illustrations, mode icons and prose this holds, by the name
    /// the disc and the settings file give it. `None` on a title with no styles.
    pub style: Option<&'static str>,
    /// Every illustrated feature this title teaches, in its own order.
    ///
    /// Empty on a title with none. The screen draws one of them; which is
    /// [`super::Screen::new`]'s to decide, because the original decides it per
    /// screen rather than per disc.
    ///
    /// See [`Feature`]; which one is up is [`Self::deck`]'s and the screen's.
    pub features: Vec<Feature>,
    /// Which of the features a race of each mode may show, when the title's
    /// rule is read. See [`oag_title::loading::Deck`].
    pub deck: Option<oag_title::loading::Deck>,
    /// How the bar fills, when the title's own loader drives it. See
    /// [`oag_title::loading::Progression`].
    pub progression: Option<oag_title::loading::Progression>,
    /// The word under the screen, resolved out of the disc's string table.
    ///
    /// `None` on a title that names no caption id, or whose table does not
    /// carry the one it names - and the screen falls back to its own heading.
    pub caption: Option<String>,
    /// The small labels the screen names its regions with, resolved. See
    /// [`Labels`].
    pub labels: Option<Labels>,
    /// The mode icon clips this title's chosen style names, read whole: a few
    /// tens of kilobytes each, and the movie cache decodes them off the frame
    /// thread. See [`ModeIcon`].
    pub mode_icons: Vec<ModeIcon>,
    /// This title's loading layout file, read and parsed, when it names one and the file is
    /// valid. `None` draws the layout this build chose in code. See [`super::layout`].
    pub layout: Option<oag_hud::loading_layout::LoadingLayouts>,
    /// Every string id the layout's labels name, resolved in the chosen language: an id the
    /// table does not carry is absent and its label is not drawn.
    pub layout_strings: Vec<(String, String)>,
    /// Lines worth printing once, describing what was found or what was not.
    pub notes: Vec<String>,
}

/// A mode's icon clip: the Bink file the original plays in the loading
/// screen's mode icon panel.
#[derive(Debug, Clone)]
pub struct ModeIcon {
    /// The source executable's mode id this clip is for.
    pub mode: u32,
    /// The archive entry it was read from.
    pub entry: String,
    /// The file, undecoded.
    pub blob: Vec<u8>,
}

/// The labels over the screen's regions, resolved out of the string table.
///
/// A label whose id the table does not carry is `None` and is not drawn: showing
/// the id would put `FE_PROG_BAR` on screen.
#[derive(Debug, Clone, Default)]
pub struct Labels {
    /// Over the feature illustration.
    pub feature_image: Option<String>,
    /// Over the feature's prose.
    pub feature_description: Option<String>,
    /// Over the progression bar.
    pub progression_bar: Option<String>,
    /// Over the mode icon.
    pub mode_icon: Option<String>,
}

/// One feature, resolved: its two strings.
///
/// The picture is not here: it shares [`Art`]'s sheet with the screen's chrome,
/// because a [`crate::render::Renderer`] is built with **one** atlas and every
/// `Draw::Sprite` on the screen indexes it.
#[derive(Debug)]
pub struct Feature {
    /// Its place in the title's own table, which is the index the original's
    /// `Feature type` reports and the one a [`oag_title::loading::Deck`] names.
    ///
    /// Kept because a feature whose picture or prose does not resolve is left
    /// out of [`Assets::features`], which would shift every position after it.
    pub slot: u8,
    /// Its name, where the title carries an id for one.
    pub title: Option<String>,
    /// The heading over its paragraph.
    pub heading: Option<String>,
    /// The paragraph under it.
    pub description: String,
}

/// A loading screen's four colours, resolved out of the source's own
/// `FEGlobals`.
///
/// See [`oag_title::loading::Palette`] for where the four names come from and
/// which of the roles below are read rather than recovered.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    /// The screen's ground.
    pub background: [f32; 4],
    /// Everything written and every mark drawn.
    pub ink: [f32; 4],
    /// The accent.
    pub accent: [f32; 4],
    /// The one translucent colour: the unfilled dots of the progression bar.
    pub dim: [f32; 4],
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
    pub sheet: oag_hud::sprite::Sheet,
    /// Where each feature's illustration sits in it, in the title's own order.
    ///
    /// **All of them, not the one on show.** The original draws a feature at
    /// random every time the screen goes up, so the picture cannot be chosen
    /// when the disc is read - see [`super::Screen::new`].
    pub illustrations: Vec<[f32; 4]>,
    /// The marker before the screen's own title.
    pub title_arrow: Option<[f32; 4]>,
    /// The marker before a feature's name.
    pub subtitle_arrow: Option<[f32; 4]>,
    /// The horizontal rules.
    pub rule: Option<[f32; 4]>,
    /// The bracket marks round a region.
    pub corner: Option<[f32; 4]>,
    /// The progression bar's dot: an 8x8 tile repeated 166 columns across the
    /// bar, lit in the accent from the left and in the dim colour past it. See
    /// `loading::bar`.
    pub dot: Option<[f32; 4]>,
    /// The square bullet before each label.
    pub square: Option<[f32; 4]>,
    /// The mode icon's pictures, where the title draws stills. See [`ModeStills`].
    pub mode_stills: Option<ModeStills>,
}

/// Where each mode's icon still sits in the sheet: `None` for a mode that draws none, or
/// whose picture did not decode.
#[derive(Debug)]
pub struct ModeStills {
    /// By mode id.
    pub modes: Vec<(u32, Option<[f32; 4]>)>,
    /// Every mode not in `modes`.
    pub otherwise: Option<[f32; 4]>,
}

impl ModeStills {
    /// The picture for `mode`, as [`oag_title::loading::ModeStills::entry`] picks it.
    #[must_use]
    pub fn get(&self, mode: Option<u32>) -> Option<[f32; 4]> {
        mode.and_then(|mode| self.modes.iter().find(|(id, _)| *id == mode))
            .map_or(self.otherwise, |(_, rect)| *rect)
    }
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
            palette: None,
            style: None,
            features: Vec::new(),
            deck: None,
            progression: None,
            layout: None,
            layout_strings: Vec::new(),
            caption: None,
            labels: None,
            mode_icons: Vec::new(),
            notes: Vec::new(),
        }
    }
}

impl Assets {
    /// Reads whatever this source's own title authors, degrading rather than
    /// failing.
    ///
    /// **Which entries those are is the title's question**, not this module's.
    /// Pulse ships a glow strip and tips, Pure neither, and Wipeout HD a
    /// full-screen still and no tips - see [`oag_title::Loading`], whose module
    /// doc carries the three answers.
    ///
    /// `style` names which front-end styling to take the illustrations from -
    /// `HD` or `FURY` on Wipeout HD, which ships every one of them twice. An
    /// unknown name takes the title's first, and an absent one the style of the front end the
    /// source serves; see [`oag_title::loading::FeatureStyle`].
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
    pub fn load(
        source: &str,
        strings: &StringTable,
        entries: Option<&str>,
        style: Option<&str>,
    ) -> Self {
        let mut notes = Vec::new();
        let mut tips = Vec::new();
        let mut strip = None;

        // **Opened as whichever title the source is**, not as Pulse. A Pure disc
        // through `oag_pulse::open` comes back `WrongTitle`, which reads as "you
        // pointed at the wrong disc" when the truth is "this title ships no
        // loading screen".
        let opened = match oag_source::title::open_source(source, Vec::new(), Vec::new()) {
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

        // **One sheet for the whole screen, and every feature in it.** The
        // illustrations and the marks the original frames this screen with go
        // into the same atlas, because a renderer is built with one and every
        // `Draw::Sprite` indexes it. All five illustrations rather than one,
        // because the original draws a feature per screen and this is read once
        // per disc - see `Screen::new`.
        // **The served front end, read first**: with no saved choice the style is the one
        // the menus are drawn in, which is the ground this archive clears to.
        // Read through the title's own colour template, as its front end is, so the four
        // globals are the ones the menus resolve (Omega's `WOHD_Colours` overwrites
        // `HD_Grey`); a title with no template reads the file as it stands.
        let screens = |blob: &[u8], template| {
            let xml = oag_ui::xml::expand(blob).ok()?;
            Some(oag_ui::screen::Screens::from_xml_with_fallbacks(
                &xml,
                &[],
                &[],
                template,
            ))
        };
        let mut served = loading.palette.and_then(|_| {
            let front_end = title.front_end?;
            let blob = archives.read_name(front_end.root).ok()?;
            screens(&blob, front_end.boot.colour_template)
        });
        let dark_ground = loading
            .palette
            .zip(served.as_ref())
            .is_some_and(|(names, served)| served.global_is_dark(names.background));
        let chosen = pick_style(loading.features, style, dark_ground, &mut notes);
        // **The chosen style's own ground.** A style chosen against the served front end's
        // ground (HD's white pictures on the black archive) takes the four globals from the copy
        // of the front-end file whose ground matches, as the original does with a skin file per
        // style; without one the served copy stands.
        if let (Some(names), Some(style), Some(front_end)) =
            (loading.palette, chosen, title.front_end)
            && served
                .as_ref()
                .is_some_and(|served| served.global_is_dark(names.background) != style.dark_ground)
        {
            let own = archives
                .read_every_name(front_end.root)
                .into_iter()
                .filter_map(|(label, blob)| {
                    Some((label, screens(&blob, front_end.boot.colour_template)?))
                })
                .find(|(_, copy)| {
                    copy.globals.contains_key(names.background)
                        && copy.global_is_dark(names.background) == style.dark_ground
                });
            match own {
                Some((label, copy)) => {
                    notes.push(format!(
                        "loading screen: {} style takes its colours from {label}",
                        style.name
                    ));
                    served = Some(copy);
                }
                None => notes.push(format!(
                    "loading screen: no copy of {} has a {} ground; the served colours stand",
                    front_end.root,
                    if style.dark_ground { "dark" } else { "light" }
                )),
            }
        }
        // **The copy that carries every feature's prose, which is not the one
        // the front end is served.** Two of Wipeout HD's five descriptions -
        // `FE_ABSORB_INST` and `FE_FLIP_INST`, the two Fury mechanics - are in
        // one copy of the string table only, and it is the same copy that
        // carries all 28 circuit names. That is `oag_ui::language::CircuitNames`'s
        // finding arriving a second time from a different direction, and it is
        // what stops this screen offering three features on a disc that ships
        // five. See `docs/formats/hd-frontend.md`.
        let wanted: Vec<String> = chosen
            .map(|style| style.features)
            .unwrap_or_default()
            .iter()
            .map(|feature| feature.description.to_string())
            .collect();
        let prose = entries.filter(|_| !wanted.is_empty()).and_then(|entries| {
            let copies: Vec<(String, StringTable)> = archives
                .read_every_name(entries)
                .into_iter()
                .filter_map(|(label, blob)| {
                    let xml = oag_ui::xml::expand(&blob).ok()?;
                    Some((label, StringTable::from_xml(&xml)))
                })
                .collect();
            let (label, table) = copies
                .into_iter()
                .find(|(_, table)| wanted.iter().all(|id| table.get(id).is_some()))?;
            notes.push(format!(
                "loading screen: {} feature description(s) from {label}",
                wanted.len()
            ));
            Some(table)
        });
        // The served table where no copy covers them all, which is every title
        // but this one and is what a miss falls back to.
        let prose = prose.as_ref().unwrap_or(strings);

        // A `.gnf` stored in HD's `.gtf` row order is reversed on the way into the sheet, the
        // way the front end's own sheet does it (`boot::sprites`); empty on every title but
        // Omega, so every other title's sheet is built as it was.
        let bottom_up = title
            .front_end
            .map_or(&[][..], |front_end| front_end.bottom_up_gnf);
        let mut blobs: Vec<(String, Vec<u8>)> = Vec::new();
        let mut reversed: Vec<oag_hud::sprite::DecodedImage> = Vec::new();
        let mut read = |entry: &str, notes: &mut Vec<String>| match archives.read_name(entry) {
            Ok(blob) if crate::boot::sprites::is_bottom_up(entry, bottom_up) => {
                match crate::boot::sprites::bottom_up_rows(entry, &blob) {
                    Some(image) => {
                        reversed.push(image);
                        true
                    }
                    None => {
                        notes.push(format!("{entry}: .gnf rows would not reverse"));
                        false
                    }
                }
            }
            Ok(blob) => {
                blobs.push((entry.to_string(), blob));
                true
            }
            Err(e) => {
                notes.push(format!("{entry}: {e}"));
                false
            }
        };
        // Only the features whose picture *and* prose both resolve are offered,
        // so a draw cannot land on a half-loaded one.
        let mut features = Vec::new();
        let mut illustrated: Vec<&str> = Vec::new();
        for (slot, feature) in chosen
            .map(|style| style.features)
            .unwrap_or_default()
            .iter()
            .enumerate()
        {
            let Some(description) = prose.get(feature.description) else {
                notes.push(format!("{}: no such string", feature.description));
                continue;
            };
            if !read(feature.image, &mut notes) {
                continue;
            }
            illustrated.push(feature.image);
            features.push(Feature {
                slot: slot as u8,
                // The same lookup as the title, with the served table behind
                // it: the heading ids are not in the copy that carries the
                // two Fury descriptions.
                heading: feature
                    .heading
                    .and_then(|id| prose.get(id).or_else(|| strings.get(id)))
                    .map(str::to_string),
                // `get` rather than `get_or_id`: a missing title draws no title,
                // where showing the id would put `FE_PILOT_ASSIST` on screen.
                title: feature
                    .title
                    .and_then(|id| prose.get(id).or_else(|| strings.get(id)))
                    .map(str::to_string),
                description: description.to_string(),
            });
        }
        for (_, entry) in loading.chrome {
            read(entry, &mut notes);
        }
        let mode_stills = chosen.map(|style| style.mode_stills);
        for entry in mode_stills.iter().flat_map(|stills| stills.entries()) {
            read(entry, &mut notes);
        }
        let mut mode_icons = Vec::new();
        if chosen.is_some_and(|style| style.mode_icons.is_empty()) {
            notes.push(
                "loading screen: this style names no mode icon clip; the mode icon panel stays empty"
                    .to_string(),
            );
        }
        for (mode, entry) in chosen.map(|style| style.mode_icons).unwrap_or_default() {
            match archives.read_name(entry) {
                Ok(blob) => mode_icons.push(ModeIcon {
                    mode: *mode,
                    entry: (*entry).to_string(),
                    blob,
                }),
                Err(e) => notes.push(format!("{entry}: {e}; mode {mode} draws no icon")),
            }
        }

        // The features whose picture was read but did not decode into the sheet: they leave
        // `features` too, so a feature's index stays its illustration's.
        let mut undecoded: Vec<usize> = Vec::new();
        let art = (!blobs.is_empty() || !reversed.is_empty()).then(|| {
            let mut sheet_notes = Vec::new();
            let sheet = oag_hud::sprite::Sheet::build_with(&blobs, reversed, &mut sheet_notes);
            notes.append(&mut sheet_notes);
            // A sheet that decoded nothing is `Sheet::default` - one
            // transparent texel - and every lookup below misses, which draws
            // the screen with no pictures rather than with one blank one.
            let at = |entry: &str| {
                sheet.get(entry).map(|placed| {
                    placed.sheet_rect([0.0, 0.0, placed.width as f32, placed.height as f32])
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
            let placed: Vec<Option<[f32; 4]>> = illustrated.iter().map(|entry| at(entry)).collect();
            undecoded = placed
                .iter()
                .enumerate()
                .filter_map(|(at, rect)| rect.is_none().then_some(at))
                .collect();
            let still = |entry: Option<&str>| entry.and_then(at);
            Art {
                mode_stills: mode_stills.map(|stills| ModeStills {
                    modes: stills
                        .modes
                        .iter()
                        .map(|(mode, entry)| (*mode, still(*entry)))
                        .collect(),
                    otherwise: still(stills.otherwise),
                }),
                illustrations: placed.into_iter().flatten().collect(),
                title_arrow: mark(Chrome::TitleArrow),
                subtitle_arrow: mark(Chrome::SubtitleArrow),
                rule: mark(Chrome::Rule),
                corner: mark(Chrome::Corner),
                dot: mark(Chrome::Dot),
                square: mark(Chrome::Square),
                sheet,
            }
        });

        if art.is_none() {
            undecoded = (0..features.len()).collect();
        }
        for at in undecoded.iter().rev() {
            features.remove(*at);
            notes.push(format!(
                "{}: its picture did not decode; the feature is not offered",
                illustrated[*at]
            ));
        }

        // **The source's own tint**, read out of the front end it actually
        // ships rather than spelled here: `HD_BG` and its three neighbours are
        // `FEGlobals`, and Wipeout HD's differ between archives - the served
        // copy decides whether this screen is white-and-blue or black-and-red.
        // See `oag_hd::loading::PALETTE`, which records the constructor those
        // four names were read out of.
        let palette = loading.palette.and_then(|names| {
            let root = title.front_end?.root;
            let globals = &served.as_ref()?.globals;
            let colour = |name: &str| {
                let raw = globals.get(name)?;
                let hex = raw.trim().trim_start_matches("0x").trim_start_matches("0X");
                let argb = u32::from_str_radix(hex, 16).ok()?;
                let byte = |shift: u32| ((argb >> shift) & 0xff) as f32 / 255.0;
                Some([byte(16), byte(8), byte(0), byte(24)])
            };
            let resolved = Palette {
                background: colour(names.background)?,
                ink: colour(names.ink)?,
                accent: colour(names.accent)?,
                dim: colour(names.dim)?,
            };
            notes.push(format!(
                "loading screen palette from {root}: {} {} {} {}",
                names.background, names.ink, names.accent, names.dim
            ));
            Some(resolved)
        });

        let caption = loading.caption.and_then(|id| {
            let text = strings.get(id);
            if text.is_none() {
                notes.push(format!("{id}: no such string; the heading stands in"));
            }
            text.map(str::to_string)
        });

        let labels = loading.labels.map(|ids| {
            let text = |id: &str, notes: &mut Vec<String>| {
                let found = strings.get(id).map(str::to_string);
                if found.is_none() {
                    notes.push(format!("{id}: no such string; its label is not drawn"));
                }
                found
            };
            Labels {
                feature_image: text(ids.feature_image, &mut notes),
                feature_description: text(ids.feature_description, &mut notes),
                progression_bar: text(ids.progression_bar, &mut notes),
                mode_icon: text(ids.mode_icon, &mut notes),
            }
        });

        let layout = read_layout(&mut archives, loading.layout, strings, &mut notes);

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
            palette,
            style: chosen.map(|style| style.name),
            features,
            deck: loading.deck,
            progression: loading.progression,
            layout: layout.0,
            layout_strings: layout.1,
            caption,
            labels,
            mode_icons,
            notes,
        }
    }
}

/// Which styling to take the illustrations from, out of the ones a title ships.
///
/// **The styling is chosen here and the feature is not.** `style` is a saved
/// setting naming one of the title's own - `HD` or `FURY` on Wipeout HD, which
/// ships every illustration twice. Which *feature* is up is drawn per screen by
/// the original and per screen here too; see [`super::Screen::new`].
///
/// No saved name takes the style matching the served front end's ground
/// (`dark_ground`), so the loading screen is in the style the menus are; a title
/// with none matching takes its first. An unknown style name falls back to the
/// title's first and says so, the same way a saved language this source does not
/// carry falls through.
pub(super) fn pick_style<'a>(
    styles: &'a [oag_title::loading::FeatureStyle],
    style: Option<&str>,
    dark_ground: bool,
    notes: &mut Vec<String>,
) -> Option<&'a oag_title::loading::FeatureStyle> {
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
        None => styles
            .iter()
            .find(|style| style.dark_ground == dark_ground)
            .or_else(|| styles.first()),
    }?;
    notes.push(format!(
        "loading screen: {} style, {} feature(s)",
        chosen.name,
        chosen.features.len()
    ));
    Some(chosen)
}

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

/// The title's loading layout file, through the asset layers, and the strings its labels name.
///
/// A file that is absent or refused is said in `notes` and the screen draws the layout this
/// build chose in code: a bad override must not black out the screen.
fn read_layout(
    archives: &mut oag_assets::Archives,
    entry: Option<&str>,
    strings: &StringTable,
    notes: &mut Vec<String>,
) -> (
    Option<oag_hud::loading_layout::LoadingLayouts>,
    Vec<(String, String)>,
) {
    let Some(entry) = entry else {
        return (None, Vec::new());
    };
    let text = match archives.read_name(entry) {
        Ok(blob) => String::from_utf8_lossy(&blob).into_owned(),
        Err(e) => {
            notes.push(format!(
                "{entry}: {e}; the loading layout of this build's code stands in"
            ));
            return (None, Vec::new());
        }
    };
    match oag_hud::loading_layout::LoadingLayouts::parse(&text) {
        Ok(layouts) => {
            let mut ids: Vec<&str> = layouts
                .layouts
                .iter()
                .flat_map(|layout| layout.elements.iter())
                .filter_map(|element| element.string_id.as_deref())
                .collect();
            ids.sort_unstable();
            ids.dedup();
            let resolved = ids
                .into_iter()
                .filter_map(|id| match strings.get(id) {
                    Some(text) => Some((id.to_string(), text.to_string())),
                    None => {
                        notes.push(format!("{id}: no such string; its label is not drawn"));
                        None
                    }
                })
                .collect();
            notes.push(format!("loading layout from {entry}"));
            (Some(layouts), resolved)
        }
        Err(e) => {
            notes.push(format!(
                "{entry}: {e}; the loading layout of this build's code stands in"
            ));
            (None, Vec::new())
        }
    }
}
