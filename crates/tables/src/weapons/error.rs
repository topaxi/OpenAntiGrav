//! What can go wrong reading a weapon table. Its own file since 2026-09-15,
//! when the Disruptor's decode put `weapons.rs` four lines over the
//! 1,000-line ratchet; nothing about the type changed.

use std::fmt;

use crate::fexml;

/// What can go wrong reading a weapon table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The blob is not text, or the shortened-XML expansion failed.
    Fexml(fexml::Error),
    /// No `<WeaponStats>` root.
    MissingRoot,
    /// A required element is absent.
    MissingElement {
        /// The element that should have held it.
        parent: &'static str,
        /// What was missing.
        element: &'static str,
    },
    /// A required attribute is absent.
    MissingAttribute {
        element: &'static str,
        attribute: &'static str,
    },
    /// An attribute is present and is not a finite number.
    NotANumber {
        element: &'static str,
        attribute: &'static str,
        value: String,
    },
}

impl From<fexml::Error> for Error {
    fn from(e: fexml::Error) -> Self {
        Self::Fexml(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fexml(e) => write!(f, "{e}"),
            Self::MissingRoot => write!(f, "no <WeaponStats> element"),
            Self::MissingElement { parent, element } => {
                write!(f, "<{parent}> has no <{element}>")
            }
            Self::MissingAttribute { element, attribute } => {
                write!(f, "<{element}> has no {attribute} attribute")
            }
            Self::NotANumber {
                element,
                attribute,
                value,
            } => write!(f, "<{element} {attribute}=\"{value}\"> is not a number"),
        }
    }
}

impl std::error::Error for Error {}
