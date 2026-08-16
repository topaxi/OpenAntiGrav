//! Window titles and the key hints printed under them.
//!
//! One module because both windowed and headless paths print them, and a
//! title belongs beside the keys it is shown with.

/// The window's title while the front end is on screen.
pub(crate) const TITLE: &str = "OpenAntiGrav";

/// Printed before the front end opens: the picker's own keys.
///
/// Escape quits here and only here, because the boot sequence is the first
/// thing on screen and has nothing behind it to go back to.
pub(crate) const MENU_KEYS: &str = "arrow keys or the left stick move, return, X or cross \
     selects, space or start skips, escape quits";

/// And once the menus have the window.
pub(crate) const SHELL_TITLE: &str = "OpenAntiGrav - menu";

/// Printed when the menus open, which have one key the picker does not.
pub(crate) const SHELL_KEYS: &str = "up and down move, left and right change a setting, \
     return, X or cross selects, backspace, circle or escape goes back";

/// And once a race has taken it over.
pub(crate) const RACE_TITLE: &str = "OpenAntiGrav - race";

pub(crate) const RACE_KEYS: &str = "arrow keys or the left stick steer, X, return or R2 thrusts, \
     Q and E or the shoulders are the airbrakes, L2 is both, C fires a pickup and Z absorbs it";

/// What escape does from a race the menus started, and from one `--race` did.
///
/// Escape is "back one level" everywhere; the difference is only that `--race`
/// has no level behind it. See [`Session::escape`].
pub(crate) const ESC_TO_MENU: &str = ", escape returns to the menus";
pub(crate) const ESC_QUITS: &str = ", escape quits";
