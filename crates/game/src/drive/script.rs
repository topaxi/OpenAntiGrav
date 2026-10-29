//! The script grammar of `--drive` / `--drive-file`.
//!
//! Steps are separated by newlines or `;`; `#` starts a comment. Words are separated by
//! spaces and a word in double quotes may contain them (`select "LAUNCH RACE"`).
//!
//! ```text
//! boot                             tap Start and Cross through the disc's own front end until our menus are up
//! until stage=menu within 3000     wait for a state; fails with the last state after N ticks
//! expect page=race                 the state must hold right now
//! select <row>                     move focus to a row by id, setting key, label or #index
//! set race.mode=time_trial         select that row, then step its value until it reads so
//! press cross down                 tap each button in turn
//! ticks 120                        let this many ticks pass
//! screenshot out.png               draw the next frame offscreen into a PNG
//! quit                             end the run (reaching the end of the script does too)
//! ```
//!
//! **Chosen, not measured:** the grammar and the default timeout.

use std::path::PathBuf;

use oag_gameplay::input::Button;

/// How many ticks a waiting step is given when it names none: thirty seconds of game time.
///
/// Ticks spent on a loading screen do not count - those are bounded by the wall clock - so a
/// slow disc read cannot spend a step's budget while the game is doing exactly what it should.
pub const DEFAULT_WITHIN: u32 = 1800;

/// One `key=value` (or `key!=value`) test on the reported state. See [`State::get`](crate::drive::State::get).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cond {
    /// The state key: `stage`, `page`, `screen`, `focus`, `race.track`, and the rest `State::get` names.
    pub key: String,
    /// The text it is compared with, case-insensitively.
    pub value: String,
    /// `!=` rather than `=`.
    pub negated: bool,
}

impl Cond {
    fn parse(word: &str) -> Result<Self, String> {
        let (key, value, negated) = match word.split_once("!=") {
            Some((key, value)) => (key, value, true),
            None => {
                let (key, value) = word
                    .split_once('=')
                    .ok_or_else(|| format!("{word:?} is not key=value"))?;
                (key, value, false)
            }
        };
        if !super::State::is_key(key) {
            return Err(format!(
                "{key:?} is not a state key (stage, frontend, page, screen, screen.id, title, focus, race.*)"
            ));
        }
        Ok(Self {
            key: key.to_string(),
            value: value.to_string(),
            negated,
        })
    }

    /// Whether `held` (what the state says for [`Self::key`]) satisfies this: any of them equals
    /// the value, or for `!=` none does.
    #[must_use]
    pub fn holds(&self, held: &[String]) -> bool {
        let equal = held
            .iter()
            .any(|held| held.eq_ignore_ascii_case(&self.value));
        equal != self.negated
    }
}

impl std::fmt::Display for Cond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.key,
            if self.negated { "!=" } else { "=" },
            self.value
        )
    }
}

/// What a run is told to do, one entry of a [`Script`].
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// Tap Start and Cross in turn while the disc's own front end (the logo, the language list)
    /// is up, until our menus open. Its screens are the disc's and carry no rows to `select`.
    Boot { within: u32 },
    /// Wait until every condition holds.
    Until { conds: Vec<Cond>, within: u32 },
    /// Every condition must hold now.
    Expect { conds: Vec<Cond> },
    /// Move focus to this row.
    Select { target: String, within: u32 },
    /// Select the row of this setting and step its value to this one.
    Set {
        setting: String,
        value: String,
        within: u32,
    },
    /// Tap each button in turn.
    Press(Vec<Button>),
    /// Let this many ticks pass.
    Ticks(u32),
    /// Write the next frame to this PNG.
    Screenshot(PathBuf),
    /// End the run.
    Quit,
}

impl std::fmt::Display for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let list = |conds: &[Cond]| {
            conds
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        };
        match self {
            Self::Boot { within } => write!(f, "boot within {within}"),
            Self::Until { conds, within } => write!(f, "until {} within {within}", list(conds)),
            Self::Expect { conds } => write!(f, "expect {}", list(conds)),
            Self::Select { target, within } => write!(f, "select {target:?} within {within}"),
            Self::Set {
                setting,
                value,
                within,
            } => write!(f, "set {setting}={value} within {within}"),
            Self::Press(buttons) => write!(f, "press {buttons:?}"),
            Self::Ticks(n) => write!(f, "ticks {n}"),
            Self::Screenshot(path) => write!(f, "screenshot {}", path.display()),
            Self::Quit => f.write_str("quit"),
        }
    }
}

/// A parsed script.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Script {
    /// The steps, in order, with the 1-based line they came from.
    pub steps: Vec<(usize, Step)>,
}

fn words(statement: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let (mut quoted, mut started) = (false, false);
    for c in statement.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    out.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            c => {
                word.push(c);
                started = true;
            }
        }
    }
    if quoted {
        return Err(format!("{statement:?}: a quote is never closed"));
    }
    if started {
        out.push(word);
    }
    Ok(out)
}

/// Splits a trailing `within N` off a statement's words.
fn within(words: &mut Vec<String>) -> Result<u32, String> {
    if words.len() >= 2 && words[words.len() - 2] == "within" {
        let n = words.pop().unwrap_or_default();
        words.pop();
        return n
            .parse()
            .map_err(|_| format!("within {n:?}: not a number of ticks"));
    }
    Ok(DEFAULT_WITHIN)
}

impl Script {
    /// Parses `text`.
    ///
    /// # Errors
    ///
    /// Names the first statement that is not a step and why.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut steps = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let line_no = index + 1;
            let line = line.split('#').next().unwrap_or_default();
            for statement in line.split(';').map(str::trim).filter(|s| !s.is_empty()) {
                let step = Self::statement(statement)
                    .map_err(|why| format!("line {line_no}: {statement:?}: {why}"))?;
                steps.push((line_no, step));
            }
        }
        Ok(Self { steps })
    }

    fn statement(statement: &str) -> Result<Step, String> {
        let mut words = words(statement)?;
        if words.is_empty() {
            return Err("empty".into());
        }
        let verb = words.remove(0).to_ascii_lowercase();
        let conds = |words: &[String]| -> Result<Vec<Cond>, String> {
            if words.is_empty() {
                return Err("needs key=value".into());
            }
            words.iter().map(|w| Cond::parse(w)).collect()
        };
        Ok(match verb.as_str() {
            "boot" => {
                let within = within(&mut words)?;
                if !words.is_empty() {
                    return Err("boot takes only `within N`".into());
                }
                Step::Boot { within }
            }
            "until" => {
                let within = within(&mut words)?;
                Step::Until {
                    conds: conds(&words)?,
                    within,
                }
            }
            "expect" => Step::Expect {
                conds: conds(&words)?,
            },
            "select" => {
                let within = within(&mut words)?;
                match words.as_slice() {
                    [target] => Step::Select {
                        target: target.clone(),
                        within,
                    },
                    _ => return Err("select takes one row (quote it if it has spaces)".into()),
                }
            }
            "set" => {
                let within = within(&mut words)?;
                let [pair] = words.as_slice() else {
                    return Err("set takes one setting=value".into());
                };
                let Some((key, value)) = pair.split_once('=').filter(|(key, _)| !key.is_empty())
                else {
                    return Err(format!("{pair:?} is not setting=value"));
                };
                let (key, value) = (key.to_string(), value.to_string());
                Step::Set {
                    setting: key,
                    value,
                    within,
                }
            }
            "press" => {
                if words.is_empty() {
                    return Err("press needs a button".into());
                }
                let buttons = words
                    .iter()
                    .map(|w| {
                        oag_core::buttons::button_from_name(w)
                            .ok_or_else(|| format!("{w:?} is not a button"))
                    })
                    .collect::<Result<_, _>>()?;
                Step::Press(buttons)
            }
            "ticks" => match words.as_slice() {
                [n] => Step::Ticks(n.parse().map_err(|_| format!("{n:?} is not a number"))?),
                _ => return Err("ticks takes one number".into()),
            },
            "screenshot" => match words.as_slice() {
                [path] => Step::Screenshot(PathBuf::from(path)),
                _ => return Err("screenshot takes one path".into()),
            },
            "quit" => Step::Quit,
            other => return Err(format!("unknown step {other:?}")),
        })
    }
}
