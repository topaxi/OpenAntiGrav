//! Wipeout Pure's SINGLE PLAYER flow: the chain of one-choice screens its front
//! end walks from the main menu to a race, read off the screens' own
//! `<Redirect>`s.
//!
//! Two files author it. `Data\Plugins\PI001\GUI\Skin.xml` holds `Main Menu`
//! (whose `Mode` menu names `Single Player` first) and `Single Player` (whose
//! `Mode` menu names the five modes and redirects each to a first screen);
//! `Selection_Definition.xml` holds `Class Selection`, `League Selection`,
//! `Tournament Selection`, `Track Selection`, `Zone Track Selection` and `Team
//! Selection`, each with the redirect that names the screen after it
//! (`docs/formats/race-setup.md`).
//!
//! The reader keeps the redirects as data and evaluates them: [`Flow::route`]
//! answers "which screens does this mode walk", so a screen order is never
//! written into the game. Branches that only apply to Multiplayer are skipped,
//! as the player is inside SINGLE PLAYER.

use crate::fexml::{self, Node};

/// Why a file did not read as the flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A screen or menu the flow needs is not in the file.
    Missing(String),
    /// A redirect names a screen this reader has no step for.
    UnknownScreen(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(what) => write!(f, "missing from the front-end XML: {what}"),
            Self::UnknownScreen(name) => {
                write!(f, "a redirect names a screen with no step: {name}")
            }
        }
    }
}

impl std::error::Error for Error {}

/// A result with this module's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// One screen of the chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step {
    /// `Single Player`: the mode list.
    Mode,
    /// `Class Selection`.
    Class,
    /// `League Selection`.
    League,
    /// `Tournament Selection`.
    Tournament,
    /// `Track Selection`: the circuits of the chosen league.
    Track,
    /// `Zone Track Selection`.
    ZoneTrack,
    /// `Team Selection`.
    Team,
    /// `Launch Game`: the race starts.
    Launch,
}

impl Step {
    /// The screen's authored `name`.
    #[must_use]
    pub fn screen(self) -> &'static str {
        match self {
            Self::Mode => "Single Player",
            Self::Class => "Class Selection",
            Self::League => "League Selection",
            Self::Tournament => "Tournament Selection",
            Self::Track => "Track Selection",
            Self::ZoneTrack => "Zone Track Selection",
            Self::Team => "Team Selection",
            Self::Launch => "Launch Game",
        }
    }

    fn from_screen(name: &str) -> Option<Self> {
        [
            Self::Mode,
            Self::Class,
            Self::League,
            Self::Tournament,
            Self::Track,
            Self::ZoneTrack,
            Self::Team,
            Self::Launch,
        ]
        .into_iter()
        .find(|step| step.screen() == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Branch {
    item: String,
    equals: String,
    goto: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Redirect {
    name: Option<String>,
    branches: Vec<Branch>,
    default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Screen {
    name: String,
    redirects: Vec<Redirect>,
}

/// The authored flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flow {
    /// `Main Menu`'s `Mode` entries, as the idstrings the string table holds.
    pub main_modes: Vec<String>,
    /// `Single Player`'s `Mode` entries: Arcade, Tournament, Time Trial, Zone,
    /// Free Play.
    pub modes: Vec<String>,
    /// `Class Selection`'s `Class` entries, in order.
    pub classes: Vec<String>,
    screens: Vec<Screen>,
}

fn collect<'a>(node: &'a Node, out: &mut Vec<&'a Node>) {
    for child in &node.children {
        if child.name.eq_ignore_ascii_case("Screen") && child.attr("name").is_some() {
            out.push(child);
        }
        collect(child, out);
    }
}

fn menu_entries(screen: &Node, menu: &str) -> Option<Vec<String>> {
    let menu = screen
        .children_named("Menu")
        .find(|node| node.attr("name") == Some(menu))?;
    Some(
        menu.children_named("Entry")
            .filter_map(|entry| entry.attr("idstring").map(str::to_string))
            .collect(),
    )
}

fn read_redirect(node: &Node) -> Redirect {
    let mut out = Redirect {
        name: node.attr("name").map(str::to_string),
        ..Redirect::default()
    };
    for child in &node.children {
        let goto = child.attr("goto").map(str::to_string);
        if child.name.eq_ignore_ascii_case("Entry")
            && let (Some(item), Some(equals), Some(goto)) =
                (child.attr("item"), child.attr("equals"), goto.clone())
        {
            out.branches.push(Branch {
                item: item.to_string(),
                equals: equals.to_string(),
                goto,
            });
        } else if child.name.eq_ignore_ascii_case("Default") {
            out.default = goto;
        }
    }
    out
}

/// Reads the flow from `Skin.xml` and `Selection_Definition.xml`, both as text.
///
/// # Errors
///
/// A screen or menu the flow is made of is absent.
pub fn parse(skin: &str, selection: &str) -> Result<Flow> {
    let skin = fexml::parse(skin);
    let selection = fexml::parse(selection);
    let mut nodes = Vec::new();
    collect(&skin, &mut nodes);
    collect(&selection, &mut nodes);
    let named = |name: &str| {
        nodes
            .iter()
            .find(|node| node.attr("name") == Some(name))
            .copied()
            .ok_or_else(|| Error::Missing(format!("screen {name}")))
    };
    let menu = |screen: &str, menu: &str| {
        menu_entries(named(screen)?, menu)
            .ok_or_else(|| Error::Missing(format!("{screen}'s {menu} menu")))
    };
    let main_modes = menu("Main Menu", "Mode")?;
    let modes = menu("Single Player", "Mode")?;
    let classes = menu("Class Selection", "Class")?;
    let screens = nodes
        .iter()
        .map(|node| Screen {
            name: node.attr("name").unwrap_or_default().to_string(),
            redirects: node.children_named("Redirect").map(read_redirect).collect(),
        })
        .collect();
    Ok(Flow {
        main_modes,
        modes,
        classes,
        screens,
    })
}

/// Whether a branch condition is about Multiplayer, which SINGLE PLAYER is not.
fn multiplayer(text: &str) -> bool {
    text.contains("Multiplayer")
}

impl Flow {
    fn redirect(&self, screen: &str, name: Option<&str>) -> Option<&Redirect> {
        let screen = self.screens.iter().find(|s| s.name == screen)?;
        screen.redirects.iter().find(|r| match name {
            Some(name) => r.name.as_deref() == Some(name),
            None => r
                .name
                .as_deref()
                .is_none_or(|n| !multiplayer(n) && n != "Go Back"),
        })
    }

    /// The screen name `redirect` goes to for `mode`: the first branch whose
    /// item is the Single Player mode and whose value is `mode`, else the
    /// default. A branch that is about Multiplayer is skipped.
    fn target<'a>(&'a self, screen: &str, redirect: &'a Redirect, mode: &str) -> Option<&'a str> {
        let branch = redirect
            .branches
            .iter()
            .filter(|b| !multiplayer(&b.item) && !multiplayer(&b.equals))
            .find(|b| b.equals == mode);
        let goto = branch
            .map(|b| b.goto.as_str())
            .or(redirect.default.as_deref())?;
        if goto == screen {
            return None;
        }
        // A named redirect inside the same screen is followed, once.
        match self.redirect(screen, Some(goto)) {
            Some(inner) => self.target(screen, inner, mode),
            None => Some(goto),
        }
    }

    /// The screen after `from` for `mode`.
    ///
    /// # Errors
    ///
    /// The redirect names a screen with no [`Step`].
    pub fn next(&self, from: Step, mode: &str) -> Result<Option<Step>> {
        let Some(redirect) = self.redirect(from.screen(), None) else {
            return Ok(None);
        };
        let Some(goto) = self.target(from.screen(), redirect, mode) else {
            return Ok(None);
        };
        Step::from_screen(goto)
            .map(Some)
            .ok_or_else(|| Error::UnknownScreen(goto.to_string()))
    }

    /// Every screen `mode` walks after `Single Player`, up to and including
    /// `Launch Game`.
    ///
    /// # Errors
    ///
    /// A redirect names an unknown screen, or the chain does not end.
    pub fn route(&self, mode: &str) -> Result<Vec<Step>> {
        let mut out = Vec::new();
        let mut at = Step::Mode;
        while at != Step::Launch {
            at = self
                .next(at, mode)?
                .ok_or_else(|| Error::Missing(format!("a redirect after {}", at.screen())))?;
            out.push(at);
            if out.len() > 16 {
                return Err(Error::Missing("an end to the chain".to_string()));
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
