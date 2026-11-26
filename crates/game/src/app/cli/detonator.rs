//! The Detonator comparison inputs, flattened into [`super::Cli`] so `cli.rs` stays under the size
//! ratchet. Each carries the original's state at a captured frame into ours.

/// `--detonator-*`.
#[derive(clap::Args, Debug)]
pub(crate) struct DetonatorArgs {
    /// Comparison input: lay the Detonator opening stage's mines at these world positions
    /// (`X,Y,Z;X,Y,Z;...`, the original's mine objects read in the frame's pause), each on the
    /// survey lane nearest to it, in place of the seeded layout.
    #[arg(
        long = "detonator-mines",
        value_name = "X,Y,Z;...",
        value_parser = parse_detonator_mines
    )]
    pub(crate) mines: Option<MinePositions>,

    /// Comparison input: hold each `--detonator-mines` mine's music envelope at the original's
    /// (`S,T[,P];...`: `m+0xf4`, `m+0xf8` and the visualiser's `PitchOffset` `m+0x108`, `0` when
    /// absent; same order), ours having no history of the music.
    #[arg(
        long = "detonator-mine-envelopes",
        value_name = "S,T[,P];...",
        value_parser = parse_mine_envelopes,
        requires = "mines"
    )]
    pub(crate) mine_envelopes: Option<MineEnvelopes>,

    /// Comparison input: hold each `--detonator-mines` mine's age at the original's (`A;...`, seconds,
    /// `m+0xc0`; same order), which turns it about its up axis. Ours' own count from its laying.
    #[arg(
        long = "detonator-mine-ages",
        value_name = "A;...",
        value_parser = parse_mine_ages,
        requires = "mines"
    )]
    pub(crate) mine_ages: Option<MineAges>,

    /// Comparison input: start the Detonator EMP at this many charges (the gauge they give), as
    /// the original's charge was written for the capture it is compared with.
    #[arg(long = "detonator-emp-charge", value_name = "N")]
    pub(crate) emp_charge: Option<u32>,

    /// Comparison input: the speed the craft's cannon gauge gate reads, the original's at the
    /// captured frame. `--pose-hold` holds ours at rest, where the gauge never updates and keeps
    /// the bind's full red.
    #[arg(long = "detonator-gauge-speed", value_name = "SPEED")]
    pub(crate) gauge_speed: Option<f32>,
}

/// `--detonator-mines`: `X,Y,Z` triples, semicolon separated.
/// The positions of `--detonator-mines`, one value to clap (a bare `Vec` would read as repeated values).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MinePositions(pub(crate) Vec<oag_core::math::Vec3>);

/// The envelopes of `--detonator-mine-envelopes`, one value to clap.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MineEnvelopes(pub(crate) Vec<(f32, f32, f32)>);

/// The ages of `--detonator-mine-ages`, one value to clap.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MineAges(pub(crate) Vec<f32>);

fn parse_mine_ages(text: &str) -> Result<MineAges, String> {
    text.split(';')
        .map(|n| {
            n.trim()
                .parse::<f32>()
                .map_err(|_| format!("`{n}` is not a number"))
        })
        .collect::<Result<Vec<_>, String>>()
        .map(MineAges)
}

fn parse_mine_envelopes(text: &str) -> Result<MineEnvelopes, String> {
    let number = |n: &str| {
        n.trim()
            .parse::<f32>()
            .map_err(|_| format!("`{n}` is not a number"))
    };
    text.split(';')
        .map(|mine| match mine.split(',').collect::<Vec<_>>()[..] {
            [s, t] => Ok((number(s)?, number(t)?, 0.0)),
            [s, t, p] => Ok((number(s)?, number(t)?, number(p)?)),
            _ => Err(format!("`{mine}` is not S,T or S,T,P")),
        })
        .collect::<Result<Vec<_>, String>>()
        .map(MineEnvelopes)
}

fn parse_detonator_mines(text: &str) -> Result<MinePositions, String> {
    text.split(';')
        .map(|triple| {
            let parts = triple
                .split(',')
                .map(|n| {
                    n.trim()
                        .parse::<f32>()
                        .map_err(|_| format!("`{n}` is not a number"))
                })
                .collect::<Result<Vec<f32>, String>>()?;
            match parts[..] {
                [x, y, z] => Ok(oag_core::math::Vec3::new(x, y, z)),
                _ => Err(format!("`{triple}` is not X,Y,Z")),
            }
        })
        .collect::<Result<Vec<_>, String>>()
        .map(MinePositions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mine_envelope_takes_an_optional_pitch_offset() {
        assert_eq!(
            parse_mine_envelopes("0.5,0.75;0.25,1,8.375"),
            Ok(MineEnvelopes(vec![(0.5, 0.75, 0.0), (0.25, 1.0, 8.375)]))
        );
        assert!(parse_mine_envelopes("0.5").is_err());
        assert!(parse_mine_envelopes("0.5,1,2,3").is_err());
    }

    #[test]
    fn mine_ages_are_one_number_a_mine() {
        assert_eq!(
            parse_mine_ages("55.7002;55.7002"),
            Ok(MineAges(vec![55.7002, 55.7002]))
        );
        assert!(parse_mine_ages("55.7,1").is_err());
    }
}
