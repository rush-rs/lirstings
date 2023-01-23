// TODO: write README.md
use std::{
    fs,
    io::{self, Write},
    iter,
    path::{Path, PathBuf},
    process,
    str::FromStr,
};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

use cache::{CACHE_FILE_PATH, CACHE_SKIP_MESSAGE, CACHE_WRITE_MESSAGE};
use config::CONFIG_FILE_PATH;

use crate::output::Output;

mod ansi;
mod cache;
mod config;
mod output;
mod theme;
mod ts;

#[derive(Parser, Hash)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(short = 'x', long, global = true, default_value = "")]
    fancyvrb_args: String,

    #[command(subcommand)]
    subcommand: Command,
}

#[derive(Subcommand, Hash)]
pub enum Command {
    #[command(visible_alias = "ts")]
    TreeSitter {
        file: PathBuf,

        #[arg(short, long)]
        raw: bool,

        #[arg(long)]
        raw_queries: bool,

        #[arg(short = 'R', long, value_delimiter = ',')]
        ranges: Vec<Range>,

        #[arg(short, long)]
        filename_strip_prefix: Option<PathBuf>,
    },
    Inline {
        file_ext: String,
        code: Vec<String>,
    },
    Ansi {
        file: PathBuf,
    },
    #[command(visible_aliases = ["tex", "include", "include-tex"])]
    TexInclude,
}

#[derive(Debug, Clone, Copy, Hash, Default)]
pub struct Range {
    start: usize,
    end: usize,
}

impl FromStr for Range {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (start, end) = s
            .split_once('-')
            .with_context(|| "no `-` found in range literal")?;
        let start = start
            .trim()
            .parse::<usize>()
            .with_context(|| "failed to parse range start literal")?
            .checked_sub(1)
            .with_context(|| "line number 0 does not exist")?;
        let end = end
            .trim()
            .parse::<usize>()
            .with_context(|| "failed to parse range end literal")?
            .checked_sub(1)
            .with_context(|| "line number 0 does not exist")?;
        if start > end {
            bail!("range start is higher than range end");
        }
        Ok(Self { start, end })
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config = config::read()
        .with_context(|| format!("could not read or create config file at `{CONFIG_FILE_PATH}`"))?
        .unwrap_or_else(|| {
            eprintln!("New configuration file was created at `{CONFIG_FILE_PATH}`");
            process::exit(200)
        });

    let mut cache = cache::read()
        .with_context(|| format!("could not read or create cache file at `{CACHE_FILE_PATH}`"))?;

    let (mut code, line_numbers) = match &cli.subcommand {
        Command::TexInclude => {
            print(include_str!("./lirstings.tex"));
            return Ok(());
        }
        Command::TreeSitter { file, ranges, .. } if ranges.is_empty() => (read_file(file)?, None),
        Command::Ansi { file } => (read_file(file)?, None),
        Command::TreeSitter { file, ranges, .. } => {
            let raw = read_file(file)?;
            let lines: Vec<_> = raw.lines().collect();
            let mut code = String::new();
            let mut line_numbers = vec![];
            let mut prev_range = Range::default();
            for (index, range) in ranges.iter().enumerate() {
                if index != 0 {
                    // take the larger indent from...
                    let indent = usize::max(
                        // ...the last line of the previous range and...
                        lines[prev_range.end]
                            .chars()
                            .take_while(|char| *char == ' ')
                            .count(),
                        // ...the first line of the following range.
                        lines[range.start]
                            .chars()
                            .take_while(|char| *char == ' ')
                            .count(),
                    );
                    code += &format!("{}// ...\n", " ".repeat(indent));
                    line_numbers.push(0..=0);
                }
                code += &lines
                    .get(range.start..=range.end)
                    .with_context(|| "range out of bounds for input file")?
                    .join("\n");
                code += "\n";
                line_numbers.push(range.start + 1..=range.end + 1);
                prev_range = *range;
            }
            (code, Some(line_numbers))
        }
        Command::Inline { code, .. } => (code.join(" "), None),
    };
    let gobble = code
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.chars().take_while(|char| *char == ' ').count())
        .min()
        .unwrap_or(0);
    if gobble > 0 {
        code = code
            .lines()
            .flat_map(|line| line.chars().skip(gobble).chain(iter::once('\n')))
            .collect::<String>();
    }
    code.truncate(code.trim_end_matches('\n').len());

    let (output, hash) = match &cli.subcommand {
        Command::TexInclude => unreachable!("`tex-include` subcommand immediately returns"),
        Command::Ansi { .. } => {
            let hash = cache::hash((&cli, &code, None::<String>));
            if let Some(cached) = cache.get_cached(hash) {
                eprintln!("{CACHE_SKIP_MESSAGE}");
                print(cached);
                return Ok(());
            }
            (
                ansi::highlight(code, &cli.fancyvrb_args, &config.ansi_colors),
                hash,
            )
        }
        Command::TreeSitter {
            raw,
            file,
            filename_strip_prefix,
            ..
        } => {
            let filename = match filename_strip_prefix {
                Some(prefix) => Some(
                    file.strip_prefix(prefix)
                        .with_context(|| "failed to strip prefix from filename")?
                        .to_string_lossy()
                        .into_owned(),
                ),
                None => None,
            };
            if *raw {
                let hash = cache::hash((&cli, &code, None::<String>));
                if let Some(cached) = cache.get_cached(hash) {
                    eprintln!("{CACHE_SKIP_MESSAGE}");
                    print(cached);
                    return Ok(());
                }
                let mut output = match line_numbers {
                    Some(numbers) => Output::new(
                        numbers.into_iter().flatten(),
                        false,
                        &cli.fancyvrb_args,
                        filename,
                    ),
                    None => Output::new(1.., false, &cli.fancyvrb_args, filename),
                };
                output.push_str(&code.replace('{', "×{").replace('}', "×}"));
                (output.finish(), hash)
            } else {
                let settings = ts::get_settings(config, &cli.subcommand)?;
                let hash_query = settings.highlights_query.clone()
                    + &settings.injection_query
                    + &settings.locals_query;
                let hash = cache::hash((&cli, &code, Some(hash_query)));
                if let Some(cached) = cache.get_cached(hash) {
                    eprintln!("{CACHE_SKIP_MESSAGE}");
                    print(cached);
                    return Ok(());
                }
                (
                    ts::highlight(&code, line_numbers, &cli, settings, filename)?,
                    hash,
                )
            }
        }
        Command::Inline { .. } => {
            let settings = ts::get_settings(config, &cli.subcommand)?;
            let hash_query = settings.highlights_query.clone()
                + &settings.injection_query
                + &settings.locals_query;
            let hash = cache::hash((&cli, &code, Some(hash_query)));
            if let Some(cached) = cache.get_cached(hash) {
                eprintln!("{CACHE_SKIP_MESSAGE}");
                print(cached);
                return Ok(());
            }
            (
                ts::highlight(&code, line_numbers, &cli, settings, None)?,
                hash,
            )
        }
    };
    print(&output);
    eprintln!("{CACHE_WRITE_MESSAGE}");
    cache
        .set_entry(hash, output)
        .with_context(|| "could not update cache file")?;

    Ok(())
}

#[inline]
fn print(input: &str) {
    let mut stdout = io::stdout().lock();
    _ = stdout.write_all(input.as_bytes());
}

fn read_file(path: &Path) -> Result<String> {
    let raw_code = fs::read_to_string(path)
        .with_context(|| format!("Could not read input file at `{}`", path.to_string_lossy()))?;
    Ok(raw_code)
}
