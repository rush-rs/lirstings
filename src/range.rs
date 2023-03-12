use std::{ops::Add, str::FromStr};

use anyhow::{bail, Context};
use regex::Regex;

#[derive(Debug, Clone, Copy, Hash, Default)]
pub struct Range {
    pub inline: bool,
    pub indent_offset: Offset,
    pub start: usize,
    pub end: usize,
    pub start_col: Option<usize>,
    pub end_col: Option<usize>,
}

impl FromStr for Range {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let regex =
            Regex::from_str(r"^ *([+-]\d+|_)? *(\d+)(?::(\d+))? *- *(\d+)(?::(\d+))? *$").unwrap();
        let groups = regex
            .captures(s)
            .with_context(|| "unable to parse range literal")?;

        let inline = groups
            .get(1)
            .map_or(false, |capture| capture.as_str() == "_");
        let indent_offset = match groups.get(1).map(|capture| capture.as_str()) {
            None | Some("_") => Offset::None,
            Some(num) if num.starts_with('+') => Offset::Positive(
                num[1..]
                    .parse::<usize>()
                    .with_context(|| "failed to parse indent offset")?,
            ),
            Some(num) => Offset::Negative(
                num[1..]
                    .parse::<usize>()
                    .with_context(|| "failed to parse indent offset")?,
            ),
        };
        let start = groups[2]
            .parse::<usize>()
            .with_context(|| "failed to parse range start literal")?
            .checked_sub(1)
            .with_context(|| "line number 0 does not exist")?;
        let end = groups[4]
            .parse::<usize>()
            .with_context(|| "failed to parse range end literal")?
            .checked_sub(1)
            .with_context(|| "line number 0 does not exist")?;
        let start_col = match groups.get(3) {
            Some(grp) => Some(
                grp.as_str()
                    .parse::<usize>()
                    .with_context(|| "failed to parse range start column literal")?,
            ),
            None => None,
        };
        let end_col = match groups.get(5) {
            Some(grp) => Some(
                grp.as_str()
                    .parse::<usize>()
                    .with_context(|| "failed to parse range end column literal")?,
            ),
            None => None,
        };
        if start > end
            || (start == end
                && matches!((start_col, end_col), (Some(start), Some(end)) if start > end))
        {
            bail!("range start is higher than range end");
        }
        Ok(Self {
            inline,
            indent_offset,
            start,
            end,
            start_col,
            end_col,
        })
    }
}

#[derive(Debug, Clone, Copy, Hash, Default)]
pub enum Offset {
    #[default]
    None,
    Positive(usize),
    Negative(usize),
}

impl Add<usize> for Offset {
    type Output = usize;

    fn add(self, rhs: usize) -> Self::Output {
        match self {
            Offset::None => rhs,
            Offset::Positive(num) => rhs + num,
            Offset::Negative(num) => rhs - num,
        }
    }
}
