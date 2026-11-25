//! The value types and parsers of the Detonator mine flags (`--detonator-mines`,
//! `--detonator-mine-envelopes`).

/// `--detonator-mines`: `X,Y,Z` triples, semicolon separated.
/// The positions of `--detonator-mines`, one value to clap (a bare `Vec` would read as repeated values).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MinePositions(pub(crate) Vec<oag_core::math::Vec3>);

/// The envelopes of `--detonator-mine-envelopes`, one value to clap.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MineEnvelopes(pub(crate) Vec<(f32, f32)>);

pub(super) fn parse_mine_envelopes(text: &str) -> Result<MineEnvelopes, String> {
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

pub(super) fn parse_detonator_mines(text: &str) -> Result<MinePositions, String> {
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
