//! A `<Block>` widget's own box: what `Block_ParseXml` (`0x0018df70` in
//! Wipeout HD/Fury's `EBOOT.elf`) reads off the element beside the label
//! [`super::Screens::collect_widgets`] already folds into a [`super::Text`].
//!
//! The label and the box are two halves of one widget. The label keeps
//! living in [`super::Screen::texts`], unchanged, so nothing that already
//! reads a `Block`'s text moves; the box lands here, in
//! [`super::Screen::blocks`], for a screen that draws `Block_Render`'s own
//! box (`crate::menu::block`) instead of bare text. See
//! `docs/ghidra/functions/ps3-hdfury-eu/menu-blocks.md`'s "A standalone
//! `<Block>`" section for the attribute chain and every default below.

use super::{Node, Screens, parse_argb};

/// One `<Block>`'s box, as the executable parses it.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockWidget {
    /// The widget's `name`, the same one its [`super::Text`] half carries.
    pub name: Option<String>,
    /// `X`, plus every enclosing `Item` offset - `+0xa4`.
    pub x: f32,
    /// `Y`, likewise - `+0xa8`.
    pub y: f32,
    /// `Width` - `+0xac`. Absent reads as `0`, which `Block_Render`'s own
    /// `w > 0` guard draws as nothing.
    pub width: f32,
    /// `Height` - `+0xb0`, **`40.0` when absent**: `Block_Construct`
    /// (`0x0018b818`) writes that default, and no `<Block>` on either
    /// `EndRace` screen authors one.
    pub height: f32,
    /// `Color` - `+0xbc`, the fill an unfocused block draws in.
    /// `Block_Construct`'s own default is `0xffffffff`.
    pub color: u32,
    /// `ActiveColor` - `+0xc0`, the fill while focused. **`0xff8ac0ca` when
    /// absent**, a literal `Block_Construct` compiles in (not a
    /// `FEGlobals` reference), which is `DATA06`'s own `HD_Blue` value.
    pub active_color: u32,
    /// `ArrowColor` - `+0x154`, the tint of the 32x32
    /// `HD_options_arrow.gtf` marker a `selectable` block creates.
    /// `None` when absent.
    pub arrow_color: Option<u32>,
    /// `shaped` - lands in `+0x11c`, the landing-style byte
    /// `Block_SetLandingStyle` writes on a strip tab: a cut top-right corner
    /// followed by a flat landing.
    pub landing: bool,
    /// `selectable="true"` - calls `0x0018c0e8`, which builds the marker
    /// image and sets `+0x159`, the byte `Block_Update` keys focus
    /// behaviour on.
    pub selectable: bool,
    /// `TextScale` - `+0xec`, the label's scale. **`0.8` when absent**,
    /// `Block_Construct`'s `0x3f4ccccd`; the label half in
    /// [`super::Screen::texts`] reads the plain `scale` attribute instead,
    /// which a `<Block>` does not author.
    pub text_scale: f32,
}

/// `Block_Construct`'s height default.
pub const DEFAULT_HEIGHT: f32 = 40.0;
/// `Block_Construct`'s `ActiveColor` default, `+0xc0 = 0xff8ac0ca`.
pub const DEFAULT_ACTIVE_COLOR: u32 = 0xff8a_c0ca;
/// `Block_Construct`'s `TextScale` default, `+0xec = 0x3f4ccccd`.
pub const DEFAULT_TEXT_SCALE: f32 = 0.8;
/// `Block_Construct`'s `Color` default, `+0xbc = 0xffffffff`.
pub const DEFAULT_COLOR: u32 = 0xffff_ffff;

impl Screens {
    pub(super) fn block_widget_from_node(&self, node: &Node, offset: (f32, f32)) -> BlockWidget {
        let argb = |attr: &str| {
            self.resolve(node.value(attr).unwrap_or_default())
                .and_then(parse_argb)
        };
        BlockWidget {
            name: node.attr("name").map(str::to_string),
            x: self.number(node.value("x")).unwrap_or(0.0) + offset.0,
            y: self.number(node.value("y")).unwrap_or(0.0) + offset.1,
            width: self.number(node.value("width")).unwrap_or(0.0),
            height: self.number(node.value("height")).unwrap_or(DEFAULT_HEIGHT),
            color: argb("Color").unwrap_or(DEFAULT_COLOR),
            active_color: argb("ActiveColor").unwrap_or(DEFAULT_ACTIVE_COLOR),
            arrow_color: argb("ArrowColor"),
            landing: node.flag("shaped").unwrap_or(false),
            selectable: node.flag("selectable").unwrap_or(false),
            text_scale: self
                .number(node.value("TextScale"))
                .unwrap_or(DEFAULT_TEXT_SCALE),
        }
    }
}
