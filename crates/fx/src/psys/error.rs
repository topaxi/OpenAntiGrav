//! What [`super::Effect::parse`] refuses a `.pob` for.

use super::MAX_EMITTER_STATES;
use oag_pob as pob;

/// Something in a `.pob` this module cannot play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The blob would not parse at all.
    Format(pob::Error),
    /// An emitter's render-mode index is past the executable's eight-entry
    /// blend table, so it has no draw handler. Refused rather than guessed:
    /// a wrong guess here draws nothing, which looks exactly like the
    /// effect never being triggered.
    UnknownDrawClass {
        /// The emitter's name.
        emitter: String,
        /// The index found.
        render_mode: u32,
    },
    /// An emitter's blend class is not one the GE state selector switches
    /// on.
    UnknownBlendClass {
        /// The emitter's name.
        emitter: String,
        /// The class found.
        blend_class: u32,
    },
    /// A channel block's mode has no traced consumer.
    UnknownChannelMode {
        /// The emitter's name.
        emitter: String,
        /// The raw mode word.
        mode: u32,
    },
    /// More emitters than [`MAX_EMITTER_STATES`] can ever run.
    TooManyEmitters {
        /// How many the file holds.
        count: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Format(error) => write!(f, "{error}"),
            Self::UnknownDrawClass {
                emitter,
                render_mode,
            } => write!(f, "{emitter}: render mode {render_mode} has no draw class"),
            Self::UnknownBlendClass {
                emitter,
                blend_class,
            } => write!(f, "{emitter}: blend class {blend_class} is not dispatched"),
            Self::UnknownChannelMode { emitter, mode } => {
                write!(f, "{emitter}: channel mode {mode} has no consumer")
            }
            Self::TooManyEmitters { count } => {
                write!(f, "{count} emitters, more than {MAX_EMITTER_STATES}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<pob::Error> for Error {
    fn from(error: pob::Error) -> Self {
        Self::Format(error)
    }
}
