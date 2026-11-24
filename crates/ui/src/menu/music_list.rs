//! A pause screen's music list (Pulse's `MusicList`): the race playlist's song titles, the one
//! shown stepped by LEFT and RIGHT whichever row the cursor is on, drawn under the rows with a
//! two-half arrow pill before it. Nothing plays while paused; the composition root reads
//! [`MusicList::pick`] when the race resumes and starts that song from its beginning
//! (`docs/ghidra/functions/psp-pulse-usa/music-list.md`).
//!
//! The list lives on the pause screen's root page only: leaving it for another page drops a
//! pick, so the list shows the playing song again on the way back (measured, GAME OPTIONS and
//! back).

use oag_title::PauseMusicList;

use super::nav::{Dir, Nav};
use super::pointer::{Part, Region};
use super::{Menu, Skin};
use crate::frontend::{Align, Draw, Placed};

/// The song titles, the one playing and the one shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicList {
    titles: Vec<String>,
    playing: usize,
    selected: usize,
    wraps: bool,
}

impl MusicList {
    /// A list over `titles`, shown on `playing` (clamped to the list). `wraps` is the widget's
    /// `Wrap`.
    #[must_use]
    pub fn new(titles: Vec<String>, playing: usize, wraps: bool) -> Self {
        let playing = playing.min(titles.len().saturating_sub(1));
        Self {
            titles,
            playing,
            selected: playing,
            wraps,
        }
    }

    /// The index shown.
    #[must_use]
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// The title shown, `None` for an empty list.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.titles.get(self.selected).map(String::as_str)
    }

    /// The song to start when the race resumes: the one shown, when it is not the one playing.
    #[must_use]
    pub fn pick(&self) -> Option<usize> {
        (self.selected != self.playing).then_some(self.selected)
    }

    /// Whether a step `by` (`-1` or `1`) has anywhere to go: the arrow pointing that way is lit.
    #[must_use]
    pub fn can_step(&self, by: i32) -> bool {
        let last = self.titles.len().saturating_sub(1);
        self.titles.len() > 1
            && (self.wraps
                || if by < 0 {
                    self.selected > 0
                } else {
                    self.selected < last
                })
    }

    /// Whether the selection is at the end a step `by` points to (the first song for `-1`, the
    /// last for `1`), wrapping or not: what shades that arrow half.
    #[must_use]
    pub fn at_end(&self, by: i32) -> bool {
        if by < 0 {
            self.selected == 0
        } else {
            self.selected + 1 >= self.titles.len()
        }
    }

    /// One step: `UPDOWN` when it moves, `DECLINE` and no change at an end the list does not
    /// wrap past (`ListWidget_StepBack`, `ListWidget_StepForward`).
    fn step(&mut self, by: i32) -> Nav {
        let dir = if by < 0 { Dir::Left } else { Dir::Right };
        if !self.can_step(by) {
            return Nav::Decline;
        }
        let len = self.titles.len();
        self.selected = if by < 0 {
            (self.selected + len - 1) % len
        } else {
            (self.selected + 1) % len
        };
        Nav::Moved(Some(dir))
    }
}

/// What the frame draws the list with: the title's numbers and the arrows' cuts in the atlas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Art {
    /// The title's numbers.
    pub spec: PauseMusicList,
    /// The two arrows' atlas rects, left then right; `None` where the sheet is not loaded, and
    /// then the title is drawn alone.
    pub arrows: Option<[[f32; 4]; 2]>,
}

impl Art {
    /// The art for `spec` out of `sprites`.
    #[must_use]
    pub fn resolve(spec: PauseMusicList, sprites: &[(String, Placed)]) -> Self {
        let placed = sprites
            .iter()
            .find(|(src, _)| src.eq_ignore_ascii_case(spec.sheet))
            .map(|(_, placed)| *placed);
        Self {
            spec,
            arrows: placed.map(|placed| spec.arrows.map(|rect| placed.sheet_rect(rect))),
        }
    }
}

/// Where the list's text and arrows sit, in the grid the page is drawn in.
struct Spots {
    text: (f32, f32),
    arrows: [[f32; 4]; 2],
}

fn spots(skin: &Skin, spec: &PauseMusicList) -> Spots {
    let (sx, sy) = skin.theirs_scale();
    let x = skin.menu_x();
    let y = spec.y * sy;
    let at = |side: usize| {
        let [_, _, w, h] = spec.arrows[side];
        let (dx, dy) = spec.arrow_offset[side];
        [x + dx * sx, y + dy * sy, w * sx, h * sy]
    };
    Spots {
        text: (x, y),
        arrows: [at(0), at(1)],
    }
}

impl Menu {
    /// Gives the pause screen its music list, or takes it away.
    pub fn set_music_list(&mut self, list: Option<MusicList>) {
        self.music = list;
    }

    /// The pause screen's music list, while its page is the one on screen.
    #[must_use]
    pub fn music_list(&self) -> Option<&MusicList> {
        self.music.as_ref().filter(|_| self.on_pause_root())
    }

    fn on_pause_root(&self) -> bool {
        self.pause_screen && self.depth() == 1
    }

    /// Back on the playing song once the pause page is not the one on screen: another page
    /// drops a pick.
    pub(super) fn settle_music_list(&mut self) {
        if !self.on_pause_root()
            && let Some(list) = self.music.as_mut()
        {
            list.selected = list.playing;
        }
    }

    /// Steps the music list `by` and logs its sound. Returns whether there is a list to step.
    pub(super) fn step_music(&mut self, by: i32) -> bool {
        if !self.on_pause_root() {
            return false;
        }
        let Some(list) = self.music.as_mut() else {
            return false;
        };
        let nav = list.step(by);
        self.nav.push(nav);
        true
    }
}

/// The list's draws: the arrow pill, a half dimmed where it points past an end, then the title.
/// Nothing when the menu has no list on screen.
pub fn draw(menu: &Menu, skin: &Skin, art: &Art, out: &mut Vec<Draw>) {
    let Some(list) = menu.music_list() else {
        return;
    };
    let spots = spots(skin, &art.spec);
    let ink = skin.normal();
    // `ListWidget_ShadeArrows`: a half at the end it points to keeps part of its alpha.
    let shade = |at_end: bool| {
        if at_end {
            [ink[0], ink[1], ink[2], ink[3] * art.spec.arrow_dim]
        } else {
            ink
        }
    };
    if let Some(uvs) = art.arrows {
        for ((rect, uv), by) in spots.arrows.into_iter().zip(uvs).zip([-1, 1]) {
            out.push(Draw::Sprite {
                rect,
                uv,
                color: shade(list.at_end(by)),
            });
        }
    }
    // `small` is the small face's slot; `menu` is the face the rows are drawn in.
    let face = if art.spec.font.eq_ignore_ascii_case("small") {
        skin.small_font()
    } else {
        None
    };
    if let Some(title) = list.title() {
        out.push(Draw::in_role(
            face,
            spots.text.0,
            spots.text.1,
            skin.row_scale(),
            ink,
            Align::Left,
            title.to_string(),
        ));
    }
}

/// The list's pointer targets: each arrow half steps its way, and the title steps forward
/// (**chosen**: the original has no pointer). `row` is the cursor's, so a hover moves nothing.
#[must_use]
pub fn regions(menu: &Menu, skin: &Skin, art: &Art, measure: &dyn Fn(&str) -> f32) -> Vec<Region> {
    let Some(list) = menu.music_list() else {
        return Vec::new();
    };
    let spots = spots(skin, &art.spec);
    let [left, right] = spots.arrows;
    let height = left[3];
    let width = list
        .title()
        .map_or(0.0, |title| measure(title) * skin.row_scale());
    let row = menu.selected();
    vec![
        Region {
            row,
            part: Part::MusicRow,
            rect: [spots.text.0, left[1], width, height],
        },
        Region {
            row,
            part: Part::MusicBack,
            rect: [left[0], left[1], right[0] - left[0], height],
        },
        Region {
            row,
            part: Part::MusicForward,
            rect: [right[0], right[1], spots.text.0 - right[0], height],
        },
    ]
}
