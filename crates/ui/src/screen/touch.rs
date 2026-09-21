//! The widgets a touch-driven front end authors and the others do not: the
//! `<TouchButton>` icon tile, and the `<LoadXML>` include a root pulls its
//! screens in through.
//!
//! Split out of `screen.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`. Both readings are Wipeout 2048's
//! (`docs/formats/2048-frontend.md`): its `NEWGUI/Skin.xml` root declares no
//! screen of its own, only `<LoadXML>` includes, and every screen a player
//! taps is a row of `<TouchButton>`s - a widget no other title's front end
//! authors at all. Neither struct carries a number the disc does not state.

use super::{Node, Screens, parse_argb};

/// One `<TouchButton>` widget, exactly as its own `<Values>` authors it.
///
/// **Authored, confidence 92** - the cap `2048-frontend.md` puts on a claim
/// read straight out of the disc's own XML. Every field is an attribute the
/// widget carries; a widget that omits one gets `None` or the zero the
/// format's own `Image`/`Text` readers already default to, never a number
/// chosen here. `oag_title::TouchButton` carries the same `idstring`/`x`/
/// `y`/`width`/`height` as a compile-time table for the two grids the type
/// names; this is the runtime reading of the same file, and the one that
/// also has the icon's `Src` and the tap's `redirect`, which that table
/// deliberately does not - see `oag_title::touch`'s module docs.
#[derive(Debug, Clone, PartialEq)]
pub struct TouchButton {
    /// The `name` attribute. Two of `Home`'s five buttons omit it.
    pub name: Option<String>,
    /// The `idstring` the label under the icon is looked up by. `None` on
    /// the bare confirm/back ticks, which author `string=""` or nothing.
    pub idstring: Option<String>,
    /// A literal `String`, for the buttons that carry one instead of an id.
    pub string: Option<String>,
    /// Left edge, in the screen's own grid.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Authored width; the tile the icon is drawn on.
    pub width: f32,
    /// Authored height.
    pub height: f32,
    /// The icon's texture, exactly as `Src` spells it - `.gtf` on 2048's
    /// XML even though the archive holds `.gxt`; the sprite loader is what
    /// knows that.
    pub src: Option<String>,
    /// Where a tap goes, from the widget's own `redirect`. `None` on a
    /// `toggle="true"` button, whose tap selects rather than leaves.
    pub redirect: Option<String>,
    /// `toggle="true"`: a tap marks the button chosen and a separate confirm
    /// button leaves the screen - `GameModeChoice`'s four mode tiles.
    pub toggle: bool,
    /// The `ImageSize` attribute: how much of the tile the icon fills, as a
    /// fraction. `None` when the widget does not say; what the executable
    /// does then is unread, so a drawer picks and says so.
    pub image_size: Option<f32>,
    /// `StringWidthLimit`: the pixel width the label wraps at. `None` when
    /// the widget does not say, which draws the label on one line.
    pub string_width_limit: Option<f32>,
    /// Modulating colour as ARGB, white when unstated.
    pub color: u32,
    /// Seconds after the screen appears before the button draws, from
    /// `delay`. The grids stagger their tiles by `0.05`.
    pub delay: f32,
}

/// One `<LoadXML>` include, as the root's own list spells it.
///
/// `src` names an archive path outright; `SrcRel` names a file beside the
/// root. `localised="true"` means the file is shipped once per SIE territory
/// with a `_AS`/`_EU`/`_JP`/`_US` suffix and the engine picks one at load -
/// a *region* switch, not a language one (`2048-frontend.md`'s "declared
/// boot chain" section).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Include {
    /// The path as authored.
    pub src: String,
    /// Whether `src` is relative to the including file's directory (`SrcRel`)
    /// rather than an archive path (`src`).
    pub relative: bool,
    /// Whether the file is region-suffixed, see the type docs.
    pub localised: bool,
    /// `DirectEmbed="true"`: the file's widgets belong to the screen the
    /// `LoadXML` sits in rather than declaring screens of their own -
    /// `TitleScreen` pulls its legal footer in this way, a file holding one
    /// bare `<Text>`.
    pub direct_embed: bool,
    /// The innermost named `Screen` the `LoadXML` sits in, which is where a
    /// [`Self::direct_embed`] include's widgets go. `None` at the root.
    pub screen: Option<String>,
}

impl Include {
    /// The include's own node, or `None` for a `LoadXML` that names nothing.
    pub(super) fn from_node(node: &Node, screen: Option<&str>) -> Option<Self> {
        let localised = node.flag("localised").unwrap_or(false);
        let direct_embed = node.flag("DirectEmbed").unwrap_or(false);
        let (src, relative) = match (node.value("src"), node.value("SrcRel")) {
            (Some(src), _) => (src, false),
            (None, Some(src)) => (src, true),
            (None, None) => return None,
        };
        Some(Self {
            src: src.to_string(),
            relative,
            localised,
            direct_embed,
            screen: screen.map(str::to_string),
        })
    }
}

impl Screens {
    pub(super) fn touch_button_from_node(&self, node: &Node, offset: (f32, f32)) -> TouchButton {
        TouchButton {
            name: node.attr("name").map(str::to_string),
            idstring: node.value("idstring").map(str::to_string),
            string: node
                .value("String")
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            x: self.number(node.value("x")).unwrap_or(0.0) + offset.0,
            y: self.number(node.value("y")).unwrap_or(0.0) + offset.1,
            width: self.number(node.value("width")).unwrap_or(0.0),
            height: self.number(node.value("height")).unwrap_or(0.0),
            src: node.value("Src").map(str::to_string),
            redirect: node.value("redirect").map(str::to_string),
            toggle: node.flag("toggle").unwrap_or(false),
            image_size: self.number(node.value("ImageSize")),
            string_width_limit: self.number(node.value("StringWidthLimit")),
            color: self
                .resolve(node.value("color").unwrap_or_default())
                .and_then(parse_argb)
                .unwrap_or(0xffff_ffff),
            delay: self.number(node.value("delay")).unwrap_or(0.0),
        }
    }
}
