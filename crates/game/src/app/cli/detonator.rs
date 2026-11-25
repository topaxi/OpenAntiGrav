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
    /// (`S,T;...`: `m+0xf4` and `m+0xf8`, same order), ours having no history of the music.
    #[arg(
        long = "detonator-mine-envelopes",
        value_name = "S,T;...",
        value_parser = parse_mine_envelopes,
        requires = "mines"
    )]
    pub(crate) mine_envelopes: Option<MineEnvelopes>,

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
pub(crate) struct MineEnvelopes(pub(crate) Vec<(f32, f32)>);

fn parse_mine_envelopes(text: &str) -> Result<MineEnvelopes, String> {
    let number = |n: &str| {
        n.trim()
            .parse::<f32>()
            .map_err(|_| format!("`{n}` is not a number"))
    };
    text.split(';')
        .map(|pair| match pair.split_once(',') {
            Some((s, t)) => Ok((number(s)?, number(t)?)),
            None => Err(format!("`{pair}` is not S,T")),
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
