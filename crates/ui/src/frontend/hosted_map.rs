//! The 2048 campaign map as a screen a title's own menus hand off to.
//!
//! Wipeout 2048 boots into the map (`newFEshell`). Omega's front end is HD's
//! menus, and its mode chooser opens the same map for the 2048 campaign: the
//! screen is `Cell Selection 2048` in `Campaign2048_Definition.xml`, and its
//! `DefaultPrevious` is the chooser it came from. This is the one map
//! implementation both titles run; what differs is the data (which screen
//! carries the canvas labels) and that leaving the map is the host's business.

use super::wipeout2048::w2048;
use super::*;

/// A map that a host menu opened and takes back.
#[derive(Debug, Clone)]
pub(super) struct MapHost {
    /// The screen holding the canvas, standing in for `newFEshell`.
    pub(super) screen: String,
    /// Back (or the shell's home key) was pressed: the host takes over.
    pub(super) left: bool,
}

impl Frontend {
    /// A front end that opens on the campaign map, `screen` being the screen
    /// of `screens` that carries the canvas.
    #[must_use]
    pub fn hosted_map(
        screens: Screens,
        strings: StringTable,
        languages: Vec<Language>,
        placements: Vec<(String, Placed)>,
        screen: &str,
    ) -> Self {
        let aspect = (SCREEN.0 as u32, SCREEN.1 as u32);
        let mut frontend = Self::booting(
            Sequence {
                steps: vec![Step {
                    state: w2048::NEW_FE_SHELL,
                    movie: MoviePlan::none(aspect),
                }],
                backdrop_parent: None,
            },
            screens,
            strings,
            languages,
            placements,
        );
        frontend.map_host = Some(MapHost {
            screen: screen.to_string(),
            left: false,
        });
        frontend
    }

    /// Whether the player backed out of a hosted map.
    #[must_use]
    pub fn left_map(&self) -> bool {
        self.map_host.as_ref().is_some_and(|host| host.left)
    }

    /// The screen that draws state `current`: the host's canvas screen for the
    /// shell, the state's own name otherwise.
    pub(super) fn screen_for<'a>(&'a self, current: &'a str) -> &'a str {
        match &self.map_host {
            Some(host) if current == w2048::NEW_FE_SHELL => &host.screen,
            _ => current,
        }
    }

    /// Back and the shell's home key leave a hosted map; there is no `Home`
    /// or `GameModeChoice` to go to. Returns whether one was pressed.
    pub(super) fn leave_hosted_map(&mut self, input: &mut Input) -> bool {
        let Some(host) = self.map_host.as_mut() else {
            return false;
        };
        for button in [Button::Circle, Button::Triangle] {
            if input.is_pressed(button) {
                input.consume_press(button);
                host.left = true;
                self.notes.push(format!(
                    "{}: {button:?} pressed, leaving the map",
                    w2048::NEW_FE_SHELL
                ));
                return true;
            }
        }
        false
    }
}
