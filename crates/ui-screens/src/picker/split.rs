//! `Team SelectionSplit`: both players' Ship Selects on one authored screen.
//!
//! **Read off Wipeout Pulse's PS2 disc**: `Selection_Definition.xml` there
//! carries `Team SelectionSplit` beside `Team Selection`, one set of widgets per
//! player - `honey0`, `Speed Bar0`, `skin0`, `List Team0` for the first and the
//! `1` set below it - on one page, top and bottom, each list `focus="true"` with
//! a global of its own (`Team0`, `Team1`). So the original's split screen picks
//! **both players at once**, which is what [`Layout::read_split`] hands the
//! composition root: one [`Layout`] per player, each exactly the single-player
//! shape and read by every function that reads one. See
//! `docs/ui/selection-screens.md`.
//!
//! The widget tree is flattened when it is parsed, so which half a widget
//! belongs to is read back: by the player index its name carries when it has one,
//! and by its height above or below the second panel when it does not
//! (`Infogradient`, the rules and bars with no name).
//!
//! **Chosen, not measured:** the preview rect of each half (the original places
//! its craft in code), which fills the column between the arrows.

use oag_ui::language::StringTable;
use oag_ui::screen::{Screen, Screens};

use super::{FaceScales, Kind, Layout, PSP_GRID};

/// The authored screen's name.
const SPLIT: &str = "Team SelectionSplit";

/// A widget's player, off the digit its name ends in; `None` when it has no
/// player index.
fn named_for(name: Option<&str>) -> Option<usize> {
    let name = name?;
    if name.len() < 2 {
        return None;
    }
    match name.as_bytes().last()? {
        b'0' => Some(0),
        b'1' => Some(1),
        _ => None,
    }
}

impl Layout {
    /// The two players' layouts off `Team SelectionSplit`, first player first,
    /// or `None` when the source authors no such screen or it does not hold two
    /// panels.
    ///
    /// The second layout carries no title: the screen has one, which the first
    /// draws.
    #[must_use]
    pub fn read_split(
        screens: &Screens,
        strings: &StringTable,
        faces: FaceScales,
        grid: [f32; 2],
    ) -> Option<[Self; 2]> {
        let source = screens.by_name(SPLIT)?;
        let scale = [grid[0] / PSP_GRID[0], grid[1] / PSP_GRID[1]];
        let mut panels: Vec<f32> = source
            .fills
            .iter()
            .filter(|fill| fill.name.as_deref() == Some("Infogradient"))
            .map(|fill| fill.y)
            .collect();
        panels.sort_by(f32::total_cmp);
        let &[first_panel, second_panel] = panels.as_slice() else {
            return None;
        };
        let _ = first_panel;
        let seats: Vec<Self> = (0..2)
            .map(|seat| {
                let half = Self::half(source, seat, second_panel);
                let mut layout = Self::from_screen(half, SPLIT, Kind::Ship, strings, faces, scale)?;
                layout.preview = preview_between_arrows(&layout.screen, layout.panel, scale);
                if seat == 1 {
                    layout.title = String::new();
                }
                Some(layout)
            })
            .collect::<Option<_>>()?;
        seats.try_into().ok()
    }

    /// `source` reduced to one player's widgets, under the single-player names.
    fn half(source: &Screen, seat: usize, second_panel: f32) -> Screen {
        let owns = |name: &Option<String>, y: f32| match named_for(name.as_deref()) {
            Some(named) => named == seat,
            None => (y >= second_panel) == (seat == 1),
        };
        let strip = |name: &mut Option<String>| {
            if named_for(name.as_deref()).is_some()
                && let Some(n) = name
            {
                n.pop();
            }
        };
        let mut screen = source.clone();
        // The online lobby's status line belongs to neither half.
        screen.texts.retain(|text| {
            text.name.as_deref() != Some("netLobbyStatus") && owns(&text.name, text.y)
        });
        screen.images.retain(|image| owns(&image.name, image.y));
        screen.fills.retain(|fill| owns(&fill.name, fill.y));
        screen
            .texts
            .iter_mut()
            .for_each(|text| strip(&mut text.name));
        screen
            .images
            .iter_mut()
            .for_each(|image| strip(&mut image.name));
        screen
            .fills
            .iter_mut()
            .for_each(|fill| strip(&mut fill.name));
        screen
    }
}

/// Where the craft goes: the column between the screen's left edge and the
/// panel, between the up arrow's bottom and the down arrow's top, so the counter
/// beside the down arrow stays clear of the 3D pass.
fn preview_between_arrows(screen: &Screen, panel: [f32; 4], scale: [f32; 2]) -> [f32; 4] {
    let arrow = |name: &str| {
        screen
            .images
            .iter()
            .find(|i| i.name.as_deref() == Some(name))
    };
    let (Some(up), Some(down)) = (arrow("up arrow"), arrow("down arrow")) else {
        return [
            10.0 * scale[0],
            panel[1],
            panel[0] - 20.0 * scale[0],
            panel[3],
        ];
    };
    let top = up.y + up.height.unwrap_or(0.0);
    [
        10.0 * scale[0],
        top,
        panel[0] - 20.0 * scale[0],
        (down.y - top).max(1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::picker::{Details, Entry, Event, Picker};
    use oag_gameplay::input::{Button, Input};

    /// `Team SelectionSplit` in miniature: two sets a panel's height apart, one
    /// title, one unsuffixed rule in each half and the lone `Suggest` line.
    const XML: &str = r#"
<Screen name="Top">
<Screen type="TeamSelection" name="Team SelectionSplit">
<Text idstring="RC_SHIPSEL" font="Title" x="0" y="0"></Text>
<LeftLayer transition="0.5">
<Image name="up arrow0" x="156" y="54" width="23" height="23" U="242" V="71" TxtrWidth="17" TxtrHeight="14" src="a.mip"></Image>
<Image name="down arrow0" x="156" y="194" width="23" height="23" U="242" V="94" TxtrWidth="17" TxtrHeight="14" src="a.mip"></Image>
<Text name="honey0" String="1/1" x="180" y="191"></Text>
<Image name="Infogradient" x="387" y="9" width="227" height="72" Color1="0x2f000000" Color2="0x2f000000" Color3="0x2f000000" Color4="0x2f000000"></Image>
<Image width="113" height="2" Color1="0x00ffffff" Color2="0x00ffffff" Color3="0x3fffffff" Color4="0x3fffffff" x="387" y="63"></Image>
<Image name="up arrow1" x="156" y="230" width="23" height="23" U="242" V="71" TxtrWidth="17" TxtrHeight="14" src="a.mip"></Image>
<Image name="down arrow1" x="156" y="370" width="23" height="23" U="242" V="94" TxtrWidth="17" TxtrHeight="14" src="a.mip"></Image>
<Text name="honey1" String="1/1" x="180" y="367"></Text>
<Image name="Infogradient" x="387" y="204" width="227" height="72" Color1="0x2f000000" Color2="0x2f000000" Color3="0x2f000000" Color4="0x2f000000"></Image>
<Image width="113" height="2" Color1="0x00ffffff" Color2="0x00ffffff" Color3="0x3fffffff" Color4="0x3fffffff" x="387" y="258"></Image>
<Text name="Suggest" String="x" x="406" y="316"></Text>
</LeftLayer>
</Screen>
</Screen>
"#;

    fn halves() -> [Layout; 2] {
        Layout::read_split(
            &Screens::from_xml(XML),
            &StringTable::from_xml(
                r#"<Strings><Entry ID="RC_SHIPSEL" String="SHIP SELECT"></Entry></Strings>"#,
            ),
            FaceScales::default(),
            [640.0, 448.0],
        )
        .expect("two panels")
    }

    #[test]
    fn each_half_keeps_its_own_widgets_under_the_single_player_names() {
        let [first, second] = halves();
        let names = |layout: &Layout| -> Vec<String> {
            let mut out: Vec<String> = layout
                .screen
                .images
                .iter()
                .filter_map(|i| i.name.clone())
                .chain(layout.screen.texts.iter().filter_map(|t| t.name.clone()))
                .collect();
            out.sort();
            out
        };
        assert_eq!(names(&first), ["down arrow", "honey", "up arrow"]);
        assert_eq!(
            names(&second),
            ["Suggest", "down arrow", "honey", "up arrow"]
        );
        assert_eq!(first.panel[1], 9.0);
        assert_eq!(second.panel[1], 204.0);
        // The unnamed rule at y 63 is the first set's, the one at 258 the second's.
        assert_eq!(first.screen.fills.len(), 2, "the panel and its rule");
        assert_eq!(second.screen.fills.len(), 2);
        assert!(first.screen.fills.iter().all(|f| f.y < 200.0));
        assert!(second.screen.fills.iter().all(|f| f.y >= 200.0));
    }

    #[test]
    fn one_title_and_a_craft_between_each_pair_of_arrows() {
        let [first, second] = halves();
        assert_eq!(first.title, "SHIP SELECT");
        assert_eq!(second.title, "");
        assert_eq!(first.preview[1], 54.0 + 23.0);
        assert_eq!(first.preview[3], 194.0 - (54.0 + 23.0));
        assert_eq!(second.preview[1], 230.0 + 23.0);
    }

    #[test]
    fn a_source_with_no_split_screen_reads_none() {
        let screens = Screens::from_xml(
            r#"<Screen name="Top"><Screen name="Team Selection"></Screen></Screen>"#,
        );
        assert!(
            Layout::read_split(
                &screens,
                &StringTable::default(),
                FaceScales::default(),
                [640.0, 448.0]
            )
            .is_none()
        );
    }

    fn team(id: &str) -> Entry {
        Entry {
            id: id.to_string(),
            label: id.to_string(),
            details: Details::Ship {
                rating: None,
                variants: Vec::new(),
                stats: Vec::new(),
                loyalty: None,
                models: Vec::new(),
            },
        }
    }

    fn press(button: Button) -> Input {
        let mut input = Input::new();
        input.begin_frame(1 << button as u32);
        input
    }

    /// A confirmed seat holds its pick: nothing moves it, and only back is heard
    /// (which the session reads as taking the confirm back).
    #[test]
    fn a_locked_picker_ignores_everything_but_back() {
        let mut picker = Picker::new(Kind::Ship, vec![team("a"), team("b")], None, None);
        picker.lock();
        assert!(picker.is_locked());
        assert!(picker.update(&mut press(Button::Down)).is_empty());
        assert_eq!(picker.index(), 0);
        assert!(picker.update(&mut press(Button::Cross)).is_empty());
        assert_eq!(picker.update(&mut press(Button::Circle)), vec![Event::Back]);
        picker.unlock();
        assert_eq!(picker.update(&mut press(Button::Down)), vec![Event::Moved]);
        assert_eq!(picker.index(), 1);
    }
}
