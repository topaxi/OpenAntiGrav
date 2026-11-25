//! `--benchmark` and `--preset`: the in-game benchmark as a run of its own.

/// The benchmark as a command line asks for it.
#[derive(clap::Args, Debug, Default)]
pub(crate) struct BenchmarkArgs {
    /// Fly the benchmark: one fixed race (Single Race, seed 1, the title's default circuit, the
    /// full field, the player on the autopilot, the title's laps) drawn in the window with your
    /// settings, ending on a stats screen and a result file in the user data directory that
    /// `just bench-compare` reads. The title is the first argument: a disc image or folder, or a
    /// title's short name (`pulse`, `pure`, `hd`, `2048`, `omega`) for the first of that title on
    /// the search path; left out, the disc the run would otherwise open. See
    /// docs/tools/in-game-benchmark.md.
    #[arg(long, conflicts_with_all = ["race", "launcher", "demo", "wallpaper", "dry_run", "screenshot"])]
    pub(crate) benchmark: bool,

    /// With `--benchmark`: draw with this preset of the title's `benchmark.toml` instead of your
    /// own graphics settings. Nothing is written back to your settings.
    #[arg(long, value_name = "NAME", requires = "benchmark")]
    pub(crate) preset: Option<String>,
}

impl BenchmarkArgs {
    /// What the run asks for, when it is a benchmark.
    pub(crate) fn request(&self) -> Option<crate::benchmark::Request> {
        self.benchmark.then(|| crate::benchmark::Request {
            preset: self.preset.clone(),
        })
    }
}

/// With `--benchmark`, a first argument that is a title's short name and no path names the first
/// playable disc of that title on the search path (Europe before the USA, as every search is).
///
/// # Errors
///
/// The word is a title's short name and none of that title is on the search path.
pub(crate) fn resolve_title(
    benchmark: &BenchmarkArgs,
    source: &mut Option<String>,
) -> anyhow::Result<()> {
    let Some(word) = source.as_deref().filter(|_| benchmark.benchmark) else {
        return Ok(());
    };
    if std::path::Path::new(word).exists() || !crate::benchmark::TITLE_KEYS.contains(&word) {
        return Ok(());
    }
    let survey = crate::launcher::survey(&oag_source::source::candidates());
    let found = survey
        .iter()
        .find(|row| row.is_playable() && crate::benchmark::title_key(row.title()) == word);
    match found {
        Some(row) => {
            log::info!("{word}: {}", row.source);
            *source = Some(row.source.clone());
            Ok(())
        }
        None => anyhow::bail!(
            "--benchmark {word}: no playable {word} disc or folder on the search path ({} found)",
            survey.len()
        ),
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;
    use crate::app::cli::Cli;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("oag-game").chain(args.iter().copied()))
    }

    #[test]
    fn a_benchmark_takes_a_title_and_a_preset() {
        let cli = parse(&["--benchmark", "pulse", "--preset", "low"]).unwrap();
        assert_eq!(cli.source.as_deref(), Some("pulse"));
        assert_eq!(
            cli.benchmark.request(),
            Some(crate::benchmark::Request {
                preset: Some("low".into())
            })
        );
        assert_eq!(parse(&["x.chd"]).unwrap().benchmark.request(), None);
    }

    #[test]
    fn a_preset_needs_a_benchmark_and_a_benchmark_is_not_a_race() {
        assert!(parse(&["--preset", "low"]).is_err());
        assert!(parse(&["--benchmark", "--race"]).is_err());
        assert!(parse(&["--benchmark", "--demo"]).is_err());
    }

    #[test]
    fn a_path_or_another_word_is_left_alone() {
        let benchmark = BenchmarkArgs {
            benchmark: true,
            preset: None,
        };
        for word in ["data/images/pulse-psp-eu.chd", "somewhere", "Cargo.toml"] {
            let mut source = Some(word.to_string());
            resolve_title(&benchmark, &mut source).unwrap();
            assert_eq!(source.as_deref(), Some(word));
        }
        let mut off = Some("pulse".to_string());
        resolve_title(&BenchmarkArgs::default(), &mut off).unwrap();
        assert_eq!(off.as_deref(), Some("pulse"));
    }
}
