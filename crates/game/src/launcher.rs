//! The chooser shown when a run names no disc image and this machine has more
//! than one.
//!
//! [`crate::source`] has always been able to find *an* image; what it could not
//! do is admit there were others. On a machine holding several pressings the
//! first one on the search path won and the rest were invisible, reachable only
//! by naming a path on the command line. This module is the other half:
//! `source::candidates` lists every image, [`survey`] says what each one
//! actually is, and [`Launcher`] is the cursor over the result.
//!
//! # Nothing here is authored
//!
//! A row's title is [`oag_title::Title::name`], reached through
//! [`oag_source::title::identify`] - the disc's own serial, through the deny-list,
//! which is the same evidence the boot itself uses. Its platform and serial are
//! [`oag_disc::TitleInfo`], read out of `UMD_DATA.BIN`, `SYSTEM.CNF` or
//! `PS3_DISC.SFB`. There is deliberately no table in this file mapping a file
//! name to a game: a name is what a player called their own dump, and reading
//! the disc is the only honest way to answer what it holds.
//!
//! **Region is not shown, because nothing reports one.** It would have to be
//! guessed from the serial's third letter, and the serial itself is on the row
//! already for anyone who reads them.
//!
//! # An image that will not open is still listed
//!
//! The live case is a PS3 disc: `hdfury-ps3-eu.iso` and `hdfury-ps3-eu-dec.iso`
//! sit side by side, identify as the same serial, and only the decrypted one
//! opens - the encrypted one's ISO 9660 directory reads fine and everything
//! inside it is noise. Hiding it would leave a player whose only copy is the
//! encrypted one staring at a list their disc is not on. So it is listed,
//! marked, skipped by the cursor, and carries the fix.
//!
//! # This is not a [`oag_ui::menu`]
//!
//! The menus are a definition of pages and entries, drawn with a title's own
//! [`oag_title::MenuSkin`] and its own font. This screen runs *before* a title
//! is known, so it has neither, and pretending otherwise would mean picking a
//! skin to choose which skin to use. It is its own small thing: one list, one
//! cursor, the engine's own 5x7 glyphs.

use std::path::{Path, PathBuf};

use oag_disc::{DiscImage, Platform, Ps3State};
use oag_gameplay::input::{Button, Input};
use oag_title::Title;

use oag_ui::frontend::{Align, Draw};

use oag_display::space::SCREEN;

pub mod key_entry;
use key_entry::KeyEntry;

/// One disc image the chooser offers.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// What to hand [`oag_source::source::resolve`]'s callers: the path, as a string.
    pub source: String,
    /// The file's own name, which is what a player recognises the row by.
    pub name: String,
    /// What the disc says it runs on, or [`Platform::Unknown`] if it says
    /// nothing.
    pub platform: Platform,
    /// The disc's serial, normalised. `None` on a source that carries none - an
    /// `oag-unpack` extract, or an image whose identity block would not read.
    pub serial: Option<String>,
    /// Whether it can be played, and if not, why not.
    pub state: State,
}

/// Whether a candidate is playable, and what to say when it is not.
#[derive(Debug, Clone)]
pub enum State {
    /// It opened, as this title.
    Playable(&'static Title),
    /// An encrypted PS3 image that no key found opens. Selecting the row opens
    /// the disc-key prompt ([`key_entry`]).
    NeedsKey,
    /// It did not open. The string is for the player, and names the fix when
    /// there is one to name.
    Unavailable(String),
}

impl Candidate {
    /// Whether the cursor may land on this row.
    #[must_use]
    pub fn is_playable(&self) -> bool {
        matches!(self.state, State::Playable(_))
    }

    /// The title this row opens as, or `None` when it did not open.
    #[must_use]
    pub fn playable(&self) -> Option<&'static Title> {
        match &self.state {
            State::Playable(title) => Some(title),
            State::Unavailable(_) | State::NeedsKey => None,
        }
    }

    /// Whether the cursor may land on this row: it plays, or it can be
    /// unlocked from here.
    #[must_use]
    pub fn is_selectable(&self) -> bool {
        matches!(self.state, State::Playable(_) | State::NeedsKey)
    }

    /// The title's name, or the reason it has none.
    #[must_use]
    pub fn title(&self) -> &str {
        match &self.state {
            State::Playable(title) => title.name,
            State::NeedsKey => NEEDS_KEY,
            State::Unavailable(_) => UNAVAILABLE,
        }
    }

    /// The platform and serial, as one column.
    #[must_use]
    pub fn provenance(&self) -> String {
        match &self.serial {
            Some(serial) => format!("{} {serial}", self.platform),
            None => self.platform.to_string(),
        }
    }
}

/// What a row that will not open is headed instead of a title.
const UNAVAILABLE: &str = "WILL NOT OPEN";

/// What an encrypted PS3 row is headed instead of a title.
const NEEDS_KEY: &str = "NEEDS DISC KEY";

/// Opens every candidate far enough to say what it is.
///
/// Two reads apiece, both of which already existed: [`DiscImage::identify`] for
/// the platform and serial, which is a handful of sectors, and
/// [`oag_source::title::identify`] for the title, which mounts the archives and
/// drops them. The second is the expensive one and it is also the only thing
/// that proves a source will open at all, which is exactly what the chooser has
/// to know before it offers a row.
///
/// Order is the order it was given, which is `source::candidates`' - known
/// names first, so the first row is the image a boot with nothing named would
/// have opened.
#[must_use]
pub fn survey(paths: &[PathBuf]) -> Vec<Candidate> {
    paths.iter().map(|path| examine(path)).collect()
}

/// Every playable candidate, one per distinct title.
///
/// Race Remix's own reader of a survey: it wants "which titles can this
/// machine race today", not "which files are on the search path" - two
/// pressings of the same title (a US and an EU disc) are one row, the first
/// one found. Order is `rows`' own, so the same "known names first" rule
/// [`survey`] documents decides which pressing wins.
#[must_use]
pub fn distinct_titles(rows: &[Candidate]) -> Vec<Candidate> {
    let mut seen = std::collections::HashSet::new();
    rows.iter()
        .filter(|row| row.is_playable() && seen.insert(row.title().to_string()))
        .cloned()
        .collect()
}

/// Whether `source` is an encrypted PS3 image no key opens, which a stated
/// source gets the chooser's key prompt for instead of a bare error.
///
/// Cheap by design: one open, which reads sector 0 and, only for an image that
/// declares encrypted regions, checks the oracle.
#[must_use]
pub fn needs_key(source: &str) -> bool {
    DiscImage::open(source).is_ok_and(|disc| disc.ps3_state() == Ps3State::Locked)
}

/// One candidate, opened.
fn examine(path: &Path) -> Candidate {
    let source = path.to_string_lossy().into_owned();
    let name = path.file_name().map_or_else(
        || source.clone(),
        |name| name.to_string_lossy().into_owned(),
    );

    // Deliberately tolerant: an image whose identity block will not read is
    // still worth listing under its file name, because the title open below is
    // what decides whether it plays and it may well succeed anyway.
    let mut locked = false;
    let info = DiscImage::open(path)
        .and_then(|mut disc| {
            locked = disc.ps3_state() == Ps3State::Locked;
            disc.identify()
        })
        .ok();
    let platform = info
        .as_ref()
        .map_or(Platform::Unknown, |info| info.platform);
    let serial = info.and_then(|info| info.serial);

    let state = match oag_source::title::identify(&source) {
        Some(title) => State::Playable(title),
        None if locked => State::NeedsKey,
        None => State::Unavailable(why_not(platform)),
    };

    Candidate {
        source,
        name,
        platform,
        serial,
        state,
    }
}

/// What to tell a player about a source that would not open.
///
/// Only the PS3 case is specific, and it is specific because it is the one
/// where the failure has a known cause and a known fix: a PS3 disc image is
/// layer-1 encrypted until it is decrypted with the disc's own `.dkey`, and its
/// ISO 9660 directory reads perfectly either way - so "the archives are not in
/// here" is what an encrypted image looks like from the inside. The wording
/// follows `just play hd`, which already prints this.
///
/// Everything else gets the honest short answer rather than a guess. A source
/// can fail to open for reasons this cannot see - a truncated dump, a disc from
/// a title nothing here knows - and naming one of them would be inventing a
/// diagnosis.
///
/// **Short enough to draw**, which is why the pointer at
/// `docs/formats/ps3-disc.md` is not in it: the screen is 480 units wide in the
/// engine's own 5x7 face and a line that runs off the right edge tells a player
/// less than a shorter one that fits. `fits_on_screen` in this module's tests
/// is what keeps that true.
fn why_not(platform: Platform) -> String {
    match platform {
        Platform::Ps3 => "no archives here: needs a .dkey beside it".to_string(),
        _ => "no archives this engine recognises".to_string(),
    }
}

/// The long form of [`State::Unavailable`]'s reason, for the boot report.
///
/// `None` for a row that opened. Separate from what the screen draws because
/// stdout has no right edge: this is where the pointer into `docs/` goes, and
/// where a maintainer reading a terminal gets the whole story rather than the
/// half that fits in 480 units.
#[must_use]
pub fn advice(row: &Candidate) -> Option<String> {
    let State::Unavailable(why) = &row.state else {
        return None;
    };
    let mut line = format!("{} will not open: {why}", row.name);
    if row.platform == Platform::Ps3 {
        line.push_str(" (a PS3 image is read encrypted in place; see docs/formats/ps3-disc.md)");
    }
    Some(line)
}

/// The chooser: a list of candidates and a cursor over the playable ones.
#[derive(Debug)]
pub struct Launcher {
    rows: Vec<Candidate>,
    cursor: usize,
    /// What to say instead of a list when there is no disc image to list, from
    /// [`Self::not_found`]. `None` for an ordinary chooser.
    notice: Option<Vec<String>>,
    /// The disc-key prompt, while one is open.
    entry: Option<KeyEntry>,
    /// Whether this build can read a clipboard, for the prompt's PASTE cell.
    can_paste: bool,
    /// The prompt asked for a paste and nothing has serviced it yet.
    paste_requested: bool,
}

impl Launcher {
    /// Takes a surveyed list, parking the cursor on the first playable row.
    ///
    /// A list with nothing playable in it is a real state - a folder of
    /// encrypted images - and it is left with the cursor at zero and
    /// [`Self::pick`] refusing. The screen still says what is there and why
    /// none of it works, which is the entire reason those rows are listed.
    #[must_use]
    pub fn new(rows: Vec<Candidate>) -> Self {
        let cursor = rows.iter().position(Candidate::is_selectable).unwrap_or(0);
        Self {
            rows,
            cursor,
            notice: None,
            entry: None,
            can_paste: false,
            paste_requested: false,
        }
    }

    /// Says whether this build can read a clipboard, which decides whether the
    /// key prompt offers PASTE.
    #[must_use]
    pub fn with_paste(mut self, can_paste: bool) -> Self {
        self.can_paste = can_paste;
        self
    }

    /// A chooser with nothing to choose, which says why instead.
    ///
    /// For a build with no terminal to print `resolve`'s not-found message to
    /// (a phone): the same screen the chooser is drawn on carries the text, so a
    /// player is told what to do rather than shown a window that closes.
    #[must_use]
    pub fn not_found(notice: Vec<String>) -> Self {
        Self {
            rows: Vec::new(),
            cursor: 0,
            notice: Some(notice),
            entry: None,
            can_paste: false,
            paste_requested: false,
        }
    }

    /// The not-found text, when this is that screen.
    #[must_use]
    pub fn notice(&self) -> Option<&[String]> {
        self.notice.as_deref()
    }

    /// The rows, in the order they are drawn.
    #[must_use]
    pub fn rows(&self) -> &[Candidate] {
        &self.rows
    }

    /// Which row the cursor is on.
    #[must_use]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Whether anything here can be played.
    #[must_use]
    pub fn has_playable(&self) -> bool {
        self.rows.iter().any(Candidate::is_playable)
    }

    /// Whether the cursor has anywhere to go: a row that plays or can be
    /// unlocked.
    fn has_selectable(&self) -> bool {
        self.rows.iter().any(Candidate::is_selectable)
    }

    /// The disc-key prompt, while one is open.
    #[must_use]
    pub fn entry(&self) -> Option<&KeyEntry> {
        self.entry.as_ref()
    }

    /// Whether the disc-key prompt is open, so a desk keyboard's keys go to it.
    #[must_use]
    pub fn typing_is_open(&self) -> bool {
        self.entry.is_some()
    }

    /// A desk-keyboard digit, while the prompt is open.
    pub fn type_digit(&mut self, c: char) {
        if let Some(entry) = &mut self.entry {
            entry.type_digit(c);
        }
    }

    /// A desk-keyboard backspace, while the prompt is open.
    pub fn delete_digit(&mut self) {
        if let Some(entry) = &mut self.entry {
            entry.delete();
        }
    }

    /// A desk-keyboard Enter, while the prompt is open.
    pub fn accept_entry(&mut self) {
        if let Some(entry) = &mut self.entry {
            let outcome = entry.accept();
            self.settle(outcome);
        }
    }

    /// Escape, while the prompt is open: closes it and says whether it did.
    pub fn cancel_entry(&mut self) -> bool {
        self.entry.take().is_some()
    }

    /// Asks for a paste into the prompt (Ctrl+V); the stage services it.
    pub fn request_paste(&mut self) {
        if self.entry.is_some() && self.can_paste {
            self.paste_requested = true;
        }
    }

    /// Whether a paste is waiting to be serviced, clearing the request.
    pub fn take_paste_request(&mut self) -> bool {
        std::mem::take(&mut self.paste_requested)
    }

    /// Hands the clipboard's text to the prompt.
    pub fn paste(&mut self, clipboard: &str) {
        if let Some(entry) = &mut self.entry {
            entry.paste(clipboard);
        }
    }

    /// Acts on what the prompt reported: closes it on cancel, closes it and
    /// re-reads the row on a stored key (so the image now opens as its title),
    /// queues a paste on request.
    fn settle(&mut self, outcome: key_entry::Outcome) {
        use key_entry::Outcome;
        match outcome {
            Outcome::Pending => {}
            Outcome::Cancelled => self.entry = None,
            Outcome::WantsPaste => self.paste_requested = self.can_paste,
            Outcome::Stored => {
                self.entry = None;
                if let Some(row) = self.rows.get_mut(self.cursor) {
                    *row = examine(Path::new(&row.source));
                }
            }
        }
    }

    /// Opens the prompt on the row under the cursor, if it needs a key.
    fn open_entry(&mut self) -> bool {
        match self.rows.get(self.cursor) {
            Some(row) if matches!(row.state, State::NeedsKey) => {
                self.entry = Some(KeyEntry::new(Path::new(&row.source), self.can_paste));
                true
            }
            _ => false,
        }
    }

    /// Moves and selects, returning the source picked when one is.
    ///
    /// Edges are consumed the way [`oag_ui::menu::Menu::update`] consumes them,
    /// and for the same reason: one press is one press whichever device saw it,
    /// and a held key must not walk the list.
    pub fn update(&mut self, input: &mut Input) -> Option<String> {
        if let Some(entry) = &mut self.entry {
            let outcome = entry.update(input);
            self.settle(outcome);
            return None;
        }
        if input.take(Button::Down) {
            self.step(1);
        }
        if input.take(Button::Up) {
            self.step(-1);
        }

        let confirmed = input.take(Button::Cross) | input.take(Button::Start);
        if confirmed {
            if self.open_entry() {
                return None;
            }
            self.pick()
        } else {
            None
        }
    }

    /// Consumes a tick of pointer input - see [`oag_ui::pointer`] - and
    /// returns the source picked when one is, on the same terms as
    /// [`Self::update`].
    ///
    /// **Ours**, like the screen. Hovering a playable row moves the cursor
    /// onto it, a click on one picks it, the wheel steps the cursor, and a
    /// row that will not open is not a target at all - the same rule
    /// [`Self::step`] applies, so the pointer cannot land the cursor where
    /// the d-pad refuses to. The rows are tested where [`draw_list`] puts
    /// them: [`row_at`] is the one place that arithmetic is written.
    pub fn pointer(&mut self, pointer: &oag_ui::pointer::Pointer) -> Option<String> {
        if let Some(entry) = &mut self.entry {
            let outcome = entry.pointer(pointer);
            self.settle(outcome);
            return None;
        }
        if pointer.is_idle() {
            return None;
        }
        if pointer.scroll != 0 {
            self.step(pointer.scroll.signum() as isize);
        }
        let row = pointer
            .at
            .and_then(|at| row_at(at, self.rows.len()))
            .filter(|&row| self.rows[row].is_selectable());
        if pointer.moved
            && let Some(row) = row
        {
            self.cursor = row;
        }
        if pointer.clicked
            && let Some(row) = row
        {
            self.cursor = row;
            if self.open_entry() {
                return None;
            }
            return self.pick();
        }
        None
    }

    /// The source under the cursor, if it is one that can be played.
    #[must_use]
    pub fn pick(&self) -> Option<String> {
        self.rows
            .get(self.cursor)
            .filter(|row| row.is_playable())
            .map(|row| row.source.clone())
    }

    /// Moves the cursor to the next playable row in `direction`, wrapping.
    ///
    /// Rows that will not open are stepped over rather than landed on: a cursor
    /// that can sit on a row nothing will happen from is a screen that looks
    /// broken. Nothing playable at all leaves the cursor where it is rather
    /// than looping forever.
    fn step(&mut self, direction: isize) {
        let count = self.rows.len();
        if count == 0 || !self.has_selectable() {
            return;
        }

        let mut at = self.cursor;
        for _ in 0..count {
            at = at
                .wrapping_add_signed(direction)
                .wrapping_add(count)
                .rem_euclid(count);
            if self.rows[at].is_selectable() {
                self.cursor = at;
                return;
            }
        }
    }
}

/// The lines for a build that has nowhere to print `resolve`'s not-found message.
///
/// `images` is the directory a disc image is searched for in, spelled the way
/// `adb push` needs it. Each line fits the 480-unit screen in the engine's own
/// 5x7 face (74 characters), which is why the destination is a line of its own
/// and `not_found_lines_fit_the_screen` pins it.
#[must_use]
pub fn not_found_notice(images: &str) -> Vec<String> {
    vec![
        "OpenAntiGrav ships no game content.".to_string(),
        "Copy your own disc image onto the device, then start".to_string(),
        "the app again. From a computer with the phone attached:".to_string(),
        String::new(),
        format!("adb push {} \\", oag_source::source::IMAGE_NAMES[0]),
        format!("  {images}/"),
        String::new(),
        "Any .chd or .iso name works for Pulse, Pure and HD.".to_string(),
    ]
}

/// What the screen looks like: a heading, one line per candidate, and a footer.
///
/// Drawn in the 480x272 grid every other screen of ours is authored in - this
/// one has no source, so it has no source's grid to borrow, and the renderer's
/// own default is the PSP's.
#[must_use]
pub fn draw_list(launcher: &Launcher) -> Vec<Draw> {
    if let Some(entry) = launcher.entry() {
        return entry.draw();
    }
    let mut out = vec![Draw::Fill {
        rect: [0.0, 0.0, SCREEN.0, SCREEN.1],
        color: BACKDROP,
    }];

    out.push(text(MARGIN, 24.0, 1.0, HEADING, "OPENANTIGRAV"));
    if let Some(notice) = launcher.notice() {
        out.push(text(
            MARGIN,
            40.0,
            1.0,
            UNAVAILABLE_COLOUR,
            "NO DISC IMAGE FOUND",
        ));
        for (index, line) in notice.iter().enumerate() {
            out.push(text(
                MARGIN,
                FIRST_ROW + index as f32 * ROW,
                1.0,
                TEXT,
                line,
            ));
        }
        return out;
    }
    out.push(text(MARGIN, 40.0, 1.0, DIM, "SELECT A DISC IMAGE"));

    for (index, row) in launcher.rows().iter().enumerate() {
        let y = FIRST_ROW + index as f32 * ROW;
        let selected = index == launcher.cursor() && row.is_selectable();
        let colour = if !row.is_selectable() {
            UNAVAILABLE_COLOUR
        } else if selected {
            SELECTED
        } else {
            TEXT
        };

        if selected {
            out.push(Draw::Fill {
                rect: [MARGIN - 4.0, y - 2.0, SCREEN.0 - MARGIN * 2.0 + 8.0, ROW],
                color: HIGHLIGHT,
            });
        }

        out.push(text(MARGIN, y, 1.0, colour, row.title()));
        out.push(text(TITLE_COLUMN, y, 1.0, colour, &row.provenance()));
        out.push(text(NAME_COLUMN, y, 1.0, colour, &row.name));
    }

    // Why each unlistenable row will not open, under the list rather than on
    // it: they are sentences, the rows are a table, and the rows the cursor
    // cannot reach are the ones a player most needs a sentence for. One line
    // each, because two encrypted images have two different names.
    let mut note = FIRST_ROW + (launcher.rows().len() as f32 + 1.0) * ROW;
    for row in launcher.rows() {
        let why = match &row.state {
            State::Unavailable(why) => why.as_str(),
            State::NeedsKey => "ENCRYPTED - PRESS X TO ENTER THE DISC KEY",
            State::Playable(_) => continue,
        };
        out.push(text(
            MARGIN,
            note,
            1.0,
            UNAVAILABLE_COLOUR,
            &format!("{}: {why}", row.name),
        ));
        note += ROW;
    }

    // Always drawn, and at the bottom rather than after the notes: the keys are
    // what a player needs on the first frame, and a screen where they vanish as
    // soon as something is wrong is a screen that helps least when it matters.
    out.push(text(MARGIN, SCREEN.1 - 20.0, 1.0, DIM, HINT));

    // The build's own short commit hash, bottom right - `env!` reads
    // `OAG_GIT_HASH`, set at compile time by `build.rs`. This is the one
    // screen every build shows before anything else can go wrong, which
    // makes it the answer to "which build is this" for a bug report or a
    // Deck deploy that isn't finding what a newer commit should have fixed.
    out.push(Draw::Text {
        x: SCREEN.0 - MARGIN,
        y: SCREEN.1 - 20.0,
        scale: 1.0,
        color: DIM,
        border: None,
        align: Align::Right,
        text: env!("OAG_GIT_HASH").to_string(),
        wrap_width: None,
    });

    out
}

/// Which row a point is over: the band [`draw_list`] highlights the
/// selected row with, from `FIRST_ROW` down at `ROW` a step and the
/// margin's width across.
fn row_at(at: (f32, f32), count: usize) -> Option<usize> {
    oag_ui::pointer::row_at(
        at,
        MARGIN - 4.0,
        SCREEN.0 - MARGIN * 2.0 + 8.0,
        FIRST_ROW - 2.0,
        ROW,
        count,
    )
}

/// One line of text, left aligned, with no border.
fn text(x: f32, y: f32, scale: f32, color: [f32; 4], body: &str) -> Draw {
    Draw::Text {
        x,
        y,
        scale,
        color,
        border: None,
        align: Align::Left,
        text: body.to_string(),
        wrap_width: None,
    }
}

/// Where the list starts, and how far apart its rows are.
const MARGIN: f32 = 16.0;
const FIRST_ROW: f32 = 64.0;
const ROW: f32 = 14.0;
/// Where the platform-and-serial column starts, and the file name after it.
///
/// **Wide enough for `oag_omega::TITLE.name`**, the longest of the five at 25
/// characters (150 units in the engine's own 5x7 face) against Wipeout HD's
/// 10 - `title_and_provenance_columns_do_not_overlap_their_neighbour` in this
/// module's tests is what caught the overlap a narrower column left when
/// Omega's row first landed here, and what would catch it again.
const TITLE_COLUMN: f32 = 176.0;
const NAME_COLUMN: f32 = 280.0;

const BACKDROP: [f32; 4] = [0.02, 0.03, 0.06, 1.0];
const HIGHLIGHT: [f32; 4] = [0.16, 0.36, 0.62, 1.0];
const HEADING: [f32; 4] = [0.92, 0.95, 1.0, 1.0];
const TEXT: [f32; 4] = [0.72, 0.78, 0.86, 1.0];
const SELECTED: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const DIM: [f32; 4] = [0.48, 0.53, 0.60, 1.0];
const UNAVAILABLE_COLOUR: [f32; 4] = [0.55, 0.36, 0.36, 1.0];

const HINT: &str = "UP/DOWN CHOOSE   ENTER OR X START   ESCAPE QUIT";

#[cfg(test)]
mod tests;
